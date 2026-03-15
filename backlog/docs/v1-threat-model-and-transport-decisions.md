---
status: active
normative: true
supersedes:
  - backlog/docs/historical/2026-03-14_v1-threat-model-and-decisions.md
superseded_by: []
---

# V1 Threat Model And Transport Decisions

## Summary

Secure Tunnel v1 uses one inner secure-channel design across two outer
carriers:

- raw `QUIC` over UDP is preferred
- `WSS` over outer TLS is the compatibility fallback

The outer carrier is an availability and reachability layer. The inner
`Noise_NX_25519_ChaChaPoly_BLAKE2s` session is the end-to-end confidentiality
and integrity boundary.

## Product Context

Secure Tunnel is intended to protect sensitive application traffic even when
the outer network path can terminate TLS through a trusted interception proxy.
That design goal is unchanged whether the client reaches the service over
`QUIC` or `WSS`.

## Security Goal

V1 must provide an application-layer channel where:

- the client authenticates the backend independently of the outer TLS trust
  store
- sensitive application traffic remains confidential from TLS-intercepting
  intermediaries
- transport choice does not change the inner trust model
- later login and device-auth steps bind to the authenticated inner session

## Threat Model

### In Scope

- On-path TLS interception where a local network or device trust store allows a
  proxy to terminate outer TLS
- Active transport downgrade pressure that forces the client onto the `WSS`
  path or blocks `QUIC`
- Replay risks caused by sending application data before the inner channel is
  fully established
- Misbinding of backend identity if the client relies only on outer TLS
  certificates
- Session or token reuse from a different machine when the app later
  distinguishes known-device reauthentication from new-device enrollment
- Configuration drift where carrier details leak into the secure-channel core
  and weaken future transport independence

### Partially In Scope

- Stolen account credentials
  - V1 can distinguish known-device reauthentication from brand-new device
    enrollment.
  - V1 does not by itself prevent account takeover if new-device enrollment
    policy remains weak.
- Device compromise short of total platform compromise
  - Secure local key storage helps, but cannot fully defend against a hostile
    endpoint.
- Availability downgrade from `QUIC` to `WSS`
  - V1 preserves inner confidentiality and integrity after fallback.
  - V1 does not guarantee `QUIC` reachability on hostile or degraded networks.

### Out Of Scope

- Full protection against a fully compromised or instrumented client device
- `QUIC` DATAGRAM, `QUIC` `0-RTT`, Noise early data, or duplicate transport
  racing in v1
- Mutual-auth Noise handshake patterns for returning devices in v1
- End-user cross-device cryptographic identity via an account key in v1
- Carrier-specific application semantics that change the inner protocol rules

## Trust Boundary

### Outer Carrier

The outer carrier exists for service reachability and network compatibility. It
is not the final confidentiality boundary for sensitive payloads.

### Inner Noise Channel

The inner Noise session is the end-to-end confidentiality and integrity
boundary. Sensitive login, session, and device-auth traffic must occur only
after the channel reaches `Secure Ready`.

## Roles Of Security Layers

### Outer Carrier

- provides network reachability over either `QUIC` or `WSS`
- may expose metadata such as timing, endpoint choice, or fallback reason
- does not prove end-to-end backend identity in the interception threat model

### Inner Noise

- authenticates the service against an application-controlled trust anchor
- derives fresh per-connection session keys
- provides the channel-binding value `h` used by later auth flows

### Session/Login State

- identifies the account using the secure channel
- remains above the Noise layer rather than replacing it
- must be sent only after `Secure Ready`

### Device Authentication

- distinguishes a known enrolled app/device instance from a newly enrolling one
- is bound to the current Noise session rather than replacing the Noise
  handshake
- depends on enrollment policy for its real takeover resistance

## Key Roles

### Server Key

The server key is the backend's long-lived Noise static key for the secure
channel. It is used only for Noise identity and is authorized by a shipped
trust anchor.

### Device Key

The device key is a per-installation signing key used after the Noise handshake
to prove possession of an enrolled device identity. In v1 it is not part of
the Noise handshake itself.

### Account Key

An account key is optional and out of scope for v1. Account identity is carried
by login and session semantics above the secure channel rather than by a
cross-device cryptographic user key.

### Session Keys

Session keys are the ephemeral transport keys derived by Noise for a single
connection. They are per-connection and discarded when the session ends.

## Locked V1 Decisions

### Transport Shape

- V1 uses transport selection with raw `QUIC` preferred and `WSS` fallback.
- The protocol core remains transport-agnostic and does not bind carrier choice
  into the inner prologue.
- One logical service identity spans both carriers.
- Carrier selection is an availability policy, not a second security design.

### Noise Pattern

- V1 uses a server-auth pattern based on `NX`.
- Returning-device variants that place client static identity inside the Noise
  handshake are deferred.

### Trust Anchor Model

- The client ships an application-controlled trust anchor.
- The server presents root-signed metadata authorizing the current server Noise
  static key.
- Trust decisions for the inner channel do not depend on the outer TLS
  certificate chain.

### Login And Enrollment Placement

- Login occurs only after Noise transport mode is established.
- Device enrollment occurs only after Noise transport mode is established.
- Known-device challenge/response also occurs after Noise transport mode is
  established.

### Replay Posture

- V1 does not send early application data during the Noise handshake.
- V1 avoids `QUIC` `0-RTT` behavior entirely.
- Replay-sensitive application actions must wait until the completed Noise
  transport is active.

## Fallback Security Rules

Fallback from `QUIC` to `WSS` is allowed only for outer-carrier or path
failures before `Secure Ready`, and only when the service descriptor explicitly
allows `WSS` fallback.

Fallback is allowed for examples such as:

- UDP blocked or blackholed
- `QUIC` connection timeout or path failure
- `QUIC` capability mismatch such as unsupported ALPN or version rejection
  before inner trust is established
- `QUIC` carrier closure before the inner channel becomes secure-ready

Fallback is not allowed for:

- Noise handshake processing failure
- invalid or expired server-key authorization
- wrong environment, service identity, or service authority
- post-handshake login failure
- device-auth or enrollment policy failure

An attacker forcing the `WSS` path is therefore treated as an availability
downgrade, not as a successful bypass of the inner trust model.

## Non-Goals For V1

- No early data or handshake-payload application requests
- No attempt to make device keys serve as account identity keys
- No attempt to defend against total client compromise
- No implementation of returning-device mutual-auth handshakes in the first
  slice

## Implementation Consequences

- Transport-selection code must surface outer-carrier failures separately from
  inner trust failures.
- No code path may silently retry over `WSS` after an inner trust failure.
- Device-policy work must preserve the difference between known-device
  reauthentication and new-device enrollment.
- Active implementation work should cite the stable `v1-*` docs, not the dated
  WSS-first baseline.
