# Task `00000020` - `implement account and device session protocol`

## Summary

Implement the post-Noise account session and known-device protocol needed after
the channel reaches `Secure Ready`.

## Motivation

The current core exposes secure-ready artifacts and session phases, but the
account login, known-device challenge, channel-binding use, and replay/freshness
rules are still mostly documented behavior. The product SDK needs these flows
before it can represent a complete secure tunnel session.

## Read-Write Repository

- Primary read-write repository: `/Users/asimi/workplace/secure-tunnel`
- Secondary read-write repository/repositories (if applicable): none
- State explicitly:
  - code changes are expected to land in Rust core/session modules and tests.
  - this repository itself is expected to change.

## Read-Only Reference Repository

- Read-only reference repository/repositories: none
- State explicitly which repositories may be inspected only for reference or
  legacy behavior:
  - none.

## Detailed Requirements / Acceptance Criteria

### A) Session protocol is implemented

- [ ] Implement the first account session open/login message flow above Noise
      transport mode.
- [ ] Implement known-device challenge/response using the documented channel
      binding and freshness rules.
- [ ] Preserve the documented distinction between new-device enrollment and
      returning-device reauthentication.

### B) Security invariants are tested

- [ ] Tests reject wrong service/environment binding where applicable.
- [ ] Tests reject stale or replayed device challenge material.
- [ ] Tests prove channel-binding material is included where the docs require
      it.

### C) SDK-facing state remains simple

- [ ] Expose coarse session phases and errors suitable for the facade from
      `task-00000018`.
- [ ] Avoid leaking low-level protocol message internals into the SDK facade
      unless this task records a deliberate exception.
- [ ] `mise run dev` passes.

## Cross-Repo Boundaries

- Primary implementation boundary: Rust session protocol code.
- Parser / upstream dependency boundary: no parser work expected.
- Downstream integration boundary: native bindings consume this later through
  the SDK facade.
- External asset / catalog / fixture boundary: local cryptographic fixtures
  only.
- If another repository is read-write, state what is implemented there versus
  what is implemented in this repository.
  - none.

## Task Dependencies

- backlog/tasks/completed/task-00000006_define-device-enrollment-and-known-device-policy.md
- backlog/tasks/completed/task-00000011_prototype-server-auth-noise-handshake-and-trust-verification-on-transport-neutral-frames.md
- backlog/tasks/task-00000018_define-product-sdk-facade-and-session-contract.md
- backlog/plans/plan-00000002_product-secure-tunnel-sdk-and-bindings.md

## Reference Tasks

- backlog/docs/v1-device-enrollment-and-known-device-policy.md
- backlog/docs/v1-core-protocol-quic-and-wss-bindings.md

## Backlog Bookkeeping

- `backlog/` is local planning state and is not version controlled by default.
- Moving a task to `backlog/tasks/completed/` is separate from pushing code.
- Commit history for work tracked by this task may live in a different
  read-write repository than this backlog entry.

## Implementation Notes

- [ ] Implementation notes added with command evidence.
- (fill in after completion)

## Implementation Plan

1. Map the active device/session docs to concrete message structs and state
   transitions.
2. Implement account and known-device flows over `FramedDuplex`.
3. Add success, replay, wrong-binding, and stale-challenge tests.
4. Run `mise run dev` and complete independent review.

## Review Notes

## Acceptance Closure

- [ ] All acceptance criteria are satisfied and marked.
- [ ] Verification commands and outcomes are recorded.
- [ ] No unresolved high/medium findings remain.
