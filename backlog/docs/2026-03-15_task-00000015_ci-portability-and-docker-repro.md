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
- [x] Inspect the follow-up failing `CI` run for post-fix regressions.
- [x] Patch the host-side test/lint fallout from the follow-up run.
- [x] Make `mise run ci` pass end-to-end after the follow-up fixes.

## Evidence

- Follow-up `CI` run `23117501169` on commit `64814c3fc7a887b03b654fb29cc1e44ad45edcfa`
  still failed after the initial portability fixes, but for different reasons.
- Follow-up `Lint` log: `copyright-check` failed because newly added
  Go/Python/Rust source files were missing the repo MPL header.
- Follow-up `Test Suite (macos-latest)` log: `cargo nextest` aborted while
  listing tests for `secure-tunnel-py` because `dyld` could not load
  `@rpath/libpython3.14.dylib`.
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
- Repo update: `mise run copyright` is the intended repo task for applying the
  missing MPL headers surfaced by `copyright-check`.
- Repo update: generic Rust host test tasks now exclude `secure-tunnel-py`,
  while `mise run python:test` remains the path that builds and tests the
  Python extension module.
- Follow-up repo update: the better final fix was not to keep that exclusion.
  `mise-tasks/rust/python-runtime-env` now exports the pinned Python library
  directory for host-side Rust runs, which lets macOS `cargo nextest` execute
  the `secure-tunnel-py` harness without `dyld` failures.
- Follow-up repo update: coverage tasks now use the same macOS Python runtime
  loader fix as the regular nextest path, and `crates/python/src/lib.rs` now
  includes Rust-side wrapper tests so the Python extension crate contributes
  real LCOV hits instead of staying at zero coverage.
- Repo update: `.markdownlint-cli2.jsonc` now ignores `AGENTS.md` and
  `backlog/**`, which removes internal workflow artifacts from the canonical
  markdown lint gate.
- Local validation: `mise run ci` completed successfully after the markdown
  lint scope change and the runtime-path plus coverage-task updates.
- Main-branch `CI` run `23118583152` still failed in `Cross Compile
  (x86_64-unknown-linux-musl)` because the workflow attempted
  `cross build --release --all-features -p secure-tunnel-py --target
  x86_64-unknown-linux-musl`, and Rust rejected `secure-tunnel-py` as a
  `cdylib` for that musl target.
- Repo update: `.github/workflows/ci.yml` now models the cross matrix with
  explicit per-target metadata and only runs the PyO3 extension-module build
  step for `aarch64-unknown-linux-gnu`; the musl leg still cross-tests the
  portable Rust workspace crates but no longer attempts the unsupported Python
  extension artifact.
- Repo update: `.gitignore` now ignores `**/lcov.info` so local `mise run ci`
  coverage runs do not keep re-dirtying the working copy with generated LCOV
  artifacts.

## Conclusions

- The failing workflow has three separate root causes, not one.
- The follow-up run confirms those three portability issues were fixed, because
  the remaining failures moved to a repo hygiene issue and a macOS-specific
  host test-harness/runtime mismatch.
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
- The host-side `secure-tunnel-py` Rust harness is not the right portability
  signal by itself on macOS, but it also should not be left broken. Pointing
  the runtime loader at the pinned Python `LIBDIR` is a better fix than simply
  excluding the crate from host-side Rust workflows.
- The canonical local/CI pipeline should not lint internal backlog and agent
  workflow notes. Treating those files as release-doc-quality Markdown created
  noisy failures that obscured actual code and workflow regressions.
- The Python cross-build step needs target-aware gating in the workflow itself.
  `secure-tunnel-py` can remain covered on the GNU cross target without asking
  the musl job to produce an unsupported `cdylib`.

## Next Actions

- Push the workflow follow-up so GitHub Actions can re-run the cross matrix
  with the musl Python-binding step removed and confirm the main branch is
  fully green.
