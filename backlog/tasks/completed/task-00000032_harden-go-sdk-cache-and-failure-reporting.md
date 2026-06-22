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

- [x] Add a Go package smoke or fixture test that passes a non-nil
      `ConnectOptions.TransportCache`.
- [x] Prove cached QUIC-bad posture selects the expected fallback/reprobe path
      without changing Rust SDK semantics.
- [x] Keep the test aligned with the same binding fixture semantics used by
      Swift, Kotlin, Python, Flutter/Dart, and Rust-client smokes.

### B) Failed connect attempts are structured for Go callers

- [x] Expose failed-connect attempt traces to Go callers as typed data rather
      than only flattened `ABIError` text.
- [x] Preserve stable SDK error kind/message information without leaking
      sensitive diagnostics into routine logs.
- [x] Add Go tests for at least one failed-connect path that includes attempt
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

- [x] Implementation notes added with command evidence.
- Created from the residual low-risk findings in the Task `00000029`
  independent re-review.
- Added a v2 connect ABI instead of changing the existing result struct in
  place: `secure_tunnel_client_connect_v2` returns
  `SecureTunnelConnectionResultV2` with caller-owned `error_details_json`.
  The existing `secure_tunnel_client_connect` remains as the legacy flattened
  result and frees/discards v2 details before returning.
- Added `ConnectError` to the Go package with `Status`, `Kind`, `Message`, and
  typed `Attempts`, while keeping `ABIError` for non-connect ABI failures.
- Added Go transport attempt decoding that flattens Rust serde enum outcomes
  into `Outcome`, `FallbackReason`, `FailureKind`, and `FailureMessage`.
- Extended the binding fixture smoke to pass a non-nil
  `ConnectOptions.TransportCache` with active QUIC-bad posture and assert WSS
  cached fallback through the same local fixture used by other SDK smokes.
- Added a failed-connect fixture path that omits local root certificates,
  verifies `errors.As(err, *ConnectError)`, and checks structured attempts.
- Validation evidence:
  - `cargo check -p secure-tunnel-ffi` passed.
  - `mise run sdk:go` passed, including header drift, `go test ./...`,
    `go test -race ./...`, and fixture smoke.
  - `mise run dev` passed: Rust nextest 95/95, Python tests and smokes,
    Go tests, lint, and formatting.

## Implementation Plan

1. [x] Add a cached-fallback Go fixture scenario using a non-nil
   `ConnectOptions.TransportCache`.
2. [x] Extend the C ABI and Go wrapper as needed to carry structured failed-connect
   attempts.
3. [x] Add Go tests for structured failed-connect errors and keep header drift
   checks current.
4. [x] Run `mise run sdk:go`, `mise run dev`, and independent review.

## Review Notes

- Read-only explorer recommended a v2 connect ABI so structured error details
  would not mutate the existing C result struct; implementation followed that
  recommendation.
- Independent reviewer found no high or medium findings.
- Residual low risk: exact spelling of `ConnectError.Kind` is currently aligned
  with the existing UniFFI `Debug`-style error kind spelling, while attempt
  failure kinds come from serde JSON. This is tracked in `task-00000027`
  under release compatibility policy.

## Acceptance Closure

- [x] All acceptance criteria are satisfied and marked.
- [x] Verification commands and outcomes are recorded.
- [x] No unresolved high/medium findings remain.
