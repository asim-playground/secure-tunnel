# Swift SDK Package

`task-00000024` packages the UniFFI Swift bindings and Rust static library as a
local SwiftPM package backed by an XCFramework. That package lane is
macOS/Xcode-only.

Generated package output lives under:

```text
target/sdk/swift/SecureTunnel/
```

That directory is intentionally untracked. It contains generated UniFFI Swift
source and binary artifacts that are not useful to review by hand. The tracked
source of truth is:

- `crates/sdk-ffi/src/secure_tunnel_sdk_ffi.udl`
- `bindings/swift/Package.swift`
- `bindings/swift/Sources/`
- `bindings/swift/Tests/`
- `bindings/swift/Smoke/`
- `mise-tasks/sdk/swift:*`

Useful tasks:

```bash
mise run sdk:smoke-swift
mise run sdk:swift:package
mise run sdk:swift:check-package
mise run sdk:swift:smoke-package
mise run sdk:swift:smoke-ios-simulator
mise run sdk:swift
```

`mise run sdk:smoke-swift` is the cross-platform compiler smoke for generated
UniFFI Swift bindings. It runs on Ubuntu 24.04 and macOS with `swiftc` and the
local Rust dynamic library. It does not build or validate the SwiftPM binary
package.

The package uses `SecureTunnel` as the public Swift module and keeps the
generated UniFFI C module name `secure_tunnel_sdk_ffiFFI` internal to the binary
target. Swift/iOS is the first production-grade SDK target; Kotlin, Python,
Flutter/Dart, and Go remain follow-on package targets.
