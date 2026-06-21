# Task `00000029` - `package go sdk over stable c abi`

## Summary

Stabilize and package the Go SDK over the manual C ABI and generated cbindgen
header.

## Motivation

UniFFI is the right default for Swift, Kotlin, and Python, but it does not cover
Go. Secure Tunnel already has a manual C ABI and Go binding scaffold, so Go
should remain a first-class SDK target through a small stable C ABI rather than
a separate hand-rolled protocol implementation. Native Go is the supported Go
SDK path; the existing Go-WASM scaffold should be deprecated or deleted unless
a future task proves concrete product need.

## Read-Write Repository

- Primary read-write repository: `/Users/asimi/workplace/secure-tunnel`
- Secondary read-write repository/repositories (if applicable): none
- State explicitly:
  - code changes are expected to land in the C ABI crate, generated headers, Go
    package code, Go-WASM deprecation or deletion, tests, and task automation.
  - this repository itself is expected to change.

## Read-Only Reference Repository

- Read-only reference repository/repositories:
  - `/Users/asimi/Downloads/references/cbindgen`
- State explicitly which repositories may be inspected only for reference or
  legacy behavior:
  - `cbindgen` may be inspected for generated header conventions; changes land
    only in `secure-tunnel`.

## Detailed Requirements / Acceptance Criteria

### A) Go SDK boundary is explicit

- [ ] Define which SDK operations Go consumes through the manual C ABI and
      which remain Rust-only.
- [ ] Keep Go aligned with the same Rust SDK facade behavior used by native and
      Flutter/Dart packages.
- [ ] Preserve explicit ownership, allocation, and error rules for every C ABI
      value exposed to Go.

### B) Go package is consumable

- [ ] Package native Go bindings with reproducible header/library generation.
- [ ] Deprecate or delete the existing Go-WASM binding scaffold; do not treat
      Go-WASM as a supported SDK target unless a future task proves concrete
      product need.
- [ ] Add generated-header drift checks so Go bindings cannot silently fall
      behind the C ABI.

### C) Go smoke tests prove behavior

- [ ] Add native Go import/build tests.
- [ ] Run at least one descriptor/config/session scenario through the Go
      package or a documented local fixture.
- [ ] Verify memory ownership and error cleanup paths in tests.

## Cross-Repo Boundaries

- Primary implementation boundary: manual C ABI, cbindgen header, native Go
  bindings, and Go-WASM scaffold deprecation or deletion in this repository.
- Parser / upstream dependency boundary: dependency changes must stay within
  Go/C ABI packaging needs and pass repo supply-chain checks.
- Downstream integration boundary: no downstream Go consumer repository is
  modified by this task.
- External asset / catalog / fixture boundary: generated C header and local
  smoke fixtures only.
- If another repository is read-write, state what is implemented there versus
  what is implemented in this repository.
  - none.

## Task Dependencies

- backlog/tasks/completed/task-00000016_update-runtimes-deps-and-add-swift-callable-library-surface.md
- backlog/tasks/task-00000018_define-product-sdk-facade-and-session-contract.md
- backlog/tasks/completed/task-00000021_build-end-to-end-tunnel-harness-and-cli-smoke-path.md
- backlog/tasks/completed/task-00000022_add-observability-and-conformance-test-matrix.md
- backlog/tasks/completed/task-00000024_package-swift-sdk-as-swiftpm-and-xcframework.md
- backlog/plans/plan-00000002_product-secure-tunnel-sdk-and-bindings.md

## Reference Tasks

- backlog/docs/2026-06-21_sdk-reference-repositories.md
- backlog/tasks/completed/task-00000002_bootstrap-repository-scaffold.md
- backlog/tasks/completed/task-00000016_update-runtimes-deps-and-add-swift-callable-library-surface.md

## Backlog Bookkeeping

- `backlog/` is local planning state and is not version controlled by default.
- Moving a task to `backlog/tasks/completed/` is separate from pushing code.
- Commit history for work tracked by this task may live in a different
  read-write repository than this backlog entry.

## Implementation Notes

- [ ] Implementation notes added with command evidence.
- Use `/Users/asimi/Downloads/references/cbindgen` as the local upstream
  reference for generated C header behavior.

## Implementation Plan

1. Map the Rust SDK facade to the stable C ABI operations Go needs.
2. Extend the C ABI and Go bindings while preserving ownership/error rules.
3. Remove or clearly deprecate the Go-WASM scaffold from supported SDK scope.
4. Add header drift, native Go import, and memory/error cleanup tests.
5. Run package validation and independent review.

## Review Notes

## Acceptance Closure

- [ ] All acceptance criteria are satisfied and marked.
- [ ] Verification commands and outcomes are recorded.
- [ ] No unresolved high/medium findings remain.
