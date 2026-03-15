# Task `00000008` - `write transport-agnostic v1 protocol plus quic and wss bindings`

## Summary

Write the updated repo-local v1 protocol specification for one inner secure-channel core carried over both `QUIC` and `WSS`.

## Motivation

The earlier protocol task documented a solid WSS-first baseline, but the later research changed the transport direction. The repo now needs a superseding protocol artifact that keeps the inner Noise, trust, and session model stable while splitting carrier-specific behavior into separate `QUIC` and `WSS` bindings.

## Detailed Requirements / Acceptance Criteria

### A) Core protocol flow is transport-agnostic

- Specify the shared secure-channel lifecycle through outer-carrier establishment, framed transport creation, Noise handshake, trust verification, transport mode, session open, login, device auth, app messages, and encrypted close.
- Define the shared prologue, protocol versioning, service identity binding, handshake-hash use, and size limits.
- Incorporate the device-enrollment and known-device policy produced by task `00000006`.

### B) QUIC binding is specified

- Specify raw `QUIC` over UDP as the preferred outer carrier.
- Define ALPN, the single bidirectional stream rule for v1, frame encoding on that stream, and error/close mapping assumptions.
- Explicitly document the v1 exclusion of QUIC DATAGRAM and `0-RTT`.

### C) WSS binding remains available as fallback

- Specify the `WSS` URI shape, subprotocol string, binary-frame mapping, and close handshake expectations.
- Ensure the binding remains semantically aligned with the shared core protocol and `QUIC` path.
- Identify any invariants the first Rust implementation must satisfy across both carriers.

### D) Unresolved protocol inputs are assigned

- Decide whether the first public bootstrap/service descriptor should carry one logical service target with multiple carriers or distinct per-carrier targets.
- Define the protocol payload or envelope shape, if any, for optional App Attest evidence so task `00000005` does not need to invent it during architecture work.

## Task Dependencies

- backlog/docs/historical/2026-03-14_initial-research.md
- backlog/tasks/completed/task-00000003_define-threat-model-and-v1-protocol-decisions.md
- backlog/tasks/completed/task-00000004_write-v1-protocol-spec-for-wss-plus-noise.md
- backlog/tasks/completed/task-00000006_define-device-enrollment-and-known-device-policy.md
- backlog/tasks/task-00000007_define-transport-selection-and-fallback-policy.md
- backlog/plans/plan-00000001_secure-channel-foundation.md

## Implementation Notes

- This task should produce a new or clearly superseding protocol artifact rather than silently mutating the earlier WSS-only spec.
- Keep the inner protocol identical across carriers unless a later plan explicitly opts into carrier-specific identities.
