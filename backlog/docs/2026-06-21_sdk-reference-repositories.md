# SDK Reference Repositories

## Purpose

This note records the local reference repositories prepared for the Secure
Tunnel SDK plan. The references are read-only inputs for future implementation
tasks; Secure Tunnel changes land in `/Users/asimi/workplace/secure-tunnel`.

## Local References

| Path | Upstream | Commit | Last Commit Date | Use |
|---|---|---|---|---|
| `/Users/asimi/Downloads/references/uniffi-rs` | `https://github.com/mozilla/uniffi-rs.git` | `0f6a2bc709101c5d4757923ae1bfde704ff8b997` | `2026-06-19` | UniFFI core docs, examples, UDL/proc-macro tradeoffs, bindgen fixtures, versioning policy |
| `/Users/asimi/Downloads/references/application-services` | `https://github.com/mozilla/application-services.git` | `ad7e69cddf8220de08d49c00da3041642ba5fac9` | `2026-06-19` | production Mozilla UniFFI usage and mobile packaging patterns |
| `/Users/asimi/workplace/flutter_template` | `https://github.com/asimihsan/flutter_template.git` | `27440cf40962d602fb35cf3f1cd208c03e71c543` | `2026-06-20` | local Flutter/Rust `mise` task patterns, generated-code policy, iOS native smoke, kache migration |
| `/Users/asimi/Downloads/references/flutter_rust_bridge` | `https://github.com/fzyzcjy/flutter_rust_bridge.git` | `2e9732a974c41e2a508381e1059e432578d9bf54` | `2026-05-23` | Flutter Rust Bridge generator, examples, package layouts, bridge smoke patterns |
| `/Users/asimi/Downloads/references/dart-native` | `https://github.com/dart-lang/native.git` | `c525f5b04382f42afe568722d86b9a81dd1e8aad` | `2026-06-19` | Dart `ffi`, `ffigen`, native assets, JNI/objective-c/native toolchain references |
| `/Users/asimi/Downloads/references/cbindgen` | `https://github.com/mozilla/cbindgen.git` | `b826cb8911488fe8a209d2b693492c0c673e8cca` | `2026-06-09` | generated C header conventions for the manual C ABI and Go package |
| `/Users/asimi/Downloads/references/pyo3` | `https://github.com/PyO3/pyo3.git` | `2ba9cda59a8b2fb07ad9b2b7f20d82e96d7ab0d2` | `2026-03-27` | Python-specific fallback or wrapper reference if UniFFI Python ergonomics are insufficient |
| `/Users/asimi/Downloads/references/maturin` | `https://github.com/PyO3/maturin.git` | `0d789219b0d9c94fb1e09107b36a40b59a493cff` | `2026-06-20` | Python wheel/package workflows for PyO3, cffi, and UniFFI-backed package layouts |
| `/Users/asimi/Downloads/references/uniffi-starter` | `https://github.com/NordSecurity/uniffi-starter.git` | `b466bc276437250cca3b477b4840b49488205a91` | `2025-11-04` | small multi-language UniFFI starter layout and local binding-generation scripts |
| `/Users/asimi/Downloads/references/cargo-swift` | `https://github.com/antoniusnaumann/cargo-swift.git` | `e11f07542b8648f3d30aa8433a9626a8a49c2b09` | `2026-05-20` | Apple static-library and XCFramework build task reference for Swift packaging |

## Planning Conclusions

- Swift/iOS is the first production-grade SDK package target.
- UniFFI remains the default shared binding path for Swift, Kotlin, and Python.
- Flutter/Dart should use the same Rust SDK facade through a Flutter-specific
  bridge path. The recommended default is Flutter Rust Bridge, with direct Dart
  FFI plus `ffigen` kept as the comparison point.
- Go should use the stable manual C ABI and cbindgen-generated header. UniFFI
  third-party Go generators may be inspected later, but they are not the default
  production path for this plan.
- `task-00000023` proves generated Swift, Kotlin, and Python clients can call
  the shared Rust SDK facade against a local Rust server fixture. Python
  FastAPI server interop and Rust-client-to-Python-server coverage belong to
  a follow-up server/package task, because they require a Python server runtime
  surface rather than only binding generation.

## Reference Tasks

- `task-00000023` inspected `uniffi-rs` and `application-services` before
  selecting the pinned UniFFI crate and project-local bindgen shape.
- `task-00000024` should inspect `uniffi-rs`, `application-services`, and
  `flutter_template` iOS smoke patterns before finalizing Swift packaging.
- `task-00000028` should inspect `flutter_template`, `flutter_rust_bridge`, and
  `dart-native` before choosing the first Flutter/Dart bridge path.
- `task-00000029` should inspect `cbindgen` before extending the C ABI and Go
  package.
- `task-00000030` should inspect `maturin`, `pyo3`, `fastapi`, and the
  checked-in UniFFI smoke clients before choosing the Python server package
  shape.
