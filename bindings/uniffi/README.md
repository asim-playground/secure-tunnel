# UniFFI Bindings

`task-00000023` keeps UniFFI source generation deterministic without checking
generated sources into the repository.

Generated Swift, Kotlin, and Python sources are written to:

```text
target/generated-bindings/uniffi/
```

The generated files are intentionally not tracked. Current UniFFI output is
thousands of lines per language, which would violate this repository's
non-Markdown code-file review limit and make hand review harder. The tracked
contract is the UDL file in `crates/sdk-ffi/src/secure_tunnel_sdk_ffi.udl`, the
Rust facade crate, and the smoke clients under `bindings/smoke/`.

Useful tasks:

```bash
mise run sdk:generate-bindings
mise run sdk:check-bindings
mise run sdk:smoke-python
mise run sdk:smoke-swift
mise run sdk:smoke-kotlin
mise run sdk:smoke
```

The smoke tasks start a local Rust `QUIC` and `WSS` fixture, generate language
bindings with the project-local pinned bindgen binary, and prove each generated
client can connect, authenticate the account, send one encrypted application
request, and close.

Swift/iOS production packaging is handled by `task-00000024`. Kotlin packaging
is handled by `task-00000025`. Python packaging and the Python FastAPI server
interop path are handled by `task-00000026` and follow-up task work. Flutter/Dart
and Go are outside UniFFI scope and are covered by `task-00000028` and
`task-00000029`.
