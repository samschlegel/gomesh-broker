//! Hook handler for MQTT client authentication.
//!
//! Called by rmqtt when a client connects. Delegates to the
//! `Authenticator` trait and maps the result to an rmqtt hook response.
//!
//! # Flow
//! 1. Extract username/password from the CONNECT packet.
//! 2. Call `authenticator.authenticate(username, password)`.
//! 3. If the CONNECT carries a Last Will, check its topic with the same
//!    publish rules the client would face for a normal PUBLISH. A Will the
//!    client could not publish by hand is refused here, at connect time.
//! 4. On success, store the `ClientIdentity` in the shared identity store.
//! 5. On denial, reject the connection with the appropriate CONNACK code.
//!
//! # Why the Will is checked here
//! rmqtt publishes a Last Will on the client's behalf when the connection
//! drops without a DISCONNECT. That path does not run the
//! `MessagePublishCheckAcl` hook, so `PublishAclHandler` never sees it.
//! Without this check, any authenticated client could set a Will on another
//! publisher's topic and have the broker deliver it under that identity.
//! Refusing the CONNECT is the only point where we can stop it.

use std::sync::Arc;

use async_trait::async_trait;
use dashmap::DashMap;
use rmqtt::hook::{Handler, HookResult, Parameter, ReturnType};
use rmqtt::types::{AuthResult, ConnectInfo, Publish};

use crate::auth::{AuthOutcome, Authenticator, MeshcoreAuthenticator};
use crate::authz::{AclDecision, Authorizer, MeshcoreAuthorizer};
use crate::types::{ClientIdentity, TopicAction};

/// Shared store mapping client IDs to their authenticated identities.
///
/// Populated during authentication and read by publish/subscribe/delivery
/// handlers for ACL checks and payload filtering.
pub type IdentityStore = Arc<DashMap<String, ClientIdentity>>;

pub struct AuthHandler {
    authenticator: Arc<MeshcoreAuthenticator>,
    authorizer: Arc<MeshcoreAuthorizer>,
    identity_store: IdentityStore,
}

impl AuthHandler {
    pub fn new(
        authenticator: Arc<MeshcoreAuthenticator>,
        authorizer: Arc<MeshcoreAuthorizer>,
        identity_store: IdentityStore,
    ) -> Self {
        Self {
            authenticator,
            authorizer,
            identity_store,
        }
    }
}

/// Decide whether a client may register `will_topic` as its Last Will.
///
/// A missing Will is always fine. A present Will must pass the same check
/// as a PUBLISH to that topic by the same identity. Returns the denial
/// reason when the Will must be refused.
pub fn check_will_topic(
    authorizer: &dyn Authorizer,
    identity: &ClientIdentity,
    will_topic: Option<&str>,
) -> Result<(), String> {
    let Some(topic) = will_topic else {
        return Ok(());
    };
    match authorizer.check(identity, TopicAction::Publish, topic) {
        AclDecision::Allow | AclDecision::AllowStripRetain => Ok(()),
        AclDecision::Deny { reason } => Err(format!("Last Will topic refused: {reason}")),
    }
}

/// Topic of the Last Will carried in a CONNECT packet, if any.
///
/// Goes through the same conversion rmqtt uses when it later publishes the
/// Will, so we check exactly the topic the broker would send.
fn will_topic_of(connect_info: &ConnectInfo) -> Option<String> {
    let lw = connect_info.last_will()?;
    let publish = Publish::try_from(lw).ok()?;
    Some(publish.topic.to_string())
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

                let identity = match outcome {
                    AuthOutcome::Publisher { public_key } => {
                        ClientIdentity::Publisher { public_key }
                    }
                    AuthOutcome::Subscriber { username, role } => {
                        ClientIdentity::Subscriber { username, role }
                    }
                    AuthOutcome::Denied { reason } => {
                        tracing::info!(target: "access",
                            event = "auth",
                            client_id = %client_id,
                            identity_type = "unknown",
                            outcome = "deny",
                            reason = %reason,
                        );
                        return (
                            false,
                            Some(HookResult::AuthResult(AuthResult::BadUsernameOrPassword)),
                        );
                    }
                };

                // The Will is published later without an ACL check, so it
                // has to be authorized now or the CONNECT refused.
                let will_topic = will_topic_of(connect_info);
                if let Err(reason) =
                    check_will_topic(self.authorizer.as_ref(), &identity, will_topic.as_deref())
                {
                    super::log_access_event(
                        "auth",
                        &client_id,
                        &identity,
                        will_topic.as_deref().unwrap_or(""),
                        "deny",
                        Some(&reason),
                    );
                    return (
                        false,
                        Some(HookResult::AuthResult(AuthResult::NotAuthorized)),
                    );
                }

                match &identity {
                    ClientIdentity::Publisher { public_key } => {
                        tracing::info!(target: "access",
                            event = "auth",
                            client_id = %client_id,
                            identity_type = "publisher",
                            public_key = %public_key,
                            outcome = "allow",
                        );
                    }
                    ClientIdentity::Subscriber { username, role } => {
                        tracing::info!(target: "access",
                            event = "auth",
                            client_id = %client_id,
                            identity_type = "subscriber",
                            username = %username,
                            role = %role,
                            outcome = "allow",
                        );
                    }
                }
                self.identity_store.insert(client_id, identity);
                (
                    false,
                    Some(HookResult::AuthResult(AuthResult::Allow(false, None))),
                )
            }
            _ => (true, acc),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SubscriberRole;

    const KEY_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const KEY_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn publisher(key: &str) -> ClientIdentity {
        ClientIdentity::Publisher {
            public_key: key.into(),
        }
    }

    #[test]
    fn no_will_is_always_allowed() {
        let authz = MeshcoreAuthorizer::new();
        assert!(check_will_topic(&authz, &publisher(KEY_A), None).is_ok());
    }

    #[test]
    fn publisher_may_set_will_on_own_topic() {
        let authz = MeshcoreAuthorizer::new();
        let topic = format!("meshcore/LAX/{KEY_A}/status");
        assert!(check_will_topic(&authz, &publisher(KEY_A), Some(&topic)).is_ok());
    }

    #[test]
    fn publisher_may_not_set_will_on_another_publishers_topic() {
        let authz = MeshcoreAuthorizer::new();
        let topic = format!("meshcore/LAX/{KEY_B}/status");
        let err = check_will_topic(&authz, &publisher(KEY_A), Some(&topic)).unwrap_err();
        assert!(err.contains("Last Will topic refused"), "got: {err}");
    }

    #[test]
    fn publisher_may_not_set_will_with_invalid_iata() {
        let authz = MeshcoreAuthorizer::new();
        let topic = format!("meshcore/ZZZ/{KEY_A}/status");
        assert!(check_will_topic(&authz, &publisher(KEY_A), Some(&topic)).is_err());
    }

    #[test]
    fn subscriber_may_not_set_a_will() {
        let authz = MeshcoreAuthorizer::new();
        let id = ClientIdentity::Subscriber {
            username: "viewer".into(),
            role: SubscriberRole::Full,
        };
        let topic = format!("meshcore/LAX/{KEY_A}/status");
        assert!(check_will_topic(&authz, &id, Some(&topic)).is_err());
    }

    #[test]
    fn admin_may_set_any_will() {
        let authz = MeshcoreAuthorizer::new();
        let id = ClientIdentity::Subscriber {
            username: "admin".into(),
            role: SubscriberRole::Admin,
        };
        assert!(check_will_topic(&authz, &id, Some("anything/at/all")).is_ok());
    }
}
