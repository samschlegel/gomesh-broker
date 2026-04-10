pub mod acl;
pub mod iata;
pub mod topic;

use crate::types::{ClientIdentity, SubscriberRole, TopicAction};

/// Result of an authorization check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AclDecision {
    /// Access allowed.
    Allow,
    /// Access denied with a reason.
    Deny { reason: String },
    /// Access allowed but MQTT retain flag must be stripped.
    AllowStripRetain,
}

/// Trait for authorizing MQTT client actions on topics.
///
/// Implementations are pure functions with no rmqtt dependency,
/// making them independently testable.
pub trait Authorizer: Send + Sync {
    /// Check whether `identity` is allowed to perform `action` on `topic`.
    fn check(&self, identity: &ClientIdentity, action: TopicAction, topic: &str) -> AclDecision;
}

/// The primary authorizer composing topic parsing, IATA validation, and ACL rules.
pub struct MeshcoreAuthorizer;

impl MeshcoreAuthorizer {
    pub fn new() -> Self {
        Self
    }
}

impl Authorizer for MeshcoreAuthorizer {
    fn check(
        &self,
        identity: &ClientIdentity,
        action: TopicAction,
        raw_topic: &str,
    ) -> AclDecision {
        // Admins bypass topic parsing and IATA validation (e.g. wildcard subscriptions)
        if matches!(
            identity,
            ClientIdentity::Subscriber {
                role: SubscriberRole::Admin,
                ..
            }
        ) {
            return AclDecision::Allow;
        }

        // Parse topic into components
        let parts = match topic::parse_topic(raw_topic) {
            Some(p) => p,
            None => {
                return AclDecision::Deny {
                    reason: "Malformed topic".into(),
                };
            }
        };

        // Validate IATA code
        if !iata::is_valid_iata(&parts.iata) {
            return AclDecision::Deny {
                reason: format!("Invalid IATA code: {}", parts.iata),
            };
        }

        // Delegate to ACL engine
        acl::check_acl(identity, action, &parts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SubscriberRole;

    #[test]
    fn admin_can_subscribe_to_wildcard() {
        let authz = MeshcoreAuthorizer::new();
        let id = ClientIdentity::Subscriber {
            username: "admin".into(),
            role: SubscriberRole::Admin,
        };
        assert_eq!(
            authz.check(&id, TopicAction::Subscribe, "#"),
            AclDecision::Allow
        );
    }

    #[test]
    fn admin_can_publish_to_any_topic() {
        let authz = MeshcoreAuthorizer::new();
        let id = ClientIdentity::Subscriber {
            username: "admin".into(),
            role: SubscriberRole::Admin,
        };
        assert_eq!(
            authz.check(&id, TopicAction::Publish, "us/LAX/aabb/telemetry"),
            AclDecision::Allow
        );
    }

    #[test]
    fn non_admin_wildcard_denied() {
        let authz = MeshcoreAuthorizer::new();
        let id = ClientIdentity::Subscriber {
            username: "viewer".into(),
            role: SubscriberRole::Full,
        };
        let result = authz.check(&id, TopicAction::Subscribe, "#");
        assert!(matches!(result, AclDecision::Deny { .. }));
    }
}
