# 0001: Use rmqtt as Embedded MQTT Broker

- **Status:** Accepted
- **Date:** 2026-03-17
- **Deciders:** Sam Schlegel

## Context

We need an MQTT broker engine for gomesh-broker that supports:

1. **Per-topic ACL** — publishers must only publish to their own pubkey-bound topics.
2. **Hook/plugin system** — we need to intercept authentication, publish, subscribe, and delivery events.
3. **Library embedding** — the broker must be embeddable in our Rust binary, not run as a separate process.

The upstream [meshcore-mqtt-broker](https://github.com/michaelhart/meshcore-mqtt-broker) uses Aedes (Node.js), which provides all three via its `authorizePublish`, `authorizeSubscribe`, `authenticate`, and `authorizeForward` hooks.

## Decision Drivers

- Must support topic-level authorization (not just client-level).
- Must support intercepting message delivery for payload filtering.
- Must be embeddable as a Rust library.
- Mature enough for production use.

## Considered Options

### Option A: rumqttd

[rumqttd](https://github.com/bytebeamio/rumqtt) is a popular Rust MQTT broker.

- **Pros:** Well-maintained, good documentation, async Rust.
- **Cons:** No topic-level authorization. The authorization model is client-level only. No hook system for intercepting publish/subscribe/delivery events. Would require forking and heavily modifying the codebase.

### Option B: rmqtt

[rmqtt](https://github.com/rmqtt/rmqtt) is a Rust MQTT broker with a plugin system.

- **Pros:** 30+ hook points covering authentication, publish, subscribe, delivery, and more. Per-topic ACL support. Plugin system allows embedding as a library. Active development.
- **Cons:** Smaller community than rumqttd. API may change between versions.

### Option C: Custom broker from scratch

Build an MQTT broker from scratch using mqtt-codec or similar.

- **Pros:** Full control over every aspect.
- **Cons:** Enormous effort. MQTT protocol compliance is complex (QoS, sessions, retained messages, will messages). Not justified when existing brokers exist.

## Decision Outcome

**Chosen: Option B (rmqtt)**

rmqtt's hook system maps directly to the Aedes hooks used in the upstream TypeScript broker. The per-topic ACL support means we don't need to implement topic authorization at the protocol level. The library embedding mode lets us ship a single binary.

## Consequences

- **Good:** Direct mapping from Aedes hooks to rmqtt hooks simplifies the port.
- **Good:** Per-topic ACL is built-in, reducing our authorization code to policy decisions.
- **Bad:** Smaller ecosystem means fewer community resources if we hit issues.
- **Risk:** rmqtt API changes between versions could require migration work.
- **Mitigation:** Pin rmqtt version, wrap all rmqtt interactions behind our own trait boundaries.
