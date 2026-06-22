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

- [x] Define the first client-facing configuration shape for an optional HTTP
      proxy used only by the `WSS` carrier.
- [x] Decide the initial proxy scope for v1, such as plain HTTP `CONNECT`
      without broader proxy-auth feature work unless the task proves it is
      required.
- [x] Keep `QUIC` out of proxy scope for this task unless the design later adds
      a separate compatible story.

### B) Selector and failure semantics stay coherent

- [x] Preserve the rule that proxy usage affects only the outer `WSS`
      connection, not the inner Noise trust model or the `QUIC` selector
      policy.
- [x] Ensure proxy-connect failures, outer TLS failures, and inner trust
      failures remain distinguishable in reporting and observability.
- [x] Define how the proxy path composes with `WSS` fallback after `QUIC`
      failure rather than creating a parallel transport mode.

### C) Validation covers proxied `WSS`

- [x] Add local tests or harness coverage for at least one successful proxied
      `WSS` path and one representative proxy failure path.
- [x] Add local tests or harness coverage for the composed proxied-`WSS` plus
      custom-root path when a proxy or managed edge terminates outer TLS.
- [x] Record any deferred work needed for proxy authentication, environment
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
- backlog/tasks/completed/task-00000013_allow-optional-custom-ca-cert-for-intercepted-wss-or-quic.md
- backlog/tasks/completed/task-00000009_define-udp-first-deployment-and-observability-requirements.md
- backlog/tasks/completed/task-00000012_prototype-quic-preferred-transport-with-wss-fallback-and-local-secure-session.md
- backlog/tasks/completed/task-00000019_implement-production-quic-and-wss-carrier-adapters.md
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

- [x] Implementation notes added with command evidence.
- Scoped v1 proxy support to explicit plain HTTP `CONNECT` for the outer `WSS`
  carrier only.
- Added `HttpProxyConfig { url: String }` and `wss_http_proxy` to the Rust SDK
  config, transport config, UniFFI DTOs, and Go DTO/JSON bridge.
- Rejected unsupported proxy URLs in the adapter path: non-HTTP schemes,
  missing explicit port, credentials, path, query, fragment, PAC/discovery,
  HTTPS proxy, SOCKS, and authentication are out of scope.
- Implemented proxy TCP/DNS/write/read/non-200/malformed response mapping to
  `outer_proxy_failure`; final TLS failures after CONNECT still map to
  `outer_tls_failure`, and WebSocket/subprotocol failures still map to
  `outer_protocol_failure`.
- Added local HTTP proxy fixtures for transport and conformance without
  growing existing code files beyond the 500-line limit.
- Added focused transport coverage for proxied WSS success, proxy rejection,
  CONNECT plus bad WSS root, QUIC ALPN rejection fallback to proxied WSS with
  attempt trace preservation, and proxy URL validation.
- Moved conformance row `proxied-wss` out of pending and into implemented
  scenarios.
- Deferred work: proxy authentication, credentials, environment variable
  discovery, PAC, platform proxy discovery, HTTPS proxy, SOCKS, and any QUIC
  proxying remain future explicit tasks.
- Focused validation:
  - `cargo test -p secure-tunnel-transport --all-features proxy -- --nocapture`
    passed.
  - `cargo test -p secure-tunnel-harness --all-features conformance_suite_runs_current_scenarios -- --nocapture`
    passed.
  - `cargo test -p secure-tunnel-sdk-ffi --all-features facade_helpers_expose_stable_sdk_defaults -- --nocapture`
    passed.
  - `mise run go:test` passed.
- Direct `go test ./...` from the repository root failed because `crates/go`
  is its own Go module and the native `secure_tunnel_ffi` library is generated
  by the repo `mise run go:test` task.
- Full required gates:
  - `mise run security:test` passed.
  - `mise run conformance` passed; implemented scenarios now include
    `proxied_wss`, and pending rows are only `abrupt-close` and
    `truncated-close`.
  - `mise run lint-all` passed after replacing proxy-fixture `expect` calls
    with poison-tolerant mutex handling and narrowing private-module
    visibility.
  - First `mise run dev` failed in Python package smoke because the UniFFI
    `ClientConfig` constructor now required `wss_http_proxy`; fixed by passing
    `None`.
  - Final `mise run dev` passed: 106 Rust tests, Rust doc tests, 11 Python
    tests plus Python package/FastAPI smokes, and Go tests.
- Review fix validation:
  - `mise run sdk:smoke` reached generated Python and Python FastAPI smokes,
    then stopped because `swiftc` is not installed on this host.
  - `mise run sdk:smoke-kotlin` passed after adding `wssHttpProxy = null`.
  - `mise run security:test`, `mise run conformance`, `mise run lint-all`,
    and final `mise run dev` passed after the review fixes.
  - Post-split code-file line counts are below 500; for example
    `crates/go/binding_test.go` is 432 lines and
    `crates/go/config_test.go` is 95 lines.
- Keep this task scoped to the explicit client proxy path for `WSS`; system
  proxy discovery, PAC handling, and proxy authentication can remain follow-up
  work if the first implementation proves the seam.
- `2026-06-21`: Plan `00000002` adds `task-00000019` as the production-adapter
  prerequisite so proxied-`WSS` validation exercises real carrier code rather
  than only the prototype harness.

## Implementation Plan

1. [x] Define the client proxy configuration shape and v1 proxy scope.
2. [x] Wire HTTP `CONNECT` proxy support into the production `WSS` adapter.
3. [x] Add successful proxy, proxy-failure, and custom-root composition tests.
4. [x] Run `mise run dev` and complete independent review.

## Review Notes

- Initial independent review found two medium findings:
  - generated UniFFI smoke clients did not pass the new `wss_http_proxy` field;
    fixed Python, Kotlin, Swift direct smoke, and Swift package test callers.
  - `crates/go/binding_test.go` exceeded the 500-line code-file rule; split
    config/default/proxy JSON tests into `crates/go/config_test.go`.
- Review also noted a non-blocking public DTO test gap for
  `outer_proxy_failure`; low-level transport coverage remains in this task,
  and broader public binding failure-smoke coverage can be added separately if
  needed.
- Re-review found one remaining medium: SwiftPM smoke source also needed
  `wssHttpProxy: nil`; fixed `bindings/swift/Smoke/SecureTunnelSmoke/main.swift`.
- Final re-review found no remaining high/medium findings.

## Acceptance Closure

- [x] All acceptance criteria are satisfied and marked.
- [x] Verification commands and outcomes are recorded.
- [x] No unresolved high/medium findings remain.
