# Task `00000019` - `implement production quic and wss carrier adapters`

## Summary

Replace test-only carrier prototypes with production `QUIC` and `WSS` client
adapters that implement the shared framed duplex contract.

## Motivation

The current prototype harness proves selector and secure-ready behavior but is
not a production transport implementation. The SDK needs real carrier adapters
before native bindings can exercise meaningful connection scenarios.

## Read-Write Repository

- Primary read-write repository: `/Users/asimi/workplace/secure-tunnel`
- Secondary read-write repository/repositories (if applicable): none
- State explicitly:
  - code changes are expected to land in Rust carrier modules or crates and in
    tests/harnesses that exercise them.
  - this repository itself is expected to change.

## Read-Only Reference Repository

- Read-only reference repository/repositories:
  - `/Users/asimi/Downloads/references/quinn`
  - `/Users/asimi/Downloads/references/tokio-tungstenite`
  - `/Users/asimi/Downloads/references/rustls-platform-verifier`
- State explicitly which repositories may be inspected only for reference or
  legacy behavior:
  - reference repos may be cloned under `/Users/asimi/Downloads/references`
    if needed to inspect packaging or adapter examples, but changes land only
    in `secure-tunnel`.

## Detailed Requirements / Acceptance Criteria

### A) Real carrier adapters exist

- [x] Implement a raw `QUIC` client connector that negotiates the v1 ALPN and
      presents framed duplex records over the selected stream.
- [x] Implement a `WSS` client connector that negotiates the v1 subprotocol and
      presents the same framed duplex record contract.
- [x] Keep carrier-specific TLS, stream, close, and framing behavior below the
      transport abstraction.

### B) Selector semantics are preserved

- [x] `QUIC` success remains preferred when available.
- [x] Fallback to `WSS` occurs only for documented fallback-eligible outer
      failures before `Secure Ready`.
- [x] Inner trust failures do not trigger `WSS` fallback.

### C) Adapter tests are real enough to support SDK work

- [x] Add local integration coverage for successful `QUIC`, successful `WSS`,
      fallback from `QUIC` to `WSS`, and malformed target failure.
- [x] Verify adapter behavior against active `v1-*` docs and descriptor
      validation.
- [x] `mise run dev` passes.

## Cross-Repo Boundaries

- Primary implementation boundary: Rust carrier adapters and local test
  harnesses.
- Parser / upstream dependency boundary: new transport dependencies may be
  added only with repo supply-chain checks intact.
- Downstream integration boundary: no native SDK packaging in this task.
- External asset / catalog / fixture boundary: local fixtures only.
- If another repository is read-write, state what is implemented there versus
  what is implemented in this repository.
  - none.

## Task Dependencies

- backlog/tasks/completed/task-00000012_prototype-quic-preferred-transport-with-wss-fallback-and-local-secure-session.md
- backlog/tasks/completed/task-00000018_define-product-sdk-facade-and-session-contract.md
- backlog/plans/plan-00000002_product-secure-tunnel-sdk-and-bindings.md

## Reference Tasks

- backlog/tasks/completed/task-00000001_consider-starter-crates.md

## Backlog Bookkeeping

- `backlog/` is local planning state and is not version controlled by default.
- Moving a task to `backlog/tasks/completed/` is separate from pushing code.
- Commit history for work tracked by this task may live in a different
  read-write repository than this backlog entry.

## Implementation Notes

- [x] Implementation notes added with command evidence.
- Added `secure-tunnel-transport`, a production adapter crate that keeps
  `tokio`, `quinn`, WebSocket, DNS, TLS, and socket I/O outside
  `secure-tunnel-core`.
- Implemented production `QuicConnector` and `WssConnector` with shared
  `FramedDuplex` record framing. `QUIC` uses one client-initiated
  bidirectional stream with v1 ALPN. `WSS` uses one binary WebSocket message per
  record with the v1 subprotocol and inbound Tungstenite frame/message caps set
  to `MAX_RECORD_PAYLOAD_SIZE`.
- Added `TransportClientConfig` with platform TLS verification by default and a
  root-DER override for local integration tests and future custom-CA work.
- Wired `SecureTunnelClient::new` to production `QUIC`/`WSS` ports and
  `SnowNxClientEvaluator`, while keeping `with_ports` test-only for the pure
  SDK tests.
- Extended core/SDK error taxonomy for terminal outer path, TLS, proxy, and
  protocol failures. `QUIC` fallback-eligible connection failures still use
  `TransportFallback(...)`; `WSS` failures are terminal.
- Added in-process TLS `QUIC` and `WSS` test servers and real secure-ready
  adapter tests covering `QUIC` success, cached `WSS` success, ALPN fallback,
  close-before-secure-ready fallback, malformed `WSS` target failure,
  oversized `WSS` message rejection, and inner trust failure with no `WSS`
  fallback.
- Updated `deny.toml` to explicitly allow `ISC` and `CDLA-Permissive-2.0`,
  both introduced by the Rustls/TLS dependency graph.
- Verification evidence:
  - `cargo test -p secure-tunnel-transport` passed: 11 tests.
  - `cargo clippy --workspace --all-targets --all-features --no-deps -- -D warnings` passed.
  - `cargo doc -p secure-tunnel-transport --no-deps` passed.
  - `mise run rust:audit` passed after the explicit license allowlist update.
  - `mise run dev` passed after review fixes: 58 Rust tests, Rust doctests,
    Python tests, Go tests, and Go-WASM tests.
- Line-count evidence: changed non-Markdown source files remain below 500
  lines; the largest changed source file is `crates/transport/src/tests/server.rs`
  at 346 lines.

## Implementation Plan

### Plan Decisions

- Keep `secure-tunnel-core` as the functional / Sans-I/O boundary. Do not add
  `tokio`, `quinn`, WebSocket, TLS, DNS, or socket dependencies to core.
- Add a dedicated production adapter crate, tentatively
  `crates/transport` / `secure-tunnel-transport`, that depends on
  `secure-tunnel-core` and owns carrier I/O side effects.
- Keep the SDK facade public surface stable. Wire production adapters into the
  SDK through its private `TransportPorts` implementation instead of exposing
  `quinn`, WebSocket, TLS, selector, or Noise internals.
- Treat custom CA and proxy as follow-up tasks `00000013` and `00000014`.
  Task 19 may add internal/test-only root handling to exercise local TLS, but
  it should not design the final SDK-facing managed-network configuration.

### Steps

1. Add the production adapter crate and dependency boundary.
   - Add `secure-tunnel-transport` to the workspace with modules for
     `config`, shared record framing, `quic`, and `wss`.
   - Depend on `secure-tunnel-core` plus the runtime/transport crates selected
     for implementation (`tokio`, `quinn`, `tokio-tungstenite`/`tungstenite`,
     `rustls`/platform verifier or native roots, `url`, and test-only TLS
     fixture crates as needed).
   - Keep reusable, deterministic codec helpers pure and unit-tested; keep all
     socket/TLS/WebSocket/QUIC work inside adapter modules.

2. Tighten error taxonomy before connecting real networks.
   - Preserve `ApiError::TransportFallback(...)` as the only fallback request,
     and only allow it for `QUIC` before `Secure Ready`.
   - Add terminal outer-carrier error variants if needed so production
     adapters can distinguish path, TLS, proxy, close, and protocol failures
     without collapsing them into `Internal`.
   - Update `secure-tunnel-sdk` error mapping so terminal WSS/outer failures
     surface as stable SDK classes such as `OuterPathFailure`,
     `OuterTlsFailure`, and `OuterProxyFailure`.
   - Add focused unit tests for these mappings and for attempt reports.

3. Implement the `QUIC` carrier connector.
   - Build a client `quinn::Endpoint` with v1 ALPN from the descriptor target,
     SNI from `sni_override` or `connect_host`, and no v1 use of 0-RTT or
     datagrams.
   - Open exactly one client-initiated bidirectional stream after connection
     establishment.
   - Implement `FramedDuplex` over that stream using the documented
     `u16be payload_length` + payload record mapping and the existing v1 size
     limits.
   - Map pre-`Secure Ready` UDP/path failures to
     `TransportFallback(OuterPathFailure)`, ALPN/version/capability rejection
     to `TransportFallback(OuterQuicRejected)`, stream-open early close to
     `TransportFallback(OuterQuicClosedEarly)`, and post-stream early close to
     `TransportClosed` so the secure-ready evaluator normalizes it to
     `OuterQuicClosedEarly`.

4. Implement the `WSS` carrier connector.
   - Parse and validate the descriptor URL with a structured URL parser, then
     connect via TCP + TLS and request the v1 WebSocket subprotocol.
   - Confirm the server-selected subprotocol before returning carrier-ready
     framed I/O.
   - Implement `FramedDuplex` with one binary WebSocket message per secure
     record; reject text messages and oversized binary messages at the adapter
     layer.
   - Keep ping/pong, WebSocket close frames, TLS shutdown, and carrier close
     details below the transport abstraction. Do not emit WSS fallback
     requests.

5. Wire production ports into the SDK.
   - Replace `SecureTunnelClient::new`'s unavailable default ports with
     production `QUIC` + `WSS` connectors and `SnowNxClientEvaluator`.
   - Keep `SecureTunnelClient::with_ports` available under `cfg(test)` for
     pure SDK fallback/cancellation/session tests.
   - Do not expose production connector concrete types through the SDK facade
     unless a later binding task proves a caller-owned injection point is
     necessary.

6. Add real local integration coverage.
   - Build local in-process `QUIC` and `WSS` test servers with ephemeral TLS
     certificates and the v1 ALPN/subprotocol.
   - Reuse or extract the existing scripted Noise responder fixture so adapter
     tests reach actual `Secure Ready` rather than stopping at carrier-ready.
   - Cover successful `QUIC`, successful `WSS`, fallback from failed `QUIC` to
     `WSS`, malformed target failure, and inner trust failure with no WSS
     fallback.
   - Assert attempt reports, selected carrier, cache snapshot, ALPN,
     subprotocol, and record mapping behavior against the active `v1-*` docs.

7. Run the completion bar and bookkeeping.
   - Verify incrementally with targeted adapter tests first, then
     `cargo fmt --all -- --check`, targeted `cargo test`/`cargo clippy` for the
     affected crates, `cargo doc` for affected public crates, and finally
     `mise run dev`.
   - Check non-Markdown code files touched by this task remain at or below 500
     lines, splitting modules before review if needed.
   - Run an independent review / fix / re-review loop until there are no
     unresolved high- or medium-severity findings.
   - Update implementation notes and acceptance criteria, update the parent
     plan, move the task to `backlog/tasks/completed/`, describe the jj change,
     push `main`, and leave a fresh empty jj working-copy change.

## Review Notes

- Independent reviewer `Raman` found three medium issues:
  - `QUIC` `open_bi` failure happened before secure-ready owned the transport
    and therefore needed to request `OuterQuicClosedEarly` fallback directly.
  - `WSS` needed a Tungstenite `WebSocketConfig` cap at
    `MAX_RECORD_PAYLOAD_SIZE` instead of default 64 MiB/16 MiB incoming limits.
  - SDK `connect` rustdoc still claimed runtime neutrality even though default
    production ports use Tokio-backed I/O.
- Fixes added regression coverage for `QUIC` close-before-secure-ready fallback
  and oversized `WSS` message rejection, then updated the SDK rustdoc.
- Re-review by the same reviewer found no unresolved high/medium findings.

## Acceptance Closure

- [x] All acceptance criteria are satisfied and marked.
- [x] Verification commands and outcomes are recorded.
- [x] No unresolved high/medium findings remain.
