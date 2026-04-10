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
use crate::types::TopicAction;

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
                        let decision =
                            self.authorizer
                                .check(&identity, TopicAction::Subscribe, &topic);
                        match decision {
                            AclDecision::Allow | AclDecision::AllowStripRetain => {
                                super::log_access_event(
                                    "subscribe",
                                    &client_id,
                                    &identity,
                                    &topic,
                                    "allow",
                                    None,
                                );
                                SubscribeReturn::new_success(subscribe.opts.qos(), None)
                            }
                            AclDecision::Deny { reason } => {
                                super::log_access_event(
                                    "subscribe",
                                    &client_id,
                                    &identity,
                                    &topic,
                                    "deny",
                                    Some(&reason),
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
                        tracing::error!(
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
