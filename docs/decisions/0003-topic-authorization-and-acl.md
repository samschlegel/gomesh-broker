# 0003: Topic Authorization and ACL

- **Status:** Accepted
- **Date:** 2026-03-17
- **Deciders:** Sam Schlegel

## Context

MeshCore MQTT topics follow the structure `{region}/{iata}/{pubkey}/{subtopic...}`. Authorization rules must enforce:

1. **Publisher isolation:** A publisher can only publish to topics containing its own public key.
2. **Subscriber role-based access:** Full-role subscribers see all data; limited-role subscribers have sensitive fields stripped from payloads.
3. **IATA validation:** Topic IATA codes must be valid airport codes.
4. **No cross-publishing:** No client can publish to another publisher's topic namespace.

The upstream TypeScript broker implements this in `authorizePublish` and `authorizeSubscribe` callbacks.

## Decision Drivers

- Security: publishers must be cryptographically bound to their topic namespace.
- Flexibility: subscriber roles may expand in the future.
- Performance: authorization checks happen on every publish/subscribe, must be fast.
- Testability: ACL logic must be testable without an MQTT broker running.

## Considered Options

### Option A: ACL configuration file

Define allowed topic patterns in a configuration file, matched at runtime.

- **Pros:** Flexible, can change without recompilation.
- **Cons:** The core rule (pubkey must match topic) is dynamic and can't be expressed as a static pattern. Would need a custom DSL.

### Option B: Programmatic ACL in Rust

Implement ACL rules as Rust functions behind a trait, composing topic parsing, IATA validation, and identity checks.

- **Pros:** Type-safe, testable, fast (no pattern matching engine). The pubkey-binding rule is naturally expressed in code. IATA validation uses a compile-time `phf` set.
- **Cons:** Rule changes require recompilation.

### Option C: Hybrid (config for subscriber roles, code for publisher binding)

- **Pros:** Subscriber accounts are already in config; publisher rules are inherently programmatic.
- **Cons:** Two authorization models to maintain.

## Decision Outcome

**Chosen: Option B (programmatic ACL) with subscriber accounts from config**

The core authorization rules are inherently programmatic — "your pubkey must match the topic's pubkey segment" cannot be expressed as a static config pattern. We implement the `Authorizer` trait with pure functions, keeping rmqtt out of the authorization logic.

Topic parsing, IATA validation, and ACL decisions are separate functions composed by the `MeshcoreAuthorizer`, enabling independent testing and parallel development.

### ACL Rules Summary

| Client Type | Action | Rule |
|------------|--------|------|
| Publisher | Publish | Allow only if topic pubkey == client pubkey. Strip retain flag. |
| Publisher | Subscribe | Deny |
| Subscriber (Full) | Subscribe | Allow |
| Subscriber (Limited) | Subscribe | Allow (payload filtered at delivery) |
| Subscriber (any) | Publish | Deny |

## Consequences

- **Good:** ACL logic is pure Rust, fully testable without MQTT infrastructure.
- **Good:** `phf` set for IATA codes gives O(1) lookup with zero runtime cost.
- **Good:** Clear separation: topic parsing, IATA validation, and ACL are independent modules.
- **Bad:** Adding new authorization rules requires code changes and redeployment.
- **Mitigation:** The trait boundary allows swapping in a config-driven authorizer later if needed.
