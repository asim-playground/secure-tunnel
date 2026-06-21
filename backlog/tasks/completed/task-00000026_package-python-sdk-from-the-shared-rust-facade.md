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

- [x] Decide whether Python uses UniFFI only, PyO3 only, or a PyO3 wrapper over
      the shared Rust facade.
- [x] Preserve a clear migration story from the current Python package.
- [x] Keep the Python public API aligned with the same coarse SDK operations
      exposed to Swift and Kotlin unless a documented Python-specific wrapper
      improves ergonomics without changing behavior.

### B) Python package builds

- [x] Build a Python package or wheel from the selected strategy.
- [x] Include the required native library and generated or wrapper Python code.
- [x] Keep `maturin`/`uv` workflows reproducible.

### C) Python smoke test proves consumption

- [x] Add a clean-environment import test.
- [x] Run at least one descriptor/config/session scenario through the Python
      package or a documented local fixture.
- [x] Preserve or upgrade the task-23 Python generated-client smoke that
      already connects, authenticates, sends one encrypted application request,
      and closes against the Rust fixture.
- [x] Coordinate with `task-00000030` so Python client packaging and the
      Python FastAPI server package do not diverge in facade shape.
- [x] `mise run dev` and Python package checks pass.

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
- backlog/tasks/completed/task-00000030_build-python-fastapi-server-and-rust-client-e2e.md

## Backlog Bookkeeping

- `backlog/` is local planning state and is not version controlled by default.
- Moving a task to `backlog/tasks/completed/` is separate from pushing code.
- Commit history for work tracked by this task may live in a different
  read-write repository than this backlog entry.

## Implementation Notes

- [x] Implementation notes added with command evidence.
- Planning evidence gathered before implementation:
  - Existing `python/` package is a maturin/PyO3 package that exposes protocol
    metadata and descriptor validation only.
  - `bindings/smoke/python/client.py` already proves the generated UniFFI
    Python module can connect, authenticate, send one encrypted application
    request, and close against the Rust binding fixture.
  - Local maturin reference docs include first-class `bindings = "uniffi"`
    support for Python wheels, so the first packaging attempt should use
    maturin's UniFFI path rather than a custom wheel builder.
  - `/Users/asimi/Downloads/references/uniffi-rs`,
    `/Users/asimi/Downloads/references/maturin`, and
    `/Users/asimi/Downloads/references/pyo3` are present for read-only
    reference while implementing.
- Implementation completed:
  - Decision: Python now uses maturin-packaged UniFFI over
    `crates/sdk-ffi` as the behavioral core. The old PyO3 crate
    `crates/python` was removed so there is not a second Rust behavior
    surface for Python.
  - The public `secure_tunnel` package wraps the generated UniFFI module under
    `secure_tunnel._native`, re-exports the coarse SDK facade types, and keeps
    descriptor compatibility functions such as
    `example_service_descriptor_json`,
    `validate_service_descriptor_json`, and
    `normalize_service_descriptor_json`.
  - `crates/uniffi-bindgen` now provides a `uniffi-bindgen` binary name so
    maturin's UniFFI backend can discover the project-local, pinned generator.
  - Added deterministic package tasks:
    `mise run python:build`, `mise run python:build-wheel`,
    `mise run python:check-wheel`, and `mise run python:smoke-package`.
  - Removed the stale PyO3 cross-link CI repro path and updated CI
    cross-compilation so it no longer expects `secure-tunnel-py`.
- Verification evidence:
  - `mise run python:test` passed: 7 Python tests plus packaged Python SDK
    session smoke against the Rust binding fixture.
  - `mise run python:lint` passed: ruff check/format and basedpyright.
  - `mise run python:check-wheel` passed: built the wheel and imported it from
    a clean temporary virtualenv. The check also installs/imports the
    `secure-tunnel[server]` optional extra so the packaged FastAPI fixture
    module has declared dependencies. Maturin still warns that the macOS wheel
    references Homebrew `libiconv`; production wheel repair belongs in release
    packaging.
  - `mise run dev` passed.

## Implementation Plan

1. Reconfirm the local reference docs and the current generated UniFFI Python
   output, then record the binding strategy decision in this task:
   - Preferred path: make generated UniFFI over `secure-tunnel-sdk-ffi` the
     behavioral core for Python, packaged with maturin's `bindings = "uniffi"`.
   - User-facing path: keep `secure_tunnel` as the public Python package and add
     a small typed Python wrapper that gives Pythonic names, migration aliases,
     and stable import ergonomics over the generated module.
   - Compatibility path: preserve the current descriptor-only PyO3 API as
     wrappers or documented deprecated aliases only if it can delegate to the
     same facade without creating a second behavior surface.
2. Reshape `python/pyproject.toml`, package source, and generated-binding
   staging so `uv run maturin build` and `uv run maturin develop` produce a
   wheel/editable install that includes:
   - the UniFFI-generated Python module;
   - the native `secure_tunnel_sdk_ffi` library for the current platform;
   - `py.typed` and checked type stubs or generated typings for the public
     `secure_tunnel` wrapper.
3. Implement the wrapper layer as an imperative shell around the shared Rust
   SDK facade, keeping protocol/session logic in Rust:
   - expose config, descriptor, connect, account auth, request, security
     artifacts, and close operations at the same coarse granularity as Swift
     and Kotlin;
   - avoid Python-side reimplementation of Noise, descriptor validation,
     transport selection, or application-frame semantics.
4. Add Python package tests:
   - clean-environment import from a built wheel;
   - backwards-compatible descriptor metadata/validation tests or explicit
     deprecation assertions;
   - fixture-backed Python-client session smoke equivalent to the task-23
     generated-client smoke.
5. Add deterministic mise automation, likely:
   - `mise run python:build-wheel`;
   - `mise run python:check-wheel`;
   - `mise run sdk:python:package-smoke` or the closest repo-local naming that
     fits existing task conventions.
6. Coordinate the package output with `task-00000030`:
   - FastAPI server work must consume this package or its dev/test extra rather
     than inventing a separate Python binding shape;
   - if server-only Rust hooks are required, place them behind an explicit
     testing/server facade instead of bloating the public client SDK.
7. Validate narrowly first, then broadly:
   - `mise run sdk:generate-bindings`;
   - Python wheel/build/import/session smoke tasks;
   - existing `mise run sdk:smoke-python`;
   - `mise run dev`.
8. Run the independent review/re-review loop, update Implementation Notes with
   command evidence, tick acceptance criteria, update the parent plan, and move
   the task to `backlog/tasks/completed/` only after implementation is done.

## Review Notes

- Independent review/re-review for task 26 and task 30 completed with no
  remaining high/medium findings. The review loop also verified the
  FastAPI/server extra packaging fix and lifecycle cleanup added during task
  30 follow-up.

## Acceptance Closure

- [x] All acceptance criteria are satisfied and marked.
- [x] Verification commands and outcomes are recorded.
- [x] No unresolved high/medium findings remain.
