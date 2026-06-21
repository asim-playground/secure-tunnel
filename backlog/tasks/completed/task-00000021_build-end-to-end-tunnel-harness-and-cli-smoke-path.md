# Task `00000021` - `build end-to-end tunnel harness and cli smoke path`

## Summary

Add an end-to-end local tunnel harness and CLI smoke path over the production
Rust library.

## Motivation

Before native SDK packaging, the repository needs one repeatable local scenario
that proves descriptor loading, transport selection, secure-ready handshake,
session establishment, application record exchange, and close work together.

## Read-Write Repository

- Primary read-write repository: `/Users/asimi/workplace/secure-tunnel`
- Secondary read-write repository/repositories (if applicable): none
- State explicitly:
  - code changes are expected to land in Rust harness, CLI, tests, and task
    automation.
  - this repository itself is expected to change.

## Read-Only Reference Repository

- Read-only reference repository/repositories: none
- State explicitly which repositories may be inspected only for reference or
  legacy behavior:
  - none.

## Detailed Requirements / Acceptance Criteria

### A) Local end-to-end harness exists

- [x] Add a local client/server or loopback harness that uses production Rust
      library paths rather than only test-only prototype transports.
- [x] Exercise descriptor/config loading, transport selection, `Secure Ready`,
      account/device session flow, application record exchange, and close.
- [x] Include both direct `QUIC` success and `WSS` fallback scenarios.

### B) CLI smoke path is useful

- [x] Add or extend CLI commands that can run the local smoke scenario.
- [x] Emit concise machine-readable or structured output for selected carrier,
      fallback reason, secure-ready status, session status, and close result.
- [x] Keep secrets or sensitive payloads out of default CLI output.

### C) Automation is wired

- [x] Add a `mise` task or documented command for the end-to-end smoke path.
- [x] Include the smoke path in an appropriate validation gate without making
      fast local development painful.
- [x] `mise run dev` passes.

## Cross-Repo Boundaries

- Primary implementation boundary: local harness, CLI, and automation in this
  repository.
- Parser / upstream dependency boundary: no parser work expected.
- Downstream integration boundary: native SDK smoke tests can reuse this later
  but are not implemented in this task.
- External asset / catalog / fixture boundary: local fixtures only.
- If another repository is read-write, state what is implemented there versus
  what is implemented in this repository.
  - none.

## Task Dependencies

- backlog/tasks/completed/task-00000019_implement-production-quic-and-wss-carrier-adapters.md
- backlog/tasks/completed/task-00000020_implement-account-and-device-session-protocol.md
- backlog/plans/plan-00000002_product-secure-tunnel-sdk-and-bindings.md

## Reference Tasks

- backlog/tasks/completed/task-00000012_prototype-quic-preferred-transport-with-wss-fallback-and-local-secure-session.md

## Backlog Bookkeeping

- `backlog/` is local planning state and is not version controlled by default.
- Moving a task to `backlog/tasks/completed/` is separate from pushing code.
- Commit history for work tracked by this task may live in a different
  read-write repository than this backlog entry.

## Implementation Notes

- [x] Implementation notes added with command evidence.

Added `secure-tunnel-harness`, a local smoke harness crate that starts loopback
`QUIC` and `WSS` services with generated TLS certificates, signs a descriptor
for the dynamic local ports, and drives the real `secure-tunnel-sdk`
production-client path. The harness exercises descriptor JSON loading,
descriptor trust roots, pinned service static key trust, production carrier
adapters, `Secure Ready`, account auth, known-device auth, application
request/response, and encrypted close.

Added an SDK-facing `ClientConfig::with_outer_root_certificates_der(...)`
builder so local harness and future managed-network tasks can pass explicit DER
outer TLS roots through the SDK into `secure-tunnel-transport`. This is a
narrow transport-config hook only; product custom-CA UX remains in
`task-00000013`.

Added `secure-tunnel-cli smoke --scenario all|quic-success|wss-fallback
--format json`. The command emits sanitized JSON with selected carrier,
fallback reason, secure-ready status, account/device status, application
exchange status, close state, and sanitized attempt summaries. It intentionally
does not print account IDs, session context hashes, credential payloads,
device signatures, plaintext app payloads, or key material.

Added `mise-tasks/smoke`, which runs the CLI smoke suite and validates the JSON
with `jq`, including a regression check that the default JSON does not include
fixed sensitive fixture values or fields. The task is included in
`mise-tasks/ci` but not `mise-tasks/dev`, so the heavy local smoke path is
available in the canonical CI pipeline without making the fast developer loop
slower.

Review fix-ups:

- Moved the example descriptor re-sign helper behind the explicit
  `secure-tunnel-core/test-support` feature, with harness opting into that
  feature. `secure-tunnel-core` still checks without default features, so this
  test-only signer is not part of the default release API.
- Added smoke JSON sanitization regression checks in both the harness suite and
  CLI integration test.
- Made `secure-tunnel-cli smoke --help` exit successfully.

Verification so far:

- `cargo fmt --all -- --check` passed.
- `cargo test -p secure-tunnel-harness -p secure-tunnel-cli -- --nocapture`
  passed: 3 harness smoke tests and 2 CLI smoke tests.
- `cargo check -p secure-tunnel-core --no-default-features` passed.
- `cargo test -p secure-tunnel-transport -- --nocapture` passed after the
  transport crate opted into `secure-tunnel-core/test-support` only as a
  dev-dependency for its local test fixtures.
- `shellcheck mise-tasks/smoke` passed.
- `mise run smoke` passed and emitted two sanitized successful scenarios:
  `quic_success` and `wss_fallback`.
- `cargo clippy --workspace --all-targets --all-features --no-deps -- -D warnings`
  passed after the harness and CLI lint fixes.
- `cargo doc -p secure-tunnel-harness --no-deps` passed.
- `mise run dev` passed, including format, lint, Rust nextest with 83 tests,
  Rust doctests, Python package/tests with 5 pytest tests, Go tests, and
  Go-WASM tests.
- `wc -l $(rg --files crates | rg '\.rs$') | sort -nr | head -25` showed all
  non-Markdown source files at or below 500 lines; the largest remain
  `crates/core/src/selector.rs` and `crates/core/src/device_session.rs` at
  exactly 500 lines.

## Implementation Plan

1. Define the smallest useful local end-to-end tunnel scenario.
2. Implement the harness and CLI entrypoint over production library paths.
3. Add automation and tests for success plus fallback.
4. Run `mise run dev` and independent review.

## Review Notes

- First independent review by Galileo found one high-severity and one
  medium-severity issue: the example descriptor signing helper was available
  in the default release API, and smoke JSON sanitization was not
  regression-tested. Both were fixed before re-review.
- Re-review by Galileo found no high/medium issues remaining. The only low
  residual is that `secure-tunnel-cli` intentionally depends on the local
  smoke harness for this task; a future production CLI could split smoke into a
  separate binary or feature.

## Acceptance Closure

- [x] All acceptance criteria are satisfied and marked.
- [x] Verification commands and outcomes are recorded.
- [x] No unresolved high/medium findings remain.
