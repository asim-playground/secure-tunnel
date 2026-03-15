---
status: active
normative: false
supersedes: []
superseded_by: []
---

# V1 Glossary

## Carrier

The outer transport used to reach the service, such as raw `QUIC` over UDP or
`WSS` over TLS.

## Framed Record

The logical message unit exposed to the secure-channel core. A framed record
carries exactly one Noise handshake message, one Noise transport ciphertext, or
one carrier-independent control payload such as the encrypted close message.

## Secure Ready

The state reached only after the outer carrier is established, the framed
record channel is available, the Noise handshake completes successfully, the
server-key authorization validates, and both peers enter Noise transport mode.

## Service Identity

The logical backend identity authenticated by the inner channel. In v1 it is
the combination of service identifier, environment identifier, and authorized
server Noise static key.

## Service Descriptor

The bootstrap configuration object that tells the client which logical service
it is connecting to, which trust anchors to use, and which carrier endpoints
are available for that service.

## Known Device

A previously enrolled app or device installation whose private device key is
still trusted by the server and whose current session has successfully
completed the known-device proof flow.

## Enrollment

The post-login process that registers a new device key under an account.
Enrollment is distinct from reconnect-time proof by an already enrolled device.

## Pinned Reconnect Context

The server-validated account and session context that is fixed before the
server sends a known-device challenge. The client must sign that pinned context
instead of loosely restating account claims.

## Handshake Hash `h`

The final Noise handshake hash from the completed handshake. V1 uses `h` as a
channel-binding value for later login, enrollment, and known-device proof
logic.

## Account Authenticated (Fresh)

An account state reached by primary authentication or an equivalent explicit
step-up inside the current secure session.

## Account Authenticated (Resumed)

An account state reached by restoring or refreshing prior server-managed session
state inside the current secure session without a fresh primary factor.
