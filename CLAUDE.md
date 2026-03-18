# gomesh-broker

MeshCore MQTT broker in Rust. Replicates the auth/authorization logic from
[meshcore-mqtt-broker](https://github.com/michaelhart/meshcore-mqtt-broker) (TypeScript/Aedes)
using rmqtt as the embedded MQTT engine.

## Build & Test

```bash
cargo check          # Type-check without building
cargo build          # Build debug binary
cargo test           # Run all tests
cargo clippy         # Lint
```

## Architecture

```
main.rs ──► hooks/ ──► auth/  (Authenticator trait)
                   ├── authz/ (Authorizer trait)
                   └── filter/ (payload filtering)

config.rs ◄── loaded by main, passed to auth/subscriber
types.rs  ◄── shared value types used by all modules
```

- **auth/**: MQTT client authentication. Publishers use Ed25519 JWT (`v1_{PUBKEY}` username).
  Subscribers use static accounts with Argon2id password hashing.
- **authz/**: Topic authorization. Publishers can only publish to their own pubkey topics.
  Subscribers access controlled by role. IATA codes validated against a known set.
- **filter/**: Payload filtering for limited-role subscribers. Strips SNR, RSSI, score, etc.
- **hooks/**: rmqtt integration layer. Composes auth + authz + filter into hook handlers.
- **config.rs**: TOML configuration loading (listener address, subscriber accounts, regions).
- **types.rs**: Shared types (`ClientIdentity`, `TopicParts`, `TopicAction`, `SubscriberRole`).

## Module Dependencies

Modules are designed for parallel development:

| Module | Dependencies | Can develop independently? |
|--------|-------------|--------------------------|
| `auth/jwt.rs`, `auth/publisher.rs` | `types` | Yes — pure crypto |
| `auth/subscriber.rs` | `types`, `config` | Yes — config + argon2 |
| `authz/topic.rs`, `authz/iata.rs`, `authz/acl.rs` | `types` | Yes — pure string parsing |
| `filter/` | none (only serde_json) | Yes — JSON transform |
| `hooks/` | all of the above | After trait interfaces stable |

## ADRs

Architecture Decision Records are in `docs/decisions/`:

- [0001: Use rmqtt as embedded broker](docs/decisions/0001-use-rmqtt-as-embedded-broker.md)
- [0002: Ed25519 JWT authentication](docs/decisions/0002-ed25519-jwt-authentication.md)
- [0003: Topic authorization and ACL](docs/decisions/0003-topic-authorization-and-acl.md)
- [0004: Module structure for parallel work](docs/decisions/0004-module-structure-for-parallel-work.md)
