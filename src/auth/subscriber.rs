use super::AuthOutcome;
use crate::config::BrokerConfig;
use crate::types::SubscriberRole;

/// Authenticate a subscriber client against static account configuration.
///
/// Looks up the username in the configured subscriber accounts and
/// verifies the password against the stored Argon2id hash.
///
/// # Arguments
/// * `config` — Broker configuration containing subscriber accounts.
/// * `username` — MQTT username.
/// * `password` — Plaintext password to verify.
pub fn authenticate_subscriber(
    config: &BrokerConfig,
    username: &str,
    password: &str,
) -> AuthOutcome {
    let account = match config.subscribers.get(username) {
        Some(acct) => acct,
        None => {
            return AuthOutcome::Denied {
                reason: "Unknown subscriber account".into(),
            };
        }
    };

    // Verify password against Argon2id hash
    match verify_password(password, &account.password_hash) {
        Ok(true) => {}
        Ok(false) => {
            return AuthOutcome::Denied {
                reason: "Invalid password".into(),
            };
        }
        Err(e) => {
            return AuthOutcome::Denied {
                reason: format!("Password verification error: {}", e),
            };
        }
    }

    let role = match account.role.as_str() {
        "full" => SubscriberRole::Full,
        "limited" => SubscriberRole::Limited,
        other => {
            return AuthOutcome::Denied {
                reason: format!("Unknown role: {}", other),
            };
        }
    };

    AuthOutcome::Subscriber {
        username: username.to_string(),
        role,
    }
}

/// Verify a plaintext password against an Argon2id hash string.
fn verify_password(password: &str, hash: &str) -> anyhow::Result<bool> {
    use argon2::Argon2;
    use argon2::password_hash::{PasswordHash, PasswordVerifier};

    let parsed_hash =
        PasswordHash::new(hash).map_err(|e| anyhow::anyhow!("Invalid hash format: {}", e))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}
