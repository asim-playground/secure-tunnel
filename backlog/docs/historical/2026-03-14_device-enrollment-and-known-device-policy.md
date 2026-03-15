---
status: historical
normative: false
supersedes: []
superseded_by:
  - backlog/docs/v1-device-enrollment-and-known-device-policy.md
---

# V1 Device Enrollment And Known-Device Policy

Historical baseline: this doc captured the first written device-policy rules.
`backlog/docs/v1-device-enrollment-and-known-device-policy.md` is the active
normative successor.

## Summary

This document defines what device keys mean in v1, how device enrollment
works, and how a previously enrolled device proves continuity on reconnect.
It is a policy document first, not a wire-format spec.

## Core Decision

V1 treats device keys as a continuity and device-binding mechanism, not as a
full account-takeover defense by themselves.

That means:

- a known device can reauthenticate as a previously enrolled app/device instance
- a newly logged-in device is still distinct from a known enrolled device
- the real strength of device auth depends on how hard new-device enrollment is

## What A Device Key Proves

In v1, a device key proves:

- this client currently possesses the private key for a previously enrolled app/device instance
- the client can bind that proof to the current Noise session
- the server can treat this device as a separate principal for revocation, inventory, and risk policy

In v1, a device key does **not** prove:

- that the account itself is cryptographically represented by the device key
- that the device is safe from full compromise
- that a newly logged-in attacker cannot enroll a new device if enrollment policy is too weak
- that the key came from a genuine Apple device unless an attestation mechanism is also used

## Device Identity Model

### Principal Shape

The device identity is a per-installation app/device principal with at least:

- `device_id`
- `account_id`
- `device_public_key`
- device status such as `pending`, `active`, `revoked`, or `suspended`
- enrollment metadata such as creation time, last-seen time, and optional attestation state

### Relationship To Other Identities

- Noise authenticates the backend and secures the channel.
- Account login identifies the user/account using the channel.
- Device authentication identifies a particular enrolled app/device instance under that account.

## Locked V1 Policy

### Enrollment Timing

- Device enrollment happens only after Noise transport mode is established.
- Device enrollment happens only after account login succeeds inside the Noise transport.
- Device auth is never part of the Noise handshake itself in v1.

### Known-Device Reauthentication

- Reconnect or session-establishment includes at most one device-auth challenge/response after the Noise channel is ready when the client wants known-device status.
- Device auth is a per-session step, not a per-message signature requirement.
- Device auth happens after the client presents an account session token or resume context inside the Noise channel, and before the server upgrades the connection to known-device status or issues refreshed privileged session state.
- The server must first validate and pin the account/session context it will bind to the device-auth proof before issuing the device challenge.
- Privileged session refresh or equivalent long-lived session upgrade should be bound to successful known-device proof in v1-capable server policy.

### New-Device Enrollment Semantics

- New-device enrollment is a distinct privileged flow, not the same thing as known-device reauthentication.
- V1 protocol semantics must distinguish `known device` from `new device`.
- V1 guarantees device continuity semantics.
- V1 does **not** require strong cross-device approval or MFA gating at the protocol level, but it must leave room for those policies.

### Security Interpretation

- If enrollment is easy, device keys mainly provide continuity, token binding, revocation, and device inventory.
- If enrollment is later gated more strongly, the same protocol shape can also provide stronger takeover resistance.

## Enrollment Flow

### Preconditions

- Outer `WSS` connection established
- Noise handshake completed
- Server Noise identity verified
- Account login completed inside the Noise transport

### Enrollment Steps

1. Client creates or loads a device signing key.
2. Client sends a request to begin enrollment over the Noise transport.
3. Server returns an enrollment challenge containing at least:
   - `server_nonce`
   - `account_id`
   - optional enrollment policy hints
   - optional attestation request fields
4. Client signs an enrollment statement that binds:
   - protocol context string such as `device-enroll-v1`
   - Noise handshake hash or equivalent session-binding value
   - `server_nonce`
   - `account_id`
   - `device_public_key`
   - optional expiry or issued-at value
5. Client sends the enrollment proof plus optional attestation evidence.
6. Server verifies the signature, channel binding, nonce freshness, and any configured policy checks.
7. Server stores the device record and returns `device_id` plus resulting device state.

## Known-Device Reconnect Flow

### Preconditions

- Outer `WSS` connection established
- Noise handshake completed
- Server Noise identity verified
- Client is operating as a previously enrolled device

### Challenge Flow

1. Client indicates it wants to authenticate as an existing device using `device_id` or another server-recognized hint, together with the account/session context needed for lookup.
2. Server validates the presented account session token or resume context enough to pin the target `account_id` and server-side session context for this reconnect attempt.
3. Server returns a challenge containing at least:
   - `server_nonce`
   - `device_id`
   - pinned `account_id`
   - pinned server-side session context identifier
   - short-lived freshness data such as timestamp or expiry
4. Client signs a device-auth statement that binds:
   - protocol context string such as `device-auth-v1`
   - Noise handshake hash or equivalent session-binding value
   - `server_nonce`
   - `device_id`
   - pinned `account_id`
   - pinned server-side session context identifier
   - freshness data if applicable
5. Server verifies the signature against the enrolled public key, confirms freshness, and checks that the signed account/session context matches the pinned reconnect context.
6. Server marks the session as authenticated by a known device.
7. Only after successful known-device proof does the server issue any upgraded or refreshed privileged session state tied to that device.

## Replay And Freshness Rules

- Enrollment and reconnect proofs must be bound to the current Noise session.
- The server challenge must contain a fresh nonce generated server-side.
- Challenges must be single-use or otherwise replay-detectable.
- Freshness windows should be short and enforced server-side.
- A proof captured from one session must not be valid on a later session.

## Recommended V1 Policy Posture

### Minimum Required Posture

- Distinguish known-device reauthentication from new-device enrollment.
- Require authenticated account session plus Noise channel before enrollment.
- Support revoking a single device without revoking the whole account.

### Recommended But Not Mandatory In Protocol V1

- Treat new-device enrollment as higher risk than known-device reconnect.
- Add step-up approval for sensitive product contexts.
- Bind refresh/session issuance to successful known-device authentication.

## App Attest Scope

App Attest is **optional** in v1 protocol semantics and should be treated as an
enrollment-time enhancement, not a mandatory interoperability requirement.

That means:

- the protocol must allow optional attestation evidence during enrollment
- absence of App Attest must not make the protocol unusable in v1
- App Attest should not be required for every reconnect or every application message
- later policy can decide whether App Attest is recommended, risk-triggered, or mandatory for certain platforms

## Implementation Consequences

- Task `00000004` should define wire messages that support both known-device and new-device flows without forcing strong enrollment gating into the core protocol.
- Task `00000005` should model device auth as an app-layer session concern above the Noise engine, not as a Noise handshake concern.
- Later implementation work should preserve per-device revocation and device-status transitions.

## Deferred Questions

- Should v1 ship with simple self-serve enrollment or with product-level step-up approval by default?
- Should refresh-token issuance require successful known-device proof from the start?
- What exact attestation payload, if any, should be carried for Apple platforms?
