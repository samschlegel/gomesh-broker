pub mod jwt;
pub mod publisher;
pub mod subscriber;

use crate::types::SubscriberRole;

/// Outcome of an authentication attempt.
#[derive(Debug, Clone)]
pub enum AuthOutcome {
    /// Publisher authenticated via Ed25519 JWT. Contains the hex public key.
    Publisher { public_key: String },
    /// Subscriber authenticated via static account. Contains username and role.
    Subscriber {
        username: String,
        role: SubscriberRole,
    },
    /// Authentication denied.
    Denied { reason: String },
}

/// Trait for authenticating MQTT clients.
///
/// Implementations are pure functions with no rmqtt dependency,
/// making them independently testable.
pub trait Authenticator: Send + Sync {
    /// Authenticate a client given their MQTT username and password.
    fn authenticate(&self, username: &str, password: &str) -> AuthOutcome;
}

/// The primary authenticator composing publisher and subscriber auth.
pub struct MeshcoreAuthenticator {
    config: crate::config::BrokerConfig,
}

impl MeshcoreAuthenticator {
    pub fn new(config: crate::config::BrokerConfig) -> Self {
        Self { config }
    }
}

impl Authenticator for MeshcoreAuthenticator {
    fn authenticate(&self, username: &str, password: &str) -> AuthOutcome {
        // Publisher usernames start with "v1_"
        if username.starts_with("v1_") {
            return publisher::authenticate_publisher(username, password);
        }

        // Otherwise try subscriber auth
        subscriber::authenticate_subscriber(&self.config, username, password)
    }
}
