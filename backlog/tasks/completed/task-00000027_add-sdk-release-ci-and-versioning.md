# Task `00000027` - `add sdk release ci and versioning`

## Summary

Add SDK release CI, artifact versioning, and package publication safeguards.

## Motivation

Once Swift, Kotlin, Python, Flutter/Dart, and Go package artifacts exist, the
repo needs a repeatable release path that ties Rust crate versions, generated
bindings, native libraries, package manifests, smoke tests, and checksums
together.

## Read-Write Repository

- Primary read-write repository: `/Users/asimi/workplace/secure-tunnel`
- Secondary read-write repository/repositories (if applicable): none
- State explicitly:
  - code changes are expected to land in CI, release scripts, task automation,
    package metadata, and documentation.
  - this repository itself is expected to change.

## Read-Only Reference Repository

- Read-only reference repository/repositories:
  - release workflow examples may be cloned under
    `/Users/asimi/Downloads/references` if needed.
- State explicitly which repositories may be inspected only for reference or
  legacy behavior:
  - examples are read-only; implementation changes land in `secure-tunnel`.

## Detailed Requirements / Acceptance Criteria

### A) Versioning is coherent

- [x] Define how Rust crate, UniFFI facade, Swift, Kotlin, and Python package
      versions move together.
- [x] Define how Flutter/Dart and Go package versions relate to the Rust SDK
      facade and native package releases.
- [x] Define compatibility policy for generated SDK APIs while UniFFI remains
      pre-1.0.
- [x] Define stable SDK error-kind spelling across generated SDKs and native
      Go, including `ConnectError.Kind` and per-attempt failure kinds.
- [x] Add checks that fail when generated bindings or package metadata are
      stale relative to the Rust facade.

### B) Release artifacts are built reproducibly

- [x] CI builds Swift, Kotlin, Python, Flutter/Dart, and Go artifacts from a
      clean checkout.
- [x] CI records checksums and package metadata for generated artifacts.
- [x] CI runs package-level import/session smoke tests before artifacts are
      accepted.

### C) Publication safeguards are documented

- [x] Document which registries or internal distribution paths are in scope for
      each package.
- [x] Keep publication manual or dry-run only until credentials and release
      policy are explicitly approved.
- [x] Record rollback or compatibility notes for SDK consumers.

## Cross-Repo Boundaries

- Primary implementation boundary: repo-local CI, task automation, package
  metadata, and release docs.
- Parser / upstream dependency boundary: no parser work expected.
- Downstream integration boundary: no downstream consumer repository is
  modified by this task.
- External asset / catalog / fixture boundary: release artifacts only.
- If another repository is read-write, state what is implemented there versus
  what is implemented in this repository.
  - none.

## Task Dependencies

- backlog/tasks/completed/task-00000022_add-observability-and-conformance-test-matrix.md
- backlog/tasks/completed/task-00000024_package-swift-sdk-as-swiftpm-and-xcframework.md
- backlog/tasks/completed/task-00000025_package-kotlin-sdk-as-jvm-or-android-artifact.md
- backlog/tasks/completed/task-00000026_package-python-sdk-from-the-shared-rust-facade.md
- backlog/tasks/completed/task-00000028_package-flutter-dart-sdk-using-rust-facade.md
- backlog/tasks/completed/task-00000029_package-go-sdk-over-stable-c-abi.md
- backlog/tasks/completed/task-00000032_harden-go-sdk-cache-and-failure-reporting.md
- backlog/plans/plan-00000002_product-secure-tunnel-sdk-and-bindings.md

## Reference Tasks

- backlog/tasks/completed/task-00000015_stabilize-ci-portability-and-add-docker-repro.md

## Backlog Bookkeeping

- `backlog/` is local planning state and is not version controlled by default.
- Moving a task to `backlog/tasks/completed/` is separate from pushing code.
- Commit history for work tracked by this task may live in a different
  read-write repository than this backlog entry.

## Implementation Notes

- [x] Implementation notes added with command evidence.
- Added `backlog/docs/sdk-release-ci-and-versioning.md` covering version source
  of truth, pre-1.0 compatibility policy, stable snake_case SDK error kind
  strings, dry-run release lane outputs, publication safeguards, and rollback
  guidance.
- Added `scripts/sdk_release.py` plus `mise run sdk:release:*` tasks:
  `check-metadata`, `manifest`, and platform-aware `dry-run`.
- Added Linux release dry-run package coverage for Kotlin, Python wheel,
  Flutter/Dart, and Go; added macOS CI dry-run coverage for Swift/XCFramework.
- Added `mise run python:smoke-wheel` so release CI proves a wheel-installed
  Python session smoke, not only an editable/develop install.
- Aligned native Go package `Version` with the Rust workspace version and added
  metadata checks to fail stale package versions.
- Standardized foreign-language SDK error kind strings on snake_case through
  `SdkErrorKind::as_str()`, UniFFI error conversion, UniFFI attempt reports,
  and native Go connect error JSON.
- Replaced hard-coded Kotlin package version paths/consumer dependency in mise
  tasks with `kotlin_sdk_version`.
- Created follow-up task `00000033` for the remaining public Go module/native
  library release layout sharp edge.
- Review fix evidence:
  - Python wheels are copied into `target/sdk-release/artifacts/`, and
    `manifest.json` plus `checksums.txt` reference that uploaded tree.
  - Flutter release archives exclude `.flutter-plugins-dependencies`, build
    directories, `pubspec.lock`, and generated host-specific caches.
  - `crates/sdk-ffi/src/lib.rs` now tests public `SecureTunnelError.kind()`
    and converted per-attempt `failure_kind` snake_case.
- Validation evidence:
  - `python3 scripts/sdk_release.py check-metadata` passed.
  - `shellcheck` passed for new release/Python smoke tasks and touched Kotlin
    task scripts.
  - `cargo test -p secure-tunnel-sdk sdk_error_kind_strings_are_stable_snake_case`
    passed.
  - `cargo check -p secure-tunnel-sdk-ffi -p secure-tunnel-ffi` passed.
  - `mise run sdk:release:dry-run` passed on Linux after review fixes; it
    built/smoked Kotlin, Python wheel, Flutter/Dart, Go, and wrote
    `target/sdk-release/manifest.json` plus `checksums.txt`.
  - `mise run lint-all` passed.
  - `mise run dev` passed: 97 Rust tests, Python tests/smokes, FastAPI smoke,
    and Go tests.

## Implementation Plan

1. [x] Define versioning and stale-binding checks.
2. [x] Add CI release jobs or dry-run jobs for all SDK artifacts.
3. [x] Add checksums, metadata, and package smoke gates.
4. [x] Run release dry-run validation and independent review.

## Review Notes

- Independent review by Boole found two medium blockers:
  1. Python wheel was checksummed from `python/dist` but CI uploaded only
     `target/sdk-release/`.
  2. Flutter archive included `.flutter-plugins-dependencies` with local paths
     and timestamps.
- Fixes copied Python wheels into `target/sdk-release/artifacts`, excluded
  Flutter host/build metadata from the tarball, and added UniFFI snake_case
  error-kind tests.
- Same reviewer re-reviewed the fix and reported no unresolved high or medium
  findings. Residual risk: macOS Swift dry-run was not run locally on this
  Linux host; CI includes a macOS release dry-run job.

## Acceptance Closure

- [x] All acceptance criteria are satisfied and marked.
- [x] Verification commands and outcomes are recorded.
- [x] No unresolved high/medium findings remain.
