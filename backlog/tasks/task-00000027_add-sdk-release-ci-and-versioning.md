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

- [ ] Define how Rust crate, UniFFI facade, Swift, Kotlin, and Python package
      versions move together.
- [ ] Define how Flutter/Dart and Go package versions relate to the Rust SDK
      facade and native package releases.
- [ ] Define compatibility policy for generated SDK APIs while UniFFI remains
      pre-1.0.
- [ ] Add checks that fail when generated bindings or package metadata are
      stale relative to the Rust facade.

### B) Release artifacts are built reproducibly

- [ ] CI builds Swift, Kotlin, Python, Flutter/Dart, and Go artifacts from a
      clean checkout.
- [ ] CI records checksums and package metadata for generated artifacts.
- [ ] CI runs package-level import/session smoke tests before artifacts are
      accepted.

### C) Publication safeguards are documented

- [ ] Document which registries or internal distribution paths are in scope for
      each package.
- [ ] Keep publication manual or dry-run only until credentials and release
      policy are explicitly approved.
- [ ] Record rollback or compatibility notes for SDK consumers.

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
- backlog/tasks/task-00000025_package-kotlin-sdk-as-jvm-or-android-artifact.md
- backlog/tasks/task-00000026_package-python-sdk-from-the-shared-rust-facade.md
- backlog/tasks/task-00000028_package-flutter-dart-sdk-using-rust-facade.md
- backlog/tasks/task-00000029_package-go-sdk-over-stable-c-abi.md
- backlog/plans/plan-00000002_product-secure-tunnel-sdk-and-bindings.md

## Reference Tasks

- backlog/tasks/completed/task-00000015_stabilize-ci-portability-and-add-docker-repro.md

## Backlog Bookkeeping

- `backlog/` is local planning state and is not version controlled by default.
- Moving a task to `backlog/tasks/completed/` is separate from pushing code.
- Commit history for work tracked by this task may live in a different
  read-write repository than this backlog entry.

## Implementation Notes

- [ ] Implementation notes added with command evidence.
- (fill in after completion)

## Implementation Plan

1. Define versioning and stale-binding checks.
2. Add CI release jobs or dry-run jobs for all SDK artifacts.
3. Add checksums, metadata, and package smoke gates.
4. Run release dry-run validation and independent review.

## Review Notes

## Acceptance Closure

- [ ] All acceptance criteria are satisfied and marked.
- [ ] Verification commands and outcomes are recorded.
- [ ] No unresolved high/medium findings remain.
