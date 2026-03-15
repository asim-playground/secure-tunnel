# Task `00000014` - `allow optional http proxy for wss client`

## Summary

Allow the client to optionally connect the `WSS` carrier through a configured
HTTP proxy while preserving the existing transport-selection and inner
secure-channel semantics.

## Motivation

Some managed or enterprise environments require outbound HTTPS and WebSocket
traffic to pass through an HTTP proxy even when raw UDP or direct TCP egress is
restricted. Since `WSS` is the compatibility carrier, the client needs an
explicit proxy path for that outer connection instead of assuming direct
network reachability.

## Detailed Requirements / Acceptance Criteria

### A) Proxy configuration is explicit

- Define the first client-facing configuration shape for an optional HTTP proxy
  used only by the `WSS` carrier.
- Decide the initial proxy scope for v1, such as plain HTTP `CONNECT` without
  broader proxy-auth feature work unless the task proves it is required.
- Keep `QUIC` out of proxy scope for this task unless the design later adds a
  separate compatible story.

### B) Selector and failure semantics stay coherent

- Preserve the rule that proxy usage affects only the outer `WSS` connection,
  not the inner Noise trust model or the `QUIC` selector policy.
- Ensure proxy-connect failures, outer TLS failures, and inner trust failures
  remain distinguishable in reporting and observability.
- Define how the proxy path composes with `WSS` fallback after `QUIC` failure
  rather than creating a parallel transport mode.

### C) Validation covers proxied `WSS`

- Add local tests or harness coverage for at least one successful proxied
  `WSS` path and one representative proxy failure path.
- Add local tests or harness coverage for the composed proxied-`WSS` plus
  custom-root path when a proxy or managed edge terminates outer TLS.
- Record any deferred work needed for proxy authentication, environment
  variables, or platform-specific proxy discovery.

## Task Dependencies

- backlog/docs/v1-service-descriptor-and-bootstrap-config.md
- backlog/docs/v1-transport-selection-and-fallback-policy.md
- backlog/tasks/task-00000013_allow-optional-custom-ca-cert-for-intercepted-wss-or-quic.md
- backlog/tasks/task-00000009_define-udp-first-deployment-and-observability-requirements.md
- backlog/tasks/task-00000012_prototype-quic-preferred-transport-with-wss-fallback-and-local-secure-session.md
- backlog/plans/plan-00000001_secure-channel-foundation.md

## Implementation Notes

- Keep this task scoped to the explicit client proxy path for `WSS`; system
  proxy discovery, PAC handling, and proxy authentication can remain follow-up
  work if the first implementation proves the seam.
