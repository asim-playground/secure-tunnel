# 2026-03-15 Task 00000011 Noise Secure-Ready Prototype

## Final Summary

Task `00000011` is complete. `crates/core` now drives the v1 `NX` handshake
over `FramedDuplex`, verifies the responder's signed
`server_key_authorization_v1` payload against shipped Ed25519 trust anchors and
the authenticated remote static key, exports the final handshake hash `h`, and
wraps the surviving carrier transport in Noise transport mode for encrypted
records and encrypted close. The review cycle also tightened the selector
seam so the evaluator now consumes the same descriptor and clock context the
selector planned against, normalizes handshake-time `QUIC` close into the
documented fallback class, and validates the descriptor before direct
handshake use.

## Summary

Working note for task `00000011`. Implement the first transport-neutral
server-authenticated `NX` handshake path in `crates/core`, verify the
responder's signed server-key authorization payload against shipped trust
anchors, expose the final handshake hash `h`, and prove encrypted close over
the shared framed transport seam.

## Working Checklist

- [x] Add the minimum `core` dependencies for Noise, small codecs, trust-key
  parsing, and signature verification.
- [x] Implement canonical descriptor-derived Noise prologue encoding.
- [x] Implement `server_key_authorization_v1` codec and trust verification.
- [x] Implement a concrete `SecureReadyEvaluator` backed by `snow` and wrap the
  surviving transport in Noise transport mode.
- [x] Add a transport-neutral scripted responder harness and focused tests for
  secure-ready success, inner trust failure, selector no-fallback behavior, and
  encrypted close.
- [x] Run local validation and record evidence.

## Evidence And Conclusions

- `crates/core/src/descriptor.rs` now derives a canonical length-prefixed Noise
  prologue directly from the descriptor fields locked by the active v1 docs.
- `crates/core/src/trust.rs` now defines a minimal
  `server_key_authorization_v1` codec and verifies the signed payload against
  shipped Ed25519 trust anchors, validity windows, descriptor-bound identity
  fields, and the authenticated responder static key returned by the Noise
  handshake.
- `crates/core/src/noise.rs` now implements `SnowNxClientEvaluator`, exposes the
  final handshake hash `h` as both `handshake_hash` and `channel_binding`, and
  returns a `NoiseFramedDuplex` wrapper that encrypts application records and
  sends an encrypted close before outer shutdown.
- `crates/core/src/noise.rs` tests now prove:
  - secure-ready success with usable post-handshake transport
  - rejection of a bad server-key authorization payload
  - rejection of a descriptor with the wrong Noise suite identifier
  - no `WSS` fallback after inner trust failure with the real evaluator
  - `WSS` fallback after handshake-time `QUIC` close normalized from
    `TransportClosed`
  - encrypted close delivery before outer close
- Validation:
  - `cargo test -p secure-tunnel-core`
  - `mise run dev`
- Independent review:
  - initial reviewer findings around evaluator context, fallback normalization,
    and prologue coverage were resolved before closure
  - final reviewer pass reported no findings in the scoped diff

## Next Actions

- Task `00000012` can now consume the real secure-ready evaluator and Noise
  transport wrapper while adding concrete `QUIC` and `WSS` adapters.
