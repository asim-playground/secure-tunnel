# Task `00000013` - `allow optional custom ca cert for intercepted wss or quic`

## Summary

Allow the client to optionally trust a configured outer-TLS CA certificate or
bundle for `WSS` and `QUIC` so deployments behind enterprise interception or
private PKI can still establish the outer carrier.

## Motivation

The v1 design keeps end-to-end trust in the inner Noise and server-key
authorization flow, but the outer carrier still depends on TLS and may need to
operate in managed environments that inject a private root or interception
certificate. The client needs an explicit configuration path for that outer-TLS
trust without weakening or confusing the inner trust model.

## Read-Write Repository

- Primary read-write repository: `/Users/asimi/workplace/secure-tunnel`
- Secondary read-write repository/repositories (if applicable): none
- State explicitly:
  - code changes are expected to land in Rust client configuration, carrier
    adapter TLS configuration, tests, and documentation in this repository.
  - this repository itself is expected to change.

## Read-Only Reference Repository

- Read-only reference repository/repositories: none
- State explicitly which repositories may be inspected only for reference or
  legacy behavior:
  - TLS or carrier adapter examples may be cloned under
    `/Users/asimi/Downloads/references` if needed, but implementation changes
    land only in `secure-tunnel`.

## Detailed Requirements / Acceptance Criteria

### A) Client trust configuration is explicit

- [x] Define the first client-facing configuration shape for an optional custom
      CA certificate or CA bundle that applies to outer `WSS` and `QUIC` TLS
      only.
- [x] Define whether the custom CA augments or replaces the platform trust
      store, and keep that behavior consistent across both carriers.
- [x] Keep the scope narrow: this task should not redesign the inner
      trust-anchor or server-key authorization model.

### B) Security and failure semantics stay separated

- [x] Preserve the rule that outer TLS trust does not replace inner Noise trust
      or service-identity validation.
- [x] Ensure the selected carrier, fallback reporting, and inner trust failures
      remain distinguishable from outer certificate or TLS failures.
- [x] Document any compatibility limits for carriers or platforms where custom
      CA injection differs.

### C) Validation covers intercepted-network behavior

- [x] Add local tests or harness coverage for at least one custom-root `WSS`
      path and one custom-root `QUIC` path before closing the task.
- [x] Verify that the custom CA path composes with the existing selector
      semantics instead of bypassing them.
- [x] Record any follow-up work needed for client packaging, certificate
      rotation, or operator UX, but keep carrier-path validation in scope for
      this task.

## Cross-Repo Boundaries

- Primary implementation boundary: outer-carrier TLS trust configuration in the
  Secure Tunnel Rust client and carrier adapters.
- Parser / upstream dependency boundary: dependency changes are allowed only
  when required for TLS configuration and must pass repo supply-chain checks.
- Downstream integration boundary: native SDKs may expose this configuration
  later through the facade, but Swift/Kotlin/Python packaging is out of scope
  for this task.
- External asset / catalog / fixture boundary: local certificate fixtures only.
- If another repository is read-write, state what is implemented there versus
  what is implemented in this repository.
  - none.

## Task Dependencies

- backlog/docs/v1-threat-model-and-transport-decisions.md
- backlog/docs/v1-service-descriptor-and-bootstrap-config.md
- backlog/docs/v1-transport-selection-and-fallback-policy.md
- backlog/tasks/completed/task-00000009_define-udp-first-deployment-and-observability-requirements.md
- backlog/tasks/completed/task-00000012_prototype-quic-preferred-transport-with-wss-fallback-and-local-secure-session.md
- backlog/tasks/task-00000019_implement-production-quic-and-wss-carrier-adapters.md
- backlog/plans/plan-00000001_secure-channel-foundation.md
- backlog/plans/plan-00000002_product-secure-tunnel-sdk-and-bindings.md

## Reference Tasks

- backlog/tasks/completed/task-00000011_prototype-server-auth-noise-handshake-and-trust-verification-on-transport-neutral-frames.md
- backlog/tasks/task-00000014_allow-optional-http-proxy-for-wss-client.md

## Backlog Bookkeeping

- `backlog/` is local planning state and is not version controlled by default.
- Moving a task to `backlog/tasks/completed/` is separate from pushing code.
- Commit history for work tracked by this task may live in a different
  read-write repository than this backlog entry.

## Implementation Notes

- [x] Implementation notes added with command evidence.
- Treat this as outer-carrier compatibility work, not as a change to the inner
  secure-channel trust model.
- Prefer one configuration surface that both `WSS` and `QUIC` adapters can
  consume, even if the underlying TLS libraries differ.
- `2026-06-21`: Plan `00000002` adds `task-00000019` as the production-adapter
  prerequisite so custom-root validation exercises real carrier code rather
  than only the prototype harness.
- `2026-06-22`: Kept the stable SDK field `outer_root_certificates_der`, but
  defined it as additional outer TLS roots added to platform trust where the
  platform verifier supports extra roots.
- `2026-06-22`: Updated production transport config to use
  `rustls_platform_verifier::Verifier::new_with_extra_roots` on supported
  platforms. Android currently returns an outer TLS failure for non-empty extra
  roots because `rustls-platform-verifier` `0.7` does not expose additive roots
  there.
- `2026-06-22`: Added transport tests for custom-root `QUIC`, custom-root
  cached `WSS`, custom-root `QUIC` ALPN rejection with `WSS` fallback, and
  wrong-root `QUIC`/`WSS` classification as outer TLS failures.
- `2026-06-22`: Added conformance rows for `custom_ca_quic_success`,
  `custom_ca_wss_success`, `custom_ca_quic_rejected_wss_fallback`, and
  `custom_ca_inner_trust_failure`; removed the
  `managed-custom-ca-product-ux` pending row.
- `2026-06-22`: Added `custom_ca_wrong_root_tls_failure` to prove the SDK
  surfaces wrong custom roots as `outer_tls_failure` with a non-empty attempt
  trace.
- `2026-06-22`: Fixed `WSS` connect error mapping so rustls certificate
  failures surfaced as `io::ErrorKind::InvalidData` map to `outer_tls_failure`
  rather than `outer_path_failure`.
- Validation evidence:
  - `cargo test -p secure-tunnel-transport --all-features custom_root`
  - `cargo test -p secure-tunnel-transport --all-features wrong_custom_root`
  - `cargo test -p secure-tunnel-transport --all-features extra_roots`
  - `cargo test -p secure-tunnel-harness --all-features conformance_suite_runs_current_scenarios`
  - `mise run conformance`
  - `mise run security:test`
  - `mise run lint-all`
  - `mise run dev`

## Implementation Plan

1. Define the client configuration shape and trust-store behavior.
2. Wire custom-root handling into production `QUIC` and `WSS` TLS
   configuration.
3. Add local custom-root fixtures and tests for both carriers.
4. Run `mise run dev` and complete independent review.

## Review Notes

- Independent review on `2026-06-22` found two medium issues:
  - public config docs overstated Android extra-root support.
  - plan/task completion state was inconsistent while review was still open.
- Fixes:
  - added Android caveats to Rust SDK, transport config, UniFFI, and Go docs.
  - updated `plan-00000001` alongside `plan-00000002`.
  - added `custom_ca_wrong_root_tls_failure` as SDK-facing conformance
    coverage.
- Re-review on `2026-06-22` found no remaining high/medium findings.

## Acceptance Closure

- [x] All acceptance criteria are satisfied and marked.
- [x] Verification commands and outcomes are recorded.
- [x] No unresolved high/medium findings remain.
