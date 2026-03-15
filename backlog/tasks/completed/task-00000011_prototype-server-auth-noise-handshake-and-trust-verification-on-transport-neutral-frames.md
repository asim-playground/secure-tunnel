# Task `00000011` - `prototype server-auth noise handshake and trust verification on transport-neutral frames`

## Summary

Prototype the inner `NX` secure-channel flow and trust verification against the shared framed transport abstraction instead of a concrete carrier.

## Motivation

The protocol’s core risk is in the inner security model, not the outer carrier. Before investing deeply in both transport adapters, the repo should prove that the Noise handshake, trust-anchor validation, handshake-hash binding, and encrypted close flow work cleanly over transport-neutral framed I/O.

## Detailed Requirements / Acceptance Criteria

### A) Inner handshake path works on framed transports

- Implement a local path that drives the `NX` handshake and transition into Noise transport mode over a mock or test framed transport.
- Verify the prologue, protocol version, and service-identity binding assumptions from the protocol docs.
- Keep login/device steps either stubbed or minimally exercised as needed to prove the interface.

### B) Trust verification path is exercised

- Implement or stub the shipped trust-anchor plus server-key authorization
  verification path strongly enough to validate the API shape.
- Add tests that distinguish outer transport success from inner trust failure.
- Preserve the rule that the same trust model applies to both `QUIC` and `WSS`.

### C) Shutdown and binding invariants are covered

- Exercise the encrypted close path at least in a local harness.
- Expose the handshake hash or channel binding output needed for later session/device work.
- Keep the implementation aligned with the docs produced by task `00000008`
  and with the failure taxonomy from task `00000009` when that advisory input
  is available.

## Task Dependencies

- backlog/tasks/task-00000005_define-rust-crate-boundaries-and-secure-channel-api.md
- backlog/tasks/task-00000008_write-transport-agnostic-v1-protocol-plus-quic-and-wss-bindings.md
- backlog/tasks/completed/task-00000010_implement-framed-duplex-abstraction-and-transport-selector.md
- backlog/plans/plan-00000001_secure-channel-foundation.md

## Advisory Inputs

- backlog/tasks/task-00000009_define-udp-first-deployment-and-observability-requirements.md

## Acceptance Closure

- [x] A) Inner handshake path works on framed transports.
  Evidence: `crates/core/src/noise.rs` now drives the client side of
  `Noise_NX_25519_ChaChaPoly_BLAKE2s` over `FramedDuplex`,
  `crates/core/src/descriptor.rs` builds the canonical descriptor-derived
  prologue, and local tests prove secure-ready success, exact prologue bytes,
  and descriptor validation on the direct proving path.
- [x] B) Trust verification path is exercised.
  Evidence: `crates/core/src/trust.rs` now decodes and verifies
  `server_key_authorization_v1` against shipped Ed25519 trust anchors,
  validity windows, descriptor-bound identity fields, and the authenticated
  responder static key, while tests prove bad authorization rejection and
  no-fallback behavior on inner trust failure with the real evaluator.
- [x] C) Shutdown and binding invariants are covered.
  Evidence: `crates/core/src/noise.rs` exports the final handshake hash `h`
  into `SecureReadyArtifacts`, wraps the surviving transport in Noise
  transport mode, proves encrypted close before outer close, and normalizes
  handshake-time `QUIC` close into the documented fallback path with a local
  selector test.

## Implementation Notes

- Favor a minimal local proving slice over a broad implementation; the purpose is to validate the transport-neutral core before deeper adapter work.
- Keep failure classes compatible with the observability task, but do not block
  the core trust-verification prototype on the full deployment memo landing
  first.
- Completed by adding `crates/core/src/noise.rs`, `crates/core/src/trust.rs`,
  and `crates/core/src/codec.rs`, extending `crates/core/src/descriptor.rs`,
  `crates/core/src/lib.rs`, and the selector evaluator seam in
  `crates/core/src/selector.rs`, and updating dependencies in
  `crates/core/Cargo.toml`.
- Verification: `cargo test -p secure-tunnel-core` and `mise run dev`.
- Independent review: the first reviewer pass identified evaluator-context,
  QUIC-close normalization, prologue-coverage, and direct-descriptor-validation
  gaps; those were fixed before a final reviewer pass reported no findings.
