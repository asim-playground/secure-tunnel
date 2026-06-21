# Task `00000009` - `define udp-first deployment and observability requirements`

## Final Summary

Task `00000009` is complete. The repo now has an active normative UDP-first
deployment and observability doc covering server front-door shape, `QUIC`
address-validation and Retry posture, carrier hostname/certificate routing,
privacy-safe telemetry, minimum dashboards, staging validation scenarios, and
rollout blockers for the `QUIC`-preferred plus `WSS` fallback model.

## Summary

Define the deployment, telemetry, and validation requirements introduced by
treating raw `QUIC` as the preferred outer transport and `WSS` as fallback.

## Motivation

Once `QUIC` is the preferred path, the project inherits UDP reachability,
address-validation, migration, and fallback-observability concerns that were
not central in the earlier WSS-first backlog. These assumptions need to be
written down before transport implementation so the first prototype emits the
right signals and is tested against realistic network conditions.

## Read-Write Repository

- Primary read-write repository: `/Users/asimi/workplace/secure-tunnel`
- Secondary read-write repository/repositories (if applicable): none
- State explicitly:
  - documentation and backlog changes land in this repository.
  - no external repository is modified for this task.

## Read-Only Reference Repository

- Read-only reference repository/repositories: none
- State explicitly which repositories may be inspected only for reference or
  legacy behavior:
  - historical research notes are reference material for why UDP-first
    operations matter.

## Detailed Requirements / Acceptance Criteria

### A) Deployment model is explicit

- [x] Describe the expected server front-door shape for `QUIC` and `WSS`,
      including listener expectations and shared service identity assumptions.
- [x] Document the initial `QUIC` address-validation / Retry posture and where
      that policy may tighten under attack or hostile edges.
- [x] Call out any certificate, hostname, or edge-routing assumptions that both
      carriers must share.

### B) Observability requirements are explicit

- [x] Define the minimum metrics and events that should distinguish `QUIC`
      success, `WSS` fallback, trust failures, and reconnect behavior.
- [x] Define how fallback reasons, migration events, and close reasons should
      be recorded.
- [x] Define the minimum dashboards or counters needed before wider rollout.

### C) Test matrix and operational risks are explicit

- [x] Define the minimum network and failure cases to exercise locally or in
      staging, including UDP blocked, migration/handoff, server-key rotation,
      and truncated close cases.
- [x] Define the operational risks that should block rollout if unmeasured.
- [x] Keep the deployment guidance aligned with the v1 constraints from tasks
      `00000007` and `00000008`.

## Cross-Repo Boundaries

- Primary implementation boundary:
  - active deployment/observability documentation in `secure-tunnel`.
- Parser / upstream dependency boundary: none.
- Downstream integration boundary:
  - future Rust observability, managed-network, carrier-adapter, SDK, and
    release tasks consume these requirements.
- External asset / catalog / fixture boundary: none.
- If another repository is read-write, state what is implemented there versus
  what is implemented in this repository:
  - none.

## Task Dependencies

- backlog/docs/historical/2026-03-14_initial-research.md
- backlog/tasks/completed/task-00000007_define-transport-selection-and-fallback-policy.md
- backlog/tasks/completed/task-00000008_write-transport-agnostic-v1-protocol-plus-quic-and-wss-bindings.md
- backlog/plans/plan-00000001_secure-channel-foundation.md

## Reference Tasks

- backlog/tasks/task-00000013_allow-optional-custom-ca-cert-for-intercepted-wss-or-quic.md
- backlog/tasks/task-00000014_allow-optional-http-proxy-for-wss-client.md
- backlog/tasks/completed/task-00000022_add-observability-and-conformance-test-matrix.md

## Backlog Bookkeeping

- `backlog/` is local planning state and is not version controlled by default.
- Moving a task to `backlog/tasks/completed/` is separate from pushing code.
- Commit history for work tracked by this task may live in a different
  read-write repository than this backlog entry.

## Implementation Notes

- [x] Implementation notes added with command evidence.
- Normative deliverable:
  `backlog/docs/v1-udp-first-deployment-and-observability.md`.
- The doc defines:
  - one logical service fronted by `QUIC` UDP and optional `WSS` HTTPS
  - shared inner service identity and descriptor-controlled carrier routing
  - `QUIC` address validation, Retry, and anti-amplification as outer carrier
    deployment concerns
  - certificate, hostname, and edge routing assumptions for both carriers
  - privacy-safe metric/event names and dimensions
  - minimum dashboards for `Secure Ready`, fallback, cache, trust, descriptor,
    close, and Retry/address-validation signals
  - validation scenarios for UDP blocked, ALPN/version rejection, cached
    fallback, descriptor rollback/freshness, server-key rotation,
    migration/handoff, and truncated close
  - rollout blockers when failures are unmeasured or misclassified
- Verification commands:
  - `mise run markdown-lint` passed.
  - `mise run dev` passed, including format, lint, Rust tests, doctests,
    Python tests, Go tests, and Go/WASM tests.

## Implementation Plan

1. Add an active normative UDP-first deployment and observability doc.
2. Cross-link it from the docs index and parent plans.
3. Refresh this task into the current template, record evidence, and move it
   to completed.
4. Run repository verification and independent review with tasks `00000007`
   and `00000008`.

## Review Notes

- First independent review for the combined tasks `00000007`, `00000008`, and
  `00000009` closure slice found two medium consistency findings: pending
  review closure text in completed tasks and stale deployment/observability gap
  analysis in `plan-00000001`.
- Fixups updated the task review closure text, the `plan-00000001` gap
  analysis, the plan `00000002` managed-network dependency table, and the
  task `00000022` redaction requirement.
- Focused re-review found no remaining high/medium findings in the task
  `00000007`/`00000008`/`00000009` closure scope.

## Acceptance Closure

- [x] All acceptance criteria are satisfied and marked.
- [x] Verification commands and outcomes are recorded.
- [x] No unresolved high/medium findings remain.
