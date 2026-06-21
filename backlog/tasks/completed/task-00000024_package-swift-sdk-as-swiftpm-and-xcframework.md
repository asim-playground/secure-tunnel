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
  - `/Users/asimi/Downloads/references/uniffi-rs`
  - `/Users/asimi/Downloads/references/application-services`
  - `/Users/asimi/Downloads/references/uniffi-starter`
  - `/Users/asimi/Downloads/references/cargo-swift`
  - `/Users/asimi/workplace/flutter_template`
- State explicitly which repositories may be inspected only for reference or
  legacy behavior:
  - examples are read-only; implementation changes land in `secure-tunnel`.

## Detailed Requirements / Acceptance Criteria

### A) Swift package artifact exists

- [x] Build the Rust library for the required Apple targets for the first SDK
      slice.
- [x] Assemble an XCFramework or equivalent artifact with the generated Swift
      binding and native library.
- [x] Add SwiftPM metadata or package layout for local consumption.
- [x] Document Swift/iOS as the first production-grade package target and list
      which other SDKs are still smoke-parity or follow-on targets.

### B) Swift smoke test proves consumption

- [x] Add a Swift import/build smoke test.
- [x] Run at least one descriptor/config/session scenario through the Swift
      package or a documented local fixture.
- [x] Preserve or upgrade the task-23 Swift generated-client smoke that already
      connects, authenticates, sends one encrypted application request, and
      closes against the Rust fixture.
- [x] Include an iOS simulator smoke path suitable for reuse by the future
      Flutter/Dart package task; descriptor-only validation is not enough for
      task closure unless the task explicitly downgrades the production-grade
      Swift/iOS claim and creates a follow-up.
- [x] Record any Xcode/Swift concurrency caveats discovered during the task.

### C) Packaging is automated

- [x] Add deterministic `mise` tasks for Swift SDK build and smoke tests.
- [x] Add macOS CI coverage for Swift package artifact checks and at least one
      package import/session smoke, while gating Apple-only work out of Linux
      jobs.
- [x] Keep the manual C ABI package path building until the UniFFI Swift path
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

- backlog/tasks/completed/task-00000022_add-observability-and-conformance-test-matrix.md
- backlog/tasks/completed/task-00000023_create-uniffi-sdk-facade-and-bindgen-tooling.md
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

- [x] Implementation notes added with command evidence.
- Planning evidence:
  - Local toolchain on `2026-06-21`: `swift --version` reports Apple Swift
    `6.3.2`; `xcodebuild -version` reports Xcode `26.5`.
  - Existing repo environment sets `MACOSX_DEPLOYMENT_TARGET=11.0`; task 24
    should explicitly lock Swift tools version and minimum iOS/macOS deployment
    targets in generated SwiftPM metadata.
  - Installed Rust Apple target before implementation is only
    `aarch64-apple-darwin`; task implementation must install or assert the iOS
    targets it needs.
  - UniFFI Swift generation currently emits
    `secure_tunnel_sdk_ffi.swift`, `secure_tunnel_sdk_ffiFFI.h`, and
    `secure_tunnel_sdk_ffiFFI.modulemap` under
    `target/generated-bindings/uniffi/swift`.
  - UniFFI docs say XCFramework consumers expect the module map to be named
    `module.modulemap`; the generated module name can remain the lower-level
    `secure_tunnel_sdk_ffiFFI` module while the public SwiftPM product exports
    a nicer `SecureTunnel` target.
  - `uniffi-starter` and `application-services` both use a local Swift package
    with a binary XCFramework target plus Swift source target. This task should
    follow that shape with repo-owned scripts rather than adopting
    `cargo-swift` as a hard dependency.
- Implemented tracked Swift package templates under `bindings/swift/` and
  generated package output under `target/sdk/swift/SecureTunnel`.
- The generated package exposes a public `SecureTunnel` SwiftPM product with a
  source target plus a relative binary target named
  `secure_tunnel_sdk_ffiFFI`.
- The Swift package target copies generated UniFFI Swift source from
  `target/generated-bindings/uniffi/swift` and keeps generated sources and
  binary artifacts untracked under `target/`.
- Built release `staticlib` artifacts for:
  - `aarch64-apple-ios`
  - `aarch64-apple-ios-sim`
  - `x86_64-apple-ios`
  - `aarch64-apple-darwin`
  - `x86_64-apple-darwin`
- The package task combines simulator and macOS universal static libraries with
  `lipo`, then assembles device, simulator, and macOS slices with
  `xcodebuild -create-xcframework`.
- Added deterministic Swift SDK tasks:
  - `mise run sdk:swift:ensure-targets`
  - `mise run sdk:swift:package`
  - `mise run sdk:swift:check-package`
  - `mise run sdk:swift:smoke-package`
  - `mise run sdk:swift:smoke-ios-simulator`
  - `mise run sdk:swift`
- Added a SwiftPM command-line smoke consumer that imports `SecureTunnel`,
  reads the Rust binding fixture JSON, connects over `QUIC`, verifies the
  service static public key pin, authenticates, sends `smoke-ping`, receives
  `smoke-pong`, and closes gracefully.
- Added an iOS simulator XCTest smoke that imports the same package and runs
  the same fixture-backed descriptor/config/connect/auth/request/close
  scenario.
- Cleared `LIBRARY_PATH` only for the Xcode simulator smoke because the repo
  environment can point it at the macOS SDK `libobjc`, which causes iOS
  simulator link failures. The SwiftPM command-line smoke does not need this
  workaround.
- Xcode caveat: `xcodebuild` emits an `IDERunDestination` warning about an empty
  supported-platforms list while testing the Swift package, but the generated
  scheme resolves an iOS simulator destination and the XCTest suite passes.
- Wired `mise run sdk:swift` into the Darwin branch of `mise run ci` and added
  a macOS-only GitHub Actions step after the full test suite. Linux and Windows
  jobs do not try to build Apple artifacts.
- Command evidence collected so far:
  - `cargo build -p secure-tunnel-sdk-ffi --lib --release --target
    aarch64-apple-ios-sim`
  - `cargo build -p secure-tunnel-sdk-ffi --lib --release --target
    aarch64-apple-ios`
  - `cargo build -p secure-tunnel-sdk-ffi --lib --release --target
    x86_64-apple-ios`
  - `cargo build -p secure-tunnel-sdk-ffi --lib --release --target
    x86_64-apple-darwin`
  - `mise run sdk:swift:package`
  - `SECURE_TUNNEL_SWIFT_SKIP_PACKAGE=1 mise run sdk:swift:check-package`
  - `SECURE_TUNNEL_SWIFT_SKIP_PACKAGE=1 mise run sdk:swift:smoke-package`
  - `SECURE_TUNNEL_SWIFT_SKIP_PACKAGE=1 mise run
    sdk:swift:smoke-ios-simulator`
  - `mise run sdk:swift`
- Final command evidence before independent review:
  - `shellcheck mise-tasks/ci mise-tasks/sdk/swift/ensure-targets
    mise-tasks/sdk/swift/package mise-tasks/sdk/swift/check-package
    mise-tasks/sdk/swift/smoke-package
    mise-tasks/sdk/swift/smoke-ios-simulator mise-tasks/sdk/swift/_default`
  - `mise run markdown-lint`
  - `mise run gha-lint`
  - `mise run sdk:check-bindings`
  - `mise run sdk:smoke-swift`
  - `mise run dev`
  - `mise run ci`
- Independent review follow-up:
  - Hardened `mise run sdk:swift:ensure-targets` to resolve the active
    `rustup` toolchain explicitly, matching the repo's WASM target installer.
  - `shellcheck mise-tasks/sdk/swift/ensure-targets
    mise-tasks/sdk/swift/package mise-tasks/sdk/swift/check-package
    mise-tasks/sdk/swift/smoke-package
    mise-tasks/sdk/swift/smoke-ios-simulator mise-tasks/sdk/swift/_default
    mise-tasks/ci`
  - `mise run sdk:swift:ensure-targets`
  - `mise run sdk:swift`

## Implementation Plan

1. [x] Lock the first Apple target matrix and setup contract.
   - Required for task completion: `aarch64-apple-ios` for device,
     `aarch64-apple-ios-sim` for Apple Silicon simulator, and
     `aarch64-apple-darwin` for local `swift test` or host smoke.
   - Optional if quick and stable: `x86_64-apple-ios` simulator support for
     Intel simulator compatibility. If included, explicitly build the
     simulator universal slice with `lipo` before XCFramework assembly.
   - Lock generated SwiftPM metadata to a concrete Swift tools version and
     minimum deployment targets. Proposed baseline: Swift tools `5.10`,
     iOS `16.0`, and macOS `11.0` unless implementation evidence points to a
     safer floor.
   - Add a deterministic `mise run sdk:swift:ensure-targets` or equivalent
     Darwin-gated helper instead of relying on global rustup state.
   - Keep generic Linux CI from trying to build Apple artifacts; expose a clear
     platform gate.
2. [x] Create the generated Swift package layout under `target/`.
   - Generate Swift bindings with the project-local bindgen from task 23.
   - Copy generated Swift source into a generated package directory such as
     `target/sdk/swift/SecureTunnel`.
   - Keep generated Swift source untracked because UniFFI output exceeds the
     repo's non-Markdown code-file review limit.
   - Track only templates/scripts/docs under `bindings/swift/`.
3. [x] Build static Rust libraries for Apple targets.
   - Use `secure-tunnel-sdk-ffi` `staticlib` artifacts, not the development
     host dylib, for the package.
   - Build release artifacts per target with reproducible environment defaults
     such as deployment target and stripped debug info.
   - Preserve the manual C ABI build path while the UniFFI package becomes the
     primary Swift SDK.
4. [x] Assemble an XCFramework for SwiftPM consumption.
   - Stage UniFFI headers and rename/copy the module map as
     `module.modulemap`.
   - Reuse the task-23 project-local bindgen output for Swift source/header
     generation in this task; do not add `cargo-swift` or a second bindgen path
     unless the generic bindgen output cannot produce a valid XCFramework
     module layout.
   - Use `xcodebuild -create-xcframework` to combine iOS device, iOS simulator,
     and host macOS slices where supported.
   - Add an artifact check that inspects the XCFramework `Info.plist`, expected
     libraries, headers, and module map.
   - Zip and compute a SwiftPM checksum only as a release-prep output; local
     task 24 consumption can use a relative binary target path.
5. [x] Add SwiftPM package metadata and wrapper target.
   - Public package/product name: `SecureTunnel`.
   - Internal binary target can keep the generated FFI module name
     `secure_tunnel_sdk_ffiFFI`.
   - Public source target should compile generated UniFFI Swift plus a small
     hand-written Swift facade only if needed for naming, docs, or future
     ergonomics; do not hand-wrap the whole API in this task.
   - Add README guidance documenting Swift/iOS as production target one and
     Kotlin/Python/Flutter/Go as follow-on or smoke-parity targets.
6. [x] Replace or augment the task-23 Swift smoke with package consumption.
   - Keep the existing direct `swiftc` smoke as a fast generated-binding check.
   - Add `mise run sdk:swift:package` to build the package artifact.
   - Add `mise run sdk:swift:smoke-package` to import `SecureTunnel` through
     SwiftPM and run the existing descriptor/config/connect/auth/request/close
     fixture scenario.
   - Add an iOS simulator XCTest/Xcode harness that imports the SwiftPM package
     and runs the same descriptor/config/connect/auth/request/close fixture
     scenario. If that proves impossible in this task, do not close task 24 as
     the production-grade Swift/iOS package; instead document the blocker and
     split an explicit follow-up before closure.
7. [x] Wire validation and CI.
   - Add a macOS-only `sdk:swift` aggregate task for target install, package
     build, artifact checks, SwiftPM host smoke, and simulator smoke when
     available.
   - Add the Swift package artifact checks and at least one package
     import/session smoke to the existing macOS GitHub Actions lane. Keep Linux
     and non-Apple jobs gated out of Apple-specific build steps.
   - Run `mise run dev`, `mise run sdk:check-bindings`,
     `mise run sdk:smoke-swift`, the new Swift package tasks, shellcheck, and
     markdown lint.
8. [x] Complete review and closure.
   - Record implementation evidence, Xcode/Swift caveats, and any skipped
     Apple target in this task file.
   - Obtain independent review and re-review until no unresolved high/medium
     findings remain.
   - Mark acceptance criteria, update the parent SDK plan, move the task to
     `backlog/tasks/completed/`, and push with `jj`.

## Review Notes

- Plan review found two medium issues: the first draft allowed iOS simulator
  smoke to stop at descriptor/config validation, and allowed local-only CI
  documentation for the first production Swift/iOS target. The plan now
  requires iOS simulator package session smoke for production-grade closure and
  macOS CI coverage for Swift package artifact checks plus package
  import/session smoke.
- Plan re-review found no remaining high- or medium-severity issues. Residual
  risk is implementation-level: the generic UniFFI modulemap/XCFramework layout
  still needs to be proven by the planned artifact checks and SwiftPM package
  smoke.
- Implementation review found no high- or medium-severity issues. It noted one
  low-risk follow-up: the Swift target installer used bare `rustup target add`
  instead of passing the active toolchain explicitly.
- Fixed the low-risk follow-up by aligning `mise run sdk:swift:ensure-targets`
  with the repo's WASM target installer: derive the active Rust toolchain,
  locate the `rustup` next to `cargo` when available, and pass `--toolchain`.
- Re-review found no high- or medium-severity findings and confirmed the
  low-risk toolchain follow-up is resolved. The reviewer also confirmed the
  repository's non-Markdown code-file line limit remains satisfied.

## Acceptance Closure

- [x] All acceptance criteria are satisfied and marked.
- [x] Verification commands and outcomes are recorded.
- [x] No unresolved high/medium findings remain.
