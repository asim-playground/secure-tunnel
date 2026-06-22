# Kotlin SDK Package

`task-00000025` packages the UniFFI Kotlin bindings and the Rust dynamic
library as a JVM-first Gradle package.

Generated package output lives under:

```text
target/sdk/kotlin/SecureTunnelKotlin/
```

That directory is intentionally untracked. It contains generated UniFFI Kotlin
source and native artifacts that are not useful to review by hand. The tracked
source of truth is:

- `crates/sdk-ffi/src/secure_tunnel_sdk_ffi.udl`
- `bindings/kotlin/build.gradle.kts`
- `bindings/kotlin/settings.gradle.kts`
- `bindings/smoke/kotlin/src/main/kotlin/Smoke.kt`
- `mise-tasks/sdk/kotlin:*`

Useful tasks:

```bash
mise run sdk:kotlin:package
mise run sdk:kotlin:check-package
mise run sdk:kotlin:smoke-package
mise run sdk:kotlin
```

The first artifact is a JVM package for the host ABI. It depends on JNA and
packages the host `secure_tunnel_sdk_ffi` dynamic library under JNA's platform
resource path so a consuming JVM application can load it without a local
`libraryOverride` property. Android AAR packaging, proxy-authored variants, and
cross-compiled ABI bundles remain future work.
