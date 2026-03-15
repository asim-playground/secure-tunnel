# Task `00000005` - `define rust crate boundaries and secure-channel api`

## Summary

Define the initial Rust crate and module responsibilities for the transport-agnostic secure-channel engine, transport selector, concrete `QUIC`/`WSS` bindings, and higher-level auth/session logic.

## Motivation

The research note now points toward a transport-agnostic secure-channel core with raw `QUIC` as the preferred outer carrier and `WSS` as the graceful fallback. Locking the architectural seams early will reduce churn when both bindings are implemented and keep transport-specific behavior from contaminating the protocol core.

## Detailed Requirements / Acceptance Criteria

### A) Architecture boundaries are documented

- Define which crate or module owns framing, Noise state, trust validation, transport selection/cache policy, `QUIC` binding, `WSS` binding, and account/device session logic.
- Ensure the secure-channel core is transport-agnostic and does not depend on Flutter, Quinn types, WebSocket types, or concrete socket types.
- Identify the first public APIs or traits needed to support a framed duplex abstraction plus carrier selection.
- Call out which behavior belongs in shared transport policy versus per-carrier adapters.

### B) Initial implementation order is defined

- Identify the smallest Rust implementation slices that can be built and verified locally for shared transport traits, transport-neutral Noise/trust, `QUIC`, and `WSS` fallback.
- Map those slices to follow-up backlog tasks.
- Call out any crate-generation or workspace changes that should wait until after the design is accepted.

### C) Transport-first constraints are preserved

- Keep v1 aligned with one reliable `QUIC` bidirectional stream and one WebSocket message lane, not transport-specific app semantics.
- Preserve the rule that `QUIC`/`WSS` choice is transport policy, not a separate security policy.
- Surface where graceful encrypted shutdown, fallback reasons, and coarse network cache state belong in the Rust API.
- Define the first config/API shape for service descriptors or carrier targets strongly enough that later transport-selector work is not blocked on unresolved ownership.

## Task Dependencies

- backlog/docs/historical/2026-03-14_initial-research.md
- backlog/docs/v1-transport-selection-and-fallback-policy.md
- backlog/docs/v1-core-protocol-quic-and-wss-bindings.md
- backlog/docs/v1-service-descriptor-and-bootstrap-config.md
- backlog/docs/v1-device-enrollment-and-known-device-policy.md
- backlog/tasks/completed/task-00000001_consider-starter-crates.md
- backlog/tasks/task-00000007_define-transport-selection-and-fallback-policy.md
- backlog/tasks/task-00000008_write-transport-agnostic-v1-protocol-plus-quic-and-wss-bindings.md
- backlog/tasks/completed/task-00000006_define-device-enrollment-and-known-device-policy.md
- backlog/plans/plan-00000001_secure-channel-foundation.md

## Advisory Inputs

- backlog/tasks/task-00000009_define-udp-first-deployment-and-observability-requirements.md

## Implementation Notes

- This task should end with concrete architecture notes, not broad brainstorming.
- The output should reflect the later QUIC-first research direction rather than the earlier WSS-only binding draft.
- Treat deployment and observability requirements as an input to the API shape,
  but not as a hard blocker for defining crate boundaries.
