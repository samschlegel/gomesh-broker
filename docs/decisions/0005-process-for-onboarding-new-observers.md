# 0005: Process for Onboarding New Observers

- **Status:** Proposed
- **Date:** 2026-03-19
- **Deciders:** Sam Schlegel

## Context

Observer (subscriber) accounts are currently defined as static entries in `config.toml` with Argon2id-hashed passwords. Adding a new observer requires an admin to edit the file and restart the broker. This works for a small number of known observers but does not scale as the mesh network grows.

We need a process for onboarding new observers that balances ease of onboarding against the risk of unauthorized access.

## Decision Drivers

- Must scale beyond manual config edits per observer.
- Must resist spoofing — unauthorized clients should not be able to self-register.
- Should work in areas where mesh coverage already exists.
- Should ideally work in areas where mesh coverage does not yet exist (bootstrapping new regions).
- Accountability: misbehaving observers should be traceable and revocable.

## Considered Options

### Option A: Mesh-heard auto-discovery

An observer that is heard on the mesh by an already-trusted observer is automatically granted an account. The broker trusts the existing observer's attestation.

- **Pros:** Simple to implement. No human intervention for regions with existing coverage. Naturally grows trust with the network's physical footprint.
- **Cons:** Cannot bootstrap in areas with no existing mesh coverage. Vulnerable to spoofing — an attacker near a trusted observer could impersonate a new one. Relies on secondary signals (source IP, movement patterns) for additional confidence, which adds complexity.

### Option B: Invite tree (vouching)

Existing observers can invite new observers. Each new account stores a reference to its sponsor. If an observer misbehaves, the broker bans the entire invite subtree (similar to lobste.rs).

- **Pros:** Provides an accountability chain — bad actors implicate their sponsors. Works in mesh-free areas since invites are not tied to radio contact. Tree-based revocation limits blast radius of compromised accounts.
- **Cons:** More complex to implement (persistent invite tree, subtree ban logic). Requires a storage backend beyond the current TOML config. Social dynamics — sponsors may be reluctant to invite if they bear revocation risk.

### Option C: Keep current manual config

Admin manually adds accounts to `config.toml` and restarts the broker.

- **Pros:** Simplest possible implementation — already works today. No new attack surface. Full admin control over who gets access.
- **Cons:** Does not scale. Requires broker restart for every new observer. Single bottleneck on the admin.

## Decision Outcome

No decision yet. This ADR captures the options under consideration. Option B (invite tree) provides the strongest accountability model, but its implementation cost is significant. Option A is appealing for organic growth but needs a solution for the bootstrap problem. A hybrid of A and B may be worth exploring.

## Consequences

- **Good:** Any dynamic option (A or B) removes the admin bottleneck for onboarding.
- **Good:** Option B's invite tree creates a natural audit trail for trust delegation.
- **Risk:** Dynamic registration increases attack surface compared to static config.
- **Risk:** Option A's spoofing vulnerability could allow unauthorized observers in dense areas.
- **Mitigation:** Regardless of option chosen, observer accounts should be revocable without broker restart.
- **Mitigation:** IP address logging and movement-pattern analysis can serve as secondary trust signals for any option.
