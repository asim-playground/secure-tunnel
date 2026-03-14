# Secure Tunnel

<p align="center">
  <a href="https://github.com/asim-playground/secure-tunnel/actions/workflows/ci.yml">
    <!-- markdownlint-disable MD013 -->
    <img src="https://img.shields.io/github/actions/workflow/status/asim-playground/secure-tunnel/ci.yml?style=flat-square" alt="CI Status">
  </a>
  <a href="https://codecov.io/gh/asim-playground/secure-tunnel">
    <img src="https://codecov.io/gh/asim-playground/secure-tunnel/branch/main/graph/badge.svg" alt="Coverage">
  </a>
  <img src="https://img.shields.io/badge/Rust-1.94.0-blue?style=flat-square" alt="Rust 1.94.0">
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
- 🐍 **Python Bindings**: Native Python extension module
- 🦫 **Go Bindings**: CGO-based Go library bindings

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

### Project Structure

```plaintext
secure-tunnel/
├── crates/
│   ├── core/       # Core library (secure-tunnel-core)
│   ├── cli/        # Command-line tool (secure-tunnel-cli)
│   ├── go/         # Rust cdylib for Go bindings
│   └── go-wasm/    # Rust crate for Go/WASI workflow
├── crates/python/  # Rust extension crate for Python bindings
├── python/         # Pure-Python packaging, tests, and pyproject metadata
├── mise-tasks/     # Script-backed mise commands
├── scripts/        # Helper scripts used by setup
└── mise.toml       # Toolchain pins and task aliases
```

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
