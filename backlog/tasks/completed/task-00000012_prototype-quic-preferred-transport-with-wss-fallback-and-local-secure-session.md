# Task `00000012` - `prototype quic-preferred transport with wss fallback and local secure session`

## Final Summary

Task `00000012` is complete. `crates/core` now has test-only prototype QUIC and
WSS connectors plus a loopback Noise responder that prove QUIC selection,
WSS fallback, QUIC ALPN rejection, malformed WSS target handling, and inner
trust failure without fallback. The prototype connector path was later
refactored around a small per-attempt observation context so the connect flow
stays flat, records validation failures consistently, and satisfies the repo's
strict Clippy configuration. Verification passed with `cargo test -p
secure-tunnel-core`, `cargo fmt --all --check`, and `mise run dev`.

## Summary

Prototype the first local end-to-end secure session using raw `QUIC` as the
preferred carrier and `WSS` as the fallback path.

## Motivation

After the shared transport and inner secure-channel layers are proven
independently, this task needs one local slice that demonstrates the actual v1
user-facing transport story: prefer `QUIC`, fall back to `WSS` when needed, and
complete the same inner secure session over either carrier.

## Read-Write Repository

- Primary read-write repository: `/Users/asimi/workplace/secure-tunnel`
- Secondary read-write repository/repositories (if applicable): none
- State explicitly:
  - where code changes are expected to land: `crates/core` and possibly local
    transport-adapter modules created for this task.
  - this repository itself is expected to change: yes; no external source-of-truth
    code repository is being tracked here.

## Read-Only Reference Repository

- Read-only reference repository/repositories: none
- State explicitly which repositories may be inspected only for reference or legacy
  behavior.

## Detailed Requirements / Acceptance Criteria

### A) QUIC preferred path works locally

- [ ] Implement the first local `QUIC` binding sufficient to establish the outer
  carrier, create framed transport records on one bidirectional stream, and run the
  shared secure-channel flow.
- [ ] Keep the implementation aligned with the v1 exclusions around QUIC DATAGRAM and
  `0-RTT`.
- [ ] Record the selected carrier and key connection metrics needed by task
  `00000009`.

### B) WSS fallback path works locally

- [ ] Implement the first local `WSS` binding sufficient to carry the same framed
  secure-channel flow.
- [ ] Demonstrate a fallback path from failed or budget-expired `QUIC` attempt to
  working `WSS`.
- [ ] Verify that upper layers do not need carrier-specific security behavior to
  complete the session.

### C) Minimum validation covers both paths

- [ ] Add local tests or harness checks for successful `QUIC`, forced `WSS` fallback,
  and inner trust failure distinctions.
- [ ] Keep failure reporting compatible with the transport-selection and
  observability docs.
- [ ] Document any deferred gaps before broader rollout.

## Cross-Repo Boundaries

- Primary implementation boundary:
  - transport adapters, selector wiring, and protocol execution for this slice.
- Parser / upstream dependency boundary: none currently.
- Downstream integration boundary:
  - language bindings / CLI are expected to consume the resulting selection + session
    primitives without carrying adapter-specific transport semantics.
- External asset / catalog / fixture boundary: none.
- If another repository is read-write, state what is implemented there versus what
  is implemented in this repository.

## Task Dependencies

- backlog/tasks/task-00000005_define-rust-crate-boundaries-and-secure-channel-api.md
- backlog/tasks/task-00000008_write-transport-agnostic-v1-protocol-plus-quic-and-wss-bindings.md
- backlog/tasks/task-00000009_define-udp-first-deployment-and-observability-requirements.md
- backlog/tasks/completed/task-00000010_implement-framed-duplex-abstraction-and-transport-selector.md
- backlog/tasks/task-00000011_prototype-server-auth-noise-handshake-and-trust-verification-on-transport-neutral-frames.md
- backlog/plans/plan-00000001_secure-channel-foundation.md

## Reference Tasks

- backlog/tasks/task-00000009_define-udp-first-deployment-and-observability-requirements.md

## Backlog Bookkeeping

- `backlog/` is local planning state and is not version controlled by default.
- Moving a task to `backlog/tasks/completed/` is separate from pushing code.
- Commit history for work tracked by this task may live in a different read-write
  repository than this backlog entry.

## Implementation Notes

- Prototype transport adapters were added in `crates/core/src/prototype_transport.rs`
  as a test-only harness that keeps the QUIC/WSS slice local to `secure-tunnel-core`.
- The connector `connect` flow was flattened around helper methods plus a
  per-attempt observation context so carrier validation, plan execution, and
  observation recording no longer duplicate nested match/return paths.
- The harness now exercises five cases end-to-end:
  - successful QUIC selection and encrypted application messaging
  - QUIC outer-path fallback to WSS
  - QUIC ALPN rejection surfaced as `outer_quic_rejected` with recorded connector metrics
  - malformed WSS target rejection with recorded connector metrics
  - inner trust failure that does not fall back to WSS
- Verification commands:
  - `cargo test -p secure-tunnel-core`
  - `cargo fmt --all --check`
  - `mise run dev`

## Implementation Plan

1. Implement a concrete `QUIC` carrier connector that maps real `QUIC` outcomes into
   transport selection failures/success, preserving the fallback behavior and
   failure taxonomy defined for task `00000010` and task `00000009`.
2. Implement a concrete `WSS` carrier connector using the documented `WSS` subprotocol
   and carrier-independent framed I/O contract.
3. Add a local end-to-end harness that executes the selector over both adapters and
   proves successful `QUIC`, forced fallback to `WSS`, and inner-trust-failure
   non-fallback semantics.

## Review Notes

- Focused follow-up review after the connector refactor found no remaining
  correctness or validation gaps in `crates/core/src/prototype_transport.rs`.

## Acceptance Closure

- [x] A) QUIC preferred path works locally.
- [x] B) WSS fallback path works locally.
- [x] C) Minimum validation covers both paths.
- [x] Verification commands and outcomes are recorded.
- [x] No unresolved high/medium findings remain.
