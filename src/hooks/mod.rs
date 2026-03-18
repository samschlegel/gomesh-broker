pub mod auth_handler;
pub mod publish_handler;
pub mod subscribe_handler;
pub mod delivery_handler;

use crate::auth::MeshcoreAuthenticator;
use crate::authz::MeshcoreAuthorizer;
use crate::config::BrokerConfig;

/// The main plugin struct that composes authentication, authorization,
/// and filtering, and registers rmqtt hook handlers.
pub struct MeshcorePlugin {
    authenticator: MeshcoreAuthenticator,
    authorizer: MeshcoreAuthorizer,
}

impl MeshcorePlugin {
    /// Create a new plugin instance from the given configuration.
    pub fn new(config: BrokerConfig) -> Self {
        Self {
            authenticator: MeshcoreAuthenticator::new(config),
            authorizer: MeshcoreAuthorizer::new(),
        }
    }

    /// Register all hook handlers with the rmqtt broker.
    ///
    /// This will be implemented once the rmqtt integration layer is built.
    /// It registers handlers for:
    /// - Client authentication (`on_client_authenticate`)
    /// - Publish authorization (`on_message_publish`)
    /// - Subscribe authorization (`on_client_subscribe`)
    /// - Message delivery filtering (`on_message_deliver`)
    pub fn register(&self) {
        todo!("Register rmqtt hook handlers — blocked on rmqtt dependency integration")
    }
}
