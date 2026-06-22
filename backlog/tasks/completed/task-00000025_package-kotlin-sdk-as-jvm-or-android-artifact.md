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

- [x] Decide whether the first artifact is JVM-only, Android AAR, or both.
- [x] Package generated Kotlin bindings with the required native libraries and
      documented JNA/runtime dependency.
- [x] Support the first required ABI matrix for the chosen artifact.

### B) Kotlin smoke test proves consumption

- [x] Add a Kotlin/JVM or Android import/build smoke test.
- [x] Run at least one descriptor/config/session scenario through the Kotlin
      package or a documented local fixture.
- [x] Preserve or upgrade the task-23 Kotlin generated-client smoke that
      already connects, authenticates, sends one encrypted application request,
      and closes against the Rust fixture.
- [x] Record any JNA/JNI/performance caveats discovered during the task.

### C) Packaging is automated

- [x] Add deterministic `mise` tasks for Kotlin SDK build and smoke tests.
- [x] Add CI coverage or a documented platform gate for the Kotlin package.
- [x] Keep the SDK API aligned with the same UniFFI facade used by Swift and
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

- backlog/tasks/completed/task-00000022_add-observability-and-conformance-test-matrix.md
- backlog/tasks/completed/task-00000023_create-uniffi-sdk-facade-and-bindgen-tooling.md
- backlog/plans/plan-00000002_product-secure-tunnel-sdk-and-bindings.md

## Reference Tasks

- backlog/tasks/completed/task-00000016_update-runtimes-deps-and-add-swift-callable-library-surface.md

## Backlog Bookkeeping

- `backlog/` is local planning state and is not version controlled by default.
- Moving a task to `backlog/tasks/completed/` is separate from pushing code.
- Commit history for work tracked by this task may live in a different
  read-write repository than this backlog entry.

## Implementation Notes

- First artifact decision: use a JVM-first Gradle package for the host ABI.
  Android AAR packaging and cross-compiled ABI bundles remain future work
  because the existing UniFFI Kotlin backend is JNA/JVM-oriented and the repo
  does not yet have an Android consumer fixture.
- Added tracked package templates under `bindings/kotlin/` and generated
  package output under `target/sdk/kotlin/SecureTunnelKotlin/`.
- The package includes generated UniFFI Kotlin source, declares the JNA runtime
  dependency, and embeds the release-built host `secure_tunnel_sdk_ffi` dynamic
  library under JNA's platform resource path. The consumer smoke does not set
  UniFFI's `libraryOverride`, so it proves the packaged native resource path
  loads.
- Added deterministic tasks:
  - `mise run sdk:kotlin:package`
  - `mise run sdk:kotlin:check-package`
  - `mise run sdk:kotlin:smoke-package`
  - `mise run sdk:kotlin`
- Pinned `java@corretto-21.0.11.10.1` and `gradle@9.3.1` in `mise.toml` and
  refreshed `mise.lock`.
- CI now runs `mise run sdk:kotlin` on non-Windows runners; Windows is gated
  until the task grows a Windows native artifact matrix.
- Verification evidence so far:
  - `mise run sdk:kotlin` passed on Linux `aarch64`, including package layout
    checks and a packaged-consumer QUIC/session smoke that printed
    `{"language":"kotlin","protocol":"secure-tunnel-v1","carrier":"quic","close":"GRACEFUL"}`.
  - Independent review found one medium issue: the first package script copied
    a debug native library into the artifact. Fixed by building
    `secure-tunnel-sdk-ffi` with `--release`, stripping the copied library when
    `strip` is available, and adding a package check that rejects native
    artifacts reported as debug or unstripped by `file`.
  - `mise run sdk:kotlin` passed again after the release/strip fix.
  - `mise run sdk:smoke-kotlin && mise run sdk:kotlin` passed, preserving the
    generated-client smoke and proving the packaged consumer path.
  - `mise run dev` passed after the release/strip fix: format, lint, 95 Rust
    tests, Python tests/smokes, Go tests, and Go-WASM tests.

## Implementation Plan

1. [x] Choose the first Kotlin artifact target and ABI matrix.
2. [x] Package generated Kotlin bindings and native libraries.
3. [x] Add and run Kotlin import/session smoke tests.
4. [x] Run package validation and independent review.

## Review Notes

- Independent review found one medium issue: the first package script copied a
  debug, unstripped native library into the Kotlin artifact. The fix changed
  package generation to build/copy the release library, strip it when `strip`
  is available, and reject native artifacts whose `file` output reports
  `not stripped` or `with debug_info`.
- Re-review found no high- or medium-severity findings. Residual low risks:
  the artifact is intentionally host-ABI only for this task, Windows package
  CI remains gated, and release/version policy remains in `task-00000027`.

## Acceptance Closure

- [x] All acceptance criteria are satisfied and marked.
- [x] Verification commands and outcomes are recorded.
- [x] No unresolved high/medium findings remain.
