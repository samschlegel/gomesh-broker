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
        // Admins bypass all checks (e.g. wildcard, $SYS, and internal subscriptions)
        if matches!(
            identity,
            ClientIdentity::Subscriber {
                role: SubscriberRole::Admin,
                ..
            }
        ) {
            return AclDecision::Allow;
        }

        match action {
            // Subscription authorization is topic-parse-independent: non-admin
            // subscribers may use broad wildcards confined to the meshcore/ root.
            TopicAction::Subscribe => acl::check_subscribe(identity, raw_topic),

            // Publishing requires a well-formed, IATA-valid topic owned by the publisher.
            TopicAction::Publish => {
                let parts = match topic::parse_topic(raw_topic) {
                    Some(p) => p,
                    None => {
                        return AclDecision::Deny {
                            reason: "Malformed topic".into(),
                        };
                    }
                };

                if !iata::is_valid_iata(&parts.iata) {
                    return AclDecision::Deny {
                        reason: format!("Invalid IATA code: {}", parts.iata),
                    };
                }

                acl::check_acl(identity, &parts)
            }
        }
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
    fn subscribers_can_subscribe_to_meshcore_wildcard() {
        let authz = MeshcoreAuthorizer::new();
        for role in [SubscriberRole::Full, SubscriberRole::Limited] {
            let id = ClientIdentity::Subscriber {
                username: "viewer".into(),
                role,
            };
            assert_eq!(
                authz.check(&id, TopicAction::Subscribe, "meshcore/#"),
                AclDecision::Allow
            );
        }
    }

    #[test]
    fn non_admin_denied_outside_meshcore() {
        let authz = MeshcoreAuthorizer::new();
        let id = ClientIdentity::Subscriber {
            username: "viewer".into(),
            role: SubscriberRole::Full,
        };
        for filter in ["#", "$SYS/#", "other/#"] {
            assert!(
                matches!(
                    authz.check(&id, TopicAction::Subscribe, filter),
                    AclDecision::Deny { .. }
                ),
                "expected deny for filter {filter}"
            );
        }
    }

    #[test]
    fn publisher_cannot_subscribe() {
        let authz = MeshcoreAuthorizer::new();
        let id = ClientIdentity::Publisher {
            public_key: "aabb".into(),
        };
        assert!(matches!(
            authz.check(&id, TopicAction::Subscribe, "meshcore/#"),
            AclDecision::Deny { .. }
        ));
    }
}
