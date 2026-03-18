/// Identity of an authenticated MQTT client.
#[derive(Debug, Clone)]
pub enum ClientIdentity {
    /// Publisher authenticated via Ed25519 JWT.
    Publisher {
        /// Hex-encoded Ed25519 public key extracted from the `v1_{PUBKEY}` username.
        public_key: String,
    },
    /// Subscriber authenticated via static account lookup.
    Subscriber {
        username: String,
        role: SubscriberRole,
    },
}

/// Role assigned to a subscriber account, controlling topic access and payload filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubscriberRole {
    /// Full access to all topics and payload fields.
    Full,
    /// Limited access — certain payload fields are stripped.
    Limited,
}

/// Parsed components of a MeshCore MQTT topic.
///
/// Topics follow the pattern: `{region}/{iata}/{pubkey}/{subtopic...}`
#[derive(Debug, Clone)]
pub struct TopicParts {
    /// Region prefix (e.g. "us", "eu").
    pub region: String,
    /// IATA airport code identifying the geographic area.
    pub iata: String,
    /// Hex-encoded Ed25519 public key of the publishing node.
    pub pubkey: String,
    /// Remaining topic segments after the pubkey.
    pub subtopic: Vec<String>,
}

/// The action a client is attempting on a topic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopicAction {
    Publish,
    Subscribe,
}
