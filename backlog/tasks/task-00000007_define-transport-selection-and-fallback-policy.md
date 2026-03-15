# Task `00000007` - `define transport selection and fallback policy`

## Summary

Define the v1 client-side transport-selection policy with raw `QUIC` preferred, `WSS` as fallback, and one shared inner security model.

## Motivation

The later research clarified that transport choice is now a first-class design decision rather than an implementation detail. The repo needs an explicit definition of how clients choose `QUIC`, when they fall back to `WSS`, what counts as `secure-ready`, and how cached transport outcomes should influence later connection attempts.

## Detailed Requirements / Acceptance Criteria

### A) Selection algorithm is explicit

- Define the v1 connect sequence for unknown and known-good networks.
- Define whether attempts are sequential or concurrent and explain why.
- Define the timeout or budget concept for the initial `QUIC` attempt without hard-coding platform-specific values prematurely.

### B) Fallback and cache semantics are explicit

- Define what conditions trigger fallback from `QUIC` to `WSS`.
- Define what counts as `secure-ready` for the purpose of considering a transport attempt successful.
- Define what state may be cached per service and coarse network class, plus the decay or reprobe rule.
- Define how fallback reasons should be surfaced to higher layers and observability.

### C) Security and correctness constraints are preserved

- Keep `QUIC`/`WSS` selection framed as transport policy, not a second security design.
- Preserve one inner Noise protocol, one trust model, and one post-handshake auth model across both carriers.
- Explicitly reject `QUIC` `0-RTT`, QUIC DATAGRAM, and duplicate-session racing for v1.

## Task Dependencies

- backlog/docs/historical/2026-03-14_initial-research.md
- backlog/tasks/completed/task-00000003_define-threat-model-and-v1-protocol-decisions.md
- backlog/tasks/completed/task-00000004_write-v1-protocol-spec-for-wss-plus-noise.md
- backlog/tasks/completed/task-00000006_define-device-enrollment-and-known-device-policy.md
- backlog/plans/plan-00000001_secure-channel-foundation.md

## Implementation Notes

- This task should produce a concrete repo-local policy doc, not just notes in the plan.
- Use the later research section that prefers short-budget sequential `QUIC` attempts over dual-handshake racing as the baseline unless new evidence contradicts it.
