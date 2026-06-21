# Task `00000025` - `package kotlin sdk as jvm or android artifact`

## Summary

Package the generated Kotlin binding and native Rust libraries as a consumable
Kotlin SDK artifact.

## Motivation

The common SDK story requires Kotlin parity with Swift and Python. UniFFI's
Kotlin path introduces JVM/JNA or platform packaging tradeoffs, so the repo
needs an explicit artifact and smoke test instead of only generated source.

## Read-Write Repository

- Primary read-write repository: `/Users/asimi/workplace/secure-tunnel`
- Secondary read-write repository/repositories (if applicable): none
- State explicitly:
  - code changes are expected to land in Kotlin/Gradle packaging files, native
    library build tasks, and CI.
  - this repository itself is expected to change.

## Read-Only Reference Repository

- Read-only reference repository/repositories:
  - Kotlin or UniFFI packaging examples may be cloned under
    `/Users/asimi/Downloads/references` if needed.
- State explicitly which repositories may be inspected only for reference or
  legacy behavior:
  - examples are read-only; implementation changes land in `secure-tunnel`.

## Detailed Requirements / Acceptance Criteria

### A) Kotlin package artifact exists

- [ ] Decide whether the first artifact is JVM-only, Android AAR, or both.
- [ ] Package generated Kotlin bindings with the required native libraries and
      documented JNA/runtime dependency.
- [ ] Support the first required ABI matrix for the chosen artifact.

### B) Kotlin smoke test proves consumption

- [ ] Add a Kotlin/JVM or Android import/build smoke test.
- [ ] Run at least one descriptor/config/session scenario through the Kotlin
      package or a documented local fixture.
- [ ] Record any JNA/JNI/performance caveats discovered during the task.

### C) Packaging is automated

- [ ] Add deterministic `mise` tasks for Kotlin SDK build and smoke tests.
- [ ] Add CI coverage or a documented platform gate for the Kotlin package.
- [ ] Keep the SDK API aligned with the same UniFFI facade used by Swift and
      Python.

## Cross-Repo Boundaries

- Primary implementation boundary: Kotlin package and native library build
  tasks in this repository.
- Parser / upstream dependency boundary: no parser work expected.
- Downstream integration boundary: no downstream Android app repository is
  modified by this task.
- External asset / catalog / fixture boundary: generated Kotlin and package
  artifacts only.
- If another repository is read-write, state what is implemented there versus
  what is implemented in this repository.
  - none.

## Task Dependencies

- backlog/tasks/task-00000022_add-observability-and-conformance-test-matrix.md
- backlog/tasks/task-00000023_create-uniffi-sdk-facade-and-bindgen-tooling.md
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

1. Choose the first Kotlin artifact target and ABI matrix.
2. Package generated Kotlin bindings and native libraries.
3. Add and run Kotlin import/session smoke tests.
4. Run package validation and independent review.

## Review Notes

## Acceptance Closure

- [ ] All acceptance criteria are satisfied and marked.
- [ ] Verification commands and outcomes are recorded.
- [ ] No unresolved high/medium findings remain.
