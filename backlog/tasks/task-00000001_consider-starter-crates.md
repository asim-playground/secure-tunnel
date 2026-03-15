# Task `00000001` - `consider starter crates`

## Summary

Evaluate the initial foundational crate choices that directly affect the v1 secure-channel design, `QUIC`/`WSS` transport split, and first implementation slices.

## Motivation

The repository is scaffolded, but the first implementation pass should make deliberate choices about the crates that shape the secure-channel core, transport bindings, transport-selection policy, trust verification, and first local validation loop.

## Detailed Requirements / Acceptance Criteria

### A) Crate shortlist exists

Produce a shortlist of candidate crates for the first implementation phase, including at least:

- TLS stack
- QUIC transport
- Noise framework support
- async runtime
- WebSocket transport
- transport abstraction support if a dedicated utility crate is warranted
- configuration / serialization for protocol messages or certificates
- test support needed for protocol and transport verification

### B) Decision criteria are explicit

Document the tradeoffs that will decide the first crate selections, such as maintenance health, interoperability, auditability, platform fit, UDP/mobile behavior, and whether a crate belongs in the secure-channel core or only in outer transport bindings.

### C) Initial recommendations are actionable

- Recommend a first-choice crate set for the Phase 2 prototype work across `QUIC`, `WSS`, shared transport policy, and secure-channel core.
- Call out which decisions can remain provisional versus which ones are needed before tasks `00000005`, `00000007`, and `00000008` finalize architecture and protocol seams.

## Task Dependencies

- backlog/docs/historical/2026-03-14_initial-research.md
- backlog/plans/plan-00000001_secure-channel-foundation.md

## Implementation Notes

- Created during repository bootstrap so crate selection work is represented in tracked backlog artifacts from day one.
- Later research now requires the shortlist to compare `quinn`-class `QUIC` options alongside `tokio-tungstenite`-class WebSocket options instead of evaluating WSS in isolation.
