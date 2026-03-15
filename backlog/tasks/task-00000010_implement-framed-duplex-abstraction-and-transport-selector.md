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

## Implementation Notes

- Keep this slice narrow; transport adapters can still be mocked or skeletal as long as the shared seam is stable enough for the next prototype tasks.
