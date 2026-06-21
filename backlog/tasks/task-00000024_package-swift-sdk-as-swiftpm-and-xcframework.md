# Task `00000024` - `package swift sdk as swiftpm and xcframework`

## Summary

Package the generated Swift binding and native Rust library as the first
production-grade Secure Tunnel SDK.

## Motivation

Swift/iOS is the first production-grade SDK target for Secure Tunnel, and the
previous C ABI work already created Swift import groundwork. The UniFFI path
must prove that an iOS/macOS caller can import the package and run a meaningful
SDK smoke scenario before other SDK targets claim the same production bar.

## Read-Write Repository

- Primary read-write repository: `/Users/asimi/workplace/secure-tunnel`
- Secondary read-write repository/repositories (if applicable): none
- State explicitly:
  - code changes are expected to land in Swift packaging files, generated
    binding integration, Rust build scripts/tasks, and CI.
  - this repository itself is expected to change.

## Read-Only Reference Repository

- Read-only reference repository/repositories:
  - Swift or UniFFI packaging examples may be cloned under
    `/Users/asimi/Downloads/references` if needed.
- State explicitly which repositories may be inspected only for reference or
  legacy behavior:
  - examples are read-only; implementation changes land in `secure-tunnel`.

## Detailed Requirements / Acceptance Criteria

### A) Swift package artifact exists

- [ ] Build the Rust library for the required Apple targets for the first SDK
      slice.
- [ ] Assemble an XCFramework or equivalent artifact with the generated Swift
      binding and native library.
- [ ] Add SwiftPM metadata or package layout for local consumption.
- [ ] Document Swift/iOS as the first production-grade package target and list
      which other SDKs are still smoke-parity or follow-on targets.

### B) Swift smoke test proves consumption

- [ ] Add a Swift import/build smoke test.
- [ ] Run at least one descriptor/config/session scenario through the Swift
      package or a documented local fixture.
- [ ] Include an iOS simulator smoke path suitable for reuse by the future
      Flutter/Dart package task.
- [ ] Record any Xcode/Swift concurrency caveats discovered during the task.

### C) Packaging is automated

- [ ] Add deterministic `mise` tasks for Swift SDK build and smoke tests.
- [ ] Add CI coverage or a documented platform gate for the Swift package.
- [ ] Keep the manual C ABI package path building until the UniFFI Swift path
      is accepted as the primary Swift SDK.

## Cross-Repo Boundaries

- Primary implementation boundary: Swift package and Apple-target Rust build
  tasks in this repository.
- Parser / upstream dependency boundary: no parser work expected.
- Downstream integration boundary: no downstream app repository is modified by
  this task.
- External asset / catalog / fixture boundary: generated Swift and packaged
  artifacts only.
- If another repository is read-write, state what is implemented there versus
  what is implemented in this repository.
  - none.

## Task Dependencies

- backlog/tasks/task-00000022_add-observability-and-conformance-test-matrix.md
- backlog/tasks/task-00000023_create-uniffi-sdk-facade-and-bindgen-tooling.md
- backlog/plans/plan-00000002_product-secure-tunnel-sdk-and-bindings.md

## Reference Tasks

- backlog/docs/2026-06-21_sdk-reference-repositories.md
- backlog/tasks/completed/task-00000016_update-runtimes-deps-and-add-swift-callable-library-surface.md
- backlog/tasks/task-00000028_package-flutter-dart-sdk-using-rust-facade.md

## Backlog Bookkeeping

- `backlog/` is local planning state and is not version controlled by default.
- Moving a task to `backlog/tasks/completed/` is separate from pushing code.
- Commit history for work tracked by this task may live in a different
  read-write repository than this backlog entry.

## Implementation Notes

- [ ] Implementation notes added with command evidence.
- (fill in after completion)

## Implementation Plan

1. Define the Apple target matrix for the first SDK package.
2. Build native Rust libraries and assemble the Swift package artifact.
3. Add and run Swift import/session smoke tests.
4. Run package validation and independent review.

## Review Notes

## Acceptance Closure

- [ ] All acceptance criteria are satisfied and marked.
- [ ] Verification commands and outcomes are recorded.
- [ ] No unresolved high/medium findings remain.
