# Task `00000003` - `define threat model and v1 protocol decisions`

## Summary

Turn the research note into an explicit v1 threat model, security boundary definition, and set of locked decisions.

## Motivation

The current repo has strong direction but not yet a crisp written statement of what v1 defends against, what it intentionally does not defend against, and which protocol choices are fixed versus deferred.

## Detailed Requirements / Acceptance Criteria

### A) Threat model is explicit

- Document the interception, compromise, replay, and enrollment assumptions for v1.
- Distinguish the roles of outer TLS, inner Noise, session/login state, server keys, device keys, and any optional account key.
- State what remains out of scope for v1, including at least one non-goal around early data and one around platform compromise.

### B) Core v1 decisions are locked

- Record the initial Noise pattern direction for v1 and any deferred alternatives.
- Record the trust-anchor and server-key rotation model.
- Record whether login and device enrollment occur before or after Noise transport is established.

## Task Dependencies

- backlog/docs/historical/2026-03-14_initial-research.md
- backlog/plans/plan-00000001_secure-channel-foundation.md

## Implementation Notes

- Completed by writing `backlog/docs/historical/2026-03-14_v1-threat-model-and-decisions.md`.
- Locked the v1 boundary around outer `HTTPS/WSS`, inner Noise, server-auth `NX`, root-signed server identity, and post-handshake login/device flows.
- Explicitly recorded no-early-data and no-returning-device-mutual-auth-handshake scope for v1.
- Later doc-set governance work preserved that output as a historical baseline
  and moved the active normative successor to
  `backlog/docs/v1-threat-model-and-transport-decisions.md`.
