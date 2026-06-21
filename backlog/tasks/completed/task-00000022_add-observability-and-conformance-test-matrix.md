# Task `00000022` - `add observability and conformance test matrix`

## Summary

Add the observability events, metrics, and conformance tests needed before SDK
rollout. This task covers the SDK and harness behavior that exists today and
records managed-network and close-failure rows as pending conformance entries
until their product features land.

## Motivation

Secure Tunnel needs to distinguish outer path, TLS, proxy, fallback, inner
trust, auth, future reconnect, and close failures. Without explicit telemetry
and a test matrix, native SDKs will be hard to operate and hard to debug in
managed or degraded networks.

## Read-Write Repository

- Primary read-write repository: `/Users/asimi/workplace/secure-tunnel`
- Secondary read-write repository/repositories (if applicable): none
- State explicitly:
  - code changes are expected to land in Rust observability types, tests,
    docs, and CI/task automation.
  - this repository itself is expected to change.

## Read-Only Reference Repository

- Read-only reference repository/repositories: none
- State explicitly which repositories may be inspected only for reference or
  legacy behavior:
  - none.

## Detailed Requirements / Acceptance Criteria

### A) Observability taxonomy is implemented

- [x] Define structured events or reports for carrier attempts, fallback,
      outer TLS/proxy failure classes, inner trust failures, auth failures, and
      close. Reconnect is intentionally future-facing because no reconnect loop
      exists yet.
- [x] Ensure SDK-facing reports use stable names and redact account IDs,
      device IDs, raw hostnames not already present in operator config, tokens,
      credentials, server nonces, handshake hashes, and message payloads.
- [x] Align event names with `task-00000009` deployment/observability guidance.

### B) Conformance scenarios are covered

- [x] Add tests for local `QUIC` success, `QUIC` rejected to `WSS` fallback,
      cached `QUIC`-bad posture, fallback disabled, server-key rotation, wrong
      service-static pin, wrong descriptor trust anchor, expired descriptor,
      descriptor rollback, replay/stale challenge, and graceful close.
- [x] Record custom CA, proxied `WSS`, abrupt close, and truncated close as
      pending conformance rows until `task-00000013`, `task-00000014`, and
      close-failure fixtures exist.
- [x] Preserve the rule that inner trust failures are not fallback-eligible.
- [x] Include fixtures that can be reused by native package smoke tests.

### C) Validation is automated

- [x] Wire the conformance suite into CI or a documented heavier validation
      task.
- [x] `mise run dev` passes, and any slower suite has a clear task entrypoint.
- [x] Independent review finds no unresolved high/medium issues.

## Cross-Repo Boundaries

- Primary implementation boundary: repo-local Rust observability and
  conformance tests.
- Parser / upstream dependency boundary: no parser work expected.
- Downstream integration boundary: native SDKs may consume stable report names
  after this task.
- External asset / catalog / fixture boundary: local fixtures only.
- If another repository is read-write, state what is implemented there versus
  what is implemented in this repository.
  - none.

## Task Dependencies

- backlog/tasks/completed/task-00000009_define-udp-first-deployment-and-observability-requirements.md
- backlog/tasks/completed/task-00000021_build-end-to-end-tunnel-harness-and-cli-smoke-path.md
- backlog/plans/plan-00000002_product-secure-tunnel-sdk-and-bindings.md

## Reference Tasks

- backlog/docs/v1-transport-selection-and-fallback-policy.md

## Backlog Bookkeeping

- `backlog/` is local planning state and is not version controlled by default.
- Moving a task to `backlog/tasks/completed/` is separate from pushing code.
- Commit history for work tracked by this task may live in a different
  read-write repository than this backlog entry.

## Implementation Notes

- [x] Implementation notes added with command evidence.
- Added `secure_tunnel_sdk` observability taxonomy exports:
  `event_names`, `metric_names`, `TelemetryEvent`, `TelemetryOutcome`,
  `FailureClass`, `AuthStage`, and `CloseClassification`.
- Added `tracing` events for descriptor validation, transport attempts,
  fallback, secure-ready, inner failure, auth, and session close. SDK-facing
  telemetry snapshots and trace tests omit account IDs, device IDs, descriptor
  hostnames, credentials, handshake hashes, and payload material.
- Added local conformance reports and scenarios in `secure_tunnel_harness`,
  exposed through `secure-tunnel-cli conformance --scenario all --format json`.
- Added `mise run conformance` and wired it into `mise run ci`; the task checks
  implemented scenario count and rejects sensitive strings or fields in JSON
  output.
- Current implemented suite covers 13 scenarios and reports 4 pending rows:
  custom CA, proxied `WSS`, abrupt close, and truncated close.
- Command evidence so far:
  - `cargo test -p secure-tunnel-sdk -- --nocapture` passed.
  - `cargo test -p secure-tunnel-harness -p secure-tunnel-cli -- --nocapture`
    passed.
  - `cargo test -p secure-tunnel-transport -- --nocapture` passed.
  - `mise run conformance` passed.
  - `mise run smoke` passed.
  - `shellcheck mise-tasks/conformance mise-tasks/smoke && cargo fmt --all -- --check`
    passed.
  - `cargo clippy --workspace --all-targets --all-features --no-deps -- -D warnings`
    passed.
  - `mise run dev` passed, including format, clippy, shell/Python/Go lint,
    Rust/Python/Go tests, Rust doc tests, and current Go-WASM checks.
  - After review fix-ups, `cargo test -p secure-tunnel-harness -p secure-tunnel-cli -p secure-tunnel-sdk -- --nocapture`
    passed.
  - After review fix-ups, `cargo clippy --workspace --all-targets --all-features --no-deps -- -D warnings`
    passed.
  - After review fix-ups, `mise run conformance` passed with exact scenario
    and pending-row checks.
  - `mise run ci` passed, including build, lint, tests, smoke, conformance,
    coverage, and dependency/audit checks.

## Implementation Plan

1. [x] Convert the deployment/observability requirements into stable report
   names.
2. [x] Add instrumentation and conformance fixtures.
3. [x] Wire fast and slow validation tasks.
4. [x] Complete independent re-review.

## Review Notes

- First independent review found three medium findings:
  - `service_key_rotation_valid` reused the generic success path and did not
    prove rotated service-static key acceptance.
  - transport adapter debug events used the stable `transport.attempt` event
    name with an incompatible field schema.
  - device auth/enrollment began by logging success before proof verification
    completed.
- Fix-ups:
  - added a dedicated service-key rotation scenario with an old pinned key,
    a serial-2 descriptor, and a new pinned service key.
  - tightened Rust and shell conformance checks to require the exact scenario
    and pending-row set.
  - renamed adapter debug events to `transport.adapter_connect`.
  - moved device/enrollment success telemetry to finish paths and added
    failure telemetry for preflight stale/replayed/invalid proof failures.
  - reconciled the parent plan so `task-00000022` no longer depends on
    `task-00000013` or `task-00000014`; those rows are pending conformance
    coverage for follow-up managed-network tasks.
- Re-review found no remaining high/medium findings. Residual low-risk note:
  `crates/sdk/src/auth/methods.rs` and `crates/sdk/src/tests.rs` are close to
  the 500-line limit and should be split before the next SDK auth/test
  expansion.

## Acceptance Closure

- [x] All acceptance criteria are satisfied and marked.
- [x] Verification commands and outcomes are recorded.
- [x] No unresolved high/medium findings remain.
