pub mod acl;
pub mod iata;
pub mod topic;

use crate::types::{ClientIdentity, TopicAction};

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
    fn check(&self, identity: &ClientIdentity, action: TopicAction, raw_topic: &str) -> AclDecision {
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
