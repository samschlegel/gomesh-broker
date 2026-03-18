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
