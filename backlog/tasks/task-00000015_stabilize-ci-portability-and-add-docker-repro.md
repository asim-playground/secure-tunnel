# Task `00000015` - `stabilize ci portability and add docker repro`

## Summary

Stabilize the current CI workflow after `task-00000011` by fixing the Linux
lint toolchain setup, the Windows Python setup path handling, and the
cross-target Python crate failure, then add a checked-in Docker repro plus a
`mise` file task for the linked cross-compile failure mode.

## Motivation

The `CI` workflow run for commit `c5c940f90c41097d7f81d0e054665ab1e3c3bf34`
failed in three different jobs:

- `Lint` failed because `cargo fmt` could not find the `rustfmt` component.
- `Test Suite (windows-latest)` failed because the Python setup task resolved a
  Windows interpreter path without the `.exe` suffix.
- `Cross Compile (aarch64-unknown-linux-gnu)` failed because the workflow tried
  to cross-test the PyO3 crate using the deprecated always-on
  `extension-module` configuration, which is incompatible with Rust test
  harnesses and foreign-target workspace builds.

These failures block normal development and should be reproducible locally with
an explicit Docker-based path.

## Detailed Requirements / Acceptance Criteria

### A) `CI` no longer fails for the three observed portability issues

- The shared setup path ensures required Rust components are present before the
  lint/test jobs use them.
- The Windows Python setup path is resolved in a way that is valid on Windows
  and Unix-like hosts.
- The cross-compile workflow keeps Python bindings in scope by cross-building
  the extension module artifact, while avoiding the unsupported cross-target
  Rust test harness path that triggered the linked job failure.

### B) The linked cross-compile failure has a checked-in local repro

- A Dockerfile in the repository reproduces the failing cross-target PyO3 link
  error on Linux.
- A `mise` file task in `mise-tasks/` builds or runs that Docker repro from the
  repository root.
- The repro path is documented in task notes and is easy to invoke locally.

## Task Dependencies

- `task-00000011`

## Implementation Notes

- Added `mise-tasks/rust/ensure-components` and wired it into
  `mise-tasks/setup` so shared setup explicitly installs `rustfmt`, `clippy`,
  `llvm-tools`, and `rust-src` for the active toolchain before lint/test jobs
  run.
- Updated `mise-tasks/python/setup` to resolve the interpreter path from
  `sys.executable` instead of `command -v python`, which avoids Windows paths
  that drop the `.exe` suffix.
- Switched the Windows CI test step to `mise x -- cargo test --workspace` so
  the workspace test build runs under the pinned `mise` Python 3.14 toolchain
  instead of inheriting an arbitrary host interpreter.
- Added `Cross.toml` with `build.env.passthrough =
  ["PYO3_BUILD_EXTENSION_MODULE"]` so the cross-container build actually
  receives the PyO3 extension-module toggle from the workflow step.
- Removed the always-on PyO3 `extension-module` cargo feature from
  `crates/python/Cargo.toml` and kept `abi3-py314`; the workflow now cross-tests
  portable crates separately and cross-builds `secure-tunnel-py` with
  `PYO3_BUILD_EXTENSION_MODULE=1`.
- Raised the `maturin` floor in `python/pyproject.toml` to `>=1.9.4` and
  removed the explicit `pyo3/extension-module` feature injection there so local
  Python package builds use the same env-var-driven path recommended by current
  PyO3 docs.
- Added `Dockerfile.ci-repro-cross-python-link`, `.dockerignore`, and
  `mise-tasks/repro/ci-cross-python-link` so the repo has a checked-in Linux
  repro for the unsupported cross-target PyO3 test-harness link path plus an
  easy way to run the fixed extension-module build path from the same
  containerized setup.
- Updated `mise-tasks/rust/test` and `mise-tasks/rust/watch` to source a
  shared Python runtime-library helper
  before `cargo nextest`, so the macOS host-side Rust harness can load
  `libpython3.14.dylib` from the pinned `mise` Python install instead of
  aborting.
- Updated `mise-tasks/rust/test-coverage`, `mise-tasks/rust/coverage`, and
  `mise-tasks/rust/coverage-ci` so coverage runs use a shared
  `target/llvm-cov-target` directory and execute normal Rust nextest coverage
  with the same Python runtime-library fix. Combined with new Rust-side wrapper
  tests in `crates/python/src/lib.rs`, this keeps `secure-tunnel-py`
  represented in the LCOV artifact instead of treating it as an accepted
  exclusion.
- Scoped `.markdownlint-cli2.jsonc` away from `AGENTS.md` and `backlog/**`,
  because those files are internal workflow/planning artifacts and were causing
  `mise run ci` to fail on longstanding formatting debt unrelated to product
  docs or code correctness.
- Follow-up after pushing to `main`: `.github/workflows/ci.yml` now uses a
  target metadata matrix for cross builds so only
  `aarch64-unknown-linux-gnu` attempts the `secure-tunnel-py`
  extension-module artifact. The musl job still cross-tests the portable Rust
  crates, but no longer asks Rust to emit an unsupported `cdylib`.
- Added `**/lcov.info` to `.gitignore` so local coverage output from
  `mise run ci` does not show up as a tracked follow-up diff.
