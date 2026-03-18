# 0004: Module Structure for Parallel Work

- **Status:** Accepted
- **Date:** 2026-03-17
- **Deciders:** Sam Schlegel

## Context

gomesh-broker has four major functional areas: JWT authentication, subscriber authentication, topic authorization, and message filtering. We want to enable parallel development across these areas with minimal merge conflicts and clear ownership boundaries.

## Decision Drivers

- Multiple workstreams must be able to develop and test independently.
- Trait boundaries must be stable before integration work begins.
- Each module should have a clear, single responsibility.
- Integration seam (hooks/) should compose modules without business logic of its own.

## Considered Options

### Option A: Flat module structure

All logic in `src/` with files like `jwt.rs`, `acl.rs`, `filter.rs`.

- **Pros:** Simple, fewer directories.
- **Cons:** No grouping of related files. `auth` concerns spread across multiple unrelated files. Harder to reason about ownership.

### Option B: Domain-grouped modules with trait boundaries

Group by domain (`auth/`, `authz/`, `filter/`, `hooks/`) with trait interfaces between them.

- **Pros:** Clear ownership. Each module group can be developed and tested independently. The hooks/ module only depends on trait interfaces, not implementations. Merge conflicts are isolated to module boundaries.
- **Cons:** More directories and files upfront.

## Decision Outcome

**Chosen: Option B (domain-grouped modules with trait boundaries)**

The module structure enables five parallel workstreams:

| Stream | Files | Dependencies | Can develop independently? |
|--------|-------|-------------|--------------------------|
| A: JWT + Publisher Auth | `auth/jwt.rs`, `auth/publisher.rs` | `types` only | Yes — pure crypto |
| B: Subscriber Auth + Config | `auth/subscriber.rs`, `config.rs` | `types` only | Yes — config + argon2 |
| C: Topic Auth + IATA | `authz/topic.rs`, `authz/iata.rs`, `authz/acl.rs` | `types` only | Yes — pure string parsing |
| D: Message Filtering | `filter/mod.rs` | `serde_json` only | Yes — JSON transform |
| E: Hook Integration | `hooks/*` | Trait interfaces from A-D | After traits are stable |

### Key Design Decisions

1. **`types.rs` is the shared vocabulary.** All modules depend on `ClientIdentity`, `TopicParts`, etc. This file must be stable early.

2. **Traits live in module `mod.rs` files.** `Authenticator` in `auth/mod.rs`, `Authorizer` in `authz/mod.rs`. Implementations are in sub-files.

3. **`hooks/` is the only module that depends on rmqtt.** All other modules are pure Rust with no broker dependency, making them testable in isolation.

4. **`filter/` has zero internal dependencies.** It only uses `serde_json` — the simplest module to implement and test.

## Consequences

- **Good:** Streams A–D can develop in parallel with no coordination beyond the shared `types.rs`.
- **Good:** Each module has focused unit tests that run without MQTT infrastructure.
- **Good:** Integration testing only needs to cover the hooks/ composition layer.
- **Bad:** More boilerplate upfront (trait definitions, module re-exports).
- **Mitigation:** Stubs with `todo!()` bodies are created in the initial scaffold so all modules compile from day one.
