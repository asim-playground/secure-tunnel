# Task `00000031` - `security hardening pass`

## Summary

Harden Secure Tunnel availability and security testing before additional SDK
rollout. The task fixes production connect timeout/cancellation behavior,
documents a STRIDE review, and adds repeatable security regression and
mutation-testing entrypoints.

## Motivation

The production `QUIC`/`WSS` adapters could await DNS/connect/open/read and
secure-ready operations without a bounded budget. Because SDK cancellation was
checked only outside the selector await, a malicious endpoint, captive network,
or packet dropper could keep a client connect attempt pending and delay
fallback/cache updates. This is a client-side availability bug in the
CWE-400/resource-exhaustion family.

## Read-Write Repository

- Primary read-write repository: `/Users/asimi/workplace/secure-tunnel`
- Secondary read-write repository/repositories: none
- Code changes are expected to land in Rust SDK/transport crates, generated
  SDK facade metadata, smoke clients, mise tasks, and backlog docs in this
  repository.

## Read-Only Reference Repository

- `/Users/asimi/Downloads/references/cargo-mutants`
- Local docs and upstream public security references cited from
  `backlog/docs/security-hardening-and-stride.md`.

## Detailed Requirements / Acceptance Criteria

### A) Timeout And Cancellation Policy

- [x] SDK config exposes typed timeout budgets for full connect, `QUIC`
      connect/open, `WSS` connect, secure-ready, record read, and record write.
- [x] Production `QUIC` and `WSS` adapters enforce phase and record budgets.
- [x] SDK connect races transport selection against cooperative cancellation.
- [x] Cancellation returns the stable cancelled error class promptly.
- [x] Timeout/fallback classification preserves the invariant that inner
      trust/noise failures never fall back to `WSS`.

### B) Security Regression Tests

- [x] A pending connector is bounded by overall connect timeout.
- [x] A cancellation handle interrupts a pending selector.
- [x] A stalled `QUIC` secure-ready attempt falls back to `WSS` when policy
      permits.
- [x] A `WSS` server that completes WebSocket setup but sends no record is
      bounded by read timeout.
- [x] Oversized record rejection remains covered.

### C) Security Automation And Documentation

- [x] `backlog/docs/security-hardening-and-stride.md` documents STRIDE,
      similar CVE/failure families, and automated security testing.
- [x] `mise run security:test` runs the focused hardening tests.
- [x] `cargo-mutants` is pinned through mise and reference notes are recorded.
- [x] `mise run security:mutants-list` lists candidates for security-critical
      Rust files.
- [x] `mise run security:mutants-smoke` is available as a small, opt-in
      mutation-testing shard.

## Cross-Repo Boundaries

- Primary implementation boundary: Secure Tunnel SDK, transport adapters,
  checked-in smoke clients, mise tasks, and backlog docs.
- Parser / upstream dependency boundary: no protocol parser rewrite; add tests
  and future fuzz guidance for descriptor/framing parsers.
- Downstream integration boundary: generated Swift/Kotlin/Python clients
  inherit the Rust timeout policy through `ClientConfig`.
- External asset / catalog / fixture boundary: cargo-mutants is inspected as a
  read-only reference clone and installed/pinned via mise.

## Task Dependencies

- `task-00000019`
- `task-00000021`
- `task-00000022`
- `task-00000023`
- `task-00000024`
- `task-00000026`
- `task-00000030`

## Reference Tasks

- `backlog/tasks/completed/task-00000019_implement-production-quic-and-wss-carrier-adapters.md`
- `backlog/tasks/completed/task-00000022_add-observability-and-conformance-test-matrix.md`
- `backlog/tasks/completed/task-00000030_build-python-fastapi-server-and-rust-client-e2e.md`

## Backlog Bookkeeping

- `backlog/` is local planning state and is not version controlled by default.
- Moving a task to `backlog/tasks/completed/` is separate from pushing code.
- Commit history for work tracked by this task may live in a different
  read-write repository than this backlog entry.

## Implementation Notes

- Added `TransportPolicyConfig` timeout budgets and propagated them through
  UniFFI `ClientConfig` so Swift, Kotlin, and Python smoke clients inherit the
  same Rust policy defaults.
- Added adapter-level Tokio timeouts for `QUIC` DNS/connect/open/read/write
  and `WSS` connect/read/write. Secure-ready selection is also bounded by a
  candidate timeout and the overall SDK connect budget.
- Changed `CancellationHandle` to notify waiters without a lost-notification
  race. SDK selection now uses guarded connectors/evaluators so cancellation
  and full-connect deadline failures are classified inside the selector and
  preserve prior attempt traces.
- Added hardening tests for cancellation during pending selection, overall
  connect timeout, stalled `QUIC` secure-ready fallback, stalled `WSS` record
  reads, control-frame-only `WSS` reads, post-fallback cancellation/timeout
  trace preservation, and direct `WSS` runtime close/error classification.
- Added `backlog/docs/security-hardening-and-stride.md` with STRIDE analysis,
  related resource-exhaustion / DoS failure families, and repeatable security
  automation guidance.
- Cloned `/Users/asimi/Downloads/references/cargo-mutants`, pinned
  `cargo:cargo-mutants` through mise, and added `mise run security:test`,
  `mise run security:mutants-list`, and `mise run security:mutants-smoke`.
- Verification evidence:
  - `cargo check -p secure-tunnel-sdk -p secure-tunnel-transport -p secure-tunnel-sdk-ffi --all-features` passed.
  - `mise run sdk:check-bindings` passed after regenerating bindings.
  - `mise run security:test` passed: 5 SDK hardening tests and 3 transport
    hardening tests.
  - `mise run security:mutants-list` passed and listed 260 candidates.
  - `mise run security:mutants-smoke` first found a missed WSS config mutant;
    after adding a config-limit test it passed: 13 mutants tested, 4 caught,
    9 unviable.
  - `cargo nextest run -p secure-tunnel-sdk -p secure-tunnel-transport --all-features --no-fail-fast` passed: 37 tests.
  - `mise run dev` passed: full Rust, Python, Go, SDK, and smoke pipeline.

## Implementation Plan

1. Add timeout policy to Rust SDK config and generated SDK facade config.
2. Enforce production `QUIC`/`WSS` connect/read/write budgets.
3. Race SDK selection against cancellation and overall connect timeout.
4. Add hardening regression tests for pending connectors, stalled secure-ready,
   stalled `WSS` reads, and oversized records.
5. Add STRIDE/security doc plus `security:*` mise tasks, including
   cargo-mutants discovery/smoke entrypoints.
6. Run focused checks, full repo validation, independent review, and
   re-review.

## Review Notes

- Independent review found two medium issues:
  - `WSS` control frames could extend the per-frame timeout forever because the
    timeout was applied to each WebSocket event rather than one logical record
    read.
  - SDK cancellation/full-connect timeout was synthesized outside the selector,
    losing prior attempt traces and misreporting the active carrier after a
    previous `QUIC` fallback.
- Fixes:
  - `WSS` now wraps the full logical record-read loop in one timeout.
  - SDK connect now wraps each connector and secure-ready evaluator with a
    shared deadline/cancellation budget so the selector records cancellation
    and timeout failures on the active candidate.
  - Added focused regression tests for both issues.
- Re-review found no high- or medium-severity findings and confirmed all
  changed first-party non-Markdown code files remain at or below 500 lines.
- Final re-review after the WSS config mutation-test fix and clippy helper
  extraction found no high- or medium-severity findings and required no
  further re-review.

## Acceptance Closure

- [x] All acceptance criteria are satisfied and marked.
- [x] Verification commands and outcomes are recorded.
- [x] No unresolved high/medium findings remain.
