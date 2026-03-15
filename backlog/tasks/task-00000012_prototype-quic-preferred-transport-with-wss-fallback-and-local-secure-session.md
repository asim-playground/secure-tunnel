# Task `00000012` - `prototype quic-preferred transport with wss fallback and local secure session`

## Summary

Prototype the first local end-to-end secure session using raw `QUIC` as the preferred carrier and `WSS` as the fallback path.

## Motivation

After the shared transport and inner secure-channel layers are proven independently, the repo needs one local slice that demonstrates the actual v1 user-facing transport story: prefer `QUIC`, fall back to `WSS` when needed, and complete the same inner secure session over either carrier.

## Detailed Requirements / Acceptance Criteria

### A) QUIC preferred path works locally

- Implement the first local `QUIC` binding sufficient to establish the outer carrier, create framed transport records on one bidirectional stream, and run the shared secure-channel flow.
- Keep the implementation aligned with the v1 exclusions around QUIC DATAGRAM and `0-RTT`.
- Record the selected carrier and key connection metrics needed by task `00000009`.

### B) WSS fallback path works locally

- Implement the first local `WSS` binding sufficient to carry the same framed secure-channel flow.
- Demonstrate a fallback path from failed or budget-expired `QUIC` attempt to working `WSS`.
- Verify that upper layers do not need carrier-specific security behavior to complete the session.

### C) Minimum validation covers both paths

- Add local tests or harness checks for successful `QUIC`, forced `WSS` fallback, and inner trust failure distinctions.
- Keep failure reporting compatible with the transport-selection and observability docs.
- Document any deferred gaps before broader rollout.

## Task Dependencies

- backlog/tasks/task-00000005_define-rust-crate-boundaries-and-secure-channel-api.md
- backlog/tasks/task-00000008_write-transport-agnostic-v1-protocol-plus-quic-and-wss-bindings.md
- backlog/tasks/task-00000009_define-udp-first-deployment-and-observability-requirements.md
- backlog/tasks/completed/task-00000010_implement-framed-duplex-abstraction-and-transport-selector.md
- backlog/tasks/task-00000011_prototype-server-auth-noise-handshake-and-trust-verification-on-transport-neutral-frames.md
- backlog/plans/plan-00000001_secure-channel-foundation.md

## Implementation Notes

- Keep this prototype narrow and locally verifiable; mobile integration and production-hardening belong to later tasks.
