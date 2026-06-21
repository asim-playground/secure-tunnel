# Task `00000023` - `create uniffi sdk facade and bindgen tooling`

## Summary

Add the project-pinned UniFFI facade crate and binding-generation tooling for
Swift, Kotlin, and Python.

## Motivation

The repository needs one generated SDK surface for Swift, Kotlin, and Python,
but that surface must remain smaller and more stable than the internal Rust
implementation. UniFFI is the default path from the binding research for those
three languages, with the manual C ABI retained for Swift bootstrap and Go.
Flutter/Dart and Go are handled by later target-specific tasks over the same
Rust SDK facade.

## Read-Write Repository

- Primary read-write repository: `/Users/asimi/workplace/secure-tunnel`
- Secondary read-write repository/repositories (if applicable): none
- State explicitly:
  - code changes are expected to land in the Rust workspace, generated binding
    directories, task automation, and CI as needed.
  - this repository itself is expected to change.

## Read-Only Reference Repository

- Read-only reference repository/repositories:
  - UniFFI reference repositories may be cloned under
    `/Users/asimi/Downloads/references` if needed.
  - `/Users/asimi/Downloads/references/uniffi-rs`
  - `/Users/asimi/Downloads/references/application-services`
- State explicitly which repositories may be inspected only for reference or
  legacy behavior:
  - cloned UniFFI examples are read-only references; implementation changes
    land only in `secure-tunnel`.

## Detailed Requirements / Acceptance Criteria

### A) UniFFI tooling is pinned

- [x] Add `uniffi` to the workspace dependency graph with an explicit pinned
      version selected during the task.
- [x] Add a project-local `uniffi-bindgen` binary so binding generation does
      not depend on globally installed generator versions.
- [x] Add deterministic `mise` tasks for generating and checking bindings.

### B) Facade crate is deliberately small

- [x] Add a `secure-tunnel-sdk-ffi` crate or equivalent that depends on the
      Rust SDK facade from `task-00000018`.
- [x] Prefer a UDL contract unless implementation evidence shows proc macros
      are materially simpler for this repo.
- [x] Expose only approved SDK records, enums, opaque session/client objects,
      and coarse operations.

### C) Generated bindings compile or import

- [x] Generate Swift, Kotlin, and Python binding source into stable repo paths.
- [x] Add smoke tests that import or build each generated binding at the source
      level before native packaging begins.
- [x] Keep the manual C ABI crate building and documented as compatibility
      during the transition.
- [x] State explicitly that Flutter/Dart and Go are out of UniFFI scope and are
      covered by `task-00000028` and `task-00000029`.

## Cross-Repo Boundaries

- Primary implementation boundary: UniFFI facade and generated binding tasks.
- Parser / upstream dependency boundary: no parser work expected.
- Downstream integration boundary: native packaging is handled in later tasks.
- External asset / catalog / fixture boundary: generated bindings are written
  under `target/generated-bindings/uniffi` and are not tracked because generated
  source exceeds the repo's non-Markdown code-file review limit.
- If another repository is read-write, state what is implemented there versus
  what is implemented in this repository.
  - none.

## Task Dependencies

- backlog/tasks/completed/task-00000018_define-product-sdk-facade-and-session-contract.md
- backlog/tasks/completed/task-00000021_build-end-to-end-tunnel-harness-and-cli-smoke-path.md
- backlog/tasks/completed/task-00000022_add-observability-and-conformance-test-matrix.md
- backlog/plans/plan-00000002_product-secure-tunnel-sdk-and-bindings.md

## Reference Tasks

- backlog/docs/2026-06-21_sdk-reference-repositories.md
- backlog/tasks/completed/task-00000016_update-runtimes-deps-and-add-swift-callable-library-surface.md

## Backlog Bookkeeping

- `backlog/` is local planning state and is not version controlled by default.
- Moving a task to `backlog/tasks/completed/` is separate from pushing code.
- Commit history for work tracked by this task may live in a different
  read-write repository than this backlog entry.

## Implementation Notes

- [x] Implementation notes added with command evidence.
- Added workspace-pinned `uniffi = "=0.31.2"` and a project-local
  `secure-tunnel-uniffi-bindgen` binary using `uniffi::uniffi_bindgen_main()`.
- Added `secure-tunnel-sdk-ffi` as a UDL-first facade crate exposing owned
  records/enums, `SecureTunnelClient`, `SecureTunnelConnection`, explicit
  `SecureTunnelError`, descriptor helpers, connect, account/device auth,
  request, report, state, and close operations.
- Added `secure-tunnel-cli binding-fixture --format json`, backed by the Rust
  harness, so generated Swift/Kotlin/Python clients can discover a local Rust
  `QUIC`/`WSS` server, root certificates, pinned service static public key, and
  shared smoke payloads.
- Added `mise run sdk:generate-bindings`, `mise run sdk:check-bindings`,
  `mise run sdk:smoke-python`, `mise run sdk:smoke-swift`,
  `mise run sdk:smoke-kotlin`, and `mise run sdk:smoke`.
- Generated bindings are intentionally untracked under
  `target/generated-bindings/uniffi`; `bindings/uniffi/README.md` documents the
  policy and follow-up packaging tasks.
- Exposed descriptor trust anchors through the generated `ClientConfig` so
  Swift/Kotlin/Python callers can validate fixture and product descriptors
  without relying on Rust-side example defaults.
- Preserved failed-connect attempt diagnostics across the UniFFI boundary via
  `SecureTunnelError.attempts()`.
- Exposed `SecureChannelArtifacts` through `SecureTunnelConnection` and made
  each generated-client smoke assert that the selected service static public key
  is one of the configured pins.
- Constrained `SECURE_TUNNEL_UNIFFI_OUT_DIR` so binding generation refuses to
  remove any directory outside the repo `target/` tree.
- Added generated-client end-to-end smokes:
  - Python imports the generated module and dylib, connects, authenticates,
    sends `smoke-ping`, receives `smoke-pong`, and closes.
  - Swift compiles generated Swift source plus the smoke client with `swiftc`,
    links the local dylib, connects, authenticates, sends the encrypted request,
    and closes.
  - Kotlin builds generated source with Gradle/JNA, points JNA at the local
    dylib, connects, authenticates, sends the encrypted request, and closes.
- Added `task-00000030` for the Python FastAPI server and
  Rust-client-to-Python-server e2e path. This remains separate from task 23
  because it is server/runtime/package work, not binding generation.
- Command evidence collected so far:
  - `cargo check -p secure-tunnel-cli -p secure-tunnel-harness
    -p secure-tunnel-sdk-ffi -p secure-tunnel-uniffi-bindgen`
  - `mise run sdk:check-bindings`
  - `mise run sdk:smoke-python`
  - `mise run sdk:smoke-swift`
  - `mise run sdk:smoke-kotlin`
- Final command evidence:
  - `cargo check -p secure-tunnel-sdk-ffi -p secure-tunnel-harness
    -p secure-tunnel-cli`
  - `cargo test -p secure-tunnel-sdk-ffi`
  - `mise run rust:check-clippy`
  - `mise run sdk:check-bindings`
  - `mise run sdk:smoke`
  - `SECURE_TUNNEL_UNIFFI_OUT_DIR="$PWD" mise run sdk:generate-bindings`
    failed safely with `refusing to remove non-target UniFFI output directory`.
  - `mise run dev`
  - `mise run ci`

## Implementation Plan

1. [x] Verify current UniFFI release, docs, and target-language caveats against
   primary sources before selecting the pinned version.
2. [x] Add project-local bindgen and the facade crate/UDL.
3. [x] Generate Swift, Kotlin, and Python source and add import/build checks.
4. [x] Document how Flutter/Dart and Go reuse the Rust SDK facade outside
   UniFFI.
5. [x] Run `mise run dev`, binding checks, and independent review.

## Review Notes

- Initial independent review found four medium findings:
  descriptor-trust-anchor configuration was not exposed to generated clients,
  failed-connect attempt diagnostics were dropped at the FFI boundary, secure
  channel artifacts were not exposed after connect, and
  `SECURE_TUNNEL_UNIFFI_OUT_DIR` could make the generation task delete an
  arbitrary directory.
- All four findings were fixed before the final validation pass.
- Re-review found no unresolved high- or medium-severity findings. Residual
  low-risk follow-ups are to eventually add generated-client runtime smokes to
  platform-aware CI, add a non-example descriptor-trust-anchor smoke, and add an
  actual failed-connect FFI regression test.

## Acceptance Closure

- [x] All acceptance criteria are satisfied and marked.
- [x] Verification commands and outcomes are recorded.
- [x] No unresolved high/medium findings remain.
