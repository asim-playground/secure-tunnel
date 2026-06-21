# Task `00000030` - `build python fastapi server and rust client e2e`

## Summary

Add the first Python server runtime for Secure Tunnel and prove Rust client
interoperability against it.

## Motivation

`task-00000023` proves generated Swift, Kotlin, and Python clients can call the
shared Rust SDK facade against a Rust server fixture. The product plan also
needs the reverse direction: a Python FastAPI server path that can host the
secure tunnel service semantics and interoperate with the Rust client. This
task keeps that work separate from UniFFI generation so the Python server can
make an explicit PyO3, maturin, or UniFFI packaging decision.

## Read-Write Repository

- Primary read-write repository: `/Users/asimi/workplace/secure-tunnel`
- Secondary read-write repository/repositories (if applicable): none
- State explicitly:
  - code changes are expected to land in Python server code, Rust/Python
    packaging tasks, local fixtures, and end-to-end tests.
  - this repository itself is expected to change.

## Read-Only Reference Repository

- Read-only reference repository/repositories:
  - `/Users/asimi/Downloads/references/fastapi`
  - `/Users/asimi/Downloads/references/pyo3`
  - `/Users/asimi/Downloads/references/maturin`
  - `/Users/asimi/Downloads/references/uniffi-rs`
- State explicitly which repositories may be inspected only for reference or
  legacy behavior:
  - reference repositories are read-only; implementation changes land only in
    `secure-tunnel`.

## Detailed Requirements / Acceptance Criteria

### A) Python server strategy is explicit

- [x] Decide whether the Python FastAPI server uses PyO3/maturin, UniFFI
      Python, or a small Python wrapper over a Rust native library.
- [x] Document how service static-key custody, descriptor signing, and pinned
      public-key disclosure differ between client and server package roles.
- [x] Keep the server runtime shape compatible with the same protocol and
      application-message semantics used by the Rust harness.

### B) FastAPI server fixture exists

- [x] Add a local FastAPI server fixture that can serve descriptor/bootstrap
      metadata and host the tunnel service path needed for one smoke scenario.
- [x] Package or load the Rust-backed secure tunnel service code in a
      reproducible Python workflow.
- [x] Keep test keys and fixture descriptors deterministic and clearly marked
      as test-only.

### C) Cross-language e2e passes

- [x] Add Rust-client-to-Python-server end-to-end coverage for descriptor load,
      `Secure Ready`, account auth, one application request, and graceful close.
- [x] Reuse the task-23 fixture semantics so Swift, Kotlin, Python-client, and
      Rust-client smokes all assert the same payload and close behavior.
- [x] Add deterministic `mise` tasks for the Python FastAPI server smoke path.

## Cross-Repo Boundaries

- Primary implementation boundary: Python server runtime and Rust client e2e in
  this repository.
- Parser / upstream dependency boundary: no parser work expected.
- Downstream integration boundary: no downstream app repository is modified by
  this task.
- External asset / catalog / fixture boundary: generated/package artifacts may
  live under `target/` unless a package layout requires tracked metadata.
- If another repository is read-write, state what is implemented there versus
  what is implemented in this repository.
  - none.

## Task Dependencies

- backlog/tasks/completed/task-00000023_create-uniffi-sdk-facade-and-bindgen-tooling.md
- backlog/tasks/completed/task-00000026_package-python-sdk-from-the-shared-rust-facade.md
- backlog/plans/plan-00000002_product-secure-tunnel-sdk-and-bindings.md

## Reference Tasks

- backlog/docs/2026-06-21_sdk-reference-repositories.md
- backlog/tasks/completed/task-00000021_build-end-to-end-tunnel-harness-and-cli-smoke-path.md
- backlog/tasks/completed/task-00000023_create-uniffi-sdk-facade-and-bindgen-tooling.md
- backlog/tasks/completed/task-00000026_package-python-sdk-from-the-shared-rust-facade.md

## Backlog Bookkeeping

- `backlog/` is local planning state and is not version controlled by default.
- Moving a task to `backlog/tasks/completed/` is separate from pushing code.
- Commit history for work tracked by this task may live in a different
  read-write repository than this backlog entry.

## Implementation Notes

- [x] Implementation notes added with command evidence.
- Planning evidence gathered before implementation:
  - `task-00000026` should decide and package the Python SDK first, because the
    FastAPI fixture should consume the same Python package shape instead of
    creating a second binding story.
  - Existing generated Python client smoke already proves Python-client to
    Rust-server interop; this task is the reverse direction.
  - Local references for FastAPI, PyO3, maturin, and UniFFI are present under
    `/Users/asimi/Downloads/references` and are read-only.
  - FastAPI's local docs show `fastapi.testclient.TestClient` for in-process
    app testing, but the Rust-client e2e needs a real subprocess/listener smoke
    so the Rust client exercises the same descriptor and transport path it will
    use outside Python tests.
- Implementation completed:
  - Decision: the Python FastAPI server fixture is an imperative shell around a
    Rust-backed fixture process, not a Python reimplementation of Noise,
    descriptor signing, transport selection, account auth, or application-frame
    semantics.
  - `python/src/secure_tunnel/fastapi_fixture.py` starts
    `secure-tunnel-cli binding-fixture` during FastAPI lifespan startup,
    exposes `/health`, `/descriptor`, `/bootstrap`, and `/binding-fixture`,
    and stops the fixture subprocess during shutdown.
  - Service static-key custody stays with the Rust server fixture. The server
    owns the test-only private key and descriptor signing path; clients receive
    only the bootstrap descriptor, outer TLS roots, and the pinned service
    static public key needed for `NK1` responder authorization.
  - Added `secure-tunnel-cli binding-fixture-client <fixture-json>` and
    `secure_tunnel_harness::run_binding_fixture_client` so a Rust SDK client
    can consume the same fixture report used by generated Swift, Kotlin, and
    Python client smokes.
  - Added deterministic tasks:
    `mise run python:fastapi-test` and
    `mise run sdk:python-fastapi-smoke`; the Rust-client-to-FastAPI smoke is
    included in `mise run test` and therefore in `mise run dev`.
  - Review and user-request follow-up: the FastAPI fixture now exposes typed
    server configuration with `FixtureSettings`, `ObservabilitySettings`,
    `FixtureRuntime`, `RustLogLevel`, and `ObservabilityFormat`. The Python
    server can configure the Rust child process with tracing level/format,
    service name, `RUST_LOG`, OTLP endpoint environment, resource attributes,
    fixture binary path, working directory, and startup/shutdown timeouts.
  - Rust `secure-tunnel-cli` now installs a stderr `tracing_subscriber` when
    `SECURE_TUNNEL_OBSERVABILITY` is enabled, preserving stdout for JSON CLI
    protocol output while allowing Python server configuration to plumb through
    to Rust structured tracing.
  - The Python package declares a `server` optional extra for FastAPI/uvicorn,
    and the clean wheel check installs/imports `secure-tunnel[server]` so the
    packaged FastAPI fixture path is covered.
  - FastAPI fixture startup now uses timeout-aware bootstrap reads and kills
    invalid or silent child processes; unit tests cover invalid bootstrap JSON
    and bootstrap timeout cleanup.
- Verification evidence:
  - `mise run python:fastapi-test` passed, covering health, descriptor,
    bootstrap, raw fixture report, observability env plumbing, and lifecycle
    cleanup with `TestClient`.
  - `mise run sdk:python-fastapi-smoke` passed with Rust client output:
    `{"carrier": "quic", "close": "graceful", "language": "rust-client",
    "server": "python-fastapi-fixture"}`.
  - `mise run python:check-wheel` passed after installing the built wheel in a
    clean venv and then installing/importing the `server` extra.
  - Direct CLI observability smoke passed with CLI JSON on stdout and Rust
    tracing JSON on stderr containing `descriptor.validation`.
  - `mise run test` passed, including 86 Rust tests, Python package tests and
    smoke, the new Rust-client-to-FastAPI smoke, Go tests, and Go-WASM tests.
  - `mise run dev` passed.

## Implementation Plan

1. Start only after `task-00000026` has a buildable/importable Python SDK
   package or implement both in one branch with task 26's package path as the
   first milestone.
2. Reinspect the local FastAPI, maturin, PyO3, and UniFFI references for the
   minimum server fixture shape, then record the strategy decision:
   - Preferred path: FastAPI is the Python imperative shell for descriptor,
     health, and process lifecycle; Rust remains the functional/protocol core.
   - The Python server must not implement Noise, transport selection,
     descriptor signing, account auth, or application-frame handling in route
     functions.
   - Use the task-26 package for client/shared types and add a small
     server/testing facade only where the existing Rust SDK facade lacks
     listener lifecycle hooks.
3. Add a deterministic Python FastAPI fixture package or example, with FastAPI
   dependencies kept optional/dev-only unless a production server package is
   intentionally created:
   - `GET /health` or equivalent readiness probe;
   - descriptor/bootstrap endpoint returning the active fixture descriptor;
   - explicit test-only service static key material owned by the server
     fixture, with only the public key disclosed/pinned by clients;
   - lifecycle startup/shutdown that starts and stops the Rust-backed tunnel
     listener cleanly.
4. If Rust server lifecycle hooks are missing, add the narrowest Rust facade
   needed for Python to start a fixture server:
   - keep it separated from the public client SDK surface;
   - expose owned values and coarse operations only;
   - keep Sans-I/O / functional-core behavior in Rust modules and leave Python
     responsible only for wiring, HTTP endpoints, and process lifecycle.
5. Add Python-side tests for the FastAPI app:
   - use `TestClient` for descriptor, health, and fixture lifecycle checks;
   - assert descriptor serial, protocol id, service static public key, and
     test-only metadata are stable and clearly marked.
6. Add the Rust-client-to-Python-server e2e smoke:
   - launch the FastAPI fixture under the repo's `uv` environment as a real
     subprocess/listener;
   - wait for readiness, fetch descriptor/bootstrap metadata, connect with the
     Rust SDK/CLI client, verify `Secure Ready`, authenticate account, send the
     same `smoke-ping`, expect `smoke-pong`, and close gracefully;
   - collect server logs on failure and guarantee cleanup of Python and Rust
     child processes.
7. Add deterministic mise automation, likely:
   - `mise run python:fastapi-test`;
   - `mise run sdk:python-fastapi-smoke`;
   - inclusion in `mise run dev` or `mise run ci` only after the smoke is
     deterministic and not too slow for local iteration.
8. Validate in increasing scope:
   - Python FastAPI unit tests;
   - Rust-client-to-Python-server smoke;
   - task-26 Python package smoke;
   - existing Swift/Kotlin/Python generated-binding smokes where relevant;
   - `mise run dev`.
9. Run the independent review/re-review loop, update Implementation Notes with
   command evidence, tick acceptance criteria, update the parent plan, and move
   the task to `backlog/tasks/completed/` only after implementation is done.

## Review Notes

- Initial independent review found two medium issues:
  - the FastAPI fixture was shipped without declared server dependencies or an
    installed-wheel server-extra check;
  - fixture startup could block or leak the Rust child on invalid or missing
    bootstrap output.
- Fixes added the `server` extra plus wheel-extra import check, timeout-aware
  startup cleanup, explicit raw endpoint `test_only` metadata, and targeted
  cleanup/config tests. Re-review is pending.
- Re-review found one new medium issue: Rust observability stderr used a pipe
  that could back-pressure long-running fixture processes. The fix switched
  child stderr to a `TemporaryFile`, reads it only for failure/shutdown
  diagnostics, and added a 2 MB stderr regression test. Final re-review found
  no remaining high/medium issues.

## Acceptance Closure

- [x] All acceptance criteria are satisfied and marked.
- [x] Verification commands and outcomes are recorded.
- [x] No unresolved high/medium findings remain.
