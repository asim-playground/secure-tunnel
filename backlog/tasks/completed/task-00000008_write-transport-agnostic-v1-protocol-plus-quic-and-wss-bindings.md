# Task `00000008` - `write transport-agnostic v1 protocol plus quic and wss bindings`

## Final Summary

Task `00000008` is complete. The active normative protocol docs now define one
transport-agnostic secure-channel core with separate `QUIC` and `WSS` carrier
bindings, one descriptor shape with per-carrier targets, shared Noise/trust
semantics, shared device-policy integration, and optional opaque enrollment
attestation evidence rather than a platform-specific App Attest schema.

## Summary

Write the updated repo-local v1 protocol specification for one inner
secure-channel core carried over both `QUIC` and `WSS`.

## Motivation

The earlier protocol task documented a solid WSS-first baseline, but the later
research changed the transport direction. The repo now needs a superseding
protocol artifact that keeps the inner Noise, trust, and session model stable
while splitting carrier-specific behavior into separate `QUIC` and `WSS`
bindings.

## Read-Write Repository

- Primary read-write repository: `/Users/asimi/workplace/secure-tunnel`
- Secondary read-write repository/repositories (if applicable): none
- State explicitly:
  - documentation and backlog changes land in this repository.
  - no external repository is modified for this task.

## Read-Only Reference Repository

- Read-only reference repository/repositories: none
- State explicitly which repositories may be inspected only for reference or
  legacy behavior:
  - historical WSS-first protocol docs remain informative background only.

## Detailed Requirements / Acceptance Criteria

### A) Core protocol flow is transport-agnostic

- [x] Specify the shared secure-channel lifecycle through outer-carrier
      establishment, framed transport creation, Noise handshake, trust
      verification, transport mode, session open, login, device auth, app
      messages, and encrypted close.
- [x] Define the shared prologue, protocol versioning, service identity
      binding, handshake-hash use, and size limits.
- [x] Incorporate the device-enrollment and known-device policy produced by
      task `00000006`.

### B) QUIC binding is specified

- [x] Specify raw `QUIC` over UDP as the preferred outer carrier.
- [x] Define ALPN, the single bidirectional stream rule for v1, frame encoding
      on that stream, and error/close mapping assumptions.
- [x] Explicitly document the v1 exclusion of QUIC DATAGRAM and `0-RTT`.

### C) WSS binding remains available as fallback

- [x] Specify the `WSS` URI shape, subprotocol string, binary-frame mapping,
      and close handshake expectations.
- [x] Ensure the binding remains semantically aligned with the shared core
      protocol and `QUIC` path.
- [x] Identify any invariants the first Rust implementation must satisfy
      across both carriers.

### D) Unresolved protocol inputs are assigned

- [x] Decide whether the first public bootstrap/service descriptor should carry
      one logical service target with multiple carriers or distinct per-carrier
      targets.
- [x] Define the protocol payload or envelope shape, if any, for optional App
      Attest evidence so task `00000005` does not need to invent it during
      architecture work.

## Cross-Repo Boundaries

- Primary implementation boundary:
  - active protocol and descriptor documentation in `secure-tunnel`.
- Parser / upstream dependency boundary: none.
- Downstream integration boundary:
  - Rust core, FFI facade, and SDK tasks must preserve the shared core protocol
    and avoid exposing carrier-specific protocol identities as separate
    security models.
- External asset / catalog / fixture boundary: none.
- If another repository is read-write, state what is implemented there versus
  what is implemented in this repository:
  - none.

## Task Dependencies

- backlog/docs/historical/2026-03-14_initial-research.md
- backlog/tasks/completed/task-00000003_define-threat-model-and-v1-protocol-decisions.md
- backlog/tasks/completed/task-00000004_write-v1-protocol-spec-for-wss-plus-noise.md
- backlog/tasks/completed/task-00000006_define-device-enrollment-and-known-device-policy.md
- backlog/tasks/completed/task-00000007_define-transport-selection-and-fallback-policy.md
- backlog/plans/plan-00000001_secure-channel-foundation.md

## Reference Tasks

- backlog/tasks/completed/task-00000005_define-rust-crate-boundaries-and-secure-channel-api.md
- backlog/tasks/completed/task-00000011_prototype-server-auth-noise-handshake-and-trust-verification-on-transport-neutral-frames.md

## Backlog Bookkeeping

- `backlog/` is local planning state and is not version controlled by default.
- Moving a task to `backlog/tasks/completed/` is separate from pushing code.
- Commit history for work tracked by this task may live in a different
  read-write repository than this backlog entry.

## Implementation Notes

- [x] Implementation notes added with command evidence.
- Normative deliverables:
  - `backlog/docs/v1-core-protocol-quic-and-wss-bindings.md`
  - `backlog/docs/v1-service-descriptor-and-bootstrap-config.md`
  - `backlog/docs/v1-device-enrollment-and-known-device-policy.md`
- The protocol doc supersedes the historical WSS-first protocol artifact and
  defines:
  - protocol id, `QUIC` ALPN, `WSS` subprotocol, and Noise suite
  - framed record model and v1 size limits
  - descriptor-derived Noise prologue
  - server-key authorization placement and validation
  - shared lifecycle from carrier ready through encrypted close
  - `QUIC` single bidirectional stream mapping
  - `WSS` binary-message mapping
- The descriptor doc resolves the bootstrap question as one logical descriptor
  with per-carrier targets.
- The App Attest decision is intentionally conservative for v1:
  - optional attestation evidence may travel inside encrypted enrollment finish
    payloads
  - absence of App Attest must not make v1 unusable
  - no platform-specific App Attest schema is frozen by this task
- Rust vocabulary check:
  - `PROTOCOL_ID_V1`, `QUIC_ALPN_V1`, `WSS_SUBPROTOCOL_V1`, and
    `NOISE_SUITE_V1` match the active docs.
  - `ServiceDescriptor` represents one logical descriptor with per-carrier
    `CarrierSet` targets.
- Verification commands:
  - `mise run markdown-lint` passed.
  - `mise run dev` passed, including format, lint, Rust tests, doctests,
    Python tests, Go tests, and Go/WASM tests.

## Implementation Plan

1. Audit the active protocol, descriptor, and device-policy docs against task
   acceptance criteria.
2. Check Rust constants and descriptor types for consistency with the active
   docs.
3. Refresh this task into the current template, record evidence, and move it
   to completed.
4. Run repository verification and independent review with tasks `00000007`
   and `00000009`.

## Review Notes

- First independent review for the combined tasks `00000007`, `00000008`, and
  `00000009` closure slice found two medium consistency findings: pending
  review closure text in completed tasks and stale deployment/observability gap
  analysis in `plan-00000001`.
- Fixups updated the task review closure text, the `plan-00000001` gap
  analysis, the plan `00000002` managed-network dependency table, and the
  task `00000022` redaction requirement.
- Focused re-review found no remaining high/medium findings in the task
  `00000007`/`00000008`/`00000009` closure scope.

## Acceptance Closure

- [x] All acceptance criteria are satisfied and marked.
- [x] Verification commands and outcomes are recorded.
- [x] No unresolved high/medium findings remain.
