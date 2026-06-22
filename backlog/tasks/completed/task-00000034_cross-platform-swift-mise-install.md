# Task 00000034 - Cross-Platform Swift mise Install

## Summary

Make the Swift toolchain installable through `mise` on macOS and Ubuntu 24.04,
and make the generated UniFFI Swift smoke runnable on Ubuntu while preserving
the macOS-only SwiftPM/XCFramework package lane.

## Motivation

Task 14 left `mise run sdk:smoke` unable to complete on this Ubuntu 24.04
ARM64 host because `swiftc` was not installed. Swift is now needed for a local
LAN client trial, but Linux validation should only prove the generated Swift
bindings and C ABI smoke. Apple package distribution remains macOS/Xcode-only.

## Read-Write Repository

- Primary read-write repository: `/home/ubuntu/workplace/secure-tunnel`
- Secondary read-write repository/repositories: none
- Code changes land in this repository. `backlog/` task notes remain local
  planning state unless explicitly tracked.

## Read-Only Reference Repository

- Read-only reference repository/repositories: none
- External references are the official mise Swift backend documentation,
  Swift.org Ubuntu 24.04 install page, and mise-action locked install behavior.

## Detailed Requirements / Acceptance Criteria

### A) Swift Toolchain Pinning

- [x] `mise.toml` pins Swift `6.3.2`.
- [x] Linux config uses `swift.platform = "ubuntu24.04"` without forcing that
  platform on macOS.
- [x] `mise.lock` contains Swift URL entries for Linux ARM64, Linux x64,
  macOS ARM64, and macOS x64.
- [x] Locked Swift install validation no longer fails with a missing Swift
  lockfile URL on Ubuntu ARM64.

### B) Cross-Platform Setup

- [x] `mise run swift:setup` exists.
- [x] On Ubuntu 24.04, setup installs or clearly verifies Swift runtime/build
  prerequisites.
- [x] On macOS, setup verifies `swift`, `swiftc`, and Xcode command-line tools.
- [x] Unsupported OSes fail with a clear Swift support message.
- [x] `mise run setup` invokes Swift setup on Linux and macOS while leaving
  unsupported platforms out of scope.

### C) Swift Smoke And Packaging Split

- [x] `mise run sdk:smoke-swift` works as the cross-platform generated UniFFI
  Swift smoke and only requires `swiftc`, the generated bindings, local Rust
  dynamic library, and the binding fixture.
- [x] `mise run sdk:swift:*` package, XCFramework, and iOS simulator tasks
  remain macOS-only.
- [x] `mise run sdk:smoke` handles Swift smoke availability explicitly instead
  of failing opaquely when `swiftc` is absent.

### D) CI And Documentation

- [x] Ubuntu CI proves the direct Swift UniFFI smoke.
- [x] macOS CI continues to prove SwiftPM/XCFramework/iOS simulator packaging.
- [x] Windows CI is removed from this repository's matrix for this pass.
- [x] Swift and release docs describe the Linux/macOS split.

## Cross-Repo Boundaries

- Primary implementation boundary: repo-local mise tasks, GitHub Actions, docs,
  and lockfile.
- Parser / upstream dependency boundary: do not change UniFFI code generation
  or Swift package wire shape.
- Downstream integration boundary: LAN trial consumer project is out of scope.
- External asset / catalog / fixture boundary: no external artifacts are
  checked in beyond `mise.lock` metadata.

## Task Dependencies

- Task 14: explicit WSS HTTP proxy support exposed the current missing Swift
  compiler gap in `mise run sdk:smoke`.
- Task 24: SwiftPM/XCFramework package lane remains the macOS packaging owner.
- Task 27: release dry-run split remains Linux for non-Swift packages and macOS
  for Swift package artifacts.

## Reference Tasks

- `backlog/tasks/completed/task-00000014_allow-optional-http-proxy-for-wss-client.md`
- `backlog/tasks/completed/task-00000024_package-swift-sdk-as-swiftpm-and-xcframework.md`
- `backlog/tasks/completed/task-00000027_add-sdk-release-ci-and-versioning.md`

## Backlog Bookkeeping

- `backlog/` is local planning state and is not version controlled by default.
- Moving a task to `backlog/tasks/completed/` is separate from pushing code.
- Commit history for work tracked by this task may live in a different
  read-write repository than this backlog entry.

## Implementation Notes

- [x] Implementation notes added with command evidence.
- Added `swift = { version = "6.3.2", os = ["linux", "macos"] }` to
  `mise.toml`.
- Added `scripts/mise-env.sh` so Linux exports `MISE_SWIFT_PLATFORM=ubuntu24.04`
  without applying that platform setting to macOS.
- Added `mise-tasks/swift/setup` for Ubuntu 24.04 prerequisite installation or
  verification, macOS Swift/Xcode CLI verification, and explicit unsupported-OS
  failure.
- Wired `mise run setup` to call `swift:setup` on Linux and macOS.
- Made `sdk:smoke-swift` validate `swiftc` before building generated UniFFI
  Swift smoke inputs.
- Made `sdk:smoke` run Swift only when `swiftc` is present, unless explicitly
  skipped with `SECURE_TUNNEL_SKIP_SWIFT_SMOKE=1`; otherwise it fails with a
  clear install/skip message.
- Removed Windows from the GitHub Actions test matrix per project direction.
- Added Ubuntu CI coverage for `mise run sdk:smoke-swift`; macOS CI still runs
  `mise run sdk:swift`.
- Updated Swift README and release docs to describe Linux compiler-level
  UniFFI validation versus macOS/Xcode package distribution.
- Added Swift lock URLs for Linux ARM64, Linux x64, macOS ARM64, and macOS x64.
  `mise lock` emitted only the Linux ARM64 checksum locally, so the URLs were
  filled in explicitly from Swift.org release artifacts.
- Added SHA-256 checksums for Linux x64 and the shared macOS package artifact
  after streaming the Swift.org release artifacts through `sha256sum`.
- Fixed an adjacent release-lane break in `bindings/flutter/rust/src/api.rs` by
  defaulting the Task 14 `wss_http_proxy` SDK field to `None` in the Flutter
  bridge config conversion. `mise run sdk:release:dry-run` exposed this and
  passed after the fix.
- Pinned all Ubuntu GitHub Actions jobs to `ubuntu-24.04` so CI matches the
  Ubuntu 24.04 Swift setup contract.
- Added `mise run sdk:smoke-swift` to the Linux branch of
  `mise run sdk:release:dry-run`, matching the release-lane documentation.

Validation evidence:

- `MISE_VERBOSE=1 mise install --dry-run swift@6.3.2` showed the sourced Linux
  Swift platform environment.
- `mise env | rg MISE_SWIFT_PLATFORM` printed `MISE_SWIFT_PLATFORM=ubuntu24.04`.
- `mise install --dry-run --locked swift@6.3.2` passed after adding lock URLs.
- `tmpdir=$(mktemp -d); MISE_DATA_DIR="$tmpdir/data" MISE_CACHE_DIR="$tmpdir/cache" mise install --dry-run --locked swift@6.3.2; rm -rf "$tmpdir"`
  passed after adding lock URLs/checksums, proving the lock does not rely on an
  already-installed Swift.
- `mise run swift:setup` passed on Ubuntu 24.04 ARM64 and verified Swift
  `6.3.2`.
- `mise x -- swift --version` passed with Swift `6.3.2`.
- `mise x -- swiftc --version` passed with Swift `6.3.2`.
- `mise run sdk:smoke-swift` passed.
- `mise run sdk:smoke` passed.
- `mise run setup` passed.
- `mise run sdk:flutter:check-package` passed after the Flutter bridge fix.
- `mise run sdk:release:dry-run` passed after the Flutter bridge fix.
- `mise run sdk:release:dry-run` passed again after adding Swift smoke to the
  Linux release lane.
- `mise run lint-all` passed.
- `mise run dev` passed: 106 Rust tests, 11 Python tests, Python package smoke,
  Python FastAPI smoke, and Go tests passed.

## Implementation Plan

1. Pin Swift in `mise.toml`, add Linux platform environment handling, and
   refresh Swift lockfile entries.
2. Add `swift:setup`, wire it into `setup`, and update CI so Ubuntu runs the
   direct Swift smoke while macOS keeps the package smoke.
3. Make Swift smoke behavior explicit, update docs, and validate targeted
   commands before broader gates.
4. Run independent review/re-review, complete task notes, move task to
   completed, then describe/push with `jj`.

## Review Notes

- Independent reviewer Faraday found three medium issues: Linux release
  dry-run documentation claimed Swift smoke but the dry-run did not run it;
  Linux x64/macOS Swift lock entries had URLs but no checksums; and CI used
  floating `ubuntu-latest` despite `swift:setup` requiring Ubuntu 24.04.
- Fixed all three findings by adding `sdk:smoke-swift` to Linux
  `sdk:release:dry-run`, adding SHA-256 checksums for missing Swift lock
  platforms, and pinning Ubuntu CI runners to `ubuntu-24.04`.
- Re-review by Faraday found no unresolved high- or medium-severity issues.

## Acceptance Closure

- [x] All acceptance criteria are satisfied and marked.
- [x] Verification commands and outcomes are recorded.
- [x] No unresolved high/medium findings remain.
