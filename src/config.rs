use miette::Diagnostic;
use serde::Deserialize;
use std::collections::HashMap;
use thiserror::Error;

/// Top-level broker configuration, loaded from TOML.
#[derive(Debug, Deserialize)]
pub struct BrokerConfig {
    /// MQTT listener address (e.g. "0.0.0.0:8083").
    pub listen: String,
    /// Path to TLS certificate file (PEM).
    pub tls_cert: String,
    /// Path to TLS private key file (PEM).
    pub tls_key: String,
    /// Static subscriber accounts keyed by username.
    pub subscribers: HashMap<String, SubscriberAccount>,
    /// Allowed IATA region codes. If empty, all codes are accepted.
    #[serde(default)]
    pub allowed_regions: Vec<String>,
    /// Access log configuration. If absent, access logging is disabled.
    pub access_log: Option<AccessLogConfig>,
}

/// Configuration for the access log file with size-based rotation.
#[derive(Debug, Deserialize)]
pub struct AccessLogConfig {
    /// Path to the access log file.
    #[serde(default = "AccessLogConfig::default_path")]
    pub path: String,
    /// Maximum size of a single log file in bytes before rotation.
    #[serde(default = "AccessLogConfig::default_max_size")]
    pub max_size: usize,
    /// Maximum number of rotated log files to keep.
    #[serde(default = "AccessLogConfig::default_max_files")]
    pub max_files: usize,
}

impl AccessLogConfig {
    fn default_path() -> String {
        "access.log".into()
    }
    fn default_max_size() -> usize {
        10 * 1024 * 1024 // 10 MiB
    }
    fn default_max_files() -> usize {
        5
    }
}

/// A static subscriber account entry.
#[derive(Debug, Deserialize)]
pub struct SubscriberAccount {
    /// Argon2id password hash.
    pub password_hash: String,
    /// Role name: "full" or "limited".
    pub role: String,
}

#[derive(Debug, Error, Diagnostic)]
pub enum ConfigError {
    #[error("Configuration file not found: {path}")]
    #[diagnostic(help(
        "Create a config file at this path, or use --config to specify a different location.\nSee config.toml.example for the expected format."
    ))]
    NotFound { path: String },

    #[error("Cannot read configuration file: {path}")]
    #[diagnostic(help("Check file permissions and ensure the path is correct."))]
    ReadError {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Invalid TOML in configuration file: {path}")]
    #[diagnostic(help("Fix the TOML syntax error shown above, then try again."))]
    ParseError {
        path: String,
        #[source]
        source: toml::de::Error,
    },
}

impl BrokerConfig {
    /// Load configuration from a TOML file.
    pub fn load(path: &str) -> Result<Self, ConfigError> {
        let contents = std::fs::read_to_string(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ConfigError::NotFound {
                    path: path.to_string(),
                }
            } else {
                ConfigError::ReadError {
                    path: path.to_string(),
                    source: e,
                }
            }
        })?;
        let config: BrokerConfig =
            toml::from_str(&contents).map_err(|e| ConfigError::ParseError {
                path: path.to_string(),
                source: e,
            })?;
        Ok(config)
    }
}
