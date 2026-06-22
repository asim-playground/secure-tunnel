# Secure Tunnel Flutter SDK

`task-00000028` packages a Flutter/Dart SDK over the Rust product SDK facade
using Flutter Rust Bridge and Dart native assets.

Generated package output lives under:

```text
target/sdk/flutter/secure_tunnel_flutter/
```

That directory is intentionally untracked. It contains generated Flutter Rust
Bridge Dart and Rust files plus native build output that is not useful to
review by hand. The tracked source of truth is:

- `bindings/flutter/**`
- `mise-tasks/sdk/flutter:*`
- `crates/sdk/**`

Useful tasks:

```bash
mise run sdk:flutter:package
mise run sdk:flutter:check-package
mise run sdk:flutter:smoke-package
mise run sdk:flutter
```

The first package is a host ABI Flutter FFI package. iOS simulator native smoke
is wired as an operator task and is not part of the default local or CI loop.
