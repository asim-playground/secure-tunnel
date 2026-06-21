# Task `00000018` - `define product sdk facade and session contract`

## Summary

Define the Rust-facing product SDK facade that native bindings will call.

## Motivation

UniFFI should expose a stable, coarse SDK contract rather than internal core
types. The repository needs a Rust facade for client configuration, descriptor
loading, connection, session state, cancellation, application messages, and
errors before generated Swift, Kotlin, or Python bindings are introduced.

## Read-Write Repository

- Primary read-write repository: `/Users/asimi/workplace/secure-tunnel`
- Secondary read-write repository/repositories (if applicable): none
- State explicitly:
  - code changes are expected to land in the Rust workspace, likely as a new
    facade crate or a narrow facade module that depends on `secure-tunnel-core`.
  - this repository itself is expected to change.

## Read-Only Reference Repository

- Read-only reference repository/repositories: none
- State explicitly which repositories may be inspected only for reference or
  legacy behavior:
  - none.

## Detailed Requirements / Acceptance Criteria

### A) SDK facade is explicit

- [ ] Define the first Rust SDK contract for client configuration, descriptor
      input, transport policy input, connect, session, send/receive or request
      operations, and close.
- [ ] Use owned records, strings, byte arrays, explicit error enums, and opaque
      stateful objects suitable for UniFFI.
- [ ] Keep internal selector, Noise, trust, and carrier adapter types out of the
      public SDK contract.

### B) Async, cancellation, and errors are decided

- [ ] Decide which operations are async in the Rust facade and how cancellation
      is represented for foreign callers.
- [ ] Define a stable error taxonomy that preserves outer path/TLS/proxy,
      fallback, inner trust, auth, and close distinctions.
- [ ] Define what observability/report records foreign callers can receive
      without exposing implementation internals.

### C) The contract is tested and documented

- [ ] Add rustdoc for all public facade types and methods.
- [ ] Add mock-backed tests that prove the facade can run through descriptor
      validation, connect planning, session state transitions, and close.
- [ ] `mise run dev` passes.

## Cross-Repo Boundaries

- Primary implementation boundary: Rust SDK facade only.
- Parser / upstream dependency boundary: no parser or dependency migration is
  expected.
- Downstream integration boundary: do not generate UniFFI bindings yet; this
  task prepares the API they will expose.
- External asset / catalog / fixture boundary: no external assets expected.
- If another repository is read-write, state what is implemented there versus
  what is implemented in this repository.
  - none.

## Task Dependencies

- backlog/tasks/completed/task-00000007_define-transport-selection-and-fallback-policy.md
- backlog/tasks/completed/task-00000008_write-transport-agnostic-v1-protocol-plus-quic-and-wss-bindings.md
- backlog/tasks/completed/task-00000009_define-udp-first-deployment-and-observability-requirements.md
- backlog/tasks/task-00000017_decompose-core-modules-before-sdk-expansion.md
- backlog/plans/plan-00000002_product-secure-tunnel-sdk-and-bindings.md

## Reference Tasks

- backlog/tasks/completed/task-00000016_update-runtimes-deps-and-add-swift-callable-library-surface.md

## Backlog Bookkeeping

- `backlog/` is local planning state and is not version controlled by default.
- Moving a task to `backlog/tasks/completed/` is separate from pushing code.
- Commit history for work tracked by this task may live in a different
  read-write repository than this backlog entry.

## Implementation Notes

- [ ] Implementation notes added with command evidence.
- (fill in after completion)

## Implementation Plan

1. Draft the SDK facade types and state model from the active v1 docs.
2. Implement the smallest mock-backed facade path without real network I/O.
3. Add tests for success and key failure classifications.
4. Run `mise run dev` and complete independent review.

## Review Notes

## Acceptance Closure

- [ ] All acceptance criteria are satisfied and marked.
- [ ] Verification commands and outcomes are recorded.
- [ ] No unresolved high/medium findings remain.
