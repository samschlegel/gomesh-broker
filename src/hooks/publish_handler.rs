//! Hook handler for MQTT publish authorization.
//!
//! Called by rmqtt when a client publishes a message. Delegates to the
//! `Authorizer` trait to check whether the client is allowed to publish
//! to the given topic.
//!
//! # Flow
//! 1. Retrieve the `ClientIdentity` from the session state.
//! 2. Call `authorizer.check(identity, TopicAction::Publish, topic)`.
//! 3. On `Allow` or `AllowStripRetain`, permit the publish (stripping retain if needed).
//! 4. On `Deny`, drop the message and optionally disconnect the client.

use std::sync::Arc;

use async_trait::async_trait;
use rmqtt::hook::{Handler, HookResult, Parameter, ReturnType};
use rmqtt::types::PublishAclResult;

use crate::authz::{AclDecision, Authorizer, MeshcoreAuthorizer};
use crate::hooks::auth_handler::IdentityStore;
use crate::types::{ClientIdentity, TopicAction};

/// Handles publish ACL checks via `MessagePublishCheckAcl`.
pub struct PublishAclHandler {
    authorizer: Arc<MeshcoreAuthorizer>,
    identity_store: IdentityStore,
}

impl PublishAclHandler {
    pub fn new(authorizer: Arc<MeshcoreAuthorizer>, identity_store: IdentityStore) -> Self {
        Self {
            authorizer,
            identity_store,
        }
    }
}

#[async_trait]
impl Handler for PublishAclHandler {
    async fn hook(&self, param: &Parameter, acc: Option<HookResult>) -> ReturnType {
        match param {
            Parameter::MessagePublishCheckAcl(session, publish) => {
                let client_id = session.id.client_id.to_string();
                let topic = publish.topic.to_string();

                let result = match self.identity_store.get(&client_id) {
                    Some(identity) => {
                        let decision = self.authorizer.check(&identity, TopicAction::Publish, &topic);
                        match decision {
                            AclDecision::Allow | AclDecision::AllowStripRetain => {
                                PublishAclResult::allow()
                            }
                            AclDecision::Deny { reason } => {
                                log::warn!("Publish denied for client {}: {}", client_id, reason);
                                PublishAclResult::rejected(false, Some(reason.into()))
                            }
                        }
                    }
                    None => {
                        log::warn!("Publish denied: no identity found for client {}", client_id);
                        PublishAclResult::rejected(true, Some("Unknown client identity".into()))
                    }
                };

                (false, Some(HookResult::PublishAclResult(result)))
            }
            _ => (true, acc),
        }
    }
}

/// Handles `MessagePublish` to strip the retain flag for publishers (per ADR 0003).
pub struct RetainStripHandler {
    identity_store: IdentityStore,
}

impl RetainStripHandler {
    pub fn new(identity_store: IdentityStore) -> Self {
        Self { identity_store }
    }
}

#[async_trait]
impl Handler for RetainStripHandler {
    async fn hook(&self, param: &Parameter, acc: Option<HookResult>) -> ReturnType {
        match param {
            Parameter::MessagePublish(Some(session), _from, publish) => {
                let client_id = session.id.client_id.to_string();
                let is_publisher = matches!(
                    self.identity_store.get(&client_id).as_deref(),
                    Some(ClientIdentity::Publisher { .. })
                );

                if is_publisher && publish.retain {
                    let mut modified = (*publish).clone();
                    modified.retain = false;
                    return (true, Some(HookResult::Publish(modified)));
                }

                (true, acc)
            }
            _ => (true, acc),
        }
    }
}
