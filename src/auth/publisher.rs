use super::AuthOutcome;

/// Authenticate a publisher client.
///
/// Publisher usernames have the format `v1_{PUBKEY}` where `{PUBKEY}` is
/// a hex-encoded Ed25519 public key. The password is an Ed25519-signed JWT
/// whose subject matches the public key.
///
/// # Arguments
/// * `username` — MQTT username, expected to start with `v1_`.
/// * `password` — Ed25519-signed JWT token.
pub fn authenticate_publisher(username: &str, password: &str) -> AuthOutcome {
    let pubkey_hex = match username.strip_prefix("v1_") {
        Some(pk) => pk,
        None => {
            return AuthOutcome::Denied {
                reason: "Publisher username must start with v1_".into(),
            };
        }
    };

    // Validate pubkey is valid hex and correct length (64 hex chars = 32 bytes)
    if pubkey_hex.len() != 64 || hex::decode(pubkey_hex).is_err() {
        return AuthOutcome::Denied {
            reason: "Invalid public key in username".into(),
        };
    }

    // Verify the JWT signature against the public key
    match super::jwt::decode_and_verify(password, pubkey_hex) {
        Ok(claims) => {
            // Optionally verify claims.sub matches pubkey
            if let Some(ref sub) = claims.sub {
                if sub != pubkey_hex {
                    return AuthOutcome::Denied {
                        reason: "JWT subject does not match public key".into(),
                    };
                }
            }

            // TODO: Check expiration (claims.exp) against current time

            AuthOutcome::Publisher {
                public_key: pubkey_hex.to_string(),
            }
        }
        Err(e) => AuthOutcome::Denied {
            reason: format!("JWT verification failed: {}", e),
        },
    }
}
