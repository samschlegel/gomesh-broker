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

            // Check expiration
            if let Some(exp) = claims.exp {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                if exp < now {
                    return AuthOutcome::Denied {
                        reason: "JWT has expired".into(),
                    };
                }
            }

            AuthOutcome::Publisher {
                public_key: pubkey_hex.to_string(),
            }
        }
        Err(e) => AuthOutcome::Denied {
            reason: format!("JWT verification failed: {}", e),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;
    use ed25519_dalek::{SigningKey, Signer};

    fn make_jwt(signing_key: &SigningKey, claims_json: &str) -> String {
        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"EdDSA","typ":"JWT"}"#);
        let payload = URL_SAFE_NO_PAD.encode(claims_json);
        let message = format!("{}.{}", header, payload);
        let signature = signing_key.sign(message.as_bytes());
        let sig_b64 = URL_SAFE_NO_PAD.encode(signature.to_bytes());
        format!("{}.{}.{}", header, payload, sig_b64)
    }

    #[test]
    fn valid_publisher_auth() {
        let signing_key = SigningKey::from_bytes(&[1u8; 32]);
        let pk_hex = hex::encode(signing_key.verifying_key().as_bytes());
        let username = format!("v1_{pk_hex}");

        let claims = serde_json::json!({
            "sub": pk_hex,
            "exp": 9999999999u64,
            "iat": 1000000000u64
        });
        let token = make_jwt(&signing_key, &claims.to_string());

        let result = authenticate_publisher(&username, &token);
        match result {
            AuthOutcome::Publisher { public_key } => {
                assert_eq!(public_key, pk_hex);
            }
            other => panic!("expected AuthOutcome::Publisher, got {:?}", other),
        }
    }

    #[test]
    fn expired_token_rejected() {
        let signing_key = SigningKey::from_bytes(&[1u8; 32]);
        let pk_hex = hex::encode(signing_key.verifying_key().as_bytes());
        let username = format!("v1_{pk_hex}");

        let claims = serde_json::json!({
            "sub": pk_hex,
            "exp": 1u64,  // expired long ago
            "iat": 0u64
        });
        let token = make_jwt(&signing_key, &claims.to_string());

        let result = authenticate_publisher(&username, &token);
        match result {
            AuthOutcome::Denied { reason } => {
                assert!(
                    reason.contains("expired"),
                    "denial reason should mention expiration, got: {reason}"
                );
            }
            other => panic!("expected Denied for expired token, got {:?}", other),
        }
    }
}
