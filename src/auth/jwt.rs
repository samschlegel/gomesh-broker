use anyhow::{anyhow, Result};
use ed25519_dalek::{Signature, VerifyingKey, Verifier};

/// Decoded JWT claims (only the fields we need).
#[derive(Debug, serde::Deserialize)]
pub struct JwtClaims {
    /// Subject — should match the publisher's public key.
    pub sub: Option<String>,
    /// Expiration time (Unix timestamp).
    pub exp: Option<u64>,
    /// Issued-at time (Unix timestamp).
    pub iat: Option<u64>,
}

/// Decode and verify an Ed25519-signed JWT.
///
/// The JWT is expected to have the format: `header.payload.signature`
/// where the header `alg` is `EdDSA` and the signature is over
/// `header.payload` using the given Ed25519 public key.
///
/// # Arguments
/// * `token` — The raw JWT string.
/// * `public_key_hex` — Hex-encoded Ed25519 public key (32 bytes / 64 hex chars).
///
/// # Returns
/// Decoded claims if the signature is valid and the token is well-formed.
pub fn decode_and_verify(token: &str, public_key_hex: &str) -> Result<JwtClaims> {
    let parts: Vec<&str> = token.splitn(3, '.').collect();
    if parts.len() != 3 {
        return Err(anyhow!("JWT must have 3 parts"));
    }

    let header_b64 = parts[0];
    let payload_b64 = parts[1];
    let signature_b64 = parts[2];

    // Decode and verify header
    let _header_bytes = base64::Engine::decode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        header_b64,
    )?;

    // Decode payload
    let payload_bytes = base64::Engine::decode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        payload_b64,
    )?;
    let claims: JwtClaims = serde_json::from_slice(&payload_bytes)?;

    // Decode public key
    let pk_bytes = hex::decode(public_key_hex)?;
    if pk_bytes.len() != 32 {
        return Err(anyhow!("Ed25519 public key must be 32 bytes"));
    }
    let verifying_key = VerifyingKey::from_bytes(pk_bytes[..32].try_into().unwrap())?;

    // Decode signature
    let sig_bytes = base64::Engine::decode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        signature_b64,
    )?;
    if sig_bytes.len() != 64 {
        return Err(anyhow!("Ed25519 signature must be 64 bytes"));
    }
    let signature = Signature::from_bytes(sig_bytes[..64].try_into().unwrap());

    // Verify: signature is over "header.payload"
    let message = format!("{}.{}", header_b64, payload_b64);
    verifying_key.verify(message.as_bytes(), &signature)?;

    Ok(claims)
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

    fn test_keypair() -> (SigningKey, String) {
        let signing_key = SigningKey::from_bytes(&[1u8; 32]);
        let pubkey_hex = hex::encode(signing_key.verifying_key().as_bytes());
        (signing_key, pubkey_hex)
    }

    #[test]
    fn valid_token_verifies() {
        let (sk, pk_hex) = test_keypair();
        let claims = serde_json::json!({
            "sub": pk_hex,
            "exp": 9999999999u64,
            "iat": 1000000000u64
        });
        let token = make_jwt(&sk, &claims.to_string());
        let result = decode_and_verify(&token, &pk_hex);
        assert!(result.is_ok(), "expected Ok, got {:?}", result);
    }

    #[test]
    fn wrong_key_rejects() {
        let (sk, _) = test_keypair();
        let other_key = SigningKey::from_bytes(&[2u8; 32]);
        let other_pk_hex = hex::encode(other_key.verifying_key().as_bytes());

        let claims = serde_json::json!({
            "sub": other_pk_hex,
            "exp": 9999999999u64,
            "iat": 1000000000u64
        });
        let token = make_jwt(&sk, &claims.to_string());
        let result = decode_and_verify(&token, &other_pk_hex);
        assert!(result.is_err(), "expected Err for wrong key, got {:?}", result);
    }

    #[test]
    fn malformed_token_three_parts() {
        let (_, pk_hex) = test_keypair();
        let result = decode_and_verify("part1.part2", &pk_hex);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("3 parts"),
            "expected '3 parts' error, got: {err_msg}"
        );
    }

    #[test]
    fn tampered_payload_rejects() {
        let (sk, pk_hex) = test_keypair();
        let claims = serde_json::json!({
            "sub": pk_hex,
            "exp": 9999999999u64,
            "iat": 1000000000u64
        });
        let token = make_jwt(&sk, &claims.to_string());

        let parts: Vec<&str> = token.splitn(3, '.').collect();
        let tampered_payload = URL_SAFE_NO_PAD.encode(
            serde_json::json!({"sub": "tampered", "exp": 9999999999u64, "iat": 1u64}).to_string(),
        );
        let tampered_token = format!("{}.{}.{}", parts[0], tampered_payload, parts[2]);

        let result = decode_and_verify(&tampered_token, &pk_hex);
        assert!(result.is_err(), "expected Err for tampered payload, got {:?}", result);
    }

    #[test]
    fn claims_decoded_correctly() {
        let (sk, pk_hex) = test_keypair();
        let claims = serde_json::json!({
            "sub": pk_hex,
            "exp": 1700000000u64,
            "iat": 1600000000u64
        });
        let token = make_jwt(&sk, &claims.to_string());
        let decoded = decode_and_verify(&token, &pk_hex).expect("should decode");
        assert_eq!(decoded.sub.as_deref(), Some(pk_hex.as_str()));
        assert_eq!(decoded.exp, Some(1700000000));
        assert_eq!(decoded.iat, Some(1600000000));
    }
}
