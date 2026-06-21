# Task `00000026` - `package python sdk from the shared rust facade`

## Summary

Package the Python SDK from the same Rust facade used by Swift and Kotlin.

## Motivation

The repository already has a PyO3/maturin Python surface, but the binding
research recommends using one deliberately small Rust library facade for Swift,
Kotlin, and Python when possible. This task decides whether Python moves to
UniFFI, keeps PyO3 as a polished wrapper over the shared facade, or temporarily
ships both during migration.

## Read-Write Repository

- Primary read-write repository: `/Users/asimi/workplace/secure-tunnel`
- Secondary read-write repository/repositories (if applicable): none
- State explicitly:
  - code changes are expected to land in Python packaging, Rust Python/UniFFI
    integration, tests, and CI.
  - this repository itself is expected to change.

## Read-Only Reference Repository

- Read-only reference repository/repositories:
  - Python packaging examples may be cloned under
    `/Users/asimi/Downloads/references` if needed.
- State explicitly which repositories may be inspected only for reference or
  legacy behavior:
  - examples are read-only; implementation changes land in `secure-tunnel`.

## Detailed Requirements / Acceptance Criteria

### A) Python binding strategy is decided

- [ ] Decide whether Python uses UniFFI only, PyO3 only, or a PyO3 wrapper over
      the shared Rust facade.
- [ ] Preserve a clear migration story from the current Python package.
- [ ] Keep the Python public API aligned with the same coarse SDK operations
      exposed to Swift and Kotlin unless a documented Python-specific wrapper
      improves ergonomics without changing behavior.

### B) Python package builds

- [ ] Build a Python package or wheel from the selected strategy.
- [ ] Include the required native library and generated or wrapper Python code.
- [ ] Keep `maturin`/`uv` workflows reproducible.

### C) Python smoke test proves consumption

- [ ] Add a clean-environment import test.
- [ ] Run at least one descriptor/config/session scenario through the Python
      package or a documented local fixture.
- [ ] Preserve or upgrade the task-23 Python generated-client smoke that
      already connects, authenticates, sends one encrypted application request,
      and closes against the Rust fixture.
- [ ] Coordinate with `task-00000030` so Python client packaging and the
      Python FastAPI server package do not diverge in facade shape.
- [ ] `mise run dev` and Python package checks pass.

## Cross-Repo Boundaries

- Primary implementation boundary: Python package and Rust facade integration
  in this repository.
- Parser / upstream dependency boundary: no parser work expected.
- Downstream integration boundary: no downstream Python project is modified by
  this task.
- External asset / catalog / fixture boundary: generated Python and wheel
  artifacts only.
- If another repository is read-write, state what is implemented there versus
  what is implemented in this repository.
  - none.

## Task Dependencies

- backlog/tasks/completed/task-00000022_add-observability-and-conformance-test-matrix.md
- backlog/tasks/completed/task-00000023_create-uniffi-sdk-facade-and-bindgen-tooling.md
- backlog/plans/plan-00000002_product-secure-tunnel-sdk-and-bindings.md

## Reference Tasks

- backlog/tasks/completed/task-00000015_stabilize-ci-portability-and-add-docker-repro.md
- backlog/tasks/completed/task-00000016_update-runtimes-deps-and-add-swift-callable-library-surface.md
- backlog/tasks/task-00000030_build-python-fastapi-server-and-rust-client-e2e.md

## Backlog Bookkeeping

- `backlog/` is local planning state and is not version controlled by default.
- Moving a task to `backlog/tasks/completed/` is separate from pushing code.
- Commit history for work tracked by this task may live in a different
  read-write repository than this backlog entry.

## Implementation Notes

- [ ] Implementation notes added with command evidence.
- (fill in after completion)

## Implementation Plan

1. Compare UniFFI Python output with the current PyO3 package against the SDK
   facade needs.
2. Implement the chosen packaging path and migration notes.
3. Add clean import and descriptor/session smoke tests.
4. Run package validation and independent review.

## Review Notes

## Acceptance Closure

- [ ] All acceptance criteria are satisfied and marked.
- [ ] Verification commands and outcomes are recorded.
- [ ] No unresolved high/medium findings remain.
