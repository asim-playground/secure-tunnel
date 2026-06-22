---
status: draft
normative: false
supersedes: []
superseded_by: []
---

# Plan `00000002` - `product secure tunnel sdk and bindings`

## Metadata

- Date: `2026-06-21`
- Status: `draft`
- Owner: `Asim Ihsan`
- Related Plans: `plan-00000001`
- Related Tasks: `task-00000007, task-00000008, task-00000009, task-00000013, task-00000014, task-00000017, task-00000018, task-00000019, task-00000020, task-00000021, task-00000022, task-00000023, task-00000024, task-00000025, task-00000026, task-00000027, task-00000028, task-00000029, task-00000030, task-00000031`

## Summary

This plan turns the current Secure Tunnel foundation into a product-ready Rust
library with SDKs for Swift/iOS, Kotlin, Python, Flutter/Dart, and Go. The
previous plan proved the v1 protocol direction, transport selector, Noise trust
path, QUIC/WSS prototype harness, and a narrow manual C ABI that Swift and Go
can import. The next work is to complete real transport/session behavior,
realign the inner Noise identity model around a service static public key,
stabilize the Rust SDK facade, and then package language-specific SDKs from
that facade. UniFFI remains the default generated path for Swift, Kotlin, and
Python; Flutter/Dart and Go get first-class paths that still share the same
Rust facade and behavior.

## Decision Summary (Locked)

- [x] Keep `secure-tunnel-core` as the pure Rust implementation boundary; do
      not bake Swift, Kotlin, or Python assumptions into core protocol modules.
- [x] Use UniFFI as the default cross-language SDK path for Swift, Kotlin, and
      Python, with a pinned project-local bindgen binary.
- [x] Keep the existing manual C ABI as a bootstrap and compatibility layer
      until UniFFI packaging, import, and smoke tests pass for all required
      targets.
- [x] Expose a deliberately small SDK surface: owned records, byte arrays,
      strings, explicit errors, coarse operations, and opaque session objects.
- [x] Keep hot loops inside Rust. Do not expose per-frame transport internals
      or hundreds of tiny getters across the foreign boundary.
- [x] Make Swift/iOS the first production-grade SDK package target.
- [x] Treat Flutter/Dart as a first-class follow-on SDK target using the Rust
      facade and Flutter-specific bridge packaging, not UniFFI.
- [x] Treat Go as a first-class follow-on SDK target over the manual C ABI and
      cbindgen-generated header, not UniFFI.
- [x] Keep native Go as the supported Go SDK path and deprecate or delete the
      existing Go-WASM scaffold unless a future task proves a concrete need.
- [x] Prefer the Codesuper-style service Noise static public key model for
      Secure Ready: client-side authorization of the service static key,
      server-only private key custody, stable context bound into the Noise
      prologue, and account/device identity proven only after Secure Ready.

## Goals

- Complete the Rust client library path from descriptor/config through carrier
  selection, `Secure Ready`, account/device session establishment, application
  records, observability, and graceful close.
- Replace test-only transport prototypes with production carrier adapters for
  raw `QUIC` and `WSS`, while preserving the existing fallback semantics.
- Define a stable Rust SDK facade that generated bindings can call without
  exposing internal protocol, selector, or transport types directly.
- Add a UniFFI facade crate and generated Swift, Kotlin, and Python bindings
  with project-pinned generator tooling.
- Package native SDK artifacts: SwiftPM/XCFramework first, then Android/JVM
  artifacts and Python wheels or a documented Python bridge path.
- Package Flutter/Dart and Go SDKs over the same Rust facade after the Swift
  path proves the production packaging shape.
- Add CI and smoke tests that prove each package can be imported and can run at
  least one descriptor/config/session scenario.

## Non-Goals

- Weaken the v1 security model, trust-anchor model, or `QUIC`-preferred plus
  `WSS` fallback decision. Task `00000020` may refine the inner Noise identity
  shape to the service static public key model, but it must preserve the
  service-authenticated inner channel and fallback security invariants.
- Replace every existing binding surface in one step. The manual C ABI and
  current Python/Go scaffolds can remain while the UniFFI path proves itself.
- Force-fitting UniFFI onto Flutter/Dart or Go. Those targets use different
  binding strategies while preserving the same Rust facade behavior.
- Expose the whole Rust API to foreign languages.
- Ship app-store-ready mobile apps or a hosted production service.
- Optimize for extremely high-frequency FFI calls before measuring a real
  boundary-call workload.

## Current State / Baseline

- `plan-00000001` is active and has completed the major proving slices through
  `task-00000016`.
- The Rust workspace has `secure-tunnel-core`, a CLI, a manual C ABI crate
  named `secure-tunnel-ffi`, a product SDK facade crate named
  `secure-tunnel-sdk`, a PyO3 Python crate, and Go/Go-WASM binding scaffolds.
  Go-WASM is now planned for deprecation or deletion rather than supported SDK
  status.
- The manual C ABI currently exposes protocol constants and service descriptor
  JSON validation/normalization, with `crates/go/binding.h` and
  `crates/go/module.modulemap` tracked for Swift import groundwork.
- The current Rust core can build a signed v1 descriptor, derive the Noise
  prologue, plan QUIC-first fallback candidates, evaluate `Secure Ready` over
  framed I/O, and test prototype QUIC/WSS behavior. Task `00000020` aligned the
  inner channel with `NK1`, pinned service static public keys, descriptor
  signatures, descriptor freshness, and serial rollback checks.
- The SDK facade defines the Rust product contract for descriptors, transport
  policy, connect, cancellation, connect reports, failed-attempt reports,
  security artifacts, sessions, send/receive/request, and close.
- Production server/harness wiring, generated bindings, and native SDK packages
  are not yet done.
- Local reference material is available for SDK implementation:
  - `backlog/docs/2026-06-21_sdk-reference-repositories.md`
  - `/Users/asimi/Downloads/references/uniffi-rs`
  - `/Users/asimi/Downloads/references/application-services`
  - `/Users/asimi/workplace/flutter_template`
  - `/Users/asimi/Downloads/references/flutter_rust_bridge`
  - `/Users/asimi/Downloads/references/dart-native`
  - `/Users/asimi/Downloads/references/cbindgen`
  - `/Users/asimi/Downloads/references/pyo3`
  - `/Users/asimi/Downloads/references/cargo-mutants`
  - `/Users/asimi/workplace/codesuper`

## Gap Analysis

### Foundation closure

- Status: tasks `00000007`, `00000008`, and `00000009` are completed and the
  active `v1-*` docs now cover transport selection, protocol bindings,
  descriptor shape, device policy, and UDP-first deployment/observability.
- Remaining impact: foundation planning is closed; Phase 1 now depends on
  production transport/session implementation rather than unresolved protocol
  documentation.
- Notes: completed tasks `00000017` and `00000018` removed the decomposition
  and facade-definition gates.

### Core decomposition

- Status: `task-00000017` split the oversized selector, Noise, and prototype
  transport modules so every non-Markdown code file is at or below 500 lines.
- Remaining impact: future adapter and binding work can build on smaller
  reviewable modules without freezing the proving-slice test layout into public
  contracts.
- Notes: public API redesign moved into the completed SDK facade in
  `task-00000018`.

### Production transport and session behavior

- Status: task `00000021` adds local end-to-end harness and CLI smoke coverage
  for descriptor loading, production `QUIC`/`WSS` adapters, `Secure Ready`,
  account/device auth, application exchange, and close.
- Missing: outer TLS custom-CA configuration product UX, HTTP proxy support,
  retry policy hardening, and broader conformance/observability coverage.
- Impact: Swift/Kotlin/Python packages can reuse a local Rust smoke oracle, but
  native package import tests still need generated bindings and artifacts.
- Notes: managed-network tasks `00000013` and `00000014` should land after the
  first real adapters exist so their tests exercise actual carrier code.

### Foreign SDK boundary

- Status: `task-00000018` added the stable Rust SDK facade above core without
  exposing selector, Noise, trust, or carrier adapter internals.
- Missing: a dedicated UniFFI crate, generated bindings, and native packaging.
- Impact: binding work now has a stable Rust product API to wrap, but it still
  cannot ship until real adapters, session auth, conformance, and packages
  land.
- Notes: prefer a UDL contract for the first UniFFI pass because reviewers can
  read it as a language-neutral API spec.

### Packaging and CI

- Missing: generated binding output, SwiftPM/XCFramework packaging, Android or
  JVM packaging, Python wheel/import strategy, Flutter/Dart bridge packaging,
  Go package stabilization, and package-level smoke tests.
- Impact: generated bindings alone will not prove the SDK can be consumed by
  real downstream projects.
- Notes: packaging remains repo-owned work; UniFFI only generates binding code.

### Flutter/Dart and Go SDKs

- Missing: first-class Flutter/Dart and Go packaging plans tied to the same Rust
  SDK facade.
- Impact: downstream app work could fork behavior away from the Swift/Kotlin/
  Python SDK or keep relying on scaffold-era Go wrappers.
- Notes: use `flutter_template`, `flutter_rust_bridge`, and `dart-native` for
  Flutter/Dart reference patterns; use the existing manual C ABI and `cbindgen`
  for Go.

## Strategy

- Close foundation planning gaps first, then split oversized core modules
  before growing the public SDK surface.
- Define a Rust SDK facade crate or module before UniFFI. This facade owns
  `ClientConfig`, `BootstrapDescriptor`, `SecureTunnelClient`, `Session`,
  explicit error enums, cancellation handles, and coarse result/report records.
- Add a separate `secure-tunnel-sdk-ffi` crate for UniFFI. It depends on the
  Rust SDK facade and contains no transport business logic.
- Default to a UDL-defined UniFFI contract for the first generated SDK because
  the language-neutral file is easier to review than scattered macro exports.
- Make Swift/iOS the first production package target. Kotlin and Python should
  keep import/build parity, but Swift drives the first full packaging bar.
- Add Flutter/Dart through a separate bridge package over the Rust SDK facade,
  with Flutter Rust Bridge as the recommended first path and direct Dart FFI
  plus `ffigen` as the comparison point.
- Keep Go on the stable manual C ABI path with cbindgen drift checks, because
  UniFFI does not cover Go.
- Keep the manual C ABI in place until the generated Swift, Kotlin, and Python
  packages and the Go package pass import/build smoke tests and at least one
  end-to-end scenario.
- Own packaging explicitly in CI: build native Rust libraries for each target,
  generate bindings with pinned bindgen, assemble native packages, and run
  language-level smoke tests.
- Treat boundary-call performance as a measured risk. Start with coarse
  operations and opaque sessions; only introduce lower-level APIs when tests or
  product requirements prove they are needed.

## Phase Plan

- Current Phase: `Phase 4 - package native SDKs`

### Phase 0 - `close foundation gates`

- Objective: remove stale planning and review blockers before public SDK work.
- Candidate Tasks:
    - `task-00000007` `define transport selection and fallback policy`
    - `task-00000008` `write transport-agnostic v1 protocol plus quic and wss bindings`
    - `task-00000009` `define udp-first deployment and observability requirements`
    - `task-00000017` `decompose core modules before sdk expansion`
- Exit Criteria:
    - [x] active protocol, selection, and deployment docs are aligned with the
          code and task acceptance workflow.
    - [x] all non-Markdown code files touched by the SDK plan are at or below
          500 lines, or have an explicit reviewed decomposition exception.
    - [x] `mise run dev` passes after the decomposition.

### Phase 1 - `complete the Rust tunnel library`

- Objective: make the Rust library useful before binding it to native SDKs.
- Candidate Tasks:
    - `task-00000018` `define product sdk facade and session contract`
    - `task-00000019` `implement production quic and wss carrier adapters`
    - `task-00000020` `implement account and device session protocol`
    - `task-00000021` `build end-to-end tunnel harness and cli smoke path`
- Exit Criteria:
    - [x] a Rust caller can create a client, load a descriptor/config, connect,
          reach `Secure Ready`, authenticate, exchange application records, and
          close cleanly.
    - [x] both `QUIC` success and `WSS` fallback run through production
          adapters, not only test-only prototype transports.
    - [x] local end-to-end tests cover success, fallback, inner trust failure,
          and graceful close.

### Phase 2 - `managed network and observability`

- Objective: make the library operable in the network environments the v1
  design already calls out.
- Candidate Tasks:
    - `task-00000013` `allow optional custom ca cert for intercepted wss or quic`
    - `task-00000014` `allow optional http proxy for wss client`
    - `task-00000022` `add observability and conformance test matrix`
    - `task-00000031` `security hardening pass`
- Exit Criteria:
    - [ ] custom outer-TLS CA configuration works without weakening inner Noise
          trust.
    - [ ] proxied `WSS` works as a fallback path without creating a separate
          security model.
    - [ ] events and metrics distinguish outer path failure, outer TLS/proxy
          failure, fallback, inner trust failure, session failure, and close.
    - [ ] production connect, secure-ready, record read/write, and
          cancellation paths are bounded and covered by adversarial tests.

### Phase 3 - `generate the common SDK bindings`

- Objective: introduce one small UniFFI facade for Swift, Kotlin, and Python.
- Candidate Tasks:
    - `task-00000023` `create uniffi sdk facade and bindgen tooling`
- Exit Criteria:
    - [x] `uniffi` and the project-local bindgen binary are pinned in the
          workspace dependency graph.
    - [x] generated Swift, Kotlin, and Python bindings expose only the approved
          SDK facade.
    - [x] generated bindings pass language-level import/build smoke tests.

### Phase 4 - `package native SDKs`

- Objective: ship consumable Swift, Kotlin, and Python artifacts rather than
  raw generated source, with Swift/iOS as the first production-grade package.
- Candidate Tasks:
    - `task-00000024` `package swift sdk as swiftpm and xcframework`
    - `task-00000025` `package kotlin sdk as jvm or android artifact`
    - `task-00000026` `package python sdk from the shared rust facade`
    - `task-00000030` `build python fastapi server and rust client e2e`
- Exit Criteria:
    - [x] Swift/iOS is the first production-grade SDK package and can run a
          descriptor/session smoke test.
    - [ ] Kotlin can import the artifact and run the same scenario through JNA
          or the documented UniFFI backend.
    - [x] Python can import the package and run the same scenario, with a clear
          decision on whether UniFFI replaces or wraps the existing PyO3 path.
    - [x] Rust client can run the same scenario against a Python FastAPI server
          fixture without changing protocol semantics.
    - [ ] Kotlin and Python are at least at generated-binding and package smoke
          parity before release CI treats them as supported SDK targets.

### Phase 5 - `package Flutter and Go SDKs`

- Objective: add Flutter/Dart and Go SDKs without changing the Rust product
  facade or the Swift-first packaging decision.
- Candidate Tasks:
    - `task-00000028` `package flutter dart sdk using rust facade`
    - `task-00000029` `package go sdk over stable c abi`
- Exit Criteria:
    - [ ] Flutter/Dart can import the package and run a descriptor/session smoke
          test through a hand-written facade over generated bridge code.
    - [ ] Go can import the package and run a descriptor/session smoke test over
          the stable C ABI.
    - [ ] Flutter/Dart and Go package checks share the same fixture semantics
          as Swift, Kotlin, and Python.

### Phase 6 - `release SDKs`

- Objective: version, build, and archive the supported SDK artifacts
  reproducibly.
- Candidate Tasks:
    - `task-00000027` `add sdk release ci and versioning`
- Exit Criteria:
    - [ ] CI builds and archives package artifacts with versioned outputs.
    - [ ] stale generated bindings and stale package metadata fail release
          checks.
    - [ ] release docs identify which targets are production-grade versus smoke
          parity.

## Backlog Task Map

| Task ID | Title | Phase | Depends On | Status |
|---|---|---|---|---|
| task-`00000007` | `define transport selection and fallback policy` | `Phase 0` | `task-00000003, task-00000004, task-00000006` | `completed` |
| task-`00000008` | `write transport-agnostic v1 protocol plus quic and wss bindings` | `Phase 0` | `task-00000007` | `completed` |
| task-`00000009` | `define udp-first deployment and observability requirements` | `Phase 0` | `task-00000007, task-00000008` | `completed` |
| task-`00000017` | `decompose core modules before sdk expansion` | `Phase 0` | `task-00000016` | `completed` |
| task-`00000018` | `define product sdk facade and session contract` | `Phase 1` | `task-00000007, task-00000008, task-00000009, task-00000017` | `completed` |
| task-`00000019` | `implement production quic and wss carrier adapters` | `Phase 1` | `task-00000012, task-00000018` | `completed` |
| task-`00000020` | `implement account and device session protocol` | `Phase 1` | `task-00000006, task-00000011, task-00000018` | `completed` |
| task-`00000021` | `build end-to-end tunnel harness and cli smoke path` | `Phase 1` | `task-00000019, task-00000020` | `completed` |
| task-`00000013` | `allow optional custom ca cert for intercepted wss or quic` | `Phase 2` | `task-00000009, task-00000012, task-00000019` | `proposed` |
| task-`00000014` | `allow optional http proxy for wss client` | `Phase 2` | `task-00000009, task-00000012, task-00000013, task-00000019` | `proposed` |
| task-`00000022` | `add observability and conformance test matrix` | `Phase 2` | `task-00000009, task-00000021` | `completed` |
| task-`00000031` | `security hardening pass` | `Phase 2` | `task-00000019, task-00000021, task-00000022, task-00000023, task-00000024, task-00000026, task-00000030` | `completed` |
| task-`00000023` | `create uniffi sdk facade and bindgen tooling` | `Phase 3` | `task-00000018, task-00000021, task-00000022` | `completed` |
| task-`00000024` | `package swift sdk as swiftpm and xcframework` | `Phase 4` | `task-00000022, task-00000023` | `completed` |
| task-`00000025` | `package kotlin sdk as jvm or android artifact` | `Phase 4` | `task-00000022, task-00000023` | `proposed` |
| task-`00000026` | `package python sdk from the shared rust facade` | `Phase 4` | `task-00000022, task-00000023` | `completed` |
| task-`00000030` | `build python fastapi server and rust client e2e` | `Phase 4` | `task-00000023, task-00000026` | `completed` |
| task-`00000028` | `package flutter dart sdk using rust facade` | `Phase 5` | `task-00000018, task-00000021, task-00000022, task-00000024` | `proposed` |
| task-`00000029` | `package go sdk over stable c abi` | `Phase 5` | `task-00000016, task-00000018, task-00000021, task-00000022, task-00000024` | `proposed` |
| task-`00000027` | `add sdk release ci and versioning` | `Phase 6` | `task-00000022, task-00000024, task-00000025, task-00000026, task-00000028, task-00000029` | `proposed` |

## Validation Strategy

- Unit/Integration: Rust unit and integration tests for descriptor validation,
  connect planning, QUIC/WSS adapters, Noise trust, account/device messages,
  cancellation, close, and failure classification.
- CLI/manual checks: `mise run dev`, `mise run ci`, local CLI tunnel smoke
  tests, and language-package import smoke tests.
- Package checks:
  - Swift: build package/XCFramework and run an import plus descriptor/session
    smoke test. Swift/iOS is the first production-grade package target.
  - Kotlin: build generated bindings plus native library packaging and run a
    JVM or Android smoke test.
  - Python: build wheel or package layout and run an import plus smoke test in
    a clean environment.
  - Flutter/Dart: build generated bridge output plus a hand-written Dart facade
    and run analyzer/import plus iOS simulator native smoke where applicable.
  - Go: build the native Go package over the C ABI and run import, ownership,
    and descriptor/session smoke tests.
- Regression safeguards: signed descriptor fixtures, service-static-key pin
  fixtures, fallback/failure snapshots, generated-binding API checks, and
  package artifact checksums.
- Security hardening: `mise run security:test`, cargo-mutants candidate
  listing/smoke shards for security-critical Rust files, and future fuzz
  targets for descriptor/framing/application parsers.
- Definition of Done:
    - [ ] Phase 0 foundation tasks are closed or explicitly refreshed.
    - [ ] Rust library can run the end-to-end local secure tunnel scenario.
    - [x] Swift, Kotlin, and Python bindings are generated from the same pinned
          facade.
    - [x] Swift/iOS is the first production-grade SDK package target.
    - [ ] Flutter/Dart and Go package tasks exist and share the same Rust SDK
          facade semantics.
    - [ ] Native packages build in CI and pass import/session smoke tests.
    - [ ] No unresolved high/medium independent review findings remain.

## Risks and Mitigations

| Risk | Trigger | Mitigation | Owner |
|---|---|---|---|
| UniFFI pre-1.0 churn breaks generated SDKs | upgrading `uniffi` or bindgen | pin `uniffi` and bindgen in the workspace; regenerate in CI; keep API small | Asim Ihsan |
| Foreign SDK leaks internal Rust design | exporting selector, Noise, or transport types directly | introduce an SDK facade and review UDL as the public contract | Asim Ihsan |
| Packages import but cannot perform useful work | binding work starts before Rust session behavior is complete | make end-to-end Rust harness a dependency of UniFFI packaging | Asim Ihsan |
| Kotlin JNA boundary is too slow for chatty calls | many small SDK calls across the boundary | keep hot loops in Rust; benchmark boundary calls before exposing lower-level APIs | Asim Ihsan |
| Swift toolchain concurrency or packaging edge cases block adoption | Xcode/Swift version incompatibility | add Swift package smoke tests before making Swift the primary SDK | Asim Ihsan |
| Python API quality regresses | replacing PyO3 abruptly with flat UniFFI output | keep PyO3 compatibility until the shared facade proves an acceptable Python package | Asim Ihsan |
| Flutter/Dart package drifts from native SDK behavior | Dart bridge exposes a different API shape from the Rust SDK facade | keep generated bridge code behind hand-written Dart facades and shared fixture tests | Asim Ihsan |
| Go SDK memory ownership bugs appear | manual C ABI grows without drift and cleanup checks | keep C ABI narrow, use cbindgen drift checks, and test allocation/error cleanup paths | Asim Ihsan |
| Managed-network support weakens inner trust semantics | custom CA or proxy code is treated as security trust | keep outer TLS/proxy config separate from inner Noise trust in API, tests, and docs | Asim Ihsan |
| Availability bugs block fallback or cancellation | malicious endpoint stalls DNS/connect/open/read/secure-ready | typed timeout policy, cancellation-aware selection, stalled-peer tests, and STRIDE review | Asim Ihsan |

## Open Questions

| Question | Needed By | Owner | Resolution |
|---|---|---|---|
| Which package should become the first production-grade SDK target? | before `task-00000024` starts | Asim Ihsan | `resolved: Swift/iOS first; Kotlin/Python smoke parity until facade stabilizes` |
| Should the UniFFI contract use UDL or proc macros? | during `task-00000023` | Asim Ihsan | `recommended: UDL first for a reviewable language-neutral API spec` |
| What cancellation semantics should the SDK expose for long-running connect/session operations? | during `task-00000018` | Asim Ihsan | `resolved: connect accepts an explicit CancellationHandle; session operation futures are safe to drop/cancel via transport lease restoration, while explicit session cancellation handles remain future work if real adapters need them` |
| Does Python ultimately use UniFFI only, PyO3 only, or a PyO3 wrapper over the shared facade? | during `task-00000026` | Asim Ihsan | `resolved: Python uses maturin-packaged UniFFI over secure-tunnel-sdk-ffi as the behavioral core, with a small secure_tunnel wrapper and legacy descriptor compatibility aliases` |
| Should the Python FastAPI server use PyO3/maturin, UniFFI Python, or a small wrapper over a Rust native library? | during `task-00000030` | Asim Ihsan | `resolved: FastAPI is the Python imperative shell over the Rust binding-fixture process; Rust keeps service static-key custody, descriptor signing, protocol, auth, and application-frame semantics` |
| Should Flutter/Dart use Flutter Rust Bridge or direct Dart FFI plus ffigen first? | during `task-00000028` | Asim Ihsan | `recommended: Flutter Rust Bridge first, compare direct Dart FFI only if packaging evidence pushes that way` |
| Should Go keep Go-WASM as supported SDK scope? | during `task-00000029` | Asim Ihsan | `resolved: native Go is supported; Go-WASM should be deprecated or deleted unless a future task proves concrete need` |

## Immediate Next Actions

1. Start `task-00000025` to package the Kotlin SDK as the next UniFFI native
   artifact.
2. Start `task-00000027` to make SDK release packaging decide how to repair
   macOS Python wheels that reference Homebrew `libiconv`, archive package
   outputs, and fail stale generated/package metadata.
3. Start `task-00000028` and `task-00000029` after Kotlin package smoke parity,
   keeping Flutter/Dart and native Go on the same fixture semantics as Swift,
   Kotlin, Python, and Rust-client/FastAPI smokes.
5. Keep `task-00000013` and `task-00000014` queued for managed-network support
   before declaring the SDK broadly production-ready.

## Implementation Notes

- Created from the Rust library binding research captured in the user-provided
  attachment and the follow-up notes in completed `task-00000016`.
- The plan intentionally treats the current manual C ABI as a bridge, not the
  final cross-language SDK strategy.
- The plan assumes UniFFI remains the default unless a future task measures a
  boundary-call or platform-toolchain blocker that makes the generated SDK path
  unsuitable.
- `2026-06-21`: User resolved the first production SDK target as Swift/iOS.
- `2026-06-21`: Local references were prepared under
  `/Users/asimi/Downloads/references`: `uniffi-rs`, `application-services`,
  `flutter_rust_bridge`, `dart-native`, `cbindgen`, and `pyo3`; the plan also
  uses `/Users/asimi/workplace/flutter_template` as the local Flutter/Dart
  packaging reference.
- `2026-06-21`: Reference clone commits and intended uses are recorded in
  `backlog/docs/2026-06-21_sdk-reference-repositories.md`.
- `2026-06-21`: User resolved Go SDK scope as native Go only, with Go-WASM to
  be deprecated or deleted unless a future task proves concrete need.
- `2026-06-21`: Foundation closure tasks `00000007`, `00000008`, and
  `00000009` completed; Phase 0 now waits on `task-00000017`.
- `2026-06-21`: `task-00000017` completed the core module decomposition gate,
  so the plan can advance to Phase 1 and the SDK facade contract work in
  `task-00000018`.
- `2026-06-21`: `task-00000018` completed the product SDK facade crate and
  session contract. Phase 1 now moves to production `QUIC`/`WSS` adapters in
  `task-00000019`.
- `2026-06-21`: `task-00000019` completed production `QUIC` and `WSS`
  adapters in `secure-tunnel-transport`, wired them into the SDK default
  client, and added real secure-ready integration coverage for success,
  fallback, close-before-secure-ready, malformed target, oversized WSS message,
  and inner-trust failure paths.
- `2026-06-21`: `task-00000020` completed the `NK1` service-static inner
  channel model, descriptor signatures/freshness/rollback checks, account and
  device session protocol, SDK auth/enrollment methods, canonical fixtures, and
  build-time obfuscation for the embedded service static public-key pin. The
  SDK now performs descriptor root/signature/freshness and service static pin
  authorization before planning or dialing descriptor-controlled endpoints.
- `2026-06-21`: `task-00000021` completed the local end-to-end harness and CLI
  smoke path over production SDK and transport adapters, with JSON smoke output
  and a dedicated `mise run smoke` task included in `mise run ci`.
- `2026-06-21`: `task-00000022` completed stable SDK observability names,
  tracing hooks, exact conformance CLI/mise automation, 13 runnable local
  conformance scenarios, and pending rows for custom CA, proxied `WSS`, abrupt
  close, and truncated close.
- `2026-06-21`: `task-00000023` completed a pinned UniFFI facade and
  project-local bindgen binary, generated Swift/Kotlin/Python sources under
  `target/generated-bindings/uniffi`, exposed descriptor trust anchors,
  failed-connect attempt diagnostics, and secure channel artifacts across the
  FFI boundary, and added end-to-end generated-client smokes against the Rust
  fixture. Python FastAPI server interop was split into `task-00000030`.
- `2026-06-21`: `task-00000024` packaged the Swift SDK as the first
  production-grade native SDK target. The tracked SwiftPM templates live under
  `bindings/swift`, generated output lives under `target/sdk/swift`, and
  `mise run sdk:swift` builds an Apple-target XCFramework, checks the package,
  runs a SwiftPM command-line session smoke, and runs an iOS simulator XCTest
  session smoke against the Rust fixture.
- `2026-06-21`: Prepared coordinated implementation plans for
  `task-00000026` and `task-00000030`. The planned Python path is
  maturin-packaged UniFFI as the behavioral core, a typed `secure_tunnel`
  wrapper for ergonomics and PyO3 migration, and a FastAPI fixture that acts as
  the Python imperative shell over Rust protocol/server lifecycle hooks.
- `2026-06-21`: `task-00000026` completed the Python SDK package migration to
  maturin-packaged UniFFI over `secure-tunnel-sdk-ffi`, removed the legacy PyO3
  Rust crate, preserved descriptor compatibility aliases in the
  `secure_tunnel` wrapper, and added wheel/import/session smoke tasks.
- `2026-06-21`: `task-00000030` completed the Python FastAPI fixture shell and
  Rust-client e2e smoke. FastAPI serves health/descriptor/bootstrap/fixture
  metadata while Rust owns service static-key custody, descriptor signing,
  protocol/auth/application semantics, and the server fixture lifecycle.
- `2026-06-21`: Python FastAPI fixture configuration now uses typed Python
  dataclasses and `StrEnum` values. `ObservabilitySettings` maps Python server
  config into Rust process environment for `tracing_subscriber` stderr logs,
  `RUST_LOG`, OTLP endpoint variables, service name, and resource attributes,
  while CLI stdout remains reserved for machine-readable JSON.
- `2026-06-21`: Added `task-00000031` for security hardening. It covers
  production timeout/cancellation policy, STRIDE documentation, stalled-peer
  regression tests, and cargo-mutants security-critical file discovery.
- `2026-06-21`: Completed `task-00000031` with typed timeout budgets,
  cancellation/deadline-aware selector inputs, bounded WSS logical record
  reads, STRIDE documentation, hardening tests, and cargo-mutants smoke/list
  automation.

## Completion Checklist

- [x] All planned tasks created under `backlog/tasks/task-<id>.md`
- [ ] All task acceptance criteria checked
- [ ] Validation strategy executed
- [ ] Plan status updated to `completed`
- [ ] Plan moved to `backlog/plans/completed/`

## Changelog

- `2026-06-21` `Initial draft plan created after dependency and Swift-callable C ABI preparation in task-00000016.`
- `2026-06-21` `Review fix: native package tasks now depend on task-00000022 so observability and conformance cannot be bypassed before SDK rollout.`
- `2026-06-21` `Locked Swift/iOS as the first production-grade SDK target and added Flutter/Dart plus Go package tasks.`
- `2026-06-21` `Review fix: UniFFI generation now depends on conformance, and Go packaging now waits for the Swift/iOS package task.`
- `2026-06-21` `Resolved Go-WASM scope: native Go is the supported SDK path, while Go-WASM should be deprecated or deleted.`
- `2026-06-21` `Marked foundation docs and tasks 00000007, 00000008, and 00000009 complete; task 00000017 is the remaining Phase 0 gate.`
- `2026-06-21` `Completed task 00000017 and advanced the plan to Phase 1.`
- `2026-06-21` `Completed task 00000018 with a new secure-tunnel-sdk facade crate, connect/session contract, cancellation semantics, and mock-backed SDK tests.`
- `2026-06-21` `Completed task 00000019 with production QUIC/WSS transport adapters, SDK default-port wiring, Rustls/Tungstenite supply-chain policy updates, and integration tests for secure-ready success/fallback/failure semantics.`
- `2026-06-21` `Completed task 00000020 with NK1 service-static trust, signed descriptor authorization, build-time public-key pin obfuscation, account/device session protocol methods, and no-dial regressions for unauthorized descriptors and service keys.`
- `2026-06-21` `Completed task 00000021 with secure-tunnel-harness, JSON-first CLI smoke scenarios for QUIC success and WSS fallback, SDK local TLS root config, and mise smoke automation in the CI task.`
- `2026-06-21` `Completed task 00000022 with SDK observability taxonomy, redacted tracing hooks, exact conformance CLI/mise automation, and explicit pending managed-network rows.`
- `2026-06-21` `Completed task 00000023 with pinned UniFFI UDL/bindgen tooling, generated Swift/Kotlin/Python smoke clients, and a follow-up FastAPI server interop task.`
- `2026-06-21` `Completed task 00000024 with a SwiftPM SecureTunnel package, XCFramework assembly, SwiftPM host smoke, iOS simulator XCTest smoke, and macOS CI wiring.`
- `2026-06-21` `Prepared coordinated Python plans for task 00000026 and task 00000030, centered on maturin-packaged UniFFI, a typed secure_tunnel wrapper, and a FastAPI shell over Rust protocol/server lifecycle hooks.`
- `2026-06-21` `Completed task 00000026 with a maturin UniFFI Python package, secure_tunnel wrapper, compatibility aliases, wheel/import checks, and Python package session smoke.`
- `2026-06-21` `Completed task 00000030 with a FastAPI fixture shell over the Rust binding fixture, a Rust binding-fixture-client command, and Rust-client-to-Python-FastAPI e2e smoke in mise run dev.`
- `2026-06-21` `Review and user-request follow-up: added Python dataclass/StrEnum server configuration, server optional extra wheel coverage, timeout-safe fixture startup cleanup, and Rust CLI tracing-subscriber configuration from Python-provided environment.`
- `2026-06-21` `Added task 00000031 for timeout/cancellation security hardening, STRIDE review, stalled-peer regressions, and cargo-mutants automation.`
- `2026-06-21` `Completed task 00000031 after independent review and re-review; final validation included mise run dev, security:test, sdk:check-bindings, cargo-mutants list, and cargo-mutants smoke.`
