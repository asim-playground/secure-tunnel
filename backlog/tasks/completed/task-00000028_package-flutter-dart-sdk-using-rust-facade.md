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

- [x] Decide the first Flutter/Dart bridge path, with Flutter Rust Bridge as
      the recommended default unless task evidence favors direct Dart FFI plus
      `ffigen`.
- [x] Keep the Flutter/Dart surface backed by the Rust SDK facade from
      `task-00000018`, not by internal selector, Noise, or transport types.
- [x] Document how the Flutter/Dart SDK relates to the Swift/iOS package from
      `task-00000024` without making Swift wrappers the Dart API boundary.

### B) Package and generated-code policy exist

- [x] Add a Flutter/Dart package layout or example app path for the SDK.
- [x] Add generated-code policy, bridge generation, and drift-check tasks.
- [x] Add hand-written Dart facades around generated bridge code so app code can
      depend on fakeable interfaces in tests.

### C) Flutter smoke tests prove consumption

- [x] Add a Flutter/Dart import or analyzer smoke test.
- [x] Add an iOS simulator native smoke path adapted from `flutter_template`
      once the Rust iOS artifact exists.
- [x] Run at least one descriptor/config/session scenario through the
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
- backlog/tasks/completed/task-00000022_add-observability-and-conformance-test-matrix.md
- backlog/tasks/completed/task-00000024_package-swift-sdk-as-swiftpm-and-xcframework.md
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

- [x] Implementation notes added with command evidence.
- Use `/Users/asimi/workplace/flutter_template` for local operator patterns and
  `/Users/asimi/Downloads/references/flutter_rust_bridge` plus
  `/Users/asimi/Downloads/references/dart-native` for upstream reference
  examples.
- Bridge decision: use Flutter Rust Bridge with Dart native assets. Direct Dart
  FFI plus `ffigen` would duplicate the C ABI ownership model that is reserved
  for Go, while FRB gives Flutter a native-assets package shape and generated
  Dart/Rust bridge code over the Rust SDK facade.
- Generated-code policy: tracked source lives under `bindings/flutter/**`.
  `mise run sdk:flutter:package` copies that template to
  `target/sdk/flutter/secure_tunnel_flutter`, runs FRB codegen there, and keeps
  generated Dart/Rust output untracked.
- Rust bridge boundary: `bindings/flutter/rust/src/api.rs` depends on
  `secure-tunnel-sdk` and mirrors the existing generated-binding runtime model:
  an opaque client owns a Tokio runtime and calls the SDK facade. It does not
  expose selector, Noise, or transport internals.
- Dart boundary: `bindings/flutter/lib/src/client.dart` and
  `bindings/flutter/lib/src/model.dart` provide hand-written fakeable facades
  over generated FRB APIs. App code imports `secure_tunnel_flutter.dart`, not
  generated bridge files.
- Swift/iOS relation: Flutter/Dart is a sibling package over the Rust facade,
  not a wrapper around the SwiftPM/XCFramework package. The iOS simulator path
  is operator-only for now and uses the Flutter native-assets package.
- Tooling note: the hosted native-assets FRB packages currently resolve as
  `2.13.0-beta.2`, so the repo pins `cargo:flutter_rust_bridge_codegen`,
  `flutter_rust_bridge`, `flutter_rust_bridge_hooks`, and the Rust crate
  dependency to that matching version.
- Validation evidence before independent review:
  - `mise run sdk:flutter:package` passed.
  - `SECURE_TUNNEL_FLUTTER_SKIP_PACKAGE=1 mise run sdk:flutter:check-package`
    passed: generated output layout, untracked target policy, `dart format`,
    `flutter analyze`, and import/fake smoke.
  - `SECURE_TUNNEL_FLUTTER_SKIP_PACKAGE=1 mise run sdk:flutter:smoke-package`
    passed: local binding fixture, QUIC connect, service static key check,
    account auth, request/response, and graceful close through the Dart facade.
- Independent review found three medium issues and all were fixed before
  re-review:
  - Empty Dart default config now preserves Rust SDK default pinned service
    static public keys and descriptor trust anchors instead of clearing them.
  - Dart facade connect/auth/request/close are now `Future`-returning APIs, and
    FRB network calls are generated as asynchronous calls rather than sync FFI.
  - `sdk:flutter:smoke-ios-simulator` now runs the generated package test
    against a named iOS simulator on Darwin instead of exiting successfully
    after printing manual instructions.
- Validation evidence after review fix-ups:
  - `mise run sdk:flutter` passed.
  - `mise run dev` passed.
- Re-review outcome: same independent reviewer found no remaining high or
  medium findings. Residual low risk: no dedicated no-config Dart default
  smoke variant yet, but static inspection verifies empty Dart pin/trust-anchor
  lists now preserve Rust SDK defaults.

## Implementation Plan

1. [x] Compare Flutter Rust Bridge and direct Dart FFI against the Secure Tunnel SDK
   facade and choose the first bridge path.
2. [x] Add Flutter/Dart package layout, generated-code policy, and bridge tasks.
3. [x] Add Dart facade tests and an iOS native smoke path.
4. [x] Run package validation and independent review.

## Review Notes

## Acceptance Closure

- [x] All acceptance criteria are satisfied and marked.
- [x] Verification commands and outcomes are recorded.
- [x] No unresolved high/medium findings remain.
