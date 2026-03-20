//! Hook handler for MQTT message delivery filtering.
//!
//! Called by rmqtt before delivering a message to a subscriber. For
//! limited-role subscribers, applies payload filtering to strip
//! sensitive fields.
//!
//! # Flow
//! 1. Retrieve the `ClientIdentity` of the receiving subscriber.
//! 2. If the subscriber has `SubscriberRole::Limited`:
//!    a. Call `filter_payload_for_limited(payload)`.
//!    b. Replace the message payload with the filtered version.
//! 3. If the subscriber has `SubscriberRole::Full`, deliver as-is.

use async_trait::async_trait;
use bytes::Bytes;
use rmqtt::hook::{Handler, HookResult, Parameter, ReturnType};

use crate::filter::filter_payload_for_limited;
use crate::hooks::auth_handler::IdentityStore;
use crate::types::{ClientIdentity, SubscriberRole};

pub struct DeliveryHandler {
    identity_store: IdentityStore,
}

impl DeliveryHandler {
    pub fn new(identity_store: IdentityStore) -> Self {
        Self { identity_store }
    }
}

#[async_trait]
impl Handler for DeliveryHandler {
    async fn hook(&self, param: &Parameter, acc: Option<HookResult>) -> ReturnType {
        match param {
            Parameter::MessageDelivered(session, _from, publish) => {
                let client_id = session.id.client_id.to_string();

                let is_limited = matches!(
                    self.identity_store.get(&client_id).as_deref(),
                    Some(ClientIdentity::Subscriber {
                        role: SubscriberRole::Limited,
                        ..
                    })
                );

                if is_limited {
                    if let Some(filtered) = filter_payload_for_limited(&publish.payload) {
                        let mut modified = (*publish).clone();
                        modified.payload = Bytes::from(filtered);
                        return (false, Some(HookResult::Publish(modified)));
                    }
                }

                (true, acc)
            }
            _ => (true, acc),
        }
    }
}
