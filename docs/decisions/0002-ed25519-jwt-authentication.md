# 0002: Ed25519 JWT Authentication

- **Status:** Accepted
- **Date:** 2026-03-17
- **Deciders:** Sam Schlegel

## Context

MeshCore nodes authenticate to the MQTT broker using Ed25519-signed JWTs. The MQTT username encodes the protocol version and public key (`v1_{PUBKEY}`), and the password field contains the JWT token signed by the corresponding private key.

The upstream meshcore-mqtt-broker implements this by manually parsing the JWT and verifying the Ed25519 signature, rather than using a full JWT library. We need to replicate this exact authentication scheme.

## Decision Drivers

- Must be byte-compatible with existing MeshCore node firmware authentication.
- Ed25519 is the only supported algorithm (not RS256, ES256, etc.).
- Minimal attack surface — no need for full JWT spec support (no encryption, no key rotation, no JWK).
- Must validate: signature correctness, subject matches pubkey, expiration time.

## Considered Options

### Option A: Use jsonwebtoken crate

[jsonwebtoken](https://crates.io/crates/jsonwebtoken) is the most popular Rust JWT library.

- **Pros:** Well-tested, handles edge cases, supports EdDSA.
- **Cons:** Pulls in many dependencies. Supports algorithms we don't need, increasing attack surface. May not match the exact JWT format produced by MeshCore firmware (custom headers, etc.).

### Option B: Manual JWT parsing with ed25519-dalek

Parse the three JWT segments manually, decode base64url, verify the Ed25519 signature using ed25519-dalek directly.

- **Pros:** Exact control over the verification process. Matches the upstream TypeScript implementation's approach. Minimal dependencies. Easy to audit.
- **Cons:** Must handle base64url decoding and JSON parsing ourselves (trivial with base64 + serde_json).

## Decision Outcome

**Chosen: Option B (manual JWT parsing with ed25519-dalek)**

This matches the upstream implementation's approach of manual JWT handling. Since we only support a single algorithm (EdDSA with Ed25519) and a minimal set of claims, the implementation is straightforward (~50 lines). Using ed25519-dalek directly gives us certainty about signature verification behavior.

## Consequences

- **Good:** Minimal dependency footprint — only ed25519-dalek, base64, serde_json.
- **Good:** Byte-for-byte compatible with upstream TypeScript implementation.
- **Good:** Easy to audit — the entire JWT verification is in one file (~50 lines).
- **Bad:** We must handle JWT edge cases ourselves (though we only support one algorithm).
- **Mitigation:** Comprehensive test suite with test vectors from the upstream broker.
