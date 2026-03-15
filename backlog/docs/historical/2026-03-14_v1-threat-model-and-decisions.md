---
status: historical
normative: false
supersedes: []
superseded_by:
  - backlog/docs/v1-threat-model-and-transport-decisions.md
---

# V1 Threat Model And Protocol Decisions

Historical baseline: this doc captured the earlier WSS-first v1 decisions.
`backlog/docs/v1-threat-model-and-transport-decisions.md` is the active
normative successor.

## Summary

This document locks the v1 security boundary and threat model for Secure Tunnel.
It translates the initial research note into implementation-facing decisions so
later protocol and architecture work can build against explicit assumptions.

## Product Context

Secure Tunnel is intended to protect application confidentiality and integrity
even when the outer network path terminates TLS through a trusted interception
proxy. The current v1 carrier is `WSS` over outer TLS, but the end-to-end trust
boundary is the inner secure channel, not the outer TLS session.

## Security Goal

V1 must provide an application-layer channel where:

- the client can authenticate the backend independently of the outer TLS trust store
- sensitive application traffic remains confidential from TLS-intercepting intermediaries
- the backend can bind later login and device-auth steps to the authenticated inner session

## Threat Model

### In Scope

- On-path TLS interception where a local network or device trust store allows a proxy to terminate outer TLS
- Replay risks caused by sending application data before the inner channel is fully established
- Misbinding of backend identity if the client relies only on outer TLS certificates
- Session or token reuse from a different machine when the app later introduces device-bound reauthentication
- Configuration drift where transport details leak into the secure-channel core and weaken future transport independence

### Partially In Scope

- Stolen account credentials
  - V1 can distinguish known-device reauthentication from brand-new device enrollment.
  - V1 does not by itself prevent account takeover if new-device enrollment policy remains weak.
- Device compromise short of total platform compromise
  - Secure local key storage helps, but cannot fully defend against a hostile endpoint.

### Out Of Scope

- Full protection against a fully compromised or instrumented client device
- QUIC transport, transport racing, or transport selection logic
- 0-RTT or Noise early-data optimization
- Mutual-auth Noise handshake patterns for returning devices in v1
- End-user cross-device cryptographic identity via an account key in v1

## Trust Boundary

### Outer TLS

Outer TLS for the `WSS` carrier exists for network compatibility, service
reachability, and operational convenience. It is not the final confidentiality
boundary for sensitive payloads.

### Inner Noise Channel

The inner Noise session is the end-to-end confidentiality and integrity
boundary. Sensitive login, session, and device-auth traffic must occur only
after the Noise handshake completes and transport mode is established.

## Roles Of Security Layers

### Outer TLS

- protects the transport from ordinary passive interception
- enables WSS connectivity through existing infrastructure
- does not prove end-to-end backend identity in the interception threat model

### Inner Noise

- authenticates the service against an application-controlled trust anchor
- derives fresh per-connection session keys
- provides the channel-binding value used by later auth flows

### Session/Login State

- identifies the account using the secure channel
- remains above the Noise layer rather than replacing it
- must be sent only after the Noise transport is established

### Device Authentication

- distinguishes a known enrolled app/device instance from a newly enrolling one
- is bound to the current Noise session rather than replacing the Noise handshake
- depends on explicit enrollment policy for its real security value

## Key Roles

### Server Key

The server key is the backend's long-lived Noise static key for the secure
channel. It is used only for Noise identity and is authorized by a shipped root
or trust anchor.

### Device Key

The device key is a per-installation or per-device signing key used after the
Noise handshake to prove possession of an enrolled device identity. In v1 it is
not part of the Noise handshake itself.

### Account Key

An account key is optional and out of scope for v1. Account identity is carried
by login/session semantics above the secure channel rather than by a
cross-device cryptographic user key.

### Session Keys

Session keys are the ephemeral transport keys derived by Noise for a single
connection. They are per-connection and discarded when the session ends.

## Locked V1 Decisions

### Transport Shape

- V1 uses outer `WSS` over TLS.
- The protocol core must remain transport-agnostic even though only a WSS binding is implemented initially.
- WebSocket is the only outer carrier in v1 implementation work.
- The research note explored a later QUIC-first direction, but task `00000003` narrows implementation scope for v1 to WSS only so protocol and architecture work have a single delivery target.

### Noise Pattern

- V1 uses a server-auth pattern based on `NX`.
- Returning-device variants that place client static identity inside the Noise handshake are deferred.
- `IK`, `XK`, `KK`, and similar returning-device handshake variants are explicitly deferred to later phases.

### Trust Anchor Model

- The client ships an application-controlled trust anchor.
- The server presents root-signed metadata authorizing the current server Noise static key.
- Trust decisions for the inner channel do not depend on the outer TLS certificate chain.

### Login And Enrollment Placement

- Login occurs only after Noise transport mode is established.
- Device enrollment occurs only after Noise transport mode is established.
- Known-device challenge/response also occurs after Noise transport mode is established.

### Replay Posture

- V1 does not send early application data during the Noise handshake.
- V1 avoids 0-RTT behavior entirely.
- Replay-sensitive application actions must wait until the completed Noise transport is active.

## Deferred Decisions

- QUIC transport and any transport-fallback strategy
- Exact framing envelope and transport-agnostic record format details
- Exact server certificate payload shape for the Noise static key
- Exact device-enrollment and known-device policy details
- App Attest inclusion, exclusion, or optional treatment in v1

## Non-Goals For V1

- No early data or handshake-payload application requests
- No attempt to make device keys serve as account identity keys
- No attempt to defend against total client compromise
- No implementation of returning-device mutual-auth handshakes in the first slice

## Implementation Consequences

- Protocol work can assume a completed Noise transport before any login or device-auth messages are defined.
- Architecture work should keep secure-channel logic separate from concrete WebSocket or Flutter concerns.
- Device-policy work should focus on post-handshake enrollment and challenge/response rather than handshake-level client identity.
- Future optimization work must justify any move away from the current no-early-data posture.

## Open Questions Left For Follow-Up Tasks

- What exact device-enrollment policy should gate new-device trust in v1?
- How transport-agnostic should the framed record format be from day one?
- Is App Attest optional, deferred, or part of the initial known-device story?
