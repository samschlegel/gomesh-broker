//! Hook handler for MQTT subscribe authorization.
//!
//! Called by rmqtt when a client subscribes to a topic filter. Delegates
//! to the `Authorizer` trait to check whether the client is allowed to
//! subscribe.
//!
//! # Flow
//! 1. Retrieve the `ClientIdentity` from the session state.
//! 2. Call `authorizer.check(identity, TopicAction::Subscribe, topic)`.
//! 3. On `Allow`, grant the subscription.
//! 4. On `Deny`, reject with the appropriate SUBACK reason code.

use std::sync::Arc;

use async_trait::async_trait;
use rmqtt::{
    codec::v5::SubscribeAckReason,
    hook::{Handler, HookResult, Parameter, ReturnType},
    types::SubscribeReturn,
};

use crate::authz::{AclDecision, Authorizer, MeshcoreAuthorizer};
use crate::hooks::auth_handler::IdentityStore;
use crate::types::{ClientIdentity, SubscriberRole, TopicAction};

pub struct SubscribeAclHandler {
    authorizer: Arc<MeshcoreAuthorizer>,
    identity_store: IdentityStore,
}

impl SubscribeAclHandler {
    pub fn new(authorizer: Arc<MeshcoreAuthorizer>, identity_store: IdentityStore) -> Self {
        Self {
            authorizer,
            identity_store,
        }
    }
}

#[async_trait]
impl Handler for SubscribeAclHandler {
    async fn hook(&self, param: &Parameter, acc: Option<HookResult>) -> ReturnType {
        match param {
            Parameter::ClientSubscribeCheckAcl(session, subscribe) => {
                let client_id = session.id.client_id.to_string();
                let topic = subscribe.topic_filter.to_string();

                let result = match self.identity_store.get(&client_id) {
                    Some(identity) => {
                        let decision = self.authorizer.check(
                            &identity,
                            TopicAction::Subscribe,
                            &topic,
                        );
                        match decision {
                            AclDecision::Allow | AclDecision::AllowStripRetain => {
                                log_access_subscribe(&client_id, &identity, &topic, "allow", None);
                                SubscribeReturn::new_success(subscribe.opts.qos(), None)
                            }
                            AclDecision::Deny { reason } => {
                                log_access_subscribe(&client_id, &identity, &topic, "deny", Some(&reason));
                                tracing::warn!(
                                    "Subscribe denied for client {}: {}",
                                    client_id,
                                    reason
                                );
                                SubscribeReturn::new_failure(SubscribeAckReason::NotAuthorized)
                            }
                        }
                    }
                    None => {
                        tracing::info!(target: "access",
                            event = "subscribe",
                            client_id = %client_id,
                            identity_type = "unknown",
                            topic = %topic,
                            outcome = "deny",
                            reason = "no identity found",
                        );
                        tracing::warn!(
                            "Subscribe denied: no identity found for client {}",
                            client_id
                        );
                        SubscribeReturn::new_failure(SubscribeAckReason::NotAuthorized)
                    }
                };

                (false, Some(HookResult::SubscribeAclResult(result)))
            }
            _ => (false, acc),
        }
    }
}

fn log_access_subscribe(
    client_id: &str,
    identity: &ClientIdentity,
    topic: &str,
    outcome: &str,
    reason: Option<&str>,
) {
    match identity {
        ClientIdentity::Publisher { public_key } => {
            tracing::info!(target: "access",
                event = "subscribe",
                client_id = %client_id,
                identity_type = "publisher",
                public_key = %public_key,
                topic = %topic,
                outcome = %outcome,
                reason = reason.unwrap_or(""),
            );
        }
        ClientIdentity::Subscriber { username, role } => {
            tracing::info!(target: "access",
                event = "subscribe",
                client_id = %client_id,
                identity_type = "subscriber",
                username = %username,
                role = %format_role(*role),
                topic = %topic,
                outcome = %outcome,
                reason = reason.unwrap_or(""),
            );
        }
    }
}

fn format_role(role: SubscriberRole) -> &'static str {
    match role {
        SubscriberRole::Full => "full",
        SubscriberRole::Limited => "limited",
        SubscriberRole::Admin => "admin",
    }
}
