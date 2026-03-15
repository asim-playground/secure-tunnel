---
status: active
normative: true
supersedes: []
superseded_by: []
---

# V1 Transport Selection And Fallback Policy

## Summary

V1 treats transport choice as an availability policy around one shared inner
secure-channel design:

- prefer raw `QUIC`
- fall back to `WSS` only when policy-allowed outer-carrier failures occur
- never change the inner trust model because of carrier choice

## Design Rules

- Attempt carriers sequentially in v1. Do not race `QUIC` and `WSS`.
- Prefer a short-budget `QUIC` attempt over concurrent dual-handshake racing.
- Do not use `QUIC` `0-RTT`, QUIC DATAGRAM, or duplicate-session racing in v1.
- Treat fallback as an outer-carrier policy only. The inner Noise, trust-anchor
  validation, and post-handshake auth flow remain identical across carriers.

## `Secure Ready`

A transport attempt counts as successful only when it reaches `Secure Ready`.

`Secure Ready` means all of the following are true:

- the chosen outer carrier is established
- the expected carrier-level selector value is confirmed
  - `QUIC` ALPN for `QUIC`
  - WebSocket subprotocol for `WSS`
- the framed record channel is available
- the Noise handshake completed successfully
- the server-key authorization validated successfully
- the prologue-bound environment, service identity, and service authority match
  the expected bootstrap context
- both peers entered Noise transport mode

Login success and device-auth success are not part of `Secure Ready`.

## Selection Algorithm

### Unknown Or Reprobe-Eligible Network

1. Load the service descriptor.
2. Compute the coarse network-class cache key.
3. Attempt `QUIC` first with a short connection budget.
4. If `QUIC` reaches `Secure Ready`, select it and update the cache.
5. If `QUIC` fails with a fallback-eligible outer-carrier reason and the
   service descriptor says `allow_wss_fallback=true`, attempt `WSS`.
6. If `QUIC` fails with a fallback-eligible outer-carrier reason but the
   descriptor disallows fallback, fail the connection attempt.
7. If `WSS` reaches `Secure Ready`, select it and update the cache with the
   observed fallback reason.
8. If neither carrier reaches `Secure Ready`, fail the connection attempt.

### Cached `QUIC`-Bad Network

If the cache says `QUIC` recently failed for an outer-carrier reason on the
same coarse network class and the reprobe deadline has not elapsed:

1. If `allow_wss_fallback=true`, attempt `WSS` first.
2. If fallback is disabled by descriptor policy, fail fast instead of silently
   overriding the descriptor.
3. Record when the selection used cached fallback posture.
4. Reprobe `QUIC` only after the cache expires or the network class changes.

This optimization exists to reduce repeated user-visible connect delays on
networks that consistently block or degrade UDP.

## Cache Model

The cache key is:

- logical service identifier
- environment identifier
- coarse network class

The cache value may include:

- last successful carrier
- last fallback-eligible `QUIC` failure class
- first-observed and last-observed timestamps
- next `QUIC` reprobe timestamp

The cache must not store:

- trust-failure results as if they were network reachability hints
- account or device-auth outcomes
- per-user sensitive identifiers not needed for transport policy

## Fallback Classification

| Failure Class | Example | Fallback To `WSS` |
|---|---|---|
| `outer_path_failure` | UDP blocked, connect timeout, network unreachable | `yes` |
| `outer_quic_rejected` | ALPN mismatch, version rejection, Retry exhaustion before inner trust | `yes` |
| `outer_quic_closed_early` | QUIC connection or stream closes before `Secure Ready` | `yes` |
| `inner_noise_failure` | malformed Noise message, handshake processing failure | `no` |
| `inner_trust_failure` | invalid signature, expired server-key authorization, wrong service identity | `no` |
| `post_handshake_auth_failure` | login rejected, device challenge rejected, enrollment denied | `no` |

The default v1 policy allows fallback for `QUIC` ALPN or version rejection only
when the service descriptor still advertises the same logical service over
`WSS` and explicitly permits fallback. That failure is a
carrier-capability mismatch, not proof that the inner service identity should
change.

## Higher-Layer Reporting

Higher layers should receive:

- the selected carrier
- whether fallback occurred
- the normalized fallback reason when fallback occurred
- whether carrier choice came from cache or a live reprobe

Higher layers must not collapse inner trust failures into generic fallback
events.

## Observability Minimums

V1 implementations should emit at least:

- `transport_attempt_total{carrier, outcome}`
- `transport_fallback_total{from, to, reason}`
- `transport_secure_ready_total{carrier}`
- `transport_inner_failure_total{class}`
- `transport_cache_decision_total{decision}`

These counters are the minimum needed to distinguish network reachability
problems from actual trust failures.

## Test Cases

Minimum validation should cover:

- `QUIC` success without fallback
- forced `WSS` fallback on UDP-blocked path
- fallback after `QUIC` ALPN or version rejection
- rejection without fallback on invalid server-key authorization
- rejection without fallback on mismatched service identity
- rejection without fallback on post-handshake login or device-auth failure
