use super::topic;
use super::AclDecision;
use crate::types::{ClientIdentity, SubscriberRole, TopicParts};

/// ACL decision engine for the **publish** path.
///
/// The topic has already been parsed and its IATA code validated by
/// `MeshcoreAuthorizer::check` before this is called.
///
/// Rules:
/// - **Admin subscribers** can publish to any topic. (Also short-circuited in
///   `MeshcoreAuthorizer::check` to bypass topic parsing.)
/// - **Publishers** can only publish to topics matching their own public key;
///   no client may publish to another publisher's topic.
/// - **Subscribers** (Full/Limited) may not publish.
pub fn check_acl(identity: &ClientIdentity, topic: &TopicParts) -> AclDecision {
    match identity {
        // Admin subscribers can publish to any topic
        ClientIdentity::Subscriber {
            role: SubscriberRole::Admin,
            ..
        } => AclDecision::Allow,

        // Publishers can only publish to their own pubkey topics
        ClientIdentity::Publisher { public_key } => {
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

        // Subscribers cannot publish
        ClientIdentity::Subscriber { .. } => AclDecision::Deny {
            reason: "Subscribers are not allowed to publish".into(),
        },
    }
}

/// ACL decision engine for the **subscribe** path.
///
/// Subscription authorization is topic-parse-independent: broad wildcards like
/// `meshcore/#` are permitted, so we work from the raw filter rather than a
/// parsed [`TopicParts`].
///
/// Rules:
/// - **Admin subscribers** can subscribe to anything (also short-circuited in
///   `MeshcoreAuthorizer::check`).
/// - **Full/Limited subscribers** can subscribe to any filter confined to the
///   `meshcore/` root (see [`topic::filter_within_meshcore_root`]). This scopes
///   them to MeshCore data and excludes `$SYS/*` and other applications' topics.
///   Limited-role payload filtering is applied separately at delivery time.
/// - **Publishers** may not subscribe.
pub fn check_subscribe(identity: &ClientIdentity, filter: &str) -> AclDecision {
    match identity {
        // Admin subscribers can subscribe to anything
        ClientIdentity::Subscriber {
            role: SubscriberRole::Admin,
            ..
        } => AclDecision::Allow,

        // Full/Limited subscribers are scoped to the meshcore/ namespace
        ClientIdentity::Subscriber { .. } => {
            if topic::filter_within_meshcore_root(filter) {
                AclDecision::Allow
            } else {
                AclDecision::Deny {
                    reason: format!(
                        "Subscription filter must be within the '{}/' namespace",
                        topic::MESHCORE_ROOT
                    ),
                }
            }
        }

        // Publishers should not subscribe
        ClientIdentity::Publisher { .. } => AclDecision::Deny {
            reason: "Publishers are not allowed to subscribe".into(),
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
            check_acl(&id, &topic("aabb")),
            AclDecision::AllowStripRetain
        );
    }

    #[test]
    fn publisher_other_topic() {
        let id = ClientIdentity::Publisher {
            public_key: "aabb".into(),
        };
        let result = check_acl(&id, &topic("ccdd"));
        assert!(matches!(result, AclDecision::Deny { .. }));
    }

    #[test]
    fn full_subscriber_can_subscribe_within_meshcore() {
        let id = ClientIdentity::Subscriber {
            username: "viewer".into(),
            role: SubscriberRole::Full,
        };
        assert_eq!(check_subscribe(&id, "meshcore/#"), AclDecision::Allow);
    }

    #[test]
    fn subscriber_denied_outside_meshcore() {
        let id = ClientIdentity::Subscriber {
            username: "viewer".into(),
            role: SubscriberRole::Limited,
        };
        assert!(matches!(
            check_subscribe(&id, "#"),
            AclDecision::Deny { .. }
        ));
    }

    #[test]
    fn publisher_cannot_subscribe() {
        let id = ClientIdentity::Publisher {
            public_key: "aabb".into(),
        };
        assert!(matches!(
            check_subscribe(&id, "meshcore/#"),
            AclDecision::Deny { .. }
        ));
    }

    #[test]
    fn subscriber_cannot_publish() {
        let id = ClientIdentity::Subscriber {
            username: "viewer".into(),
            role: SubscriberRole::Limited,
        };
        let result = check_acl(&id, &topic("aabb"));
        assert!(matches!(result, AclDecision::Deny { .. }));
    }

    #[test]
    fn admin_can_subscribe() {
        let id = ClientIdentity::Subscriber {
            username: "admin".into(),
            role: SubscriberRole::Admin,
        };
        assert_eq!(check_subscribe(&id, "#"), AclDecision::Allow);
    }

    #[test]
    fn admin_can_publish() {
        let id = ClientIdentity::Subscriber {
            username: "admin".into(),
            role: SubscriberRole::Admin,
        };
        assert_eq!(check_acl(&id, &topic("aabb")), AclDecision::Allow);
    }
}
