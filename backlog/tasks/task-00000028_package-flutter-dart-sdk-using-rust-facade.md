# Task `00000028` - `package flutter dart sdk using rust facade`

## Summary

Package a Flutter/Dart SDK surface over the same Rust product facade used by
the native SDKs, using the Flutter Rust Bridge pattern from
`~/workplace/flutter_template` as the primary reference.

## Motivation

Swift, Kotlin, and Python can share a UniFFI facade, but Flutter/Dart is a
different consumer shape. Flutter should get a first-class SDK path without
forcing Dart through Swift/Kotlin wrappers or exposing internal Rust transport
types. The local Flutter template already has useful patterns for generated
bridge policy, hand-written Dart facades, native iOS smoke tests, and `mise`
task wiring.

## Read-Write Repository

- Primary read-write repository: `/Users/asimi/workplace/secure-tunnel`
- Secondary read-write repository/repositories (if applicable): none
- State explicitly:
  - code changes are expected to land in Flutter/Dart package files, Rust
    bridge/facade integration, generated-code policy, tests, and task
    automation in this repository.
  - this repository itself is expected to change.

## Read-Only Reference Repository

- Read-only reference repository/repositories:
  - `/Users/asimi/workplace/flutter_template`
  - `/Users/asimi/Downloads/references/flutter_rust_bridge`
  - `/Users/asimi/Downloads/references/dart-native`
- State explicitly which repositories may be inspected only for reference or
  legacy behavior:
  - these references may be inspected for Flutter Rust Bridge, Dart FFI,
    generated-code policy, iOS native smoke, and `mise` task patterns, but
    changes land only in `secure-tunnel`.

## Detailed Requirements / Acceptance Criteria

### A) Flutter/Dart binding strategy is explicit

- [ ] Decide the first Flutter/Dart bridge path, with Flutter Rust Bridge as
      the recommended default unless task evidence favors direct Dart FFI plus
      `ffigen`.
- [ ] Keep the Flutter/Dart surface backed by the Rust SDK facade from
      `task-00000018`, not by internal selector, Noise, or transport types.
- [ ] Document how the Flutter/Dart SDK relates to the Swift/iOS package from
      `task-00000024` without making Swift wrappers the Dart API boundary.

### B) Package and generated-code policy exist

- [ ] Add a Flutter/Dart package layout or example app path for the SDK.
- [ ] Add generated-code policy, bridge generation, and drift-check tasks.
- [ ] Add hand-written Dart facades around generated bridge code so app code can
      depend on fakeable interfaces in tests.

### C) Flutter smoke tests prove consumption

- [ ] Add a Flutter/Dart import or analyzer smoke test.
- [ ] Add an iOS simulator native smoke path adapted from `flutter_template`
      once the Rust iOS artifact exists.
- [ ] Run at least one descriptor/config/session scenario through the
      Flutter/Dart facade or a documented local fixture.

## Cross-Repo Boundaries

- Primary implementation boundary: Flutter/Dart SDK package, bridge generation,
  and local smoke tests in this repository.
- Parser / upstream dependency boundary: dependency changes must stay within
  Flutter/Dart bridge and package needs and pass repo supply-chain checks.
- Downstream integration boundary: no downstream Flutter app repository is
  modified by this task.
- External asset / catalog / fixture boundary: generated Dart bridge artifacts
  and local smoke fixtures only.
- If another repository is read-write, state what is implemented there versus
  what is implemented in this repository.
  - none.

## Task Dependencies

- backlog/tasks/task-00000018_define-product-sdk-facade-and-session-contract.md
- backlog/tasks/completed/task-00000021_build-end-to-end-tunnel-harness-and-cli-smoke-path.md
- backlog/tasks/task-00000022_add-observability-and-conformance-test-matrix.md
- backlog/tasks/task-00000024_package-swift-sdk-as-swiftpm-and-xcframework.md
- backlog/plans/plan-00000002_product-secure-tunnel-sdk-and-bindings.md

## Reference Tasks

- backlog/docs/2026-06-21_sdk-reference-repositories.md
- /Users/asimi/workplace/flutter_template/backlog/docs/task-00000037_rust-owned-flutter-api-client-boundary-working-note.md
- /Users/asimi/workplace/flutter_template/backlog/docs/task-00000069_kache-build-cache-migration-working-note.md
- /Users/asimi/workplace/flutter_template/docs/how-to/ios-native-smoke.md

## Backlog Bookkeeping

- `backlog/` is local planning state and is not version controlled by default.
- Moving a task to `backlog/tasks/completed/` is separate from pushing code.
- Commit history for work tracked by this task may live in a different
  read-write repository than this backlog entry.

## Implementation Notes

- [ ] Implementation notes added with command evidence.
- Use `/Users/asimi/workplace/flutter_template` for local operator patterns and
  `/Users/asimi/Downloads/references/flutter_rust_bridge` plus
  `/Users/asimi/Downloads/references/dart-native` for upstream reference
  examples.

## Implementation Plan

1. Compare Flutter Rust Bridge and direct Dart FFI against the Secure Tunnel SDK
   facade and choose the first bridge path.
2. Add Flutter/Dart package layout, generated-code policy, and bridge tasks.
3. Add Dart facade tests and an iOS native smoke path.
4. Run package validation and independent review.

## Review Notes

## Acceptance Closure

- [ ] All acceptance criteria are satisfied and marked.
- [ ] Verification commands and outcomes are recorded.
- [ ] No unresolved high/medium findings remain.
