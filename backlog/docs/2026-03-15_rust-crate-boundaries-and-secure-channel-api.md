# 2026-03-15 Rust Crate Boundaries And Secure-Channel API

## Summary

This working note closes task `00000005` by fixing the first Rust ownership
boundaries for the v1 secure-channel stack and by defining the initial public
API surface that later implementation tasks can target without importing
`quinn`, WebSocket types, Flutter types, or concrete sockets into the
secure-channel core.

## Working Checklist

- [x] Assign crate or module ownership for framing, Noise, trust validation,
  transport policy, `QUIC`, `WSS`, and session logic.
- [x] Define the first public Rust API for framed duplex I/O, service
  descriptors, transport candidates, fallback reasons, coarse cache posture,
  secure-ready reporting, and graceful encrypted close.
- [x] Split shared transport policy from per-carrier adapter behavior.
- [x] Map the smallest implementation slices to follow-up backlog tasks.
- [x] Record which workspace changes should wait until the architecture is
  accepted.

## Current Code Anchor

The accepted API lives in the existing
`secure-tunnel-core` crate for now:

- `crates/core/src/lib.rs` exports the stable public surface.
- `crates/core/src/descriptor.rs` owns the first descriptor and transport-plan
  shape.
- `crates/core/src/transport.rs` owns the framed duplex and carrier connector
  traits plus carrier-neutral transport taxonomy.
- `crates/core/src/session.rs` owns secure-ready and graceful-close reporting.

This keeps the first transport-neutral contracts available immediately without
forcing a premature multi-crate split while the rest of the repo is still
bootstrap scaffolding. The current `descriptor` and `transport` modules are
still intentionally colocated, so this is not yet the final crate seam.

## Intended Crate Boundaries

### `secure-tunnel-core`

Owns:

- framed record semantics and size limits
- Noise handshake and transport state machine
- prologue construction from the service descriptor
- trust-anchor validation and server-key authorization checks
- secure-ready state transitions
- encrypted close semantics and channel-binding outputs

Must not own:

- `quinn` types
- WebSocket stream types
- runtime-specific network sockets
- fallback cache persistence policy
- account login or device-enrollment business logic

### `secure-tunnel-transport`

Owns:

- carrier selection order
- coarse network cache posture
- fallback reason normalization
- selector reporting such as selected carrier, cached-fallback use, and reprobe
  posture
- mapping from one logical service descriptor to ordered carrier attempts

Must not own:

- Noise state
- trust-anchor verification
- account or device session semantics
- carrier-specific handshake mechanics

### `secure-tunnel-transport-quic`

Owns:

- raw `QUIC` connection establishment
- ALPN confirmation
- one reliable bidirectional stream for v1
- conversion between stream reads or writes and framed duplex records
- `QUIC` close and early-close mapping into shared fallback taxonomy

### `secure-tunnel-transport-ws`

Owns:

- `WSS` connection establishment
- WebSocket subprotocol confirmation
- binary-message mapping to framed duplex records
- WebSocket close mapping into shared transport outcomes

### `secure-tunnel-session`

Owns:

- account login, recovery, and server session open flows
- device enrollment and known-device challenge or response messages
- session freshness policy hooks above `Secure Ready`
- policy-facing interpretation of `SecureReadyReport`

Must not own:

- carrier selection
- concrete network adapters
- Noise transcript processing internals

## First Public Rust API

The first API is intentionally narrow:

- `ServiceDescriptor`, `SelectionPolicy`, `CarrierSet`, `QuicTarget`,
  `WssTarget`, and `TrustAnchor` define one logical service with per-carrier
  targets.
- `ServiceDescriptor::validate()` owns descriptor-shape checks that are stable
  before cryptographic verification is implemented.
- `ServiceDescriptor::connect_plan(...)` owns transport-order planning only.
  It does not open sockets or run the selector state machine.
- `CarrierKind`, `TransportTarget`, `TransportCandidate`,
  `TransportCacheSnapshot`, and `FallbackReason` define the transport policy
  seam.
- `FramedDuplex` and `CarrierConnector` define the transport-neutral I/O seam
  that later tasks can mock locally or back with real adapters.
- `SessionPhase`, `SecureReadyReport`, `CacheDisposition`, and
  `CloseDirective` define the first core-to-session reporting seam.

## Shared Policy Vs Adapter Behavior

Shared transport policy owns:

- sequential `QUIC`-first attempt ordering
- coarse cache posture for `QUIC`-bad networks
- normalized fallback reasons
- whether a connection used live probing, cached fallback, or reprobe posture

Per-carrier adapters own:

- socket creation
- TLS and outer handshake details
- ALPN or subprotocol confirmation
- outer close or error conversion into shared fallback classes
- the byte-level conversion between concrete carrier I/O and framed records

This preserves the v1 rule that `QUIC` versus `WSS` is transport policy, not a
separate security policy.

## Graceful Shutdown, Fallback, And Cache Ownership

- Graceful encrypted shutdown belongs in the secure-channel core API through
  `CloseDirective` and later encrypted close message handling.
- Fallback reasons belong in shared transport policy through
  `FallbackReason` and `SecureReadyReport`.
- Coarse network cache state belongs in shared transport policy through
  `TransportCacheSnapshot`, not in trust verification or session logic.

## Smallest Implementation Slices

### Task `00000010`

Build:

- the selector state machine on top of `ServiceDescriptor::connect_plan(...)`
- an in-memory cache implementation for `TransportCacheSnapshot`
- mock `CarrierConnector` and `FramedDuplex` test doubles

Verify locally:

- unknown-network `QUIC` first ordering
- cached `QUIC`-bad network `WSS` first ordering
- reprobe after cache expiry

### Task `00000011`

Build:

- Noise `NX` handshake driver over `FramedDuplex`
- prologue construction from `ServiceDescriptor`
- trust-anchor and server-key authorization validation
- secure-ready and encrypted-close reporting

Verify locally:

- transport-neutral handshake success
- inner trust failure without fallback
- handshake-hash output availability for later session work

### Task `00000012`

Build:

- one real `QUIC` carrier adapter
- one real `WSS` carrier adapter
- selector integration with fallback reporting
- one local end-to-end secure session over either carrier

Verify locally:

- `QUIC` success
- forced `WSS` fallback
- inner trust failure separation from outer fallback reasons

## Deferred Workspace Changes

Do not create the dedicated transport, `QUIC`, `WSS`, or session crates yet.
Wait until:

- task `00000010` confirms the selector and framed-duplex traits are stable
- task `00000011` confirms the Noise and trust surface is stable
- task `00000012` proves the same API works across both real carriers

At that point the workspace split should be lower-risk, but not purely
mechanical, because descriptor planning and transport taxonomy are still
intentionally colocated in one crate today:

1. move transport-neutral cryptographic logic into `secure-tunnel-core`
2. move selector and cache logic into `secure-tunnel-transport`
3. move concrete adapters into `secure-tunnel-transport-quic` and
   `secure-tunnel-transport-ws`
4. add `secure-tunnel-session` only when account and device flows are ready to
   stop being protocol placeholders

## Evidence And Conclusions

- The active v1 docs already fix the shared protocol, transport policy, service
  descriptor shape, and device policy.
- The new Rust surface in `crates/core/src/*.rs` matches those docs while
  keeping carrier-specific dependencies out of the core API.
- The repository keeps a temporary parser compatibility shim only to avoid
  mixing this architecture task with unrelated language-binding regeneration.

## Next Actions

- Implement task `00000010` directly against the exported descriptor and
  transport modules.
- Keep later tasks from adding `quinn` or WebSocket imports to the secure-core
  modules.
- Remove the parser compatibility shim once binding surfaces are regenerated
  around real secure-channel APIs.
