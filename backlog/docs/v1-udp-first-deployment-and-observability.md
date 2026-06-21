---
status: active
normative: true
supersedes: []
superseded_by: []
---

# V1 UDP-First Deployment And Observability Requirements

## Summary

V1 deployments expose one logical Secure Tunnel service through two outer
carrier paths:

- raw `QUIC` over UDP as the preferred carrier
- `WSS` over HTTPS as the compatibility fallback

Both carriers terminate into the same inner service identity, trust-anchor
model, Noise prologue, and post-handshake session policy. Deployment topology
and telemetry may vary by carrier; inner security semantics must not.

## Relationship To Other V1 Docs

This document depends on:

- `backlog/docs/v1-service-descriptor-and-bootstrap-config.md`
- `backlog/docs/v1-transport-selection-and-fallback-policy.md`
- `backlog/docs/v1-core-protocol-quic-and-wss-bindings.md`
- `backlog/docs/v1-device-enrollment-and-known-device-policy.md`

The transport-selection policy defines when a client may attempt or fall back
between carriers. This document defines the minimum deployment, telemetry, and
validation posture needed to operate that policy.

## Server Front Door

The deployment should present one logical service descriptor with per-carrier
targets:

- a `QUIC` UDP listener for the descriptor's `quic` target
- a `WSS` HTTPS endpoint for the descriptor's `wss` target when fallback is
  enabled
- shared service identity fields: `environment_id`, `service_id`, and
  `service_authority`
- shared service static public key authorization semantics

The `QUIC` and `WSS` carrier endpoints may terminate on different edge
components, load balancers, or hostnames only when the descriptor explicitly
maps those carrier routing names. The inner service authority remains the value
bound into the Noise prologue and descriptor-authorized service static public
key.

Implementations must not infer that two carrier endpoints represent the same
logical service only because their hostnames look related. The descriptor is
the routing and identity source of truth.

## QUIC Address Validation And Retry Posture

`QUIC` address validation, Retry, amplification limits, and edge rate limiting
belong to the outer carrier deployment. They protect the service front door but
do not replace inner Noise service static public key authorization.

V1 default posture:

- start with the platform or `QUIC` library's standard address-validation and
  anti-amplification behavior
- allow operators to enable stricter Retry or edge challenge policy under
  attack or hostile-network conditions
- classify repeated Retry exhaustion or carrier rejection before `Secure Ready`
  as `outer_quic_rejected` when the failure happens before inner trust
  validation
- expose Retry and validation failures as observable carrier outcomes, not as
  generic connection failures

Changing Retry posture must not change the Noise prologue, service identity,
or fallback rules. A stricter edge may increase fallback or failure rates, so
it must be visible in telemetry before wider rollout.

## Certificates, Hostnames, And Edge Routing

Outer TLS identity and inner service identity are related but separate:

- `WSS` uses HTTPS/TLS certificate validation for the outer WebSocket endpoint
- `QUIC` uses the transport library's TLS stack and ALPN for the outer carrier
- inner service identity is validated through the descriptor-bound Noise
  prologue and service static public key authorization

The descriptor must explicitly carry any carrier-local host, authority, SNI, or
URL mapping that differs from `service_authority`. Private outer-TLS trust and
managed-network proxying remain compatibility extensions; they must not weaken
inner trust validation.

## Telemetry Requirements

Implementations should emit privacy-safe metrics and structured events that
distinguish at least:

- carrier attempted: `quic` or `wss`
- carrier outcome: `success`, `fallback`, `failure`, `secure_ready`
- fallback reason: `outer_path_failure`, `outer_quic_rejected`, or
  `outer_quic_closed_early`
- inner failure class: `inner_noise_failure`, `inner_trust_failure`, or
  `post_handshake_auth_failure`
- cache state: `live_probe`, `cached_fallback`, or `reprobe`
- descriptor validation result
- encrypted close classification: `graceful`, `abrupt`, or `truncated`

Recommended metric names build on the transport-policy minimums:

- `transport_attempt_total{carrier,outcome,failure_class}`
- `transport_fallback_total{from,to,reason,cache_state}`
- `transport_secure_ready_total{carrier,cache_state}`
- `transport_inner_failure_total{class}`
- `transport_cache_decision_total{decision}`
- `descriptor_validation_total{outcome,reason}`
- `session_close_total{carrier,classification}`
- `quic_address_validation_total{outcome}`
- `quic_retry_total{outcome}`

Allowed dimensions should remain coarse and non-sensitive:

- `environment_id`
- `service_id`
- `carrier`
- `network_class`
- `cache_state`
- `failure_class`
- deployment region or edge name when configured by operators

Telemetry must not include account identifiers, device identifiers, raw
hostnames not already present in operator configuration, tokens, credentials,
server nonces, handshake hashes, or message payloads.

## Minimum Dashboards And Counters

Before wider rollout, operators should be able to answer:

- what percentage of sessions reach `Secure Ready` on `QUIC`
- what percentage fall back from `QUIC` to `WSS`
- which fallback reasons dominate by environment and coarse network class
- whether cached fallback is reducing repeated failed `QUIC` attempts
- whether inner trust failures are occurring and are not being treated as
  fallback
- whether descriptor validation failures or rollback/freshness rejections are
  increasing
- whether abrupt or truncated closes are concentrated on one carrier or edge
- whether Retry/address-validation changes correlate with fallback spikes

## Validation Matrix

Minimum local or staging validation should cover:

| Scenario | Expected Result |
|---|---|
| `QUIC` reachable and server key valid | client reaches `Secure Ready` on `QUIC` |
| UDP blocked or timed out | client falls back to `WSS` when descriptor allows fallback |
| `QUIC` ALPN or version rejected before inner trust | fallback reason is `outer_quic_rejected` |
| `QUIC` stream closes before `Secure Ready` | fallback reason is `outer_quic_closed_early` |
| cached `QUIC`-bad network | client attempts `WSS` first until reprobe deadline |
| fallback disabled by descriptor | client fails instead of silently using `WSS` |
| invalid service static public key authorization | client fails without fallback |
| wrong service identity or service authority | client fails without fallback |
| descriptor rollback or expired descriptor | client rejects descriptor before carrier selection |
| service-key rotation with valid descriptor update | client accepts the new authorized service key |
| service-key rotation with invalid descriptor update | client rejects without fallback |
| network migration or handoff before `Secure Ready` | outcome is classified as outer carrier failure |
| network migration or handoff after `Secure Ready` | session either survives or records a close/failure classification |
| encrypted close sent and acknowledged | close classification is `graceful` |
| outer carrier closes without encrypted close | close classification is `abrupt` or `truncated` |

Managed-network cases such as custom outer-TLS CA trust and explicit HTTP
proxying are required for enterprise compatibility, but their implementation
belongs to the follow-up managed-network tasks.

## Rollout Blockers

Do not treat a deployment as production-ready if any of these remain true:

- fallback rates cannot be separated by reason and carrier
- inner trust failures can be misclassified as outer fallback
- `Secure Ready` success is not measured by carrier
- descriptor rollback/freshness rejection is not tested
- descriptor signature, service static key pin, and key-rotation success/failure
  are not tested
- Retry or address-validation failures are invisible
- close classifications cannot distinguish graceful close from abrupt carrier
  loss
- telemetry redaction rules are not documented and tested
