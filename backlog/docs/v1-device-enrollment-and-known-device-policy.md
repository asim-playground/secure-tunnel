---
status: active
normative: true
supersedes:
  - backlog/docs/historical/2026-03-14_device-enrollment-and-known-device-policy.md
superseded_by: []
---

# V1 Device Enrollment And Known-Device Policy

## Summary

This document defines what device keys mean in v1, how device enrollment works,
and how a previously enrolled device proves continuity on reconnect.

V1 treats device keys as a continuity and device-binding mechanism, not as a
full account-takeover defense by themselves.

## State Anchors

This policy uses the shared protocol states defined in
`backlog/docs/v1-core-protocol-quic-and-wss-bindings.md`.

- enrollment requires `Account Authenticated (fresh)` by default
- known-device reauthentication begins after `Secure Ready` once the server has
  pinned account or resume context for the reconnect attempt
- privileged refresh should require `Known Device Authenticated`

## What A Device Key Proves

In v1, a device key proves:

- this client currently possesses the private key for a previously enrolled
  app/device instance
- the client can bind that proof to the current Noise session
- the server can treat this device as a separate principal for revocation,
  inventory, and risk policy

In v1, a device key does not prove:

- that the account itself is cryptographically represented by the device key
- that the device is safe from full compromise
- that a newly logged-in attacker cannot enroll a new device if enrollment
  policy is too weak
- that the key came from a genuine Apple device unless attestation is also used

## Key Material Inventory

### Root Authorization Keypair

- Private key location: server-side or offline signing system only
- Public key location: shipped in the client bootstrap material
- Purpose: signs server-key authorization objects and signed descriptor updates
- Used by client for verification only, never for client-originated signatures

### Server Noise Static Keypair

- Private key location: backend service instance or secure key service
- Public key location: distributed to the client only through the signed
  server-key authorization object
- Purpose: authenticates the backend in the Noise handshake
- Never reused for device-auth or enrollment signatures

### Candidate Device Signing Keypair

- Private key location: current client installation only
- Public key location: sent during enrollment
- Purpose: signs the enrollment proof that introduces a new device key to the
  server
- Postcondition on successful enrollment: becomes the enrolled device signing
  keypair for the issued `device_id`

### Enrolled Device Signing Keypair

- Private key location: current client installation only
- Public key location: stored in the server's device record for the associated
  `device_id`
- Purpose: signs reconnect-time known-device proofs
- Preconditions: the server-side device record must still be in a state that
  allows known-device auth, usually `active`

## Fresh Versus Resumed Account State

### `Account Authenticated (fresh)`

Reached by successful primary authentication or an explicit step-up inside the
current secure session.

This is a short-lived server-defined capability, not a permanent session label.

### `Account Authenticated (resumed)`

Reached by restoring prior server-managed session state inside the current
secure session without a fresh primary factor.

### V1 Enrollment Rule

New-device enrollment requires `Account Authenticated (fresh)` by default.

A rollout that allows enrollment from `Account Authenticated (resumed)` must
document an equivalent step-up control before shipping. Resumed session state
alone is not sufficient by default.

The server must expire `Account Authenticated (fresh)` for enrollment purposes
after a short freshness window, after a session-recovery path that no longer
has fresh-factor assurance, or after a successful enrollment attempt. Once that
happens, the session is treated as `Account Authenticated (resumed)` until the
client completes fresh auth or step-up again.

## Device Identity Model

The device identity is a per-installation app/device principal with at least:

- `device_id`
- `account_id`
- `device_public_key`
- device state
- enrollment metadata such as creation time, last-seen time, and optional
  attestation state

Device authentication identifies a particular enrolled app/device instance under
that account. It does not replace backend authentication or account login.

## Device Lifecycle

| State | Meaning | May satisfy known-device auth | Typical Entry | Typical Exit |
|---|---|---|---|---|
| `pending` | created but not yet trusted for normal reconnect policy | `no` | gated enrollment awaiting approval, attestation, or risk checks | `active`, `revoked` |
| `active` | normal enrolled device | `yes` | successful enrollment or approval of `pending` device | `suspended`, `revoked` |
| `suspended` | temporarily blocked from privileged refresh and known-device status | `no` | risk hold, admin action, attestation concern | `active`, `revoked` |
| `revoked` | permanently invalid for future proofs | `no` | user removal, compromise response, policy action | none |

By default, reinstalling the app creates a new device identity. Reusing an old
`device_id` without proving possession of the old private key is not allowed.

## Locked V1 Policy

### Enrollment Timing

- device enrollment happens only after `Secure Ready`
- device enrollment happens only after `Account Authenticated (fresh)` unless a
  documented step-up policy overrides that default
- device auth is never part of the Noise handshake itself in v1

### Known-Device Reauthentication

- reconnect includes at most one device-auth challenge and response after
  `Secure Ready` when the client wants known-device status
- device auth is a per-session step, not a per-message signature requirement
- the server must first validate and pin the account or session context it will
  bind to the device-auth proof before issuing the challenge
- privileged session refresh or equivalent long-lived session upgrade should be
  bound to successful known-device proof in v1-capable server policy

### New-Device Enrollment Semantics

- new-device enrollment is a distinct privileged flow, not the same thing as
  known-device reauthentication
- v1 protocol semantics must distinguish `known device` from `new device`
- v1 guarantees device continuity semantics
- v1 leaves room for stronger product-level approval or MFA policy above the
  protocol

## Enrollment Flow

### Preconditions

- connection is in `Account Authenticated (fresh)`
- the server-defined fresh-auth window for enrollment has not expired
- server Noise identity has already been verified
- the client has a candidate device signing keypair available, or can generate
  one before sending the enrollment proof

### Steps

1. Client creates or loads a device signing key.
2. Client sends a request to begin enrollment over Noise transport.
3. Server returns an enrollment challenge containing at least:
   - `server_nonce`
   - `account_id`
   - optional enrollment policy hints
   - optional attestation request fields
4. Client signs, using the candidate device private signing key, an enrollment
   statement that binds:
   - `device-enroll-v1`
   - final Noise handshake hash `h`
   - `server_nonce`
   - `account_id`
   - `device_public_key`
   - optional expiry or issued-at value
5. Client sends the enrollment proof plus optional attestation evidence.
6. Server verifies the signature, channel binding, nonce freshness, and any
   configured policy checks.
7. Server stores the device record and returns `device_id` plus resulting
   device state.

### Postconditions

- the server has associated the candidate device public key with a new
  `device_id`
- the client now treats that same local keypair as the enrolled device signing
  keypair for that `device_id`
- the resulting device state is explicit, for example `active` or `pending`

## Known-Device Reconnect Flow

### Preconditions

- connection is in `Secure Ready`
- client is operating as a previously enrolled device
- the client still possesses the enrolled device private signing key that
  corresponds to the presented `device_id`
- the server-side device record is expected to allow known-device auth, usually
  `active`

### Challenge Flow

1. Client indicates it wants to authenticate as an existing device using
   `device_id` or another server-recognized hint, together with the account or
   resume context needed for lookup.
2. Server validates the presented account session token or resume context
   enough to pin the target `account_id` and server-side session context for
   this reconnect attempt.
3. Server returns a challenge containing at least:
   - `server_nonce`
   - `device_id`
   - pinned `account_id`
   - pinned server-side session context identifier
   - short-lived freshness data such as timestamp or expiry
4. Client signs, using the enrolled device private signing key already bound to
   that `device_id`, a device-auth statement that binds:
   - `device-auth-v1`
   - final Noise handshake hash `h`
   - `server_nonce`
   - `device_id`
   - pinned `account_id`
   - pinned server-side session context identifier
   - freshness data if applicable
5. Server verifies the signature against the enrolled public key, confirms
   freshness, and checks that the signed account or session context matches the
   pinned reconnect context.
6. Server marks the session as `Known Device Authenticated`.
7. Only after successful known-device proof does the server issue any upgraded
   or refreshed privileged session state tied to that device.

### Postconditions

- the server has verified possession of the enrolled device private signing key
  for the target `device_id`
- the session is marked `Known Device Authenticated`
- privileged refresh or equivalent upgraded session state may now be issued

## Replay And Freshness Rules

- enrollment and reconnect proofs must be bound to the current Noise session
- the server challenge must contain a fresh nonce generated server-side
- challenges must be single-use or otherwise replay-detectable
- freshness windows should be short and enforced server-side
- a proof captured from one session must not be valid on a later session

## App Attest Scope

App Attest is optional in v1 protocol semantics and should be treated as an
enrollment-time enhancement, not a mandatory interoperability requirement.

That means:

- the protocol must allow optional attestation evidence during enrollment
- absence of App Attest must not make the protocol unusable in v1
- App Attest should not be required for every reconnect or every application
  message
- later policy can decide whether App Attest is recommended, risk-triggered, or
  mandatory for certain platforms

## Implementation Consequences

- app-layer message design must support both known-device and new-device flows
  without forcing strong enrollment gating into the core Noise handshake
- architecture work should model device auth above the Noise engine, not as a
  Noise handshake concern
- product rollout work must explicitly choose whether any enrollment policy
  stronger than the default fresh-auth requirement is needed
