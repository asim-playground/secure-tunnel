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

## Detailed Requirements / Acceptance Criteria

### A) Client trust configuration is explicit

- Define the first client-facing configuration shape for an optional custom CA
  certificate or CA bundle that applies to outer `WSS` and `QUIC` TLS only.
- Define whether the custom CA augments or replaces the platform trust store,
  and keep that behavior consistent across both carriers.
- Keep the scope narrow: this task should not redesign the inner trust-anchor
  or server-key authorization model.

### B) Security and failure semantics stay separated

- Preserve the rule that outer TLS trust does not replace inner Noise trust or
  service-identity validation.
- Ensure the selected carrier, fallback reporting, and inner trust failures
  remain distinguishable from outer certificate or TLS failures.
- Document any compatibility limits for carriers or platforms where custom CA
  injection differs.

### C) Validation covers intercepted-network behavior

- Add local tests or harness coverage for at least one custom-root `WSS` path
  and one custom-root `QUIC` path before closing the task.
- Verify that the custom CA path composes with the existing selector semantics
  instead of bypassing them.
- Record any follow-up work needed for client packaging, certificate rotation,
  or operator UX, but keep carrier-path validation in scope for this task.

## Task Dependencies

- backlog/docs/v1-threat-model-and-transport-decisions.md
- backlog/docs/v1-service-descriptor-and-bootstrap-config.md
- backlog/docs/v1-transport-selection-and-fallback-policy.md
- backlog/tasks/task-00000009_define-udp-first-deployment-and-observability-requirements.md
- backlog/tasks/task-00000012_prototype-quic-preferred-transport-with-wss-fallback-and-local-secure-session.md
- backlog/plans/plan-00000001_secure-channel-foundation.md

## Implementation Notes

- Treat this as outer-carrier compatibility work, not as a change to the inner
  secure-channel trust model.
- Prefer one configuration surface that both `WSS` and `QUIC` adapters can
  consume, even if the underlying TLS libraries differ.
