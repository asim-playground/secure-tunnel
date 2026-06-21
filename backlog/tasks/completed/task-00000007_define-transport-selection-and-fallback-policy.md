# Task `00000007` - `define transport selection and fallback policy`

## Final Summary

Task `00000007` is complete. The active normative transport policy now defines
v1 `QUIC`-first selection, sequential fallback to `WSS`, `Secure Ready`,
fallback eligibility, cache semantics, higher-layer reporting, observability
minimums, and explicit v1 exclusions for concurrent racing, `QUIC` `0-RTT`,
and QUIC DATAGRAM.

## Summary

Define the v1 client-side transport-selection policy with raw `QUIC`
preferred, `WSS` as fallback, and one shared inner security model.

## Motivation

The later research clarified that transport choice is now a first-class design
decision rather than an implementation detail. The repo needs an explicit
definition of how clients choose `QUIC`, when they fall back to `WSS`, what
counts as `Secure Ready`, and how cached transport outcomes should influence
later connection attempts.

## Read-Write Repository

- Primary read-write repository: `/Users/asimi/workplace/secure-tunnel`
- Secondary read-write repository/repositories (if applicable): none
- State explicitly:
  - documentation and backlog changes land in this repository.
  - no external repository is modified for this task.

## Read-Only Reference Repository

- Read-only reference repository/repositories: none
- State explicitly which repositories may be inspected only for reference or
  legacy behavior:
  - historical backlog docs under `backlog/docs/historical/` are reference
    material, not active source of truth.

## Detailed Requirements / Acceptance Criteria

### A) Selection algorithm is explicit

- [x] Define the v1 connect sequence for unknown and known-good networks.
- [x] Define whether attempts are sequential or concurrent and explain why.
- [x] Define the timeout or budget concept for the initial `QUIC` attempt
      without hard-coding platform-specific values prematurely.

### B) Fallback and cache semantics are explicit

- [x] Define what conditions trigger fallback from `QUIC` to `WSS`.
- [x] Define what counts as `Secure Ready` for the purpose of considering a
      transport attempt successful.
- [x] Define what state may be cached per service and coarse network class,
      plus the decay or reprobe rule.
- [x] Define how fallback reasons should be surfaced to higher layers and
      observability.

### C) Security and correctness constraints are preserved

- [x] Keep `QUIC`/`WSS` selection framed as transport policy, not a second
      security design.
- [x] Preserve one inner Noise protocol, one trust model, and one
      post-handshake auth model across both carriers.
- [x] Explicitly reject `QUIC` `0-RTT`, QUIC DATAGRAM, and duplicate-session
      racing for v1.

## Cross-Repo Boundaries

- Primary implementation boundary:
  - active policy documentation and backlog bookkeeping in `secure-tunnel`.
- Parser / upstream dependency boundary: none.
- Downstream integration boundary:
  - Rust selector, SDK facade, and observability work must consume this policy
    without redefining fallback semantics.
- External asset / catalog / fixture boundary: none.
- If another repository is read-write, state what is implemented there versus
  what is implemented in this repository:
  - none.

## Task Dependencies

- backlog/docs/historical/2026-03-14_initial-research.md
- backlog/tasks/completed/task-00000003_define-threat-model-and-v1-protocol-decisions.md
- backlog/tasks/completed/task-00000004_write-v1-protocol-spec-for-wss-plus-noise.md
- backlog/tasks/completed/task-00000006_define-device-enrollment-and-known-device-policy.md
- backlog/plans/plan-00000001_secure-channel-foundation.md

## Reference Tasks

- backlog/tasks/completed/task-00000010_implement-framed-duplex-abstraction-and-transport-selector.md
- backlog/tasks/completed/task-00000012_prototype-quic-preferred-transport-with-wss-fallback-and-local-secure-session.md

## Backlog Bookkeeping

- `backlog/` is local planning state and is not version controlled by default.
- Moving a task to `backlog/tasks/completed/` is separate from pushing code.
- Commit history for work tracked by this task may live in a different
  read-write repository than this backlog entry.

## Implementation Notes

- [x] Implementation notes added with command evidence.
- Normative deliverable:
  `backlog/docs/v1-transport-selection-and-fallback-policy.md`.
- The doc defines:
  - sequential `QUIC`-first attempts with short `QUIC` budget
  - cached `QUIC`-bad network behavior and reprobe rule
  - `Secure Ready` as carrier established, selector confirmed, framed channel
    available, Noise handshake complete, server-key authorization valid, and
    Noise transport mode entered
  - fallback classes limited to outer carrier failures
  - no fallback on inner Noise, inner trust, login, or device-auth failures
  - higher-layer report fields and minimum counters
- Rust vocabulary check:
  - `CarrierKind`, `CandidateSource`, `TransportCacheSnapshot`, and
    `FallbackReason` in `crates/core/src/transport.rs` match the policy shape.
  - `SecureReadyReport`, `CacheDisposition`, and `SessionPhase` in
    `crates/core/src/session.rs` expose the expected higher-layer state.
- Verification commands:
  - `mise run markdown-lint` passed.
  - `mise run dev` passed, including format, lint, Rust tests, doctests,
    Python tests, Go tests, and Go/WASM tests.

## Implementation Plan

1. Audit the active transport policy against task acceptance criteria.
2. Check Rust-facing names and constants for consistency with the policy.
3. Refresh this task into the current template, record evidence, and move it
   to completed.
4. Run repository verification and independent review with tasks `00000008`
   and `00000009`.

## Review Notes

- First independent review for the combined tasks `00000007`, `00000008`, and
  `00000009` closure slice found two medium consistency findings: pending
  review closure text in completed tasks and stale deployment/observability gap
  analysis in `plan-00000001`.
- Fixups updated the task review closure text, the `plan-00000001` gap
  analysis, the plan `00000002` managed-network dependency table, and the
  task `00000022` redaction requirement.
- Focused re-review found no remaining high/medium findings in the task
  `00000007`/`00000008`/`00000009` closure scope.

## Acceptance Closure

- [x] All acceptance criteria are satisfied and marked.
- [x] Verification commands and outcomes are recorded.
- [x] No unresolved high/medium findings remain.
