# Task `00000016` - `update runtimes deps and add swift callable library surface`

## Summary

Refresh the repository's managed runtimes and dependencies, migrate the Rust
compiler wrapper from `sccache` to `kache` 0.6.0, and replace the scaffold
parser binding with a small Secure Tunnel C ABI that Swift can import.

## Motivation

The workspace has enough transport-neutral Secure Tunnel API now that the
template-era arithmetic parser is no longer the right language-binding anchor.
The next integration slice needs a reproducible, current toolchain and a stable
C-compatible library surface that Swift can call before full mobile transport
adapters exist.

## Read-Write Repository

- Primary read-write repository: `/Users/asimi/workplace/secure-tunnel`
- Secondary read-write repository/repositories (if applicable): none
- State explicitly:
  - code changes are expected to land in this repository's Rust workspace,
    Python/Go binding shims, mise config, and CI/task wiring.
  - this repository itself is expected to change; `~/workplace/flutter_template`
    is read-only reference material for the `kache` migration pattern.

## Read-Only Reference Repository

- Read-only reference repository/repositories:
  - `/Users/asimi/workplace/flutter_template`
- State explicitly which repositories may be inspected only for reference or
  legacy behavior:
  - `flutter_template` may be inspected for the established `kache` 0.6.0
    mise/CI pattern, but changes for this task land only in `secure-tunnel`.

## Detailed Requirements / Acceptance Criteria

### A) Runtimes and dependencies are current

- [x] Refresh managed runtimes and tools in `mise.toml`/`mise.lock`, including
      major-version bumps where compatible.
- [x] Refresh Rust crate manifests and `Cargo.lock`; update Python `uv.lock`
      and Go module locks where applicable.
- [x] Keep supply-chain safety controls intact and record any policy-blocked
      upgrades instead of bypassing them.

### B) Rust compiler cache uses `kache` only

- [x] Remove `sccache` from local mise tooling and the lockfile.
- [x] Pin `github:kunobi-ninja/kache = "0.6.0"` and set `RUSTC_WRAPPER=kache`.
- [x] Remove `SCCACHE_*` environment/settings and update CI or task references
      to use `kache` names and diagnostics where relevant.

### C) Swift-callable library surface exists

- [x] Replace the arithmetic-parser C ABI with Secure Tunnel protocol and
      descriptor entry points that Swift can call through a generated C header.
- [x] Define clear string ownership and error-reporting rules for FFI callers.
- [x] Keep the surface narrow: protocol metadata and descriptor validation are
      in scope; concrete mobile network transport adapters are not.

### D) Binding scaffolds stop advertising parser semantics

- [x] Update Go, WASM, Python, README, and generated headers/tests that still
      describe Secure Tunnel as an arithmetic expression parser.
- [x] Preserve existing binding verification where practical with the new
      protocol/descriptor entry points.

## Cross-Repo Boundaries

- Primary implementation boundary:
  - `secure-tunnel` workspace manifests, core APIs, C ABI crate, language
    binding shims, task scripts, CI, and documentation.
- Parser / upstream dependency boundary:
  - remove the bootstrap parser from public bindings; keep dependency updates
    in repo-native package managers and lockfiles.
- Downstream integration boundary:
  - expose a C-compatible ABI suitable for Swift import, but do not implement
    iOS packaging, Swift package manifests, or mobile transport adapters in this
    task.
- External asset / catalog / fixture boundary:
  - no external runtime assets are expected beyond package-manager locks.
- If another repository is read-write, state what is implemented there versus
  what is implemented in this repository.
  - none.

## Task Dependencies

- backlog/docs/2026-03-15_rust-crate-boundaries-and-secure-channel-api.md
- backlog/tasks/completed/task-00000005_define-rust-crate-boundaries-and-secure-channel-api.md
- backlog/tasks/completed/task-00000015_stabilize-ci-portability-and-add-docker-repro.md
- backlog/plans/plan-00000001_secure-channel-foundation.md

## Reference Tasks

- `/Users/asimi/workplace/flutter_template/backlog/tasks/completed/task-00000069_kache-build-cache-migration.md`

## Backlog Bookkeeping

- `backlog/` is local planning state and is not version controlled by default.
- Moving a task to `backlog/tasks/completed/` is separate from pushing code.
- Commit history for work tracked by this task may live in a different
  read-write repository than this backlog entry.

## Implementation Notes

- [x] Implementation notes added with command evidence.
- Initial scope decision: make Swift support ride on a C ABI plus generated C
  header, because Swift can import C headers directly and the existing Go FFI
  crate already uses cbindgen.
- Runtime refresh completed:
  - Rust toolchain: `rustc 1.96.0 (ac68faa20 2026-05-25)`.
  - Python: `3.14.6`.
  - Go: `1.26.4`.
  - `uv`: `0.11.23`.
  - Rust cache wrapper: `kache 0.6.0`.
- `mise.toml` and `mise.lock` now pin `github:kunobi-ninja/kache = "0.6.0"`
  and no active `sccache` or `SCCACHE_*` configuration remains.
- Added `scripts/mise-postinstall.sh` and `[hooks].postinstall` so `mise
  install` idempotently ensures both Rust WASM targets:
  `wasm32-unknown-unknown` and `wasm32-wasip1`.
- The active Rust toolchain currently has installed targets:
  `aarch64-apple-darwin`, `wasm32-unknown-unknown`, and `wasm32-wasip1`.
- Replaced parser-oriented FFI with `secure-tunnel-ffi`, a small C ABI that
  exposes protocol metadata plus service descriptor JSON validation and
  normalization.
- Tracked `crates/go/binding.h` beside `crates/go/module.modulemap` so a clean
  checkout has the Swift-importable C surface available without first running
  cbindgen.
- Strengthened descriptor validation before exposing it through C, Go,
  Go/WASM, and Python wrappers. Validation now rejects invalid v1 descriptor
  version, required empty fields, invalid carrier selectors, and malformed WSS
  authority shape before connector execution.
- Dependency checks:
  - `mise run deps-check` reported Rust dependencies up to date and
    `cargo-audit`/`cargo-deny` passed. The only output was existing
    `license-not-encountered` warnings for allowed licenses that are not
    currently present.
- Validation:
  - `mise install` exercised the postinstall hook and reported the Rust WASM
    targets already installed for `1.96.0`.
  - `bash -x scripts/mise-postinstall.sh` reported both required Rust WASM
    targets already installed for `1.96.0`.
  - `mise exec -- rustup target list --installed --toolchain
    1.96.0-aarch64-apple-darwin` reported `aarch64-apple-darwin`,
    `wasm32-unknown-unknown`, and `wasm32-wasip1`.
  - `mise exec -- kache --version` reported `kache 0.6.0`.
  - `rg -n "sccache|SCCACHE" mise.toml mise.lock .github scripts mise-tasks
    Cargo.toml rust-toolchain.toml` returned no active tool/config matches.
  - `mise run dev` passed after the descriptor validation and test split fixes.
  - `mise run ci` passed after the descriptor validation and test split fixes,
    including build, lint, tests, coverage, and `deps-check`.

## Follow-up Plan from Rust Library Binding Research

The attached research recommends treating the current C ABI as the bootstrap
surface and evaluating a deliberate UniFFI SDK layer as follow-up, not as part
of this dependency-maintenance slice.

1. Create a future SDK task for a small `secure-tunnel-sdk-ffi` facade crate
   that depends on `secure-tunnel-core` but does not expose internal Rust types.
2. Pin `uniffi` and a project-local `uniffi-bindgen` binary in the workspace
   dependency graph before generating Swift, Kotlin, or Python bindings.
3. Define the foreign SDK contract as coarse-grained operations: protocol
   metadata, descriptor validation/normalization, bootstrap config/session
   creation, and explicit error enums. Avoid exposing transport internals or
   hundreds of tiny getters across the boundary.
4. Keep the manual C ABI available until the UniFFI spike proves packaging,
   performance, and Swift/Kotlin/Python ergonomics are acceptable.
5. Add CI smoke tests for each generated package target before treating UniFFI
   as the primary path:
   - Swift import/build smoke test for the generated Swift module or future
     XCFramework/SwiftPM package.
   - Kotlin/JVM or Android smoke test covering JNA packaging.
   - Python wheel/import smoke test.
6. Benchmark or at least load-test boundary calls before moving high-frequency
   operations to UniFFI; keep hot loops inside Rust.
7. Track current UniFFI risks in the future task: pre-1.0 upgrade churn, Swift
   strict-concurrency/Xcode compatibility, Kotlin JNA/JNI tradeoffs, async
   cancellation behavior, and packaging ownership.
8. Decompose the large prototype test/support modules before expanding the SDK
   surface further, especially `prototype_transport.rs`, `noise.rs`, and
   `selector.rs`, to satisfy the repo's 500-line non-Markdown code-file rule.

## Implementation Plan

1. Inspect dependency/tool freshness and apply compatible runtime, lockfile,
   and manifest upgrades without weakening supply-chain policy.
2. Port the `flutter_template` local `kache` 0.6.0 pattern into this repo and
   remove `sccache`.
3. Replace the parser-oriented FFI/binding surface with protocol metadata and
   descriptor validation helpers.
4. Run focused checks first, then the repo-native `mise run dev` gate.
5. Complete independent review and re-review until no unresolved high/medium
   findings remain.

## Review Notes

- First independent review found medium issues in descriptor validation,
  generated-header reproducibility, and large-module decomposition. The
  validation and generated-header issues were fixed by strengthening
  `ServiceDescriptor::validate`, tracking `crates/go/binding.h`, and adding
  `crates/go/module.modulemap`.
- Second re-review found two remaining descriptor-validation holes and called
  out `descriptor.rs` as newly over the 500-line non-Markdown code-file rule.
  The validation holes were fixed with Ed25519 trust-anchor parsing and stricter
  `wss://` authority checks, with regression tests in
  `crates/core/src/descriptor_tests.rs`; `descriptor.rs` is now 408 lines.
- Third re-review found no remaining high/medium findings for this
  dependency/runtime/tooling pass and Swift-callable C ABI groundwork.
- Reviewer accepted the still-large `prototype_transport.rs`, `noise.rs`, and
  `selector.rs` files as non-blocking design debt for this scoped pass because
  the user asked to focus on dependency updates and a future library-binding
  plan. Decomposition should become blocking before expanding the Rust library,
  UniFFI, or SDK surface further.
- Residual low risks recorded by review: `wss://` authority validation remains
  a deliberately small string check rather than a full URL parser, and Swift
  coverage is import groundwork only until a future Swift compile/import smoke
  test is added.

## Acceptance Closure

- [x] All acceptance criteria are satisfied and marked.
- [x] Verification commands and outcomes are recorded.
- [x] No unresolved high/medium findings remain.
