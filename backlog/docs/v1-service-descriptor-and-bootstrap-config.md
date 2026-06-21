---
status: active
normative: true
supersedes: []
superseded_by: []
---

# V1 Service Descriptor And Bootstrap Config

## Summary

V1 uses one logical service descriptor with multiple carrier entries. The
service is one thing; `QUIC` and `WSS` are two ways to reach it.

This descriptor is the bootstrap source of truth for:

- logical service identity
- environment identifier
- trusted root keys
- expected protocol identifiers
- available carrier targets
- transport-selection policy inputs

## Core Decision

The first public bootstrap shape is one logical descriptor with per-carrier
targets, not separate unrelated configs for `QUIC` and `WSS`.

That keeps:

- one inner trust model
- one prologue context
- one service identity
- one transport-selection policy surface

## Required Fields

The descriptor must carry at least:

- `descriptor_version`
- `descriptor_serial`
- `not_before`
- `not_after`
- `environment_id`
- `service_id`
- `service_authority`
- `protocol_id`
- `noise_suite`
- `service_static_public_key`
- `signed_descriptor_hash`
- `descriptor_signature`
- one or more trust anchors used to authorize signed descriptor updates
- a `carriers` section with optional `quic` and `wss` entries
- selection policy flags such as `preferred_carrier=quic` and
  `allow_wss_fallback=true`

`descriptor_version` identifies the schema. `descriptor_serial` identifies the
monotonic instance version for one logical service and environment.

## Carrier Entries

### `quic`

The `quic` entry must define at least:

- `connect_host`
- `port`
- `alpn`

It may also define:

- explicit outer authority or SNI override if routing requires it
- initial path or endpoint metadata needed by the client transport adapter

### `wss`

The `wss` entry must define at least:

- `url`
- `subprotocol`

It may also define:

- explicit outer authority override if the URL host is not the logical service
  authority

## Identity Semantics

### Service Identity

`service_id`, `environment_id`, and the authorized service Noise static public
key identify the inner service.

### Service Authority

`service_authority` is the application-level authority string bound into the
Noise prologue and device-proof signing bytes.

It is the stable inner identity value, not simply "whatever hostname the outer
carrier happened to use."

### Carrier Routing Names

Carrier-specific routing names may differ from `service_authority`, but only
when the descriptor says so explicitly.

If `connect_host`, URL host, or outer SNI/Host differ from
`service_authority`, the descriptor must contain that mapping. Implementations
must not infer identity equivalence from ad hoc hostname conventions.

## Trust Anchors

The client must know the expected trust anchors before first connect.

V1 allows two bootstrap sources:

- trust anchors and descriptors shipped in the app bundle
- later descriptor updates signed by an already trusted root

The v1 implementation supports a pinned-root descriptor path directly. The
descriptor carries the service Noise static public key as public identity
material, a canonical descriptor-body hash in `signed_descriptor_hash`, and a
root Ed25519 `descriptor_signature` over that hash. The client must already
have the expected descriptor root and service static public-key pin; it must not
accept a key merely because a network-provided descriptor contains it.

The service Noise static public key is not secret. Client builds may still
obfuscate an embedded pin to defeat casual strings/raw-byte extraction, but the
authenticity boundary is the pinned root plus pinned public key, not the
obfuscation. The service Noise static private key remains server-only.

V1 does not allow "learn the service identity from the network first and trust
it later."

## Descriptor Freshness And Anti-Rollback

Signed descriptor updates must include:

- a monotonic `descriptor_serial`
- a validity window defined by `not_before` and `not_after`

For one `(environment_id, service_id)` pair, the client must persist the
highest accepted `descriptor_serial` and reject any later update with a lower
serial unless an explicit operator reset or app reinstall policy says
otherwise.

The client must also reject descriptor updates that are outside their validity
window. This prevents replay of previously valid signed descriptors that would
reintroduce retired endpoints, trust anchors, or fallback posture.

The current Rust cache surface stores this as
`TransportCacheSnapshot.highest_descriptor_serial` for the service/environment
cache key.

## Prologue Inputs

The Noise prologue must be derived from the descriptor, not from carrier-local
runtime guesses.

V1 prologue fields are:

- domain label `secure-tunnel-inner-prologue-v1\0`
- product label `secure-tunnel`
- inner protocol version `1`
- `service_id`
- `environment_id`
- `service_authority`
- `signed_descriptor_hash`
- `noise_suite`

Carrier choice does not change these fields.

## Selection Inputs

The transport selector consumes the descriptor as follows:

- carrier order from the selection policy
- `QUIC` endpoint and ALPN from the `quic` entry
- `WSS` URI and subprotocol from the `wss` entry
- cache key fields from `environment_id` and `service_id`

## Example Shape

```yaml
descriptor_version: 1
descriptor_serial: 7
not_before: 2026-03-15T00:00:00Z
not_after: 2026-06-15T00:00:00Z
environment_id: prod
service_id: secure-tunnel-api
service_authority: api.example.com
protocol_id: secure-tunnel-v1
noise_suite: Noise_NK1_25519_ChaChaPoly_BLAKE2s
service_static_public_key: <base64-32-byte-x25519-public-key>
signed_descriptor_hash: <base64-32-byte-descriptor-hash>
descriptor_signature:
  key_id: root-2026-01
  algorithm: ed25519
  signature: <base64-64-byte-signature>
trust_anchors:
  - key_id: root-2026-01
    algorithm: ed25519
    public_key: <base64>
selection_policy:
  preferred_carrier: quic
  allow_wss_fallback: true
carriers:
  quic:
    connect_host: api.example.com
    port: 443
    alpn: secure-tunnel-v1
  wss:
    url: wss://api.example.com/tunnel/v1
    subprotocol: secure-tunnel-v1
```

## Embedded Public-Key Obfuscation

Rust client builds reconstruct the default pinned service static public key from
`crates/core/build.rs` generated bytes. The build script reads an optional
`SECURE_TUNNEL_SERVICE_STATIC_PUBLIC_KEY_FILE` containing exactly 32 raw bytes,
or falls back to the local example public key. It emits masked/permuted byte
arrays plus a small atomic-load decoder with local control-flow noise.

This has one purpose: the final binary should not contain the key as ASCII,
hex, base64, PEM, or as the exact contiguous 32-byte value in ordinary static
scans. It is not a secrecy mechanism. A determined reverser can recover the key
at runtime.

Release validation should include a scan for:

- obvious text encodings of the public key or service identity
- the exact 32-byte public key sequence in the built binary
- accidental reintroduction of string-based pins

## Operational Consequences

- Deployment can front one logical service with multiple carrier adapters
  without changing inner trust semantics.
- The bootstrap surface remains small enough to ship in the app or sign as a
  compact update.
- Architecture and transport-selection work now have one concrete config object
  to target instead of ad hoc per-carrier settings.
