# Task `00000019` - `implement production quic and wss carrier adapters`

## Summary

Replace test-only carrier prototypes with production `QUIC` and `WSS` client
adapters that implement the shared framed duplex contract.

## Motivation

The current prototype harness proves selector and secure-ready behavior but is
not a production transport implementation. The SDK needs real carrier adapters
before native bindings can exercise meaningful connection scenarios.

## Read-Write Repository

- Primary read-write repository: `/Users/asimi/workplace/secure-tunnel`
- Secondary read-write repository/repositories (if applicable): none
- State explicitly:
  - code changes are expected to land in Rust carrier modules or crates and in
    tests/harnesses that exercise them.
  - this repository itself is expected to change.

## Read-Only Reference Repository

- Read-only reference repository/repositories: none
- State explicitly which repositories may be inspected only for reference or
  legacy behavior:
  - reference repos may be cloned under `/Users/asimi/Downloads/references`
    if needed to inspect packaging or adapter examples, but changes land only
    in `secure-tunnel`.

## Detailed Requirements / Acceptance Criteria

### A) Real carrier adapters exist

- [ ] Implement a raw `QUIC` client connector that negotiates the v1 ALPN and
      presents framed duplex records over the selected stream.
- [ ] Implement a `WSS` client connector that negotiates the v1 subprotocol and
      presents the same framed duplex record contract.
- [ ] Keep carrier-specific TLS, stream, close, and framing behavior below the
      transport abstraction.

### B) Selector semantics are preserved

- [ ] `QUIC` success remains preferred when available.
- [ ] Fallback to `WSS` occurs only for documented fallback-eligible outer
      failures before `Secure Ready`.
- [ ] Inner trust failures do not trigger `WSS` fallback.

### C) Adapter tests are real enough to support SDK work

- [ ] Add local integration coverage for successful `QUIC`, successful `WSS`,
      fallback from `QUIC` to `WSS`, and malformed target failure.
- [ ] Verify adapter behavior against active `v1-*` docs and descriptor
      validation.
- [ ] `mise run dev` passes.

## Cross-Repo Boundaries

- Primary implementation boundary: Rust carrier adapters and local test
  harnesses.
- Parser / upstream dependency boundary: new transport dependencies may be
  added only with repo supply-chain checks intact.
- Downstream integration boundary: no native SDK packaging in this task.
- External asset / catalog / fixture boundary: local fixtures only.
- If another repository is read-write, state what is implemented there versus
  what is implemented in this repository.
  - none.

## Task Dependencies

- backlog/tasks/completed/task-00000012_prototype-quic-preferred-transport-with-wss-fallback-and-local-secure-session.md
- backlog/tasks/task-00000018_define-product-sdk-facade-and-session-contract.md
- backlog/plans/plan-00000002_product-secure-tunnel-sdk-and-bindings.md

## Reference Tasks

- backlog/tasks/completed/task-00000001_consider-starter-crates.md

## Backlog Bookkeeping

- `backlog/` is local planning state and is not version controlled by default.
- Moving a task to `backlog/tasks/completed/` is separate from pushing code.
- Commit history for work tracked by this task may live in a different
  read-write repository than this backlog entry.

## Implementation Notes

- [ ] Implementation notes added with command evidence.
- (fill in after completion)

## Implementation Plan

1. Confirm final crate/module location for production carrier adapters.
2. Implement `QUIC` and `WSS` connectors behind the existing transport traits.
3. Add integration tests for success, fallback, and failure classification.
4. Run `mise run dev`, then independent review and re-review.

## Review Notes

## Acceptance Closure

- [ ] All acceptance criteria are satisfied and marked.
- [ ] Verification commands and outcomes are recorded.
- [ ] No unresolved high/medium findings remain.
