pub mod auth_handler;
pub mod publish_handler;
pub mod subscribe_handler;
pub mod delivery_handler;

use std::sync::Arc;

use rmqtt::hook::{Register, Type};

use crate::auth::MeshcoreAuthenticator;
use crate::authz::MeshcoreAuthorizer;
use crate::config::BrokerConfig;

use auth_handler::{AuthHandler, IdentityStore};
use publish_handler::{PublishAclHandler, RetainStripHandler};
use subscribe_handler::SubscribeAclHandler;
use delivery_handler::DeliveryHandler;

/// The main plugin struct that composes authentication, authorization,
/// and filtering, and registers rmqtt hook handlers.
pub struct MeshcorePlugin {
    authenticator: Arc<MeshcoreAuthenticator>,
    authorizer: Arc<MeshcoreAuthorizer>,
    identity_store: IdentityStore,
}

impl MeshcorePlugin {
    /// Create a new plugin instance from the given configuration.
    pub fn new(config: BrokerConfig) -> Self {
        Self {
            authenticator: Arc::new(MeshcoreAuthenticator::new(config)),
            authorizer: Arc::new(MeshcoreAuthorizer::new()),
            identity_store: IdentityStore::default(),
        }
    }

    /// Register all hook handlers with the rmqtt broker.
    ///
    /// Registers handlers for:
    /// - Client authentication (`ClientAuthenticate`)
    /// - Publish ACL (`MessagePublishCheckAcl`)
    /// - Retain stripping (`MessagePublish`)
    /// - Subscribe ACL (`ClientSubscribeCheckAcl`)
    /// - Message delivery filtering (`MessageDelivered`)
    pub async fn register(&self, register: &dyn Register) {
        register
            .add(
                Type::ClientAuthenticate,
                Box::new(AuthHandler::new(
                    self.authenticator.clone(),
                    self.identity_store.clone(),
                )),
            )
            .await;

        register
            .add(
                Type::MessagePublishCheckAcl,
                Box::new(PublishAclHandler::new(
                    self.authorizer.clone(),
                    self.identity_store.clone(),
                )),
            )
            .await;

        register
            .add(
                Type::MessagePublish,
                Box::new(RetainStripHandler::new(self.identity_store.clone())),
            )
            .await;

        register
            .add(
                Type::ClientSubscribeCheckAcl,
                Box::new(SubscribeAclHandler::new(
                    self.authorizer.clone(),
                    self.identity_store.clone(),
                )),
            )
            .await;

        register
            .add(
                Type::MessageDelivered,
                Box::new(DeliveryHandler::new(self.identity_store.clone())),
            )
            .await;
    }
}
