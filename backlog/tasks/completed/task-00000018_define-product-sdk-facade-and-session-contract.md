# Task `00000018` - `define product sdk facade and session contract`

## Final Summary

Task `00000018` is complete. The workspace now has a new
`secure-tunnel-sdk` crate that defines the product-facing Rust SDK facade above
`secure-tunnel-core`, with owned descriptor/config/report records, explicit SDK
error taxonomy, cooperative cancellation, and opaque client/session objects.
The facade keeps selector, Noise, trust, and carrier internals behind private
ports, follows a Sans-I/O / functional-core shape for deterministic planning
and state/report mapping, and is proven with mock-backed tests. `mise run dev`
passes.

## Summary

Define the Rust-facing product SDK facade that native bindings will call.

## Motivation

UniFFI should expose a stable, coarse SDK contract rather than internal core
types. The repository needs a Rust facade for client configuration, descriptor
loading, connection, session state, cancellation, application messages, and
errors before generated Swift, Kotlin, or Python bindings are introduced.

## Read-Write Repository

- Primary read-write repository: `/Users/asimi/workplace/secure-tunnel`
- Secondary read-write repository/repositories (if applicable): none
- State explicitly:
  - code changes are expected to land in the Rust workspace, likely as a new
    facade crate or a narrow facade module that depends on `secure-tunnel-core`.
  - this repository itself is expected to change.

## Read-Only Reference Repository

- Read-only reference repository/repositories: none
- State explicitly which repositories may be inspected only for reference or
  legacy behavior:
  - none.

## Detailed Requirements / Acceptance Criteria

### A) SDK facade is explicit

- [x] Define the first Rust SDK contract for client configuration, descriptor
      input, transport policy input, connect, session, send/receive or request
      operations, and close.
- [x] Use owned records, strings, byte arrays, explicit error enums, and opaque
      stateful objects suitable for UniFFI.
- [x] Keep internal selector, Noise, trust, and carrier adapter types out of the
      public SDK contract.

### B) Async, cancellation, and errors are decided

- [x] Decide which operations are async in the Rust facade and how cancellation
      is represented for foreign callers.
- [x] Define a stable error taxonomy that preserves outer path/TLS/proxy,
      fallback, inner trust, auth, and close distinctions.
- [x] Define what observability/report records foreign callers can receive
      without exposing implementation internals.

### C) The contract is tested and documented

- [x] Add rustdoc for all public facade types and methods.
- [x] Add mock-backed tests that prove the facade can run through descriptor
      validation, connect planning, session state transitions, and close.
- [x] `mise run dev` passes.

## Cross-Repo Boundaries

- Primary implementation boundary: Rust SDK facade only.
- Parser / upstream dependency boundary: no parser or dependency migration is
  expected.
- Downstream integration boundary: do not generate UniFFI bindings yet; this
  task prepares the API they will expose.
- External asset / catalog / fixture boundary: no external assets expected.
- If another repository is read-write, state what is implemented there versus
  what is implemented in this repository.
  - none.

## Task Dependencies

- backlog/tasks/completed/task-00000007_define-transport-selection-and-fallback-policy.md
- backlog/tasks/completed/task-00000008_write-transport-agnostic-v1-protocol-plus-quic-and-wss-bindings.md
- backlog/tasks/completed/task-00000009_define-udp-first-deployment-and-observability-requirements.md
- backlog/tasks/completed/task-00000017_decompose-core-modules-before-sdk-expansion.md
- backlog/plans/plan-00000002_product-secure-tunnel-sdk-and-bindings.md

## Reference Tasks

- backlog/tasks/completed/task-00000016_update-runtimes-deps-and-add-swift-callable-library-surface.md

## Backlog Bookkeeping

- `backlog/` is local planning state and is not version controlled by default.
- Moving a task to `backlog/tasks/completed/` is separate from pushing code.
- Commit history for work tracked by this task may live in a different
  read-write repository than this backlog entry.

## Implementation Notes

- [x] Implementation notes added with command evidence.
- Added workspace package `secure-tunnel-sdk` in `crates/sdk`.
- Public SDK contract now includes:
  - `BootstrapDescriptor` for parsed, validated, normalized descriptor JSON.
  - `ClientConfig`, `TransportPolicyConfig`, and `ConnectOptions`.
  - `SecureTunnelClient` and `SecureTunnelSession` opaque stateful objects.
  - `CancellationHandle` for cooperative connect cancellation.
  - `SdkError` / `SdkErrorKind` for stable public error taxonomy.
  - `ConnectError` / `ConnectResult` so failed connects preserve attempt
    reports.
  - `ConnectReport`, `SecureChannelArtifacts`, `TransportAttemptReport`,
    `TransportCacheSnapshot`, `CloseReport`, `CandidateSource`, `Carrier`,
    `FallbackReason`, and `SessionState`.
- Internal SDK structure follows the planned Sans-I/O / functional-core shape:
  - deterministic descriptor planning lives in `planning.rs`;
  - core-to-SDK report/error mapping is pure data conversion;
  - transport I/O is hidden behind private `TransportPorts`;
  - the default production port set returns unavailable until task `00000019`
    wires real adapters.
- Public SDK types do not expose core selector, Noise, trust, transport target,
  or carrier connector types.
- Review-driven fixes:
  - re-exported `CandidateSource` because public report records use it;
  - split `SecureChannelArtifacts` out of log-safe `ConnectReport`;
  - added `ConnectError` so failed connect attempts are observable;
  - made session transport leases restore on future drop/cancellation;
  - changed public descriptor/error message getters to return owned strings.
- Added nine SDK tests covering descriptor normalization and invalid
  descriptors, deterministic connect planning without I/O, QUIC success, WSS
  fallback report mapping, inner trust failure mapping, cancellation during
  mock connect, session send/receive/request, close, closed-session errors, and
  dropped pending session sends restoring the transport.
- Code-size check:
  - `wc -l crates/sdk/src/*.rs crates/sdk/src/tests/*.rs` reported every new
    SDK Rust source file under 500 lines; the largest file is
    `crates/sdk/src/tests/mock.rs` at 308 lines.
- Focused verification:
  - `cargo fmt --all -- --check` passed.
  - `cargo test -p secure-tunnel-sdk` passed with 9 tests.
  - `cargo clippy -p secure-tunnel-sdk --all-targets --all-features --no-deps
    -- -D warnings` passed.
  - `cargo doc -p secure-tunnel-sdk --no-deps` passed and generated SDK docs.
  - `cargo clippy --workspace --all-targets --all-features --no-deps --
    -D warnings` passed.
- Plain `cargo test --workspace` passed Rust tests through `secure-tunnel-ffi`
  but failed at the Python extension test binary because the shell lacked the
  mise Python dylib runtime path. The repo-native `mise run dev` gate below
  supplies that environment and passed.
- Full verification:
  - `mise run dev` passed, including format, strict Clippy, shell lint, Python
    lint/build/tests, Rust nextest and doctests, Go tests, and Go-WASM tests.

## Implementation Plan

1. Draft the SDK facade types and state model from the active v1 docs.
2. Implement the smallest mock-backed facade path without real network I/O.
3. Add tests for success and key failure classifications.
4. Run `mise run dev` and complete independent review.

## Review Notes

- First independent review found four medium findings:
  - `CandidateSource` was used by public report records but not re-exported.
  - failed connect attempt traces were dropped when mapping selector errors.
  - dropping a pending session operation future could leave the transport
    missing from the session.
  - `ConnectReport` included channel-binding material while being described as
    log/report friendly.
- Fixes:
  - re-exported `CandidateSource`;
  - added `ConnectError` / `ConnectResult` with owned failed-attempt reports;
  - added `TransportLease` drop restoration and a pending-send regression test;
  - moved transcript/channel-binding bytes into explicit
    `SecureChannelArtifacts`.
- Re-review found no unresolved high/medium findings.
- Low residual note from re-review: only `send` has an explicit pending-future
  drop test; `receive` and `close` share the same lease guard. A future close
  drop test would be useful but is not blocking for this facade task.

## Acceptance Closure

- [x] All acceptance criteria are satisfied and marked.
- [x] Verification commands and outcomes are recorded.
- [x] No unresolved high/medium findings remain.
