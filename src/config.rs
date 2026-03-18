use serde::Deserialize;
use std::collections::HashMap;

/// Top-level broker configuration, loaded from TOML.
#[derive(Debug, Deserialize)]
pub struct BrokerConfig {
    /// MQTT listener address (e.g. "0.0.0.0:1883").
    pub listen: String,
    /// Static subscriber accounts keyed by username.
    pub subscribers: HashMap<String, SubscriberAccount>,
    /// Allowed IATA region codes. If empty, all codes are accepted.
    #[serde(default)]
    pub allowed_regions: Vec<String>,
}

/// A static subscriber account entry.
#[derive(Debug, Deserialize)]
pub struct SubscriberAccount {
    /// Argon2id password hash.
    pub password_hash: String,
    /// Role name: "full" or "limited".
    pub role: String,
}

impl BrokerConfig {
    /// Load configuration from a TOML file.
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let config: BrokerConfig = toml::from_str(&contents)?;
        Ok(config)
    }
}
