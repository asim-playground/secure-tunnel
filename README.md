# Secure Tunnel

<p align="center">
  <a href="https://github.com/asim-playground/secure-tunnel/actions/workflows/ci.yml">
    <!-- markdownlint-disable MD013 -->
    <img src="https://img.shields.io/github/actions/workflow/status/asim-playground/secure-tunnel/ci.yml?style=flat-square" alt="CI Status">
  </a>
  <a href="https://codecov.io/gh/asim-playground/secure-tunnel">
    <img src="https://codecov.io/gh/asim-playground/secure-tunnel/branch/main/graph/badge.svg" alt="Coverage">
  </a>
  <img src="https://img.shields.io/badge/Rust-1.96.0-blue?style=flat-square" alt="Rust 1.96.0">
  <a href="https://crates.io/crates/secure-tunnel-core">
    <img src="https://img.shields.io/crates/v/secure-tunnel-core?style=flat-square" alt="Crates.io">
  </a>
  <a href="https://docs.rs/secure-tunnel-core">
    <img src="https://docs.rs/secure-tunnel-core/badge.svg" alt="Documentation">
  </a>
  <a href="LICENSE">
    <img src="https://img.shields.io/badge/License-MPL%202.0-brightgreen.svg?style=flat-square" alt="License">
  </a>
</p>

Secure Tunnel is a multi-platform Rust project with:

- 🦀 **Core Library**: Shared functionality in `secure-tunnel-core`
- 🖥️ **CLI Tool**: Command-line interface in `secure-tunnel-cli`
- 🐍 **Python SDK**: UniFFI-backed package over the shared Rust SDK facade
- 🔗 **Swift SDK and C ABI**: SwiftPM/XCFramework packaging plus C-compatible ABI
- 🦫 **Go Bindings**: CGO-based Go library bindings over the C ABI

## Quick Start

### Prerequisites

- **[Mise](https://mise.jdx.dev/)**: Manages all tools and dependencies
- **Git**: For version control

### Installation

1. **Clone and setup**:

   ```bash
   git clone https://github.com/asim-playground/secure-tunnel.git
   cd secure-tunnel
   ./scripts/dev-setup.sh  # Installs mise if needed
   mise install            # Installs the pinned toolchain and cargo helpers
   ```

2. **Initialize the project**:

   ```bash
   mise run setup          # Sets up local tools and optional language environments
   mise run copyright      # Adds copyright headers
   ```

3. **Build and test**:

   ```bash
   mise run dev            # Fast local loop
   mise run ci             # Canonical full pipeline
   ```

## Development

### Available Commands

- `mise run format` - Format Rust and optional frontend code
- `mise run lint` - Check formatting and run clippy
- `mise run test` - Run the default test suite
- `mise run ci` - Run the canonical local/CI pipeline
- `mise run deps-report` - Report outdated Rust and optional web dependencies
- `mise run deps-check` - Run dependency freshness reporting plus security/license checks
- `mise run rust:test-doc` - Run doctests
- `mise run rust:coverage` - Generate coverage output
- `mise run rust:audit` - Run `cargo audit` and `cargo deny`
- `mise run rust:outdated` - Report outdated dependencies
- `mise run rust:insta-test` - Exercise snapshot assertions
- `mise run rust:insta-review` - Review pending snapshot updates
- `mise run security:test` - Run timeout, cancellation, and stalled-peer hardening tests
- `mise run security:mutants-list` - List mutation-test candidates in security-critical Rust files
- `mise run security:mutants-smoke` - Run a small opt-in cargo-mutants smoke shard

### Project Structure

```plaintext
secure-tunnel/
├── crates/
│   ├── core/       # Core library (secure-tunnel-core)
│   ├── cli/        # Command-line tool (secure-tunnel-cli)
│   ├── go/         # C ABI plus Go bindings
│   ├── go-wasm/    # Rust crate for Go/WASI workflow
│   ├── sdk/        # Product SDK facade
│   └── sdk-ffi/    # UniFFI facade used by Swift, Kotlin, and Python
├── python/         # Python SDK wrapper, packaging, tests, and pyproject metadata
├── mise-tasks/     # Script-backed mise commands
├── scripts/        # Helper scripts used by setup
└── mise.toml       # Toolchain pins and task aliases
```

### Swift And C ABI

The FFI crate emits `libsecure_tunnel_ffi` plus a generated C header at
`crates/go/binding.h`. Swift callers can import that header through the module
map in `crates/go/module.modulemap` and use the descriptor/protocol helpers:

- `secure_tunnel_protocol_id_v1`
- `secure_tunnel_example_service_descriptor_json`
- `secure_tunnel_validate_service_descriptor_json`
- `secure_tunnel_normalize_service_descriptor_json`

Strings returned through `SecureTunnelStringResult.value` are caller-owned and
must be released with `secure_tunnel_free_string`.

### Python SDK

The Python package is built from the shared UniFFI SDK facade and exposes the
stable `secure_tunnel` wrapper module:

```bash
mise run python:build
mise run python:test
mise run python:check-wheel
mise run sdk:python-fastapi-smoke
```

### Go SDK

The native Go SDK is a cgo module under `crates/go`:

```text
github.com/asim-playground/secure-tunnel/crates/go
```

The release dry-run packages the module root with `binding.h`, `native.json`,
and a host `secure_tunnel_ffi` dynamic library under
`native/<goos>-<goarch>/`. The external-consumer smoke unpacks that artifact
outside the monorepo and verifies compile, link, dynamic load, connect,
account auth, request/response, and close:

```bash
mise run sdk:go:smoke-release
```

The public package keeps the historical descriptor helpers while adding the
coarse SDK client/session operations shared with the generated Swift and Kotlin
bindings.

The FastAPI fixture is packaged behind the optional `secure-tunnel[server]`
extra. It is a Python imperative shell around the Rust fixture process; Rust
keeps descriptor signing, service static-key custody, Noise, auth, and
application-frame semantics. Configure the fixture with
`FixtureSettings`/`ObservabilitySettings` or these environment variables:

- `SECURE_TUNNEL_BINDING_FIXTURE_BIN`
- `SECURE_TUNNEL_FIXTURE_WORKDIR`
- `SECURE_TUNNEL_OBSERVABILITY_LEVEL`
- `SECURE_TUNNEL_OBSERVABILITY_FORMAT`
- `SECURE_TUNNEL_OBSERVABILITY_SERVICE_NAME`
- `SECURE_TUNNEL_RUST_LOG`
- `OTEL_EXPORTER_OTLP_ENDPOINT`

When `SECURE_TUNNEL_OBSERVABILITY=1`, the Rust CLI installs a tracing
subscriber that writes structured logs to stderr so stdout remains reserved for
machine-readable JSON.

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Development Workflow

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/amazing-feature`
3. Make your changes
4. Run quality checks: `mise run ci`
5. Commit your changes: `git commit -m 'Add amazing feature'`
6. Push to the branch: `git push origin feature/amazing-feature`
7. Open a Pull Request

## License

This project is licensed under the Mozilla Public License 2.0 - see the [LICENSE](LICENSE) file for details.
