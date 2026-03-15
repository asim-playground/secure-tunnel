---
status: active
normative: false
supersedes: []
superseded_by: []
---

# 2026-03-15 Starter Crate Recommendations

## Final Summary

`task-00000001` now has an implementation-facing starter stack for the Phase 2
prototype path: `tokio` for async runtime, `rustls` plus
`rustls-platform-verifier` for outer TLS trust, `quinn` for `QUIC`,
`tokio-tungstenite` plus `tokio-rustls` for `WSS`, `snow` for the inner Noise
channel, and hand-written record codecs over `bytes` instead of a generic wire
serializer.

The decisions that need to be effectively locked before `task-00000005`,
`task-00000007`, and `task-00000008` are now explicit: use Tokio across
network-facing crates, keep the secure-channel core transport-agnostic, prefer
`quinn` with `WSS` fallback rather than a broader transport abstraction
framework, and keep critical wire framing small and explicit rather than
`bincode`-style serialization.

## Checklist

- [x] Review the active transport-policy and protocol docs for crate-sensitive
      architectural seams.
- [x] Produce a shortlist covering runtime, TLS, `QUIC`, `WSS`, Noise, wire
      framing, config, signatures, errors, observability, and tests.
- [x] Record the tradeoffs that decide first-choice selections versus reserve
      alternatives.
- [x] Mark which decisions must be locked before crate-boundary and protocol
      API work proceeds.

## Constraints From The Active V1 Docs

- The secure-channel core starts at the framed-record layer and must not depend
  on concrete carrier APIs.
- V1 prefers raw `QUIC` with `WSS` fallback but keeps one inner Noise-based
  protocol and one post-handshake auth model.
- V1 intentionally avoids `QUIC` `0-RTT`, `QUIC` DATAGRAM, and handshake-time
  client-static authentication.
- Fallback classification depends on keeping outer-carrier failures distinct
  from Noise/trust failures, so transport bindings need narrow and explicit
  seams.

## Decision Criteria

The first implementation stack should optimize for:

- maintenance health and ecosystem adoption
- auditability of the security boundary
- clean fit with Tokio-based async Rust
- compatibility with a UDP-first `QUIC` path plus a boring `WSS` fallback
- ability to keep transport code out of the secure-channel core
- straightforward local testing with generated certificates and framed I/O

The strongest repository-specific constraint is that the wire protocol is a
security boundary. That makes explicit codecs over `bytes` easier to reason
about and fuzz than a generic serializer that hides layout decisions behind
derive macros.

## Recommended Starter Stack

| Concern | First Choice | Why This Is The Default |
|---|---|---|
| Async runtime | `tokio` | Common runtime for `quinn`, `tokio-rustls`, and `tokio-tungstenite`; keeps transport crates aligned. |
| Outer TLS | `rustls` | Modern Rust-native TLS with a good fit for both `QUIC` and `WSS` paths. |
| Client trust for outer TLS | `rustls-platform-verifier` | Best default for app clients that should follow platform trust stores and enterprise roots. |
| `QUIC` transport | `quinn` | Clean Tokio integration and a direct mapping to the repo's one-bidi-stream v1 design. |
| `WSS` transport | `tokio-tungstenite` | Straightforward Tokio binding for WebSockets without forcing a larger framework. |
| `WSS` server TLS stream | `tokio-rustls` | Pairs naturally with `tokio-tungstenite` server accept flows. |
| Inner secure channel | `snow` | Matches the required `HandshakeState -> TransportState` flow and exposes handshake hash for channel binding. |
| Wire framing | `bytes` + hand-written codecs | Keeps record boundaries explicit and security-sensitive payloads small, typed, and fuzzable. |
| Local config | `serde` + `toml` | Good fit for repo-local descriptors and config without putting Serde on the critical wire path. |
| Optional fixture/debug formats | `serde_json` | Useful for fixtures or external descriptors, but not required on the secure wire. |
| Root/server-key authorization signatures | `ed25519-dalek` | Compact, well-understood fit for an app-shipped trust anchor authorizing rotating server Noise keys. |
| Device-proof verification | `p256` | Matches the iOS Secure Enclave / CryptoKit P-256 path on the verification side. |
| Library errors | `thiserror` | Enough structure for library crates without overcomplicating public error types. |
| App/test errors | `anyhow` | Useful in harnesses, CLI flows, and tests where ergonomic context matters more than stable APIs. |
| Secret handling | `zeroize`, `secrecy` | Reasonable minimum hygiene for long-lived secrets and buffers. |
| Observability | `tracing`, `tracing-subscriber` | Aligns with async Rust instrumentation and transport fallback diagnostics. |
| Test support | `rcgen`, `proptest` | Covers local certificate generation plus parser/state-machine property testing. |
| Fuzz support | `arbitrary`, `cargo-fuzz` | Reserve for the framing/trust-validation code as soon as byte-driven parsers exist. |

## Decisions To Lock Now

These choices should be treated as inputs to `task-00000005`,
`task-00000007`, and `task-00000008`, not as late implementation details:

- Standardize on `tokio` for network-facing Rust crates.
- Use `quinn` for the first `QUIC` binding and `tokio-tungstenite` for the
  first `WSS` binding.
- Keep the secure-channel core centered on `snow` plus explicit `bytes`-based
  framing.
- Use `rustls-platform-verifier` for client-side outer TLS verification by
  default.
- Keep wire serialization for the secure-channel protocol hand-written and
  transport-neutral instead of adopting `bincode` or another generic serializer.

Without these decisions, the later crate-boundary work risks drifting into a
WebSocket-shaped core, a transport abstraction with more surface area than the
repo needs, or an opaque wire format that is harder to audit.

## Decisions That Can Stay Provisional

- Whether a small helper such as `trait-variant` or `async-trait` is worth
  using for transport traits. Start without either unless `task-00000005`
  exposes a concrete ergonomics problem.
- Whether `serde_json` belongs in production dependencies or only in tests and
  tooling.
- Whether `tracing-subscriber` stays only in binaries/test harnesses while
  library crates emit bare `tracing` events.
- Whether fuzzing support lands during `task-00000010` or immediately after the
  first framed parser exists.

## Explicit Non-Recommendations For V1

- Do not use `bincode` on the critical wire path.
- Do not adopt a generic middleware or request/response transport framework.
- Do not force the secure-channel core to depend on WebSocket, Quinn, or Tokio
  concrete types.
- Do not use the current TLV work as the v1 critical-path codec until the spec,
  implementation, and derive layer converge.

## Reserve Alternatives

Keep these as fallback options rather than first picks:

- `s2n-quic` if `quinn` later proves to be an operational mismatch.
- `fastwebsockets` if `tokio-tungstenite` becomes a correctness or performance
  bottleneck and lower-level frame control becomes necessary.
- `webpki-roots` only for controlled environments where static embedded roots
  are a deliberate choice instead of platform trust.

## Proposed Workspace Dependency Map

This is a planning map for future crate work, not a request to add all
dependencies immediately to the root manifest.

- `secure-tunnel-core`: `snow`, `bytes`, `ed25519-dalek`, `p256`,
  `thiserror`, `zeroize`, `secrecy`
- future transport layer crate: `tokio`, `tracing`
- future `QUIC` binding crate: `quinn`, `rustls`, `rustls-platform-verifier`
- future `WSS` binding crate: `tokio-tungstenite`, `tokio-rustls`, `rustls`,
  `rustls-platform-verifier`
- config/tooling crates: `serde`, `toml`, optional `serde_json`
- test kit or integration harnesses: `rcgen`, `proptest`, `arbitrary`,
  `anyhow`

## Next Actions

- Use this stack as an input to `task-00000005` when defining crate boundaries
  and the first secure-channel API seams.
- Keep actual `Cargo.toml` edits minimal until the architecture task decides
  which crates should exist in the workspace versus stay deferred.
- Add fuzz targets once the first framed record parser and
  `server_key_authorization_v1` codec exist.
