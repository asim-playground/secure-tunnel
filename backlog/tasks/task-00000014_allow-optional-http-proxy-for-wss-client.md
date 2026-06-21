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

## Read-Write Repository

- Primary read-write repository: `/Users/asimi/workplace/secure-tunnel`
- Secondary read-write repository/repositories (if applicable): none
- State explicitly:
  - code changes are expected to land in Rust client configuration, the `WSS`
    carrier adapter, proxy tests, and documentation in this repository.
  - this repository itself is expected to change.

## Read-Only Reference Repository

- Read-only reference repository/repositories: none
- State explicitly which repositories may be inspected only for reference or
  legacy behavior:
  - proxy or WebSocket examples may be cloned under
    `/Users/asimi/Downloads/references` if needed, but implementation changes
    land only in `secure-tunnel`.

## Detailed Requirements / Acceptance Criteria

### A) Proxy configuration is explicit

- [ ] Define the first client-facing configuration shape for an optional HTTP
      proxy used only by the `WSS` carrier.
- [ ] Decide the initial proxy scope for v1, such as plain HTTP `CONNECT`
      without broader proxy-auth feature work unless the task proves it is
      required.
- [ ] Keep `QUIC` out of proxy scope for this task unless the design later adds
      a separate compatible story.

### B) Selector and failure semantics stay coherent

- [ ] Preserve the rule that proxy usage affects only the outer `WSS`
      connection, not the inner Noise trust model or the `QUIC` selector
      policy.
- [ ] Ensure proxy-connect failures, outer TLS failures, and inner trust
      failures remain distinguishable in reporting and observability.
- [ ] Define how the proxy path composes with `WSS` fallback after `QUIC`
      failure rather than creating a parallel transport mode.

### C) Validation covers proxied `WSS`

- [ ] Add local tests or harness coverage for at least one successful proxied
      `WSS` path and one representative proxy failure path.
- [ ] Add local tests or harness coverage for the composed proxied-`WSS` plus
      custom-root path when a proxy or managed edge terminates outer TLS.
- [ ] Record any deferred work needed for proxy authentication, environment
      variables, or platform-specific proxy discovery.

## Cross-Repo Boundaries

- Primary implementation boundary: explicit `WSS` proxy configuration and
  adapter behavior in the Secure Tunnel Rust client.
- Parser / upstream dependency boundary: dependency changes are allowed only
  when required for proxy support and must pass repo supply-chain checks.
- Downstream integration boundary: native SDKs may expose this configuration
  later through the facade, but Swift/Kotlin/Python packaging is out of scope
  for this task.
- External asset / catalog / fixture boundary: local proxy fixtures only.
- If another repository is read-write, state what is implemented there versus
  what is implemented in this repository.
  - none.

## Task Dependencies

- backlog/docs/v1-service-descriptor-and-bootstrap-config.md
- backlog/docs/v1-transport-selection-and-fallback-policy.md
- backlog/tasks/task-00000013_allow-optional-custom-ca-cert-for-intercepted-wss-or-quic.md
- backlog/tasks/completed/task-00000009_define-udp-first-deployment-and-observability-requirements.md
- backlog/tasks/completed/task-00000012_prototype-quic-preferred-transport-with-wss-fallback-and-local-secure-session.md
- backlog/tasks/task-00000019_implement-production-quic-and-wss-carrier-adapters.md
- backlog/plans/plan-00000001_secure-channel-foundation.md
- backlog/plans/plan-00000002_product-secure-tunnel-sdk-and-bindings.md

## Reference Tasks

- backlog/tasks/completed/task-00000011_prototype-server-auth-noise-handshake-and-trust-verification-on-transport-neutral-frames.md

## Backlog Bookkeeping

- `backlog/` is local planning state and is not version controlled by default.
- Moving a task to `backlog/tasks/completed/` is separate from pushing code.
- Commit history for work tracked by this task may live in a different
  read-write repository than this backlog entry.

## Implementation Notes

- [ ] Implementation notes added with command evidence.
- Keep this task scoped to the explicit client proxy path for `WSS`; system
  proxy discovery, PAC handling, and proxy authentication can remain follow-up
  work if the first implementation proves the seam.
- `2026-06-21`: Plan `00000002` adds `task-00000019` as the production-adapter
  prerequisite so proxied-`WSS` validation exercises real carrier code rather
  than only the prototype harness.

## Implementation Plan

1. Define the client proxy configuration shape and v1 proxy scope.
2. Wire HTTP `CONNECT` proxy support into the production `WSS` adapter.
3. Add successful proxy, proxy-failure, and custom-root composition tests.
4. Run `mise run dev` and complete independent review.

## Review Notes

## Acceptance Closure

- [ ] All acceptance criteria are satisfied and marked.
- [ ] Verification commands and outcomes are recorded.
- [ ] No unresolved high/medium findings remain.
