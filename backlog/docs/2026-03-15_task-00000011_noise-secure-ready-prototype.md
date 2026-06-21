# 2026-03-15 Task 00000011 Noise Secure-Ready Prototype

## Final Summary

Task `00000011` completed the first `Secure Ready` prototype. Task `00000020`
later replaced the active v1 handshake shape with descriptor-pinned service
static public key authorization and `Noise_NK1_25519_ChaChaPoly_BLAKE2s`. The
task-11 review cycle tightened the selector seam so the evaluator consumes the
same descriptor and clock context the selector planned against.

## Summary

Working note for task `00000011`. Implement the first transport-neutral
server-authenticated handshake path in `crates/core`, expose the final
handshake hash `h`, and prove encrypted close over the shared framed transport
seam.

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

> Update: task `00000020` replaced the active v1 inner-channel shape with
> `Noise_NK1_25519_ChaChaPoly_BLAKE2s` and a descriptor-pinned service static
> public key. The notes below describe the completed task-11 prototype history.

- `crates/core/src/descriptor.rs` now derives a canonical length-prefixed Noise
  prologue directly from the descriptor fields locked by the active v1 docs.
- `crates/core/src/trust.rs` now keeps trust-anchor parsing for descriptor
  validation; active service-key authorization is descriptor-pinned in task 20.
- `crates/core/src/noise.rs` now implements `SnowNk1ClientEvaluator`, exposes the
  final handshake hash `h` as both `handshake_hash` and `channel_binding`, and
  returns a `NoiseFramedDuplex` wrapper that encrypts application records and
  sends an encrypted close before outer shutdown.
- `crates/core/src/noise.rs` tests now prove:
  - secure-ready success with usable post-handshake transport
  - rejection of a bad service static public key or unexpected handshake payload
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
