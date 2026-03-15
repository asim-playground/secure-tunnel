---
status: historical
normative: false
supersedes: []
superseded_by:
  - backlog/docs/v1-core-protocol-quic-and-wss-bindings.md
---

# V1 Core Protocol And WSS Binding

Historical baseline: this doc captured the earlier WSS-first protocol artifact.
`backlog/docs/v1-core-protocol-quic-and-wss-bindings.md` is the active
normative successor for v1 implementation work.

## Summary

This document specifies the Secure Tunnel v1 protocol as:

- a transport-agnostic framed record protocol
- a server-authenticated Noise channel carried over those records
- post-handshake login and device-auth flows
- an initial `WSS` carrier binding for v1 implementation

It was the primary source of truth for task `00000004` during the earlier
WSS-first phase.

## Scope

This spec covers:

- the framed record abstraction used by the secure-channel core
- the Noise handshake and trust-validation flow
- post-handshake login, device enrollment, and known-device reauthentication
- the `WSS` binding used in v1

This spec does not cover:

- QUIC or any other outer carrier
- 0-RTT or early application data
- handshake-level client static authentication
- final Rust API design

## Protocol Layers

The v1 stack is:

1. outer `WSS` over TLS
2. framed duplex record channel
3. Noise `NX` handshake
4. Noise transport messages
5. application session messages such as login and device auth

The secure-channel core starts at layer 2, not at the WebSocket API.

## Core Concepts

### Framed Record

A framed record is the smallest message unit exposed to the secure-channel core.

Properties:

- reliable and ordered
- preserves full-record boundaries
- max payload size of `65535` bytes in v1
- one Noise handshake or transport message fits inside one framed record

The core protocol never depends on TCP chunks, WebSocket frames, or any other
lower-level segmentation behavior.

### Service Identity

The inner authenticated service identity is represented by the server Noise
static key plus root-signed authorization metadata. It is independent of the
outer TLS certificate chain.

### Channel Binding

Post-handshake auth flows bind to the final Noise handshake hash `h` from the
completed handshake.

## Versioning

V1 uses these fixed identifiers:

- protocol id: `secure-tunnel-v1`
- WSS subprotocol string: `secure-tunnel-v1`
- Noise pattern family: server-auth `NX`

The protocol id is part of the Noise prologue and the WSS binding.

## Noise Suite

V1 freezes the inner handshake suite as:

- `Noise_NX_25519_ChaChaPoly_BLAKE2s`

The implementation must expose:

- a handshake state
- a transport state
- access to the final handshake hash `h`

## Prologue

The Noise prologue must bind stable context that both peers know before the
handshake.

V1 prologue fields:

- protocol id: `secure-tunnel-v1`
- environment id such as `prod` or `staging`
- service identity string such as the logical API service name
- application-level hostname or authority string expected by the client

The prologue must not vary by carrier in v1. In particular, it must not bind
`transport=wss` because the inner identity is intended to survive future
transport additions.

## Server-Key Authorization

### Trust Anchor

The client ships an application-controlled root public key or compact trust
anchor used only to authorize server Noise static keys.

### Server Certificate Payload

The server's handshake payload must carry authorization metadata for the current
server Noise static key. The exact binary format may evolve, but v1 requires the
payload to cover at least:

- server Noise public key
- key identifier
- not-before timestamp
- not-after timestamp
- environment id
- service identity
- hostname or authority binding
- protocol version or compatibility marker
- signature from the shipped trust anchor

### Validation Rules

The client must reject the connection if:

- the trust-anchor signature is invalid
- the presented server Noise key does not match the authorized key in the payload
- the certificate is expired or not yet valid
- the environment, service identity, or hostname binding does not match the expected prologue context

Trust validation happens before the session is considered secure-ready.

### Rotation Model

V1 server-key rotation behavior is:

- the shipped trust anchor remains stable during ordinary server-key rotation
- the server may rotate its Noise static key by presenting a newly signed authorization payload for the replacement key
- overlapping validity windows for old and new server-key authorizations are allowed
- the client accepts any currently valid authorized server key whose identity fields match the expected prologue context
- expired server-key authorizations must always be rejected
- v1 relies on expiry plus replacement for ordinary rotation; explicit revocation lists are out of scope
- backup trust anchors and root rollover are deferred and must not be inferred by implementations

## Record Types

The core protocol uses framed records with the following semantic types.

### 1. Noise Handshake Record

- carries exactly one Noise handshake message
- for the responder's certificate-bearing message, also carries the root-signed server-key payload
- valid only until the handshake completes

### 2. Noise Transport Record

- carries exactly one Noise transport message
- after the handshake completes, all application messages are encrypted inside this layer

### 3. Encrypted Close Message

- an application-level message sent inside Noise transport before outer closure
- communicates normal shutdown intent and optional close reason category

The outer carrier close is not the semantic close signal for the protocol.

## Connection State Machine

### State 0: Outer Connected

Preconditions:

- `WSS` connection established
- correct WSS subprotocol negotiated

No application credentials or device proofs may be sent here.

### State 1: Noise Handshake In Progress

Client starts the `NX` initiator handshake and sends one Noise handshake record.

Server replies with a Noise handshake record that includes:

- the responder Noise handshake payload
- the root-signed server-key authorization metadata

Client validates:

- Noise message processing succeeded
- server key authorization succeeded
- prologue-bound identity fields match expectation

If validation fails, the connection must be aborted without entering transport
mode.

### State 2: Secure Ready

The handshake is complete and both peers have entered Noise transport mode.

From this point onward:

- login is allowed
- account session recovery is allowed
- all sensitive application messages must be encrypted inside Noise transport

Device enrollment first becomes legal only after `Account Authenticated`.
Known-device reauthentication also requires the account/session context rules
defined below.

### State 3: Account Authenticated

The client has completed login or session recovery inside Noise transport.

This state identifies the account but does not yet imply known-device status.

### State 4: Known Device Authenticated

The connection is bound both to an authenticated account/session context and to
successful proof of an enrolled device key.

This state may be required before the server issues privileged refreshed session
state.

### State 5: Closing

One peer sends the encrypted close message, optionally waits for ack/drain
policy, then closes the outer `WSS` connection.

## Message Flow

### Flow A: Initial Secure Connection

1. Establish `WSS`.
2. Negotiate subprotocol `secure-tunnel-v1`.
3. Exchange Noise handshake records.
4. Client validates root-signed server-key authorization.
5. Enter Noise transport mode.

### Flow B: Login

1. Client sends encrypted login message inside Noise transport.
2. Server validates login and returns encrypted account/session result.
3. Connection enters `Account Authenticated`.

### Flow C: New-Device Enrollment

1. Client must already be in `Account Authenticated`.
2. Client sends encrypted enrollment-start request.
3. Server returns encrypted enrollment challenge containing at least:
   - `server_nonce`
   - `account_id`
   - optional enrollment policy hints
   - optional attestation request fields
4. Client signs an enrollment proof binding:
   - `device-enroll-v1`
   - handshake hash `h`
   - `server_nonce`
   - `account_id`
   - `device_public_key`
   - optional freshness fields
5. Client sends enrollment proof plus optional attestation evidence.
6. Server verifies proof and stores device record.
7. Server returns encrypted enrollment result with `device_id` and device state.

### Flow D: Known-Device Reauthentication

1. Client establishes the Noise channel.
2. Client presents account session token or resume context plus `device_id` or device hint inside Noise transport.
3. Server validates and pins the reconnect context:
   - target `account_id`
   - target server-side session context identifier
4. Server returns encrypted device-auth challenge with:
   - `server_nonce`
   - `device_id`
   - pinned `account_id`
   - pinned session context identifier
   - freshness data
5. Client signs a device-auth proof binding:
   - `device-auth-v1`
   - handshake hash `h`
   - `server_nonce`
   - `device_id`
   - pinned `account_id`
   - pinned session context identifier
   - freshness data
6. Server verifies proof and marks the session as `Known Device Authenticated`.
7. Only after this step may the server issue privileged refreshed session state.

## Replay And Early-Data Rules

V1 replay posture is intentionally conservative.

Rules:

- no application login data in handshake payloads
- no device proofs in handshake payloads
- no 0-RTT or early application data
- enrollment challenges must be fresh and replay-detectable
- known-device challenges must be fresh and replay-detectable
- channel binding must ensure a proof from one Noise session is invalid in another

## Encrypted Application Messages

The core protocol expects application messages above Noise transport. V1 does
not yet freeze the final binary schema, but message families must include at
least:

- `login_request`
- `login_result`
- `device_enroll_start`
- `device_enroll_challenge`
- `device_enroll_finish`
- `device_auth_start`
- `device_auth_challenge`
- `device_auth_finish`
- `close`

Task `00000005` should map these into Rust types without changing the state
machine specified here.

## WSS Binding

### Endpoint Shape

V1 uses `wss://` endpoints only.

### Subprotocol

The client must request:

- `Sec-WebSocket-Protocol: secure-tunnel-v1`

The server must reject the connection if that subprotocol is not negotiated.

### Record Mapping

The `WSS` binding maps one framed record to one binary WebSocket message.

Rules:

- text WebSocket messages are invalid
- each binary WebSocket message contains exactly one framed record
- WebSocket fragmentation is ignored by the core; only the reassembled message matters
- ping/pong and carrier keepalive stay in the WSS binding, not in the secure-channel core

### Carrier Errors

The secure-channel core must treat outer `WSS` closure before encrypted close as
an abrupt transport failure, not as a graceful protocol shutdown.

## Encrypted Shutdown

V1 requires an explicit encrypted close message before normal outer shutdown.

Minimum behavior:

1. send encrypted `close`
2. optionally wait for peer `close` or drain acknowledgment according to implementation policy
3. close the outer `WSS` connection

This prevents semantic shutdown from depending on a raw carrier close alone.

## Minimum Validation Invariants

The first Rust implementation must be able to verify at least these invariants:

- a connection cannot reach secure-ready without successful server-key authorization
- login messages cannot be processed before Noise transport mode
- device enrollment cannot happen before account login
- known-device proof cannot complete before reconnect context is pinned
- a proof from one Noise session is rejected on another session
- missing or wrong WSS subprotocol prevents protocol start
- outer close without encrypted close is treated as abrupt termination

## Suggested Test Vectors And Scenarios

Minimum scenario coverage for initial implementation work:

- valid `NX` handshake with valid server-key authorization
- valid `Noise_NX_25519_ChaChaPoly_BLAKE2s` handshake reaching transport mode
- invalid trust-anchor signature
- expired server-key authorization payload
- overlapping old/new valid server-key authorizations during server-key rotation
- mismatched hostname or service identity in authorization payload
- attempt to send login before transport mode
- successful login then successful enrollment
- successful reconnect with pinned session context and valid device proof
- replayed device challenge response on a new Noise session
- outer `WSS` close before encrypted close

## Follow-On Work

Historical note: the task references below describe the follow-on backlog shape
at the time this baseline artifact was written.

- Task `00000005` should turn this spec into crate boundaries and Rust-facing APIs.
- Task `00000007` should implement the framed duplex abstraction assumed here.
- Task `00000008` should prove the `NX` handshake plus server-key validation path.
- Task `00000009` should implement the `WSS` binding exactly as described here.
