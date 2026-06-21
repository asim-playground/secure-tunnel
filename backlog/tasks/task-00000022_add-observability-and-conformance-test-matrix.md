# Task `00000022` - `add observability and conformance test matrix`

## Summary

Add the observability events, metrics, and conformance tests needed before SDK
rollout.

## Motivation

Secure Tunnel needs to distinguish outer path, TLS, proxy, fallback, inner
trust, auth, reconnect, and close failures. Without explicit telemetry and a
test matrix, native SDKs will be hard to operate and hard to debug in managed
or degraded networks.

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

- [ ] Define structured events or reports for carrier attempts, fallback,
      outer TLS/proxy failures, inner trust failures, auth failures, reconnect,
      and close.
- [ ] Ensure SDK-facing reports use stable names and redact account IDs,
      device IDs, raw hostnames not already present in operator config, tokens,
      credentials, server nonces, handshake hashes, and message payloads.
- [ ] Align event names with `task-00000009` deployment/observability guidance.

### B) Conformance scenarios are covered

- [ ] Add tests for UDP blocked, `QUIC` rejected, `WSS` fallback, custom CA,
      proxied `WSS`, server-key rotation, wrong trust anchor, replay/stale
      challenge, and truncated close where applicable.
- [ ] Preserve the rule that inner trust failures are not fallback-eligible.
- [ ] Include fixtures that can be reused by native package smoke tests.

### C) Validation is automated

- [ ] Wire the conformance suite into CI or a documented heavier validation
      task.
- [ ] `mise run dev` passes, and any slower suite has a clear task entrypoint.
- [ ] Independent review finds no unresolved high/medium issues.

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
- backlog/tasks/task-00000013_allow-optional-custom-ca-cert-for-intercepted-wss-or-quic.md
- backlog/tasks/task-00000014_allow-optional-http-proxy-for-wss-client.md
- backlog/tasks/task-00000021_build-end-to-end-tunnel-harness-and-cli-smoke-path.md
- backlog/plans/plan-00000002_product-secure-tunnel-sdk-and-bindings.md

## Reference Tasks

- backlog/docs/v1-transport-selection-and-fallback-policy.md

## Backlog Bookkeeping

- `backlog/` is local planning state and is not version controlled by default.
- Moving a task to `backlog/tasks/completed/` is separate from pushing code.
- Commit history for work tracked by this task may live in a different
  read-write repository than this backlog entry.

## Implementation Notes

- [ ] Implementation notes added with command evidence.
- (fill in after completion)

## Implementation Plan

1. Convert the deployment/observability requirements into stable report names.
2. Add instrumentation and conformance fixtures.
3. Wire fast and slow validation tasks.
4. Run validation and complete independent review.

## Review Notes

## Acceptance Closure

- [ ] All acceptance criteria are satisfied and marked.
- [ ] Verification commands and outcomes are recorded.
- [ ] No unresolved high/medium findings remain.
