# Task `00000020` - `implement account and device session protocol`

## Summary

Implement the service-static-key Noise identity model plus the post-Noise
account session and known-device protocol needed after the channel reaches
`Secure Ready`.

## Motivation

The current core exposes secure-ready artifacts and session phases, but the
inner channel still needs to prefer the service static public key model used by
Codesuper, and the account login, known-device challenge, channel-binding use,
and replay/freshness rules are still mostly documented behavior. The product
SDK needs these flows before it can represent a complete secure tunnel session.

## Read-Write Repository

- Primary read-write repository: `/Users/asimi/workplace/secure-tunnel`
- Secondary read-write repository/repositories (if applicable): none
- State explicitly:
  - code changes are expected to land in Rust core/session modules and tests.
  - this repository itself is expected to change.

## Read-Only Reference Repository

- Read-only reference repository/repositories:
  - `/Users/asimi/workplace/codesuper`
- State explicitly which repositories may be inspected only for reference or
  legacy behavior:
  - Codesuper secure tunnel docs and core implementation may be inspected for
    protocol shape, canonical bytes, Noise service static key handling, and
    fixture strategy only. Do not modify Codesuper for this task.

## Detailed Requirements / Acceptance Criteria

### A) Session protocol is implemented

- [x] Prefer a service Noise static public key model for Secure Ready, with the
      service private key remaining server-only.
- [x] Bind stable service/application context into canonical Noise prologue
      bytes before account or device protocol messages are accepted.
- [x] Implement the first account session open/login message flow above Noise
      transport mode.
- [x] Implement known-device challenge/response using the documented channel
      binding and freshness rules.
- [x] Preserve the documented distinction between new-device enrollment and
      returning-device reauthentication.

### B) Security invariants are tested

- [x] Tests reject wrong service static public key authorization and mismatched
      signed descriptor/prologue context.
- [x] Tests reject wrong service/environment binding where applicable.
- [x] Tests reject stale or replayed device challenge material.
- [x] Tests prove channel-binding material is included where the docs require
      it.
- [x] Tests prove fallback attempts do not replay the prior Noise first
      message, and early/handshake-payload application data is rejected.

### C) SDK-facing state remains simple

- [x] Expose coarse session phases and errors suitable for the facade from
      `task-00000018`.
- [x] Avoid leaking low-level protocol message internals into the SDK facade
      unless this task records a deliberate exception.
- [x] `mise run dev` passes.

## Cross-Repo Boundaries

- Primary implementation boundary: Rust session protocol code.
- Parser / upstream dependency boundary: no parser work expected.
- Downstream integration boundary: native bindings consume this later through
  the SDK facade.
- External asset / catalog / fixture boundary: local cryptographic fixtures
  only.
- If another repository is read-write, state what is implemented there versus
  what is implemented in this repository.
  - none.

## Task Dependencies

- backlog/tasks/completed/task-00000006_define-device-enrollment-and-known-device-policy.md
- backlog/tasks/completed/task-00000011_prototype-server-auth-noise-handshake-and-trust-verification-on-transport-neutral-frames.md
- backlog/tasks/completed/task-00000018_define-product-sdk-facade-and-session-contract.md
- backlog/plans/plan-00000002_product-secure-tunnel-sdk-and-bindings.md

## Reference Tasks

- backlog/docs/v1-device-enrollment-and-known-device-policy.md
- backlog/docs/v1-core-protocol-quic-and-wss-bindings.md
- /Users/asimi/workplace/codesuper/backlog/docs/2026-06-14_secure-ingress-noise-channel-decision.md
- /Users/asimi/workplace/codesuper/docs/secure-tunnel-canonical-encoding.md
- /Users/asimi/workplace/codesuper/crates/core/src/secure_tunnel/handshake.rs
- /Users/asimi/workplace/codesuper/backlog/tasks/completed/task-00000072_add-secure-tunnel-inner-noise-channel-and-device-proof.md

## Backlog Bookkeeping

- `backlog/` is local planning state and is not version controlled by default.
- Moving a task to `backlog/tasks/completed/` is separate from pushing code.
- Commit history for work tracked by this task may live in a different
  read-write repository than this backlog entry.

## Implementation Notes

- [x] Implementation notes added with command evidence.

Implemented the v1 service-static inner channel model by changing the active
Noise suite to `Noise_NK1_25519_ChaChaPoly_BLAKE2s`, adding descriptor fields
for `service_static_public_key`, `signed_descriptor_hash`, and
`descriptor_signature`, and binding product, protocol version, service id,
environment id, authority, descriptor hash, and suite into canonical prologue
bytes.

Review fix: descriptor service-static-key authorization is no longer
self-asserted JSON. `SnowNk1ClientEvaluator` verifies descriptor freshness,
canonical descriptor hash, root Ed25519 descriptor signature, cached serial
rollback state, and a pinned service static public key before accepting the
`NK1` handshake. `TransportCacheSnapshot` now carries
`highest_descriptor_serial` for local anti-rollback state.

Review fix: the SDK now authorizes descriptors before planning or dialing any
descriptor-controlled endpoint. `ClientConfig` carries descriptor roots and
service static public-key pins, `SecureTunnelClient::connect()` verifies
descriptor signature/freshness and static-key pins before selector access, and
production ports receive the same trust material. Regression tests assert that
tampered descriptors and unpinned service keys do not touch any connector.

Added custom build-time public-key obfuscation for the default pinned service
static public key. `crates/core/build.rs` reads an optional
`SECURE_TUNNEL_SERVICE_STATIC_PUBLIC_KEY_FILE` containing exactly 32 raw bytes,
or falls back to the example key, and emits masked/permuted atomic byte arrays
plus a small `#[inline(never)]` decoder with local control-flow noise. This is
documented as obfuscation only, not a secrecy boundary.

Added `secure-tunnel-core` protocol modules for the binary application envelope,
account auth request/result messages, and device auth/enrollment challenge,
proof, result, canonical signing bytes, purpose codes, freshness checks, and
Ed25519 proof verification. Added checked fixture vectors in
`crates/core/tests/fixtures/secure_tunnel_v1_vectors.json` for the canonical
prologue and both device-proof purpose byte strings.

Updated `secure-tunnel-sdk` with account authentication, known-device auth, and
new-device enrollment methods on `SecureTunnelSession`. The SDK surface keeps
wire messages internal and exposes owned account/device request, challenge, and
report records; canonical bytes are exposed only where platform key storage must
sign them.

Review fix: known-device auth and enrollment now reject service results whose
`device_key_id` does not match the pending proof challenge, and tests cover both
mismatch paths.

Updated active protocol, descriptor, threat-model, transport, device-policy,
glossary, and historical task docs to prefer the service static key / NK1 model.
Remaining `Noise_NX` references are confined to historical docs.

Verification:

- `cargo fmt --all -- --check` passed.
- `cargo test -p secure-tunnel-core -p secure-tunnel-sdk -p secure-tunnel-transport`
  passed: 43 core, 16 SDK, and 11 transport tests.
- `cargo clippy --workspace --all-targets --all-features --no-deps -- -D warnings`
  passed.
- `cargo doc -p secure-tunnel-core --no-deps` passed.
- `cargo doc -p secure-tunnel-sdk --no-deps` passed.
- `cargo build -p secure-tunnel-cli --release` passed; a binary scan of
  `target/release/secure-tunnel-cli` found no occurrence of the example service
  static public key as raw 32-byte value, lowercase hex, uppercase hex, or
  base64.
- `mise run dev` passed, including format, lint, Rust nextest with 78 tests,
  Python package/tests with 5 pytest tests, Go tests, and Go-WASM tests.
- `wc -l $(rg --files crates | rg '\.rs$') | sort -nr | head -20` showed all
  non-Markdown code files at or below 500 lines; the largest are
  `crates/core/src/selector.rs` and `crates/core/src/device_session.rs` at
  exactly 500 lines.

## Implementation Plan

### Plan Decisions

- Prefer the Codesuper secure-ingress model for the v1 inner channel: the
  client knows or can authorize the service Noise static public key before
  completing Secure Ready, and the service private key stays server-only.
- Treat the current `Noise_NX_25519_ChaChaPoly_BLAKE2s` descriptor/handshake
  shape as interim. Task 20 should start by realigning the protocol docs and
  core handshake toward `Noise_NK1_25519_ChaChaPoly_BLAKE2s` or a reviewed
  equivalent supported by the active Rust Noise library.
- The service static public key is public trust material, not a credential. It
  may be bundled directly when acceptable for the target deployment, or
  authorized through descriptor/trust-anchor metadata. Never bundle symmetric
  secrets, service private keys, bearer tokens, account credentials, or global
  client private keys.
- Keep client, account, and device identity out of the Noise handshake. Known
  device reauth and new-device enrollment happen as application-protocol
  control flows after Secure Ready over encrypted records.
- Bind stable application context into the Noise prologue using canonical
  bytes: product, inner protocol version, service id, environment id, service
  authority, signed descriptor hash, and allowed Noise suite. Do not bind the
  selected carrier because QUIC/WSS fallback must preserve the same service
  identity semantics.
- Use secure-tunnel domain labels for canonical bytes, for example
  `secure-tunnel-inner-prologue-v1\0` and
  `secure-tunnel-device-proof-v1\0`. Encoding should use fixed field order,
  big-endian integers, `u16` length-prefixed UTF-8 strings, raw fixed-width
  hashes/challenges/public keys, and no JSON/default/field-name material in
  signed bytes.
- Preserve the existing binary application envelope direction for
  `secure-tunnel`, but keep the Codesuper separation: encrypted control records
  are the application protocol, while proof/signature bytes are a separate
  canonical contract that can be verified identically from Swift, Kotlin,
  Python, Dart, and Go.
- Device-proof signed bytes should bind the Noise handshake hash, server
  challenge, service/environment/authority, signed descriptor hash, allowed
  Noise suite, account context hash, device key id, purpose, and expiry. Use
  explicit purpose codes, with known-device reauth and new-device enrollment as
  distinct values.
- Descriptor authorization must not trust `service_static_public_key` merely
  because it appears in JSON. A client must verify the canonical descriptor
  hash, root Ed25519 signature, validity window, cached serial rollback state,
  and a pinned service static public key before `Secure Ready`.
- The service static public key is public identity material, not a secret. Use
  build-time byte obfuscation for embedded pins to defeat casual string and raw
  byte scans, but keep authentication security in signatures and pins.
- Every carrier attempt starts fresh Noise state and fresh ephemeral keys.
  Fallback can retry only before Secure Ready and must not replay a previous
  first Noise message. Early data, handshake-payload application requests, QUIC
  0-RTT, and TLS early data remain disabled/ignored.
- Outer carrier metadata must not carry account IDs, session tokens, bearer
  credentials, device IDs, tenant secrets, or sensitive routing hints. Account
  and device routing decisions must come from encrypted application records
  after Secure Ready.
- The account/device protocol still lives above Noise transport mode and should
  avoid carrier-specific behavior. The one deliberate exception is
  inner-channel identity alignment, because account/device proof must bind to
  the service static key authorization and final Noise handshake hash.
- Keep `secure-tunnel-core` as the functional protocol core. Add deterministic
  message encoding, canonical proof bytes, validation, and small state-machine
  helpers there; keep SDK methods as an imperative shell over
  `SecureTunnelSession`.
- Use a small binary v1 application-message envelope instead of JSON for
  signed/protocol material: version byte, message-family/type byte, and
  length-prefixed fields using the existing `codec` helpers. Keep each message
  under `MAX_APPLICATION_PLAINTEXT_SIZE`.
- Treat account credentials/session-resume tokens as opaque payloads in this
  task. Do not invent a product login provider or password/OAuth schema.
- Represent account freshness explicitly as `fresh` versus `resumed`; new-device
  enrollment requires `fresh` unless a later product policy task records an
  exception.
- Keep device private keys out of the SDK facade when possible. Expose
  challenge records plus canonical bytes to sign, then accept caller-provided
  signatures. This is the deliberate exception to hiding all low-level protocol
  details, because Swift/iOS and Android will likely need Secure Enclave /
  Keystore signing without exporting private keys.
- Model server freshness and replay checks in deterministic local test fixtures,
  not as durable production storage. The production service/harness belongs in
  `task-00000021`.
- Add cross-language fixture vectors while implementing the Rust core:
  canonical prologue bytes, deterministic NK1 handshake transcript where
  practical, transport record bytes, and canonical device-proof bytes for both
  purpose codes. These vectors are part of the future SDK contract, not just
  Rust tests.

### Steps

1. Realign inner-channel identity before adding account/device flows.
   - Update protocol docs and constants to prefer the service Noise static
     public key model from Codesuper.
   - Replace or stage replacement of the interim NX server-key payload model
     with NK1-style client-side service static key authorization.
   - Add descriptor/trust metadata fields needed to carry or authorize the
     service static public key and signed descriptor hash.
   - Add descriptor signature verification, descriptor freshness checks, cached
     serial anti-rollback state, and build-time obfuscation for the embedded
     default service static public-key pin.
   - Add tests for wrong service static public key, prologue mismatch,
     handshake payload rejection, fresh-handshake non-replay, and descriptor
     hash/suite mismatches.

2. Define the core application-session protocol modules.
   - Add focused modules under `crates/core/src/`, for example
     `app_message.rs`, `account_session.rs`, and `device_session.rs`, keeping
     every non-Markdown code file below 500 lines.
   - Define stable message-family constants for `login_request`,
     `login_result`, `device_enroll_start`, `device_enroll_challenge`,
     `device_enroll_finish`, `device_enroll_result`, `device_auth_start`,
     `device_auth_challenge`, `device_auth_finish`, and `device_auth_result`.
   - Add encode/decode tests for valid messages, trailing bytes, invalid UTF-8,
     length overflow, unknown version/type, and size-limit rejection.

3. Implement account-authenticated state transitions.
   - Add core types for account auth requests/results with opaque credential or
     resume payloads, `account_id`, server session context id, and freshness.
   - Add a helper that sends an encrypted login/session-recovery request over
     a `FramedDuplex`, receives the result, maps rejection to
     `PostHandshakeAuthFailure`, and returns `SessionPhase::AccountAuthenticated`
     plus a small report.
   - Update `secure-tunnel-sdk` with owned request/result records and a
     `SecureTunnelSession` method such as `authenticate_account(...)` that
     advances state from `SecureReady` to `AccountAuthenticated`.

4. Implement known-device reauthentication.
   - Add core challenge and proof records that bind:
     `secure-tunnel-device-proof-v1\0`, channel binding / handshake hash,
     server challenge, service/environment/authority, signed descriptor hash,
     allowed Noise suite, account context hash, device key id, purpose code,
     expiry, and freshness data.
   - Add canonical signed-byte construction and verification helpers using
     `ed25519-dalek` for tests and Rust callers.
   - Add SDK methods for the two-step flow, for example
     `begin_known_device_auth(...)` and `finish_known_device_auth(...)`, so
     platform SDKs can sign canonical bytes with platform key storage.
   - Enforce preconditions: cannot start before account context is pinned;
     wrong account/session binding, stale challenge, replayed nonce, wrong
     channel binding, or bad signature returns `AuthFailure` /
     `PostHandshakeAuthFailure` and does not advance state.

5. Implement new-device enrollment as a distinct flow.
   - Add enrollment challenge/proof records that bind:
     `secure-tunnel-device-proof-v1\0`, channel binding / handshake hash,
     server challenge, service/environment/authority, signed descriptor hash,
     allowed Noise suite, account context hash, candidate device key id/public
     key, enrollment purpose code, expiry, and freshness data.
   - Add SDK methods for `begin_device_enrollment(...)` and
     `finish_device_enrollment(...)`, returning `device_id` plus explicit
     device state such as `active` or `pending`.
   - Enforce fresh-account precondition and test that resumed account state
     cannot enroll a new device.

6. Update SDK session state and reports without leaking wire internals.
   - Store secure-ready artifacts, account context, pending challenge state, and
     known-device state inside `SecureTunnelSession` internals.
   - Keep public SDK types coarse and owned: account auth report, device auth
     challenge, device auth report, enrollment challenge, enrollment report,
     device state enum, and stable SDK errors.
   - Keep `send`, `receive`, `request`, and `close` behavior compatible with
     task `00000018`; add lease-drop tests for any new pending session
     operation that can temporarily take the transport.

7. Build deterministic local protocol fixtures.
   - Extend the existing SDK mock transport or add core-level scripted session
     peer tests that respond with account, enrollment, and known-device
     messages over the encrypted `FramedDuplex`.
   - Include explicit fixture clocks and nonce stores so replay and stale
     challenge tests are deterministic.
   - Write fixture vectors for canonical prologue bytes and canonical
     device-proof bytes for both purpose codes, using stable test keys and
     fixed clocks so future Swift/Kotlin/Python/Dart/Go bindings can verify
     the same bytes.
   - Avoid production server/database scope; leave real end-to-end server/CLI
     harness work to task `00000021`.

8. Verification and review.
   - Run focused tests first:
     `cargo test -p secure-tunnel-core` and
     `cargo test -p secure-tunnel-sdk`.
   - Run `cargo fmt --all -- --check`,
     `cargo clippy --workspace --all-targets --all-features --no-deps --
     -D warnings`, `cargo doc -p secure-tunnel-core --no-deps`,
     `cargo doc -p secure-tunnel-sdk --no-deps`, and finally `mise run dev`.
   - Check changed non-Markdown source files remain at or below 500 lines.
   - Run independent review / fix / re-review until there are no unresolved
     high- or medium-severity findings.
   - Update implementation notes and acceptance criteria, update the parent
     plan, move the task to `backlog/tasks/completed/`, then describe and push
     the jj change if implementation is requested.

## Review Notes

- Independent review found three blocking issues in the first pass:
  descriptor service static keys were self-asserted by JSON, descriptor
  freshness/serial rollback was not fully enforced, and SDK device auth finish
  paths trusted mismatched device-result key ids. All three were fixed and
  covered by tests.
- Re-review found two remaining blocking issues: SDK connect planning could use
  descriptor-controlled endpoints before full descriptor authorization, and the
  public SDK config could not pass production descriptor roots or service
  static-key pins to production transports. Both were fixed and covered by
  no-dial regression tests.
- Final re-review reported no unresolved high or medium findings. The only
  residual note is low priority: the SDK facade currently exposes
  `secure_tunnel_core::TrustAnchor` directly in `ClientConfig`; clean this up
  before treating the SDK facade as stable for foreign bindings.

## Acceptance Closure

- [x] All acceptance criteria are satisfied and marked.
- [x] Verification commands and outcomes are recorded.
- [x] No unresolved high/medium findings remain.
