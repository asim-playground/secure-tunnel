# Task `00000032` - `harden go sdk cache and failure reporting`

## Summary

Close low-risk Go SDK observability and cache-coverage gaps left after native
Go package stabilization.

## Motivation

Task `00000029` made native Go a first-class SDK package over the stable C ABI.
Independent re-review found no high or medium blockers, but noted two residual
risks before release hardening: the Go fixture smoke does not explicitly prove
a non-nil transport cache drives cached fallback selection, and failed-connect
attempt traces are not exposed as structured Go error data.

## Read-Write Repository

- Primary read-write repository: `/Users/asimi/workplace/secure-tunnel`
- Secondary read-write repository/repositories (if applicable): none
- State explicitly:
  - code changes are expected to land in the C ABI crate, native Go package,
    Go tests, and task automation.
  - this repository itself is expected to change.

## Read-Only Reference Repository

- Read-only reference repository/repositories: none required.
- State explicitly which repositories may be inspected only for reference or
  legacy behavior.
  - none.

## Detailed Requirements / Acceptance Criteria

### A) Go transport cache behavior is proven

- [ ] Add a Go package smoke or fixture test that passes a non-nil
      `ConnectOptions.TransportCache`.
- [ ] Prove cached QUIC-bad posture selects the expected fallback/reprobe path
      without changing Rust SDK semantics.
- [ ] Keep the test aligned with the same binding fixture semantics used by
      Swift, Kotlin, Python, Flutter/Dart, and Rust-client smokes.

### B) Failed connect attempts are structured for Go callers

- [ ] Expose failed-connect attempt traces to Go callers as typed data rather
      than only flattened `ABIError` text.
- [ ] Preserve stable SDK error kind/message information without leaking
      sensitive diagnostics into routine logs.
- [ ] Add Go tests for at least one failed-connect path that includes attempt
      trace data.

## Cross-Repo Boundaries

- Primary implementation boundary: Go package facade, manual C ABI, generated
  header, and Go package tests in this repository.
- Parser / upstream dependency boundary: avoid new dependencies unless
  structured JSON/error decoding requires them.
- Downstream integration boundary: no downstream Go consumer repository is
  modified by this task.
- External asset / catalog / fixture boundary: local fixture-generated JSON
  only.
- If another repository is read-write, state what is implemented there versus
  what is implemented in this repository.
  - none.

## Task Dependencies

- backlog/tasks/completed/task-00000029_package-go-sdk-over-stable-c-abi.md

## Reference Tasks

- backlog/tasks/completed/task-00000022_add-observability-and-conformance-test-matrix.md
- backlog/tasks/completed/task-00000029_package-go-sdk-over-stable-c-abi.md
- backlog/tasks/completed/task-00000031_security-hardening-pass.md

## Backlog Bookkeeping

- `backlog/` is local planning state and is not version controlled by default.
- Moving a task to `backlog/tasks/completed/` is separate from pushing code.
- Commit history for work tracked by this task may live in a different
  read-write repository than this backlog entry.

## Implementation Notes

- [ ] Implementation notes added with command evidence.
- Created from the residual low-risk findings in the Task `00000029`
  independent re-review.

## Implementation Plan

1. Add a cached-fallback Go fixture scenario using a non-nil
   `ConnectOptions.TransportCache`.
2. Extend the C ABI and Go wrapper as needed to carry structured failed-connect
   attempts.
3. Add Go tests for structured failed-connect errors and keep header drift
   checks current.
4. Run `mise run sdk:go`, `mise run dev`, and independent review.

## Review Notes

## Acceptance Closure

- [ ] All acceptance criteria are satisfied and marked.
- [ ] Verification commands and outcomes are recorded.
- [ ] No unresolved high/medium findings remain.
