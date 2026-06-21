# Task `00000030` - `build python fastapi server and rust client e2e`

## Summary

Add the first Python server runtime for Secure Tunnel and prove Rust client
interoperability against it.

## Motivation

`task-00000023` proves generated Swift, Kotlin, and Python clients can call the
shared Rust SDK facade against a Rust server fixture. The product plan also
needs the reverse direction: a Python FastAPI server path that can host the
secure tunnel service semantics and interoperate with the Rust client. This
task keeps that work separate from UniFFI generation so the Python server can
make an explicit PyO3, maturin, or UniFFI packaging decision.

## Read-Write Repository

- Primary read-write repository: `/Users/asimi/workplace/secure-tunnel`
- Secondary read-write repository/repositories (if applicable): none
- State explicitly:
  - code changes are expected to land in Python server code, Rust/Python
    packaging tasks, local fixtures, and end-to-end tests.
  - this repository itself is expected to change.

## Read-Only Reference Repository

- Read-only reference repository/repositories:
  - `/Users/asimi/Downloads/references/fastapi`
  - `/Users/asimi/Downloads/references/pyo3`
  - `/Users/asimi/Downloads/references/maturin`
  - `/Users/asimi/Downloads/references/uniffi-rs`
- State explicitly which repositories may be inspected only for reference or
  legacy behavior:
  - reference repositories are read-only; implementation changes land only in
    `secure-tunnel`.

## Detailed Requirements / Acceptance Criteria

### A) Python server strategy is explicit

- [ ] Decide whether the Python FastAPI server uses PyO3/maturin, UniFFI
      Python, or a small Python wrapper over a Rust native library.
- [ ] Document how service static-key custody, descriptor signing, and pinned
      public-key disclosure differ between client and server package roles.
- [ ] Keep the server runtime shape compatible with the same protocol and
      application-message semantics used by the Rust harness.

### B) FastAPI server fixture exists

- [ ] Add a local FastAPI server fixture that can serve descriptor/bootstrap
      metadata and host the tunnel service path needed for one smoke scenario.
- [ ] Package or load the Rust-backed secure tunnel service code in a
      reproducible Python workflow.
- [ ] Keep test keys and fixture descriptors deterministic and clearly marked
      as test-only.

### C) Cross-language e2e passes

- [ ] Add Rust-client-to-Python-server end-to-end coverage for descriptor load,
      `Secure Ready`, account auth, one application request, and graceful close.
- [ ] Reuse the task-23 fixture semantics so Swift, Kotlin, Python-client, and
      Rust-client smokes all assert the same payload and close behavior.
- [ ] Add deterministic `mise` tasks for the Python FastAPI server smoke path.

## Cross-Repo Boundaries

- Primary implementation boundary: Python server runtime and Rust client e2e in
  this repository.
- Parser / upstream dependency boundary: no parser work expected.
- Downstream integration boundary: no downstream app repository is modified by
  this task.
- External asset / catalog / fixture boundary: generated/package artifacts may
  live under `target/` unless a package layout requires tracked metadata.
- If another repository is read-write, state what is implemented there versus
  what is implemented in this repository.
  - none.

## Task Dependencies

- backlog/tasks/completed/task-00000023_create-uniffi-sdk-facade-and-bindgen-tooling.md
- backlog/plans/plan-00000002_product-secure-tunnel-sdk-and-bindings.md

## Reference Tasks

- backlog/docs/2026-06-21_sdk-reference-repositories.md
- backlog/tasks/completed/task-00000021_build-end-to-end-tunnel-harness-and-cli-smoke-path.md
- backlog/tasks/completed/task-00000023_create-uniffi-sdk-facade-and-bindgen-tooling.md
- backlog/tasks/task-00000026_package-python-sdk-from-the-shared-rust-facade.md

## Backlog Bookkeeping

- `backlog/` is local planning state and is not version controlled by default.
- Moving a task to `backlog/tasks/completed/` is separate from pushing code.
- Commit history for work tracked by this task may live in a different
  read-write repository than this backlog entry.

## Implementation Notes

- [ ] Implementation notes added with command evidence.
- (fill in after completion)

## Implementation Plan

1. Reinspect `fastapi`, `pyo3`, `maturin`, and the task-23 generated-client
   smoke clients for the minimal server package shape.
2. Add a deterministic FastAPI server fixture backed by the Rust secure tunnel
   service path.
3. Add Rust-client-to-Python-server e2e coverage for the same descriptor,
   account auth, payload, and close scenario used by the task-23 clients.
4. Add `mise` automation, run validation, and complete independent review.

## Review Notes

## Acceptance Closure

- [ ] All acceptance criteria are satisfied and marked.
- [ ] Verification commands and outcomes are recorded.
- [ ] No unresolved high/medium findings remain.
