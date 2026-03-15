# Task `00000010` - `implement framed duplex abstraction and transport selector`

## Summary

Implement the first shared transport layer that exposes framed duplex I/O and `QUIC`-preferred transport selection without embedding carrier logic into the secure-channel core.

## Motivation

The first proving slice should establish the seam that all later network code depends on: a transport-neutral framed interface and a selector that can attempt `QUIC`, fall back to `WSS`, and report the chosen carrier cleanly to the upper layers.

## Detailed Requirements / Acceptance Criteria

### A) Shared transport interfaces exist

- Introduce the initial Rust traits and types for framed duplex I/O, transport kind, transport target, and fallback reason.
- Keep these interfaces free of Quinn or WebSocket concrete types.
- Add local tests for framing-independent selector behavior where feasible.

### B) Transport selector skeleton exists

- Implement the first selector/control-flow skeleton for `QUIC`-first attempts and `WSS` fallback.
- Reflect the documented `secure-ready` and fallback semantics from task `00000007`.
- Keep cache and reprobe behavior scoped to the minimum needed for the first prototype.

### C) Secure-channel core remains decoupled

- Wire the shared abstractions so later Noise/trust work can operate on framed transports without transport-specific imports.
- Document any intentionally deferred behavior needed by task `00000012`.

## Task Dependencies

- backlog/tasks/task-00000005_define-rust-crate-boundaries-and-secure-channel-api.md
- backlog/tasks/task-00000007_define-transport-selection-and-fallback-policy.md
- backlog/tasks/task-00000008_write-transport-agnostic-v1-protocol-plus-quic-and-wss-bindings.md
- backlog/plans/plan-00000001_secure-channel-foundation.md

## Acceptance Closure

- [x] A) Shared transport interfaces exist.
  Evidence: `crates/core/src/transport.rs`, `crates/core/src/session.rs`, and
  `crates/core/src/selector.rs` now expose carrier-neutral framed I/O,
  fallback taxonomy, secure-ready artifacts, and the selector-facing
  `SecureReadyEvaluator` seam without importing Quinn or WebSocket concrete
  types into the core crate.
- [x] B) Transport selector skeleton exists.
  Evidence: `crates/core/src/descriptor.rs` now marks reprobe candidates
  explicitly, `crates/core/src/selector.rs` implements the first
  sequential `QUIC`-first / `WSS`-fallback control flow, and local selector
  tests cover live fallback, cached fallback, reprobe, exhausted fallback, and
  no-fallback inner failure behavior.
- [x] C) Secure-channel core remains decoupled.
  Evidence: task `00000011` can now plug into `SecureReadyEvaluator` and
  `SecureReadyArtifacts`, while task `00000012` remains responsible for
  replacing the mock connectors with concrete `QUIC` and `WSS` adapters.

## Implementation Notes

- Keep this slice narrow; transport adapters can still be mocked or skeletal as long as the shared seam is stable enough for the next prototype tasks.
- Completed by adding `crates/core/src/selector.rs`, extending
  `crates/core/src/session.rs` and `crates/core/src/error.rs`, updating the
  descriptor reprobe source in `crates/core/src/descriptor.rs`, and recording
  the task evidence in
  `backlog/docs/2026-03-15_framed-duplex-selector-implementation.md`.
- Verification: `cargo test -p secure-tunnel-core` and `mise run dev`.
- Independent review: Mendel reviewer findings were resolved by carrying
  secure-ready artifacts through the selector seam, preserving fallback reasons
  in terminal selection errors, and rejecting carrier-mismatched returned
  transports before task closure.
