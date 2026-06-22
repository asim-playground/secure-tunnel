# SDK release CI and versioning

Task `00000027` defines the dry-run release lane for Secure Tunnel SDK
artifacts. Publication remains manual and disabled until credentials, registry
ownership, signing policy, and release approval are explicitly documented.

## Version source of truth

`Cargo.toml` `[workspace.package].version` is the intended SDK release version.
The following package metadata must match it:

- Rust `secure-tunnel-sdk` and `secure-tunnel-sdk-ffi` dependency versions.
- Python `python/pyproject.toml` `[project].version`.
- Kotlin `bindings/kotlin/build.gradle.kts` `version`.
- Flutter/Dart `bindings/flutter/pubspec.yaml` `version`.
- Flutter bridge crate `bindings/flutter/rust/Cargo.toml` package and
  `secure-tunnel-*` dependency versions.
- Native Go `crates/go/types.go` `Version`.

SwiftPM path packages do not carry a package version in `Package.swift`; the
Swift SDK version is the Git tag or release artifact version that matches the
Rust workspace version. The dry-run metadata check still verifies the Swift
package and product names.

Run this before release work:

```bash
mise run sdk:release:check-metadata
```

## Compatibility policy before 1.0

All SDKs are pre-1.0. Patch releases may add compatible API surface and bug
fixes but must not remove fields, rename functions, or change stable string
spelling. Minor releases may make generated API breaking changes, but release
notes must call out migration steps and the release dry-run manifest must be
kept with the release evidence.

Generated Swift, Kotlin, and Python APIs are derived from the UniFFI facade and
may still move while the facade is pre-1.0. The tracked contract is the Rust SDK
facade, `crates/sdk-ffi/src/secure_tunnel_sdk_ffi.udl`, and language smoke
clients. Generated sources remain untracked.

External SDK error kind strings are stable snake_case. This includes
`SecureTunnelError.kind()` from UniFFI-generated SDKs, native Go
`ConnectError.Kind`, and per-attempt `failure_kind` values. Additive error
kinds may appear in a minor release; renames and removals require a migration
note and a version bump that is treated as breaking for SDK consumers. Binding
configuration errors use `invalid_config`; unmapped internal errors use
`internal`.

## Dry-run release lane

Use the platform-aware release dry-run:

```bash
mise run sdk:release:dry-run
```

Linux builds and smokes:

- UniFFI generated binding presence and untracked-output policy.
- Kotlin JVM package, local Maven artifact, and fixture-backed consumer smoke.
- Python wheel import check and wheel-installed fixture-backed session smoke.
- Flutter/Dart generated package, analyzer/import tests, and session smoke.
- Native Go cgo package, generated C header freshness, race tests, and session
  smoke.

macOS builds and smokes:

- UniFFI generated binding presence and untracked-output policy.
- SwiftPM package, XCFramework, macOS consumer smoke, and iOS simulator smoke.

The dry-run writes:

```text
target/sdk-release/manifest.json
target/sdk-release/checksums.txt
target/sdk-release/artifacts/
```

The manifest records package metadata and SHA-256 checksums for generated
release evidence. CI uploads `target/sdk-release/` for Linux and macOS dry-run
jobs.

## Publication safeguards

Publication is out of scope for the dry-run task. Current distribution scope:

- Rust crates: crates.io publication is manual only and requires a separate
  approval task.
- Swift: GitHub release or internal artifact distribution of the SwiftPM
  package/XCFramework; no registry credential is required for path validation.
- Kotlin: local Maven artifact in dry-run; Maven Central or GitHub Packages
  publication requires credentials and signing policy approval.
- Python: wheel artifact in dry-run; PyPI publication is disabled until package
  ownership and token policy are approved.
- Flutter/Dart: `publish_to: none` blocks pub.dev publication by default.
- Go: dry-run source module bundle at
  `github.com/asim-playground/secure-tunnel/crates/go`, packaged with
  `native.json`, `binding.h`, and host `secure_tunnel_ffi` under
  `native/<goos>-<goarch>/`. Consumers must use the documented native library
  path or install equivalent checked artifacts before loading the cgo package.

Rollback guidance: keep the previous release artifact manifest and checksums
available, revert consumers to the previous version or tag, and avoid reusing a
published version number for changed bits. If an SDK compatibility issue is
found after release, publish a new patch or minor version with explicit notes;
do not mutate existing package artifacts.
