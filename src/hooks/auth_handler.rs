//! Hook handler for MQTT client authentication.
//!
//! Called by rmqtt when a client connects. Delegates to the
//! `Authenticator` trait and maps the result to an rmqtt hook response.
//!
//! # Flow
//! 1. Extract username/password from the CONNECT packet.
//! 2. Call `authenticator.authenticate(username, password)`.
//! 3. On success, store the `ClientIdentity` in the shared identity store.
//! 4. On denial, reject the connection with the appropriate CONNACK code.

use std::sync::Arc;

use async_trait::async_trait;
use dashmap::DashMap;
use rmqtt::hook::{Handler, HookResult, Parameter, ReturnType};
use rmqtt::types::AuthResult;

use crate::auth::{AuthOutcome, Authenticator, MeshcoreAuthenticator};
use crate::types::ClientIdentity;

/// Shared store mapping client IDs to their authenticated identities.
///
/// Populated during authentication and read by publish/subscribe/delivery
/// handlers for ACL checks and payload filtering.
pub type IdentityStore = Arc<DashMap<String, ClientIdentity>>;

pub struct AuthHandler {
    authenticator: Arc<MeshcoreAuthenticator>,
    identity_store: IdentityStore,
}

impl AuthHandler {
    pub fn new(authenticator: Arc<MeshcoreAuthenticator>, identity_store: IdentityStore) -> Self {
        Self {
            authenticator,
            identity_store,
        }
    }
}

#[async_trait]
impl Handler for AuthHandler {
    async fn hook(&self, param: &Parameter, acc: Option<HookResult>) -> ReturnType {
        match param {
            Parameter::ClientAuthenticate(connect_info) => {
                let username = match connect_info.username() {
                    Some(u) => u.to_string(),
                    None => {
                        return (
                            false,
                            Some(HookResult::AuthResult(AuthResult::BadUsernameOrPassword)),
                        );
                    }
                };
                let password = match connect_info.password() {
                    Some(p) => String::from_utf8_lossy(p).to_string(),
                    None => {
                        return (
                            false,
                            Some(HookResult::AuthResult(AuthResult::BadUsernameOrPassword)),
                        );
                    }
                };

                let outcome = self.authenticator.authenticate(&username, &password);
                let client_id = connect_info.id().client_id.to_string();

                match outcome {
                    AuthOutcome::Publisher { public_key } => {
                        tracing::info!(target: "access",
                            event = "auth",
                            client_id = %client_id,
                            identity_type = "publisher",
                            public_key = %public_key,
                            outcome = "allow",
                        );
                        self.identity_store
                            .insert(client_id, ClientIdentity::Publisher { public_key });
                        (
                            false,
                            Some(HookResult::AuthResult(AuthResult::Allow(false, None))),
                        )
                    }
                    AuthOutcome::Subscriber { username, role } => {
                        tracing::info!(target: "access",
                            event = "auth",
                            client_id = %client_id,
                            identity_type = "subscriber",
                            username = %username,
                            role = %role,
                            outcome = "allow",
                        );
                        self.identity_store
                            .insert(client_id, ClientIdentity::Subscriber { username, role });
                        (
                            false,
                            Some(HookResult::AuthResult(AuthResult::Allow(false, None))),
                        )
                    }
                    AuthOutcome::Denied { reason } => {
                        tracing::info!(target: "access",
                            event = "auth",
                            client_id = %client_id,
                            identity_type = "unknown",
                            outcome = "deny",
                            reason = %reason,
                        );
                        tracing::warn!("Auth denied for {}: {}", client_id, reason);
                        (
                            false,
                            Some(HookResult::AuthResult(AuthResult::BadUsernameOrPassword)),
                        )
                    }
                }
            }
            _ => (true, acc),
        }
    }
}
