# Task `00000015` - `stabilize ci portability and add docker repro`

## Working Summary

- Investigate the failing `CI` run for commit
  `c5c940f90c41097d7f81d0e054665ab1e3c3bf34`.
- Add a checked-in Docker repro for the linked cross-compile failure.
- Fix the repo-side portability issues surfaced by Linux, Windows, and cross
  jobs.

## Checklist

- [x] Inspect the failing GitHub Actions run and download logs.
- [x] Confirm the Linux lint failure mode.
- [x] Confirm the Windows Python setup failure mode.
- [x] Confirm the linked cross-target PyO3 failure mode.
- [x] Patch workflow and task scripts.
- [x] Add Docker repro and `mise` task.
- [x] Validate locally.
- [x] Run independent review.

## Evidence

- `Lint` log: `cargo fmt` failed because `cargo-fmt` was not installed for
  toolchain `1.94.0-x86_64-unknown-linux-gnu`.
- `Test Suite (windows-latest)` log: `uv venv` failed with
  `No interpreter found at path C:/Users/runneradmin/AppData/Local/mise/installs/python/3.14.3/python`.
- `Cross Compile (aarch64-unknown-linux-gnu)` log: `secure-tunnel-py` failed at
  link time with many unresolved Python symbols such as
  `PyErr_GetRaisedException` and `PyUnicode_FromStringAndSize`.
- PyO3 docs: the `extension-module` cargo feature is deprecated and breaks
  `cargo test` / workspace builds because it disables `libpython` linkage for
  all targets; `maturin >= 1.9.4` should rely on
  `PYO3_BUILD_EXTENSION_MODULE` instead.
- PyO3 docs: `abi3` extension modules can be cross-compiled, but the
  extension-module artifact build and the Rust test harness build are different
  paths with different linker expectations.
- Local container probe: a fresh `mise install rust` path starts downloading the
  requested Rust components, which suggests the repo config is valid and the CI
  lint failure is better handled by explicit component verification in setup.
- Local validation: `mise run rust:ensure-components` completed successfully and
  reported `rustfmt`, `clippy`, `llvm-tools`, and `rust-src` as up to date.
- Local validation: `mise run python:setup` selected the pinned
  `/Users/asimi/.local/share/mise/installs/python/3.14.3/bin/python`, and
  `mise run python:build` completed successfully with maturin after the PyO3
  feature change.
- Local validation: bare `cargo test --workspace --no-run` still used the host
  Python 3.12 interpreter and failed the `abi3-py314` check, while
  `mise x -- cargo test --workspace --no-run` succeeded. This confirmed the
  Windows CI step should run through `mise x`.
- Local validation: direct `cross test ... --target aarch64-unknown-linux-gnu`
  from the macOS host attempted non-host toolchain provisioning before reaching
  the Linux target path, so Docker is the more faithful local repro route for
  the linked CI failure.
- Local validation: `mise run repro:ci-cross-python-link` now reproduces the
  unsupported cross-target test-harness link failure in Linux by failing to
  link `secure-tunnel-py` against `-lpython3.14`.
- Local validation: `SECURE_TUNNEL_REPRO_ACTION=build
  SECURE_TUNNEL_REPRO_BUILD_EXTENSION_MODULE=1 mise run
  repro:ci-cross-python-link` completed successfully, which validates the
  corrected Linux extension-module artifact build semantics in the same Docker
  environment.
- Repo update: `Cross.toml` now passes `PYO3_BUILD_EXTENSION_MODULE` through to
  cross containers, so the GitHub Actions `cross build` step uses the intended
  PyO3 extension-module mode instead of silently falling back to the default
  libpython-linking behavior.
- Repo update: `python/pyproject.toml` now requires `maturin>=1.9.4` for both
  build-system and dev installs, and no longer injects `pyo3/extension-module`
  from `tool.maturin`, so local Python package builds match the current
  env-var-based PyO3 guidance.
- Reviewer follow-up: the first review identified missing `cross` env
  passthrough and an outdated `maturin` version floor; both were fixed and the
  second review reported no remaining correctness findings.

## Conclusions

- The failing workflow has three separate root causes, not one.
- The linked cross job is the best candidate for a Docker repro because it is a
  Linux-targeted failure mode with deterministic linker output.
- The Windows issue is a path-resolution bug in the repo task script and should
  be fixed in the task, not only in CI.
- The correct Python-binding fix is not to drop PyO3 from CI. It is to stop
  forcing deprecated `extension-module` behavior on all build targets and to
  split cross-target Python extension builds from cross-target Rust test
  harnesses.
- The checked-in Docker repro should provision only the Rust/Python tools needed
  for the aarch64 PyO3 experiment. Loading the full repo `mise.toml` inside the
  container pulled in unrelated tools and obscured the failure with rate-limit
  noise, so the final Dockerfile uses the documented `mise` container install
  pattern with explicit `python@3.14.3` and `rust@1.94.0` activation instead.

## Next Actions

- Push the branch or open a PR so GitHub Actions can exercise the exact
  `cross build` path with the new `Cross.toml` passthrough in place.
