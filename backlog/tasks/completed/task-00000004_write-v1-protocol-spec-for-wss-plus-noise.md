# Task `00000004` - `write v1 core protocol spec and wss binding`

## Summary

Write the first repo-local protocol specification for Secure Tunnel’s transport-agnostic core channel plus the initial WSS binding.

## Motivation

The research note contains the core concepts, but implementation needs exact message sequencing, framing assumptions, trust validation steps, and post-handshake behavior without hardwiring transport details into the secure-channel core.

## Detailed Requirements / Acceptance Criteria

### A) Core protocol flow is specified

- Specify the transport-agnostic secure-channel lifecycle through framed records, Noise handshake, trust verification, transport mode, login, and device-auth flows.
- State how the prologue, protocol versioning, and service identity are bound into the inner channel.
- Incorporate the device-enrollment and known-device policy produced by task `00000006`.

### B) WSS binding and safety-sensitive rules are specified

- Specify how the WSS carrier maps onto the framed core protocol in v1.
- Specify the root-signed server-key validation path and rotation model.
- Specify v1 behavior for early data, replay prevention posture, and encrypted shutdown semantics.
- Identify the minimum test vectors or invariants the first Rust implementation must satisfy.

## Task Dependencies

- backlog/docs/historical/2026-03-14_initial-research.md
- backlog/tasks/task-00000003_define-threat-model-and-v1-protocol-decisions.md
- backlog/tasks/task-00000006_define-device-enrollment-and-known-device-policy.md
- backlog/plans/plan-00000001_secure-channel-foundation.md

## Implementation Notes

- Prefer a single self-contained core spec plus a concise WSS-binding section that future implementation tasks can cite directly.
- Crate-evaluation output from task `00000001` may inform library choices, but it is not a prerequisite for defining the protocol itself.
- Completed by writing `backlog/docs/historical/2026-03-14_v1-core-protocol-and-wss-binding.md`.
- The spec defines the framed record abstraction, `NX` handshake lifecycle, root-signed server-key validation path, post-handshake login/device flows, replay posture, encrypted shutdown, and the v1 `WSS` binding.
- Later doc-set governance work preserved that output as a historical baseline
  and moved the active normative successor to
  `backlog/docs/v1-core-protocol-quic-and-wss-bindings.md`.
