---
status: active
normative: true
supersedes:
  - backlog/docs/historical/2026-03-14_v1-core-protocol-and-wss-binding.md
superseded_by: []
---

# V1 Core Protocol With QUIC And WSS Bindings

## Summary

This document is the active v1 protocol source of truth for:

- the transport-agnostic framed secure-channel core
- the shared server-authenticated Noise handshake
- the shared post-handshake session and device-auth model
- the carrier bindings for raw `QUIC` and `WSS`

Transport selection and fallback behavior are defined in
`backlog/docs/v1-transport-selection-and-fallback-policy.md`. This spec defines
what each carrier must do once selected.

## Scope

This spec covers:

- the framed record abstraction used by the secure-channel core
- the Noise handshake and trust-validation flow
- post-handshake login, device enrollment, and known-device reauthentication
- the `QUIC` and `WSS` bindings for v1

This spec does not cover:

- `QUIC` DATAGRAM or `QUIC` `0-RTT`
- handshake-level client static authentication
- final Rust API design
- carrier selection cache heuristics beyond the normative rules in the
  transport-selection doc

## Versioning Identifiers

V1 keeps separate identifiers for separate protocol surfaces even though they
start with the same string value:

- protocol id: `secure-tunnel-v1`
- `QUIC` ALPN: `secure-tunnel-v1`
- `WSS` subprotocol: `secure-tunnel-v1`
- Noise suite: `Noise_NX_25519_ChaChaPoly_BLAKE2s`

The identifiers may evolve independently in later versions. Implementations
must not assume that "same string today" means "same identifier kind forever."

## Protocol Layers

The v1 stack is:

1. selected outer carrier
2. framed record channel
3. Noise `NX` handshake
4. Noise transport messages
5. application session messages such as login and device auth

The secure-channel core starts at layer 2, not at the concrete carrier API.

## Framed Record Model

### Record Semantics

A framed record is the smallest message unit exposed to the secure-channel
core.

Properties:

- reliable and ordered
- preserves one complete secure-channel payload per record
- carries exactly one Noise handshake message, one Noise transport ciphertext,
  or one carrier-independent control payload such as encrypted close

The v1 record envelope has no extra type byte. Record meaning is determined by
the current protocol state.

### Size Limits

V1 uses these limits:

- `max_record_payload_size = 65535`
- `max_noise_handshake_message_size = 65535`
- `max_noise_transport_ciphertext_size = 65535`
- `max_application_plaintext_size = 65519`

`max_application_plaintext_size` leaves room for the Noise transport AEAD tag
inside one record and assumes no extra app envelope overhead beyond the future
message family encoding.

## Prologue

The Noise prologue binds stable context known before the handshake. V1 derives
it from the service descriptor, not from the carrier.

V1 prologue fields:

- protocol id
- environment id
- service id
- service authority

The prologue must not include `transport=quic` or `transport=wss`.

## Server-Key Authorization

### Placement

The responder's Noise handshake payload is the server-key authorization object.
It is carried inside the responder's Noise handshake message, not as framed
sidecar metadata.

### Required Fields

The serialized `server_key_authorization_v1` payload must cover at least:

- server Noise public key
- key identifier
- not-before timestamp
- not-after timestamp
- environment id
- service id
- service authority
- protocol id or compatibility marker
- trust-anchor signature authorizing the above fields

### Validation Rules

The client must reject the connection if:

- the trust-anchor signature is invalid
- the presented server Noise key does not match the authorized key in the
  payload
- the authorization is expired or not yet valid
- the environment id, service id, or service authority do not match the
  expected prologue context

Trust validation completes before the session may enter `Secure Ready`.

## Connection State Machine

### State 0: Carrier Ready

Preconditions:

- the chosen carrier is established
- the carrier-specific selector value is confirmed
  - `QUIC` ALPN for `QUIC`
  - WebSocket subprotocol for `WSS`

No application credentials or device proofs may be sent here.

### State 1: Noise Handshake In Progress

The client starts the `NX` initiator handshake and sends one Noise handshake
record. The server replies with one responder handshake record whose handshake
payload contains `server_key_authorization_v1`.

The client validates:

- Noise message processing succeeded
- server-key authorization succeeded
- prologue-bound identity fields match expectation

If validation fails, the connection must be aborted without entering transport
mode and without fallback on a second carrier.

### State 2: Secure Ready

The handshake is complete and both peers have entered Noise transport mode.

From this point onward:

- login is allowed
- account session recovery is allowed
- all sensitive application messages must be encrypted inside Noise transport

### State 3: Account Authenticated

The client has completed either:

- `Account Authenticated (fresh)`, or
- `Account Authenticated (resumed)`

inside Noise transport.

This state identifies the account but does not yet imply known-device status.

### State 4: Known Device Authenticated

The connection is bound both to an authenticated account or resume context and
to successful proof of an enrolled device key.

### State 5: Closing

One peer sends the encrypted close message, optionally waits for ack or drain
policy, then closes the outer carrier.

## Shared Message Flow

### Flow A: Initial Secure Connection

1. Select and establish the outer carrier.
2. Confirm the expected carrier selector.
3. Exchange Noise handshake records.
4. Validate `server_key_authorization_v1`.
5. Enter Noise transport mode and reach `Secure Ready`.

### Flow B: Login

1. Client sends encrypted login or session-recovery message.
2. Server validates it and returns encrypted account or session result.
3. Connection enters `Account Authenticated`.

### Flow C: Device Enrollment

Device enrollment is defined in
`backlog/docs/v1-device-enrollment-and-known-device-policy.md`. The protocol
requirements here are:

- enrollment occurs only after `Account Authenticated (fresh)` unless an
  explicit step-up policy says otherwise
- the `Account Authenticated (fresh)` privilege must still be inside its active
  server-defined freshness window
- the enrollment proof is signed by the candidate device private signing key
  that the client wants to register
- the enrollment proof binds the current handshake hash `h`
- optional attestation evidence travels as part of the encrypted enrollment
  finish payload, not as carrier metadata

### Flow D: Known-Device Reauthentication

Known-device reauthentication is also defined in the device-policy doc. The
protocol requirements here are:

- the account or resume context is pinned before challenge issuance
- the device proof is signed by the already enrolled device private signing key
  bound to the presented `device_id`
- the device proof binds the current handshake hash `h`
- only after successful proof may the session reach `Known Device Authenticated`

## Replay And Early-Data Rules

V1 replay posture is intentionally conservative.

Rules:

- no login data in handshake payloads
- no device proofs in handshake payloads
- no Noise early application data
- no `QUIC` `0-RTT`
- enrollment challenges must be fresh and replay-detectable
- known-device challenges must be fresh and replay-detectable
- a proof from one Noise session must be invalid on another session

## Encrypted Application Message Families

V1 does not yet freeze the final binary schema, but message families must
include at least:

- `login_request`
- `login_result`
- `device_enroll_start`
- `device_enroll_challenge`
- `device_enroll_finish`
- `device_auth_start`
- `device_auth_challenge`
- `device_auth_finish`
- `close`

## Large Payload Transfer

The v1 secure-channel core is message-oriented even when the selected carrier
is a `QUIC` byte stream.

That means:

- one encrypted application message must fit within
  `max_application_plaintext_size`
- the protocol does not expose a special raw byte-stream bypass around Noise
  framing
- large application objects such as images, archives, or other binary payloads
  must be transferred as an app-layer sequence of chunked encrypted messages

For large-object transfer, the application layer should define a chunked family
such as:

- `blob_open` or equivalent metadata message with transfer identifier,
  content-type hint, expected size, and optional digest
- `blob_chunk` messages carrying ordered byte ranges that each fit inside one
  encrypted application message
- `blob_finish` carrying final size, final digest, or other end-of-transfer
  confirmation

The receiver should validate transfer ordering, byte counts, and any declared
digest or integrity trailer. Carrier choice does not change this rule: large
payload handling stays above Noise transport on both `QUIC` and `WSS`.

## QUIC Binding

### Endpoint And ALPN

The client uses the `quic` carrier entry from the service descriptor and must
offer the expected `QUIC` ALPN.

### Stream Rule

V1 uses one dedicated client-initiated bidirectional stream for the secure
channel.

Rules:

- the inner protocol must not span multiple `QUIC` streams in v1
- `QUIC` DATAGRAM is out of scope
- `QUIC` `0-RTT` is out of scope

### Record Mapping

On the `QUIC` stream, each framed record is encoded as:

- `u16be payload_length`
- `payload`

Records are concatenated on the stream in order.

### Carrier Errors

`QUIC` connection or stream closure before encrypted close is an abrupt
transport failure. Detailed fallback rules are defined in the transport policy
doc.

## WSS Binding

### Endpoint And Subprotocol

The client uses the `wss` carrier entry from the service descriptor and must
request the expected WebSocket subprotocol.

### Record Mapping

The `WSS` binding maps one framed record payload to one binary WebSocket
message.

Rules:

- text WebSocket messages are invalid
- each binary WebSocket message contains exactly one framed record payload
- WebSocket fragmentation is ignored by the secure-channel core; only the
  reassembled message matters
- ping, pong, and carrier keepalive stay in the `WSS` binding, not in the
  secure-channel core

### Carrier Errors

Outer `WSS` closure before encrypted close is an abrupt transport failure. It
is not a graceful secure-channel shutdown by itself.

## Encrypted Shutdown

V1 requires an explicit encrypted close message before normal carrier shutdown.

Minimum behavior:

1. send encrypted `close`
2. optionally wait for peer `close` or drain acknowledgment according to local
   implementation policy
3. close the outer carrier

## Minimum Validation Invariants

The first implementation slices must verify at least:

- a connection cannot reach `Secure Ready` without successful server-key
  authorization
- login messages cannot be processed before Noise transport mode
- device enrollment cannot happen before `Account Authenticated (fresh)` unless
  explicit step-up policy says otherwise
- enrollment must be rejected or downgraded to step-up-required when the
  server-defined fresh-auth window has expired
- known-device proof cannot complete before reconnect context is pinned
- a proof from one Noise session is rejected on another session
- missing or wrong carrier selector prevents protocol start
- outer carrier close before encrypted close is treated as abrupt termination

## Relationship To Historical Docs

The archived
`backlog/docs/historical/2026-03-14_v1-core-protocol-and-wss-binding.md` doc
remains a useful baseline for the earlier WSS-first phase, but this document
is the active normative successor for v1 implementation work.
