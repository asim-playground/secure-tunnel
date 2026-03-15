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
- backlog/tasks/task-00000010_implement-framed-duplex-abstraction-and-transport-selector.md
- backlog/plans/plan-00000001_secure-channel-foundation.md

## Advisory Inputs

- backlog/tasks/task-00000009_define-udp-first-deployment-and-observability-requirements.md

## Implementation Notes

- Favor a minimal local proving slice over a broad implementation; the purpose is to validate the transport-neutral core before deeper adapter work.
- Keep failure classes compatible with the observability task, but do not block
  the core trust-verification prototype on the full deployment memo landing
  first.
