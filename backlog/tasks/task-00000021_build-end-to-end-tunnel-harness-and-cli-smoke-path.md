# Task `00000021` - `build end-to-end tunnel harness and cli smoke path`

## Summary

Add an end-to-end local tunnel harness and CLI smoke path over the production
Rust library.

## Motivation

Before native SDK packaging, the repository needs one repeatable local scenario
that proves descriptor loading, transport selection, secure-ready handshake,
session establishment, application record exchange, and close work together.

## Read-Write Repository

- Primary read-write repository: `/Users/asimi/workplace/secure-tunnel`
- Secondary read-write repository/repositories (if applicable): none
- State explicitly:
  - code changes are expected to land in Rust harness, CLI, tests, and task
    automation.
  - this repository itself is expected to change.

## Read-Only Reference Repository

- Read-only reference repository/repositories: none
- State explicitly which repositories may be inspected only for reference or
  legacy behavior:
  - none.

## Detailed Requirements / Acceptance Criteria

### A) Local end-to-end harness exists

- [ ] Add a local client/server or loopback harness that uses production Rust
      library paths rather than only test-only prototype transports.
- [ ] Exercise descriptor/config loading, transport selection, `Secure Ready`,
      account/device session flow, application record exchange, and close.
- [ ] Include both direct `QUIC` success and `WSS` fallback scenarios.

### B) CLI smoke path is useful

- [ ] Add or extend CLI commands that can run the local smoke scenario.
- [ ] Emit concise machine-readable or structured output for selected carrier,
      fallback reason, secure-ready status, session status, and close result.
- [ ] Keep secrets or sensitive payloads out of default CLI output.

### C) Automation is wired

- [ ] Add a `mise` task or documented command for the end-to-end smoke path.
- [ ] Include the smoke path in an appropriate validation gate without making
      fast local development painful.
- [ ] `mise run dev` passes.

## Cross-Repo Boundaries

- Primary implementation boundary: local harness, CLI, and automation in this
  repository.
- Parser / upstream dependency boundary: no parser work expected.
- Downstream integration boundary: native SDK smoke tests can reuse this later
  but are not implemented in this task.
- External asset / catalog / fixture boundary: local fixtures only.
- If another repository is read-write, state what is implemented there versus
  what is implemented in this repository.
  - none.

## Task Dependencies

- backlog/tasks/task-00000019_implement-production-quic-and-wss-carrier-adapters.md
- backlog/tasks/completed/task-00000020_implement-account-and-device-session-protocol.md
- backlog/plans/plan-00000002_product-secure-tunnel-sdk-and-bindings.md

## Reference Tasks

- backlog/tasks/completed/task-00000012_prototype-quic-preferred-transport-with-wss-fallback-and-local-secure-session.md

## Backlog Bookkeeping

- `backlog/` is local planning state and is not version controlled by default.
- Moving a task to `backlog/tasks/completed/` is separate from pushing code.
- Commit history for work tracked by this task may live in a different
  read-write repository than this backlog entry.

## Implementation Notes

- [ ] Implementation notes added with command evidence.
- (fill in after completion)

## Implementation Plan

1. Define the smallest useful local end-to-end tunnel scenario.
2. Implement the harness and CLI entrypoint over production library paths.
3. Add automation and tests for success plus fallback.
4. Run `mise run dev` and independent review.

## Review Notes

## Acceptance Closure

- [ ] All acceptance criteria are satisfied and marked.
- [ ] Verification commands and outcomes are recorded.
- [ ] No unresolved high/medium findings remain.
