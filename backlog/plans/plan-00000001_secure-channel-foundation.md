---
status: active
normative: false
supersedes: []
superseded_by: []
---

# Plan `00000001` - `secure channel foundation`

## Metadata

- Date: `2026-03-14`
- Status: `active`
- Owner: `Asim Ihsan`
- Related Plans: `none`
- Related Tasks: `task-00000001, task-00000003, task-00000004, task-00000005, task-00000006, task-00000007, task-00000008, task-00000009, task-00000010, task-00000011, task-00000012, task-00000013, task-00000014`

## Summary

This plan turns the initial research for Secure Tunnel into a concrete v1 protocol and implementation foundation. The repo already has a generated multi-language scaffold, but the security model, transport split, fallback behavior, and crate boundaries are still only partially captured in notes. The later research clarification freezes the transport direction as raw `QUIC` over UDP when available, with `WSS` as the compatibility fallback, while keeping the inner Noise, trust-anchor, and post-handshake auth model unchanged. The immediate goal is to update the backlog so implementation starts from a transport-agnostic protocol core, explicit transport-selection policy, and a UDP-first operational model instead of the earlier WSS-first draft.

## Decision Summary (Locked)

- [x] v1 uses transport selection with outer `QUIC` preferred and outer `WSS` as graceful fallback, while inner Noise remains the end-to-end boundary.
- [x] v1 uses server-auth `NX` in v1 and defers returning-device client-static-in-handshake variants.
- [x] v1 avoids Noise early data, QUIC `0-RTT`, and QUIC DATAGRAM in v1, and keeps login plus device actions inside completed Noise transport messages.
- [x] v1 uses a shipped trust anchor plus root-signed server Noise key metadata instead of TOFU.
- [x] v1 defines a transport-agnostic core protocol with separate `QUIC` and `WSS` bindings and a transport-selection policy.

## Goals

- Define a precise v1 security and trust model for interception-resistant application traffic.
- Produce a concrete protocol specification for one inner protocol carried over both `QUIC` and `WSS`.
- Define transport-selection, fallback, and cache behavior around a `QUIC`-first policy.
- Establish Rust crate boundaries and responsibilities that preserve transport independence.
- Capture UDP-first deployment and observability requirements early enough to guide implementation.

## Non-Goals

- Add QUIC-specific application semantics that WSS cannot match in v1.
- Use QUIC DATAGRAM, QUIC `0-RTT`, or Noise early data in v1.
- Deliver full Flutter or iOS app integration in the first slice.

## Current State / Baseline

- The repository scaffold is present and builds locally.
- Earlier completed docs captured a WSS-first baseline for the protocol and threat model.
- The later research clarification in `backlog/docs/historical/2026-03-14_initial-research.md` updates the transport direction to raw `QUIC` first, `WSS` fallback, one inner protocol, and transport selection as policy.
- Stable `v1-*` docs now exist for the active threat model, transport policy, service descriptor, shared protocol, and device policy, while the dated WSS-first docs remain historical baseline artifacts.

## Document Governance

- Stable `v1-*.md` docs under `backlog/docs/` are the active source of truth
  for implementation work.
- Dated backlog docs remain informative historical baseline artifacts unless a
  newer doc marks itself as their active normative successor.
- Planning docs stay informative; normative protocol and policy decisions belong
  in the active `v1-*` docs.
- Backlog task status and doc status are intentionally decoupled:
  - a task can remain `proposed` while an active draft doc exists
  - a task becomes `completed` only after the backlog acceptance workflow is
    actually closed

## Gap Analysis

### Protocol definition

- Status: active repo-local docs now define the transport-agnostic core plus
  distinct `QUIC` and `WSS` bindings.
- Remaining impact: implementation can still drift if code references the older
  WSS-first artifacts instead of the stable `v1-*` docs.
- Notes: the earlier WSS binding doc remains useful as historical background,
  but no longer serves as the active implementation target.

### Transport selection and fallback

- Status: active policy doc now defines `QUIC`-first selection,
  `secure-ready`, fallback classes, cache shape, and reprobe rules.
- Remaining impact: client behavior will still drift if future implementation
  shortcuts collapse inner trust failures into generic fallback events.
- Notes: v1 still prefers sequential `QUIC` attempts with a short budget over
  concurrent handshake racing.

### Architectural boundaries

- Missing: a crate/module plan that keeps the secure-channel engine independent from concrete transports while introducing explicit `tunnel_transport`, `tunnel_transport_quic`, and `tunnel_transport_ws` seams.
- Impact: `QUIC` or WebSocket details may leak into the security layer and make later fallback behavior harder to maintain.
- Notes: the research now argues for transport adapters over a shared framed-duplex abstraction rather than a WSS-shaped core.

### Enrollment and trust policy

- Missing: implementation of the now-documented device-policy rules in protocol
  and architecture work.
- Impact: downstream tasks must preserve the documented distinction between known-device reauthentication and new-device enrollment.
- Notes: task `00000006` now defines continuity semantics, challenge/response ordering, and optional App Attest scope.

### Deployment and observability

- Missing: repo-local operational guidance for UDP-first rollout, `QUIC` address-validation posture, fallback telemetry, and migration-related testing.
- Impact: implementation could land without the metrics and runbook assumptions needed to operate `QUIC` safely on hostile or degraded networks.
- Notes: the later research calls deployment and observability first-class once `QUIC` is in scope.

### Enterprise network compatibility

- Missing: backlog tasks that explicitly cover client operation behind private
  PKI, TLS interception, and HTTP proxy requirements on managed networks.
- Impact: the first real carrier adapters could work in direct-connect
  environments yet still fail in the enterprise network classes already called
  out by the threat model.
- Notes: these concerns belong above the inner secure-channel trust model and
  should extend the outer-carrier compatibility story rather than weaken it.

## Strategy

- Keep the earlier security decisions, but supersede the older WSS-first path with transport-selector documentation before writing network code.
- Keep v1 intentionally narrow: `QUIC` preferred, `WSS` fallback, one reliable stream or message lane, post-handshake login and device auth, no early data, no datagrams.
- Keep the protocol core transport-agnostic while making transport selection and carrier-specific bindings explicit.
- Split design work so crate evaluation, protocol specification, selection policy, architecture, and deployment guidance can advance independently but still converge into one coherent implementation plan.
- Use the resulting tasks to drive the first executable Rust slices: framed transport traits, transport-neutral Noise engine, trust verification, `QUIC` binding, `WSS` fallback binding, and end-to-end local secure session.
- Treat deployment and observability guidance as an advisory input for some
  architecture and prototype work, not a blanket blocker for every earlier
  implementation slice.
- Follow the first working `QUIC`/`WSS` prototype with explicit client
  compatibility tasks for optional custom CA trust and optional proxied `WSS`
  so managed-network support lands as deliberate scope instead of ad hoc
  adapter flags.

## Phase Plan

- Current Phase: `Phase 1 - shape implementation seams`
- Phase Summary:
  - `Phase 0` is complete: the baseline v1 decisions, device policy, and
    historical protocol spec are captured and the active `v1-*` docs now carry
    the implementation-facing source of truth.
  - `Phase 1` is in progress: the active transport-policy and
    protocol/binding docs cover much of tasks `00000007` and `00000008`,
    `task-00000001` now provides the starter crate set for the first prototype
    slices, `task-00000009` remains only partially addressed, and crate/API
    architecture work is still open.
  - `Phase 2` has started with `task-00000010` complete: the shared selector
    and framed-duplex proving slice is in place, while `task-00000011` and
    `task-00000012` still depend on the remaining backlog closure around
    transport policy, protocol/binding docs, and deployment guidance.

### Phase 0 - `lock v1 decisions`

- Objective: turn the research note into explicit v1 design decisions and identify what the older WSS-first artifacts no longer cover.
- Candidate Tasks:
    - `task-00000003` `define threat model and v1 protocol decisions`
    - `task-00000006` `define device enrollment and known-device policy`
    - `task-00000004` `write v1 core protocol spec and wss binding`
- Exit Criteria:
    - [x] v1 threat model and non-goals are written down.
    - [x] protocol pattern, trust anchor model, device-policy assumptions, and anti-replay stance are explicit.
  Status note: `task-00000003`, `task-00000006`, and `task-00000004` capture the earlier baseline; later tasks in this plan supersede the WSS-first assumptions without discarding those historical artifacts.

### Phase 1 - `shape implementation seams`

- Objective: define the Rust architecture that will host the protocol cleanly.
- Candidate Tasks:
    - `task-00000001` `consider starter crates`
    - `task-00000005` `define rust crate boundaries and secure-channel api`
    - `task-00000007` `define transport selection and fallback policy`
    - `task-00000008` `write transport-agnostic v1 protocol plus quic and wss bindings`
    - `task-00000009` `define udp-first deployment and observability requirements`
- Exit Criteria:
    - [x] secure-channel, transport, and auth/session responsibilities are separated.
    - [ ] `QUIC` preference and `WSS` fallback behavior are documented precisely enough to implement.
    - [x] initial crate choices are concrete enough to support the first proving slices.
    - [x] initial implementation order is clear enough to begin coding without structural churn.

### Phase 2 - `start proving slices`

- Objective: begin the smallest end-to-end implementation slices against the approved design.
- Candidate Tasks:
    - `task-00000010` `implement framed duplex abstraction and transport selector`
    - `task-00000011` `prototype server-auth noise handshake and trust verification on transport-neutral frames`
    - `task-00000012` `prototype quic-preferred transport with wss fallback and local secure session`
- Exit Criteria:
    - [ ] a local Rust path can complete the inner handshake and enter transport mode.
    - [ ] local validation covers both `QUIC` success and `WSS` fallback paths.
    - [ ] implementation work is driven by protocol docs instead of ad hoc decisions.

### Phase 3 - `managed-network compatibility`

- Objective: extend the first real client transport paths to operate in managed
  environments that require private outer-TLS trust or explicit `WSS` proxying.
- Candidate Tasks:
    - `task-00000013` `allow optional custom ca cert for intercepted wss or quic`
    - `task-00000014` `allow optional http proxy for wss client`
- Exit Criteria:
    - [ ] the client can optionally trust a configured outer-TLS CA for
      compatible `WSS` and `QUIC` deployments without weakening inner trust.
    - [ ] the client can optionally route `WSS` through a configured HTTP proxy
      without changing transport-selection semantics.
    - [ ] compatibility work distinguishes outer network-policy failures from
      inner trust failures in tests and observability.

## Backlog Task Map

| Task ID | Title | Phase | Depends On | Status |
|---|---|---|---|---|
| task-`00000001` | `consider starter crates` | `Phase 1` | `none` | `completed` |
| task-`00000003` | `define threat model and v1 protocol decisions` | `Phase 0` | `none` | `completed` |
| task-`00000004` | `write v1 core protocol spec and wss binding` | `Phase 0` | `task-00000003, task-00000006` | `completed` |
| task-`00000005` | `define rust crate boundaries and secure-channel api` | `Phase 1` | `task-00000001, task-00000006, task-00000007, task-00000008` | `completed` |
| task-`00000006` | `define device enrollment and known-device policy` | `Phase 0` | `task-00000003` | `completed` |
| task-`00000007` | `define transport selection and fallback policy` | `Phase 1` | `task-00000003, task-00000004, task-00000006` | `proposed` |
| task-`00000008` | `write transport-agnostic v1 protocol plus quic and wss bindings` | `Phase 1` | `task-00000003, task-00000004, task-00000006, task-00000007` | `proposed` |
| task-`00000009` | `define udp-first deployment and observability requirements` | `Phase 1` | `task-00000007, task-00000008` | `proposed` |
| task-`00000010` | `implement framed duplex abstraction and transport selector` | `Phase 2` | `task-00000005, task-00000007, task-00000008` | `completed` |
| task-`00000011` | `prototype server-auth noise handshake and trust verification on transport-neutral frames` | `Phase 2` | `task-00000005, task-00000008, task-00000010` | `proposed` |
| task-`00000012` | `prototype quic-preferred transport with wss fallback and local secure session` | `Phase 2` | `task-00000005, task-00000008, task-00000009, task-00000010, task-00000011` | `proposed` |
| task-`00000013` | `allow optional custom ca cert for intercepted wss or quic` | `Phase 3` | `task-00000009, task-00000012` | `proposed` |
| task-`00000014` | `allow optional http proxy for wss client` | `Phase 3` | `task-00000009, task-00000012, task-00000013` | `proposed` |

## Dependency Notes

- `task-00000009` remains an advisory input for `task-00000005` so crate and
  API work stay aware of telemetry and deployment assumptions, but it is not a
  hard blocker for defining the architecture.
- `task-00000009` also remains an advisory input for `task-00000011` so the
  first trust-verification prototype emits compatible failure distinctions, but
  deployment guidance should not block a local transport-neutral proof slice.
- `task-00000007` and `task-00000008` still show `proposed` because the active
  docs already exist as active implementation-facing artifacts, but the
  backlog acceptance workflow for those tasks has not yet been explicitly
  closed.
- `task-00000013` and `task-00000014` intentionally land after
  `task-00000012` so private-CA and proxy compatibility can target the first
  real `WSS` and `QUIC` client adapters rather than the earlier mock seams.
- `task-00000014` depends on `task-00000013` so proxy-path work reuses the
  first custom-CA config decisions instead of inventing a second overlapping
  outer-TLS trust surface.

## Validation Strategy

- Unit/Integration: protocol docs must map directly to testable invariants and first implementation slices must come with local tests.
- CLI/manual checks: use local Rust integration harnesses before any mobile integration.
- Regression safeguards: keep parity checks between documented message flow, fallback rules, trust validation, and the first Rust APIs.
- Definition of Done:
    - [ ] all Phase 0 and Phase 1 task docs exist with acceptance criteria
    - [ ] v1 decisions are explicit enough to block contradictory implementation work
    - [ ] first proving-slice tasks can begin without reopening core protocol questions

## Risks and Mitigations

| Risk | Trigger | Mitigation | Owner |
|---|---|---|---|
| Protocol overreach in v1 | adding DATAGRAM, 0-RTT, or concurrent transport racing too early | keep v1 limited to `QUIC` preferred, `WSS` fallback, one inner protocol, and post-handshake device auth | Asim Ihsan |
| Security semantics stay fuzzy | implementation starts before trust and enrollment policy are explicit | complete Phase 0 tasks before major implementation tasks | Asim Ihsan |
| Architecture couples transport and security layers | `QUIC` or WebSocket details leak into the secure-channel core | define crate boundaries before coding transport integration | Asim Ihsan |
| UDP path is unreliable in real networks | fallback and observability rules stay implicit | document selection policy and deployment telemetry before transport prototypes | Asim Ihsan |

## Open Questions

| Question | Needed By | Owner | Resolution |
|---|---|---|---|
| How much of the framing and session envelope should be transport-agnostic from day one? | before task-00000005 completes | Asim Ihsan | `resolved by task-00000005` |
| What exact attestation evidence schema should the protocol standardize beyond the current opaque optional enrollment payload? | before task-00000008 completes | Asim Ihsan | `open` |

## Immediate Next Actions

1. Close the acceptance workflow for the active `v1-*` transport-policy and protocol/binding docs represented by tasks `00000007` and `00000008`, and finish the remaining deployment/observability guidance for `task-00000009`.
2. Begin `task-00000011` against the exported selector, framed transport, and secure-ready artifact seams from completed `task-00000010`.
3. Begin `task-00000012` after the remaining Phase 1 backlog workflow is explicitly closed and the transport-neutral secure-ready prototype from `task-00000011` is accepted.
4. After the first real client adapters exist, schedule `task-00000013` and
   `task-00000014` to cover private-CA and proxied-`WSS` operation on managed
   networks.

## Implementation Notes

- Plan created from the initial research note in `backlog/docs/historical/2026-03-14_initial-research.md`.
- Task `00000003` completed with `backlog/docs/historical/2026-03-14_v1-threat-model-and-decisions.md`, which locks the v1 threat model and baseline protocol decisions.
- Account keys are explicitly out of scope for v1; account identity remains above the secure channel unless a later plan changes that decision.
- Task `00000006` completed with `backlog/docs/historical/2026-03-14_device-enrollment-and-known-device-policy.md`, which defines device continuity semantics, enrollment flow, reconnect challenge policy, and optional App Attest scope.
- Task `00000004` completed with `backlog/docs/historical/2026-03-14_v1-core-protocol-and-wss-binding.md`, which defines the framed core protocol, `NX` handshake flow, trust validation, post-handshake session/device flows, and the earlier WSS mapping.
- The later research clarification in `backlog/docs/historical/2026-03-14_initial-research.md` supersedes the WSS-first execution plan and requires new Phase 1 tasks for transport selection, `QUIC` binding, `WSS` fallback, and UDP-first operations.
- The active implementation-facing docs now live under stable `v1-*` filenames,
  including threat model, transport policy, service descriptor, shared
  protocol, glossary, and device policy artifacts.
- `2026-03-15`: phase tracking refreshed so the plan status is `active`,
  `Phase 0` is marked complete, and `Phase 1` is explicitly identified as the
  current active phase while tasks `00000007` and `00000008` remain open for
  backlog workflow closure and `task-00000009` remains only partially covered
  by the current active docs.
- `2026-03-15`: `task-00000001` completed with
  `backlog/docs/2026-03-15_starter-crate-recommendations.md`, which locks the
  first-choice Phase 2 crate stack and records which dependency decisions are
  still provisional.
- `2026-03-15`: `task-00000005` completed with
  `backlog/docs/2026-03-15_rust-crate-boundaries-and-secure-channel-api.md`
  plus the first transport-neutral Rust API in `crates/core/src/`, which
  fixes ownership boundaries across the future secure core, selector, carrier
  adapters, and session layer without forcing the multi-crate split early.
- `2026-03-15`: `task-00000010` completed with
  `backlog/docs/2026-03-15_framed-duplex-selector-implementation.md` plus the
  selector implementation in `crates/core/src/selector.rs`, which adds the
  `QUIC`-first selection skeleton, secure-ready artifact seam, cache/reprobe
  updates, normalized exhausted-fallback reporting, and transport-neutral tests
  before the real carrier adapters land.

## Completion Checklist

- [ ] All planned tasks created under `backlog/tasks/task-<id>.md`
- [ ] All task acceptance criteria checked
- [ ] Validation strategy executed
- [ ] Plan status updated to `completed`
- [ ] Plan moved to `backlog/plans/completed/`

## Changelog

- `2026-03-14` `Initial plan created from the research note and bootstrap backlog state.`
- `2026-03-14` `Plan updated after later research clarification to treat raw QUIC as the preferred outer transport and WSS as fallback, with one unchanged inner protocol.`
- `2026-03-15` `Refreshed phase tracking to mark Phase 0 complete, set the plan status to active, and identify Phase 1 as the current execution phase.`
