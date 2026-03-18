use crate::types::{ClientIdentity, SubscriberRole, TopicAction, TopicParts};
use super::AclDecision;

/// Core ACL decision engine.
///
/// Rules:
/// - **Publishers** can only publish to topics matching their own public key.
/// - **Subscribers with Full role** can subscribe to any valid topic.
/// - **Subscribers with Limited role** can subscribe to any valid topic,
///   but payload filtering is applied separately (see `filter` module).
/// - No client may publish to another publisher's topic.
pub fn check_acl(
    identity: &ClientIdentity,
    action: TopicAction,
    topic: &TopicParts,
) -> AclDecision {
    match (identity, action) {
        // Publishers can only publish to their own pubkey topics
        (ClientIdentity::Publisher { public_key }, TopicAction::Publish) => {
            if topic.pubkey == *public_key {
                AclDecision::AllowStripRetain
            } else {
                AclDecision::Deny {
                    reason: format!(
                        "Publisher {} cannot publish to topic owned by {}",
                        public_key, topic.pubkey
                    ),
                }
            }
        }

        // Publishers should not subscribe
        (ClientIdentity::Publisher { .. }, TopicAction::Subscribe) => AclDecision::Deny {
            reason: "Publishers are not allowed to subscribe".into(),
        },

        // Full subscribers can subscribe to anything
        (
            ClientIdentity::Subscriber {
                role: SubscriberRole::Full,
                ..
            },
            TopicAction::Subscribe,
        ) => AclDecision::Allow,

        // Limited subscribers can subscribe (filtering is applied at delivery time)
        (
            ClientIdentity::Subscriber {
                role: SubscriberRole::Limited,
                ..
            },
            TopicAction::Subscribe,
        ) => AclDecision::Allow,

        // Subscribers cannot publish
        (ClientIdentity::Subscriber { .. }, TopicAction::Publish) => AclDecision::Deny {
            reason: "Subscribers are not allowed to publish".into(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn topic(pubkey: &str) -> TopicParts {
        TopicParts {
            region: "us".into(),
            iata: "LAX".into(),
            pubkey: pubkey.into(),
            subtopic: vec!["telemetry".into()],
        }
    }

    #[test]
    fn publisher_own_topic() {
        let id = ClientIdentity::Publisher {
            public_key: "aabb".into(),
        };
        assert_eq!(
            check_acl(&id, TopicAction::Publish, &topic("aabb")),
            AclDecision::AllowStripRetain
        );
    }

    #[test]
    fn publisher_other_topic() {
        let id = ClientIdentity::Publisher {
            public_key: "aabb".into(),
        };
        let result = check_acl(&id, TopicAction::Publish, &topic("ccdd"));
        assert!(matches!(result, AclDecision::Deny { .. }));
    }

    #[test]
    fn full_subscriber_can_subscribe() {
        let id = ClientIdentity::Subscriber {
            username: "admin".into(),
            role: SubscriberRole::Full,
        };
        assert_eq!(
            check_acl(&id, TopicAction::Subscribe, &topic("aabb")),
            AclDecision::Allow
        );
    }

    #[test]
    fn subscriber_cannot_publish() {
        let id = ClientIdentity::Subscriber {
            username: "viewer".into(),
            role: SubscriberRole::Limited,
        };
        let result = check_acl(&id, TopicAction::Publish, &topic("aabb"));
        assert!(matches!(result, AclDecision::Deny { .. }));
    }
}
