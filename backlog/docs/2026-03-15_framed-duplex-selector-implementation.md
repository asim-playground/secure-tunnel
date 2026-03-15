# 2026-03-15 Framed Duplex Selector Implementation

## Final Summary

Task `00000010` is complete. The core crate now exports a transport selector
that drives the descriptor connect plan through carrier connectors and a
transport-neutral secure-ready evaluator, records normalized fallback behavior,
preserves cache and reprobe posture, and forwards secure-ready artifacts needed
by task `00000011`. Local selector tests now cover unknown-network `QUIC`
success, live `WSS` fallback, cached fallback, reprobe, no-fallback inner
trust failure, exhausted fallback reporting, and returned-carrier mismatch
rejection.

## Summary

Working note for task `00000010`. Implement the first transport-selector
control flow on top of the existing transport-neutral descriptor and framed I/O
surface without introducing `QUIC`, WebSocket, or runtime-specific types into
the secure-channel core.

## Working Checklist

- [x] Add a selector-facing secure-ready seam and failure taxonomy that keeps
  later Noise and trust work transport-neutral.
- [x] Implement the `QUIC`-first, `WSS`-fallback selector skeleton with
  minimum cache and reprobe updates.
- [x] Add transport-neutral tests for unknown-network success, live fallback,
  cached fallback, reprobe, and no-fallback inner failure behavior.
- [x] Document deferred follow-up behavior for task `00000012`.

## Current Code Anchor

- `crates/core/src/transport.rs` already defines carrier-neutral framed duplex
  I/O, fallback reasons, and descriptor-facing transport targets.
- `crates/core/src/descriptor.rs` already computes ordered carrier candidates
  but does not run the selection state machine.
- `crates/core/src/session.rs` already defines `SecureReadyReport` and cache
  disposition reporting for higher layers.

## Evidence And Conclusions

- The active transport-policy doc defines success as `Secure Ready`, not outer
  carrier establishment alone.
- The current Rust surface now uses
  `CandidateSource::QuicReprobeAfterCachedFallback` in both descriptor planning
  and selector reporting, which keeps reprobe behavior aligned with the policy
  doc.
- `SecureReadyEvaluator` now returns `SecureReadyTransport`, so task
  `00000011` has a stable place to expose handshake-hash or channel-binding
  artifacts without widening the selector API again.
- The selector now preserves normalized fallback reasons in terminal exhausted
  errors and rejects carrier-mismatched returned transports before they can
  poison `SecureReadyReport` or cache state.
- `cargo test -p secure-tunnel-core` and `mise run dev` both pass after the
  selector implementation and the Mendel reviewer fix-up cycle.

## Next Actions

- Task `00000011` should implement the first `NX` handshake driver behind
  `SecureReadyEvaluator` and populate `SecureReadyArtifacts` with the actual
  handshake hash or equivalent channel binding.
- Task `00000012` should replace the mock connectors with concrete `QUIC` and
  `WSS` adapters while preserving the fallback and cache invariants proven here.
