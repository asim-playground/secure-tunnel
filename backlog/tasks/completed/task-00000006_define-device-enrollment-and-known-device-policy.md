# Task `00000006` - `define device enrollment and known-device policy`

## Summary

Define how Secure Tunnel will treat device keys, device enrollment, known-device reauthentication, and higher-risk new-device registration.

## Motivation

The research note correctly points out that device keys only add as much security value as the enrollment policy around them. That policy should be explicit before device-auth code or message formats are implemented.

## Detailed Requirements / Acceptance Criteria

### A) Device trust semantics are defined

- Define what a device key proves in v1 and what it does not prove.
- Distinguish known-device reauthentication from new-device enrollment.
- State whether v1 includes only device continuity, or also stronger gated enrollment semantics.

### B) Enrollment and challenge flow is defined

- Specify the post-Noise enrollment flow, including challenge-response and channel binding requirements.
- Specify the reconnect flow for known devices, including the server challenge shape and replay/freshness requirements.
- State whether App Attest is in scope for v1, later, or optional.

## Task Dependencies

- backlog/docs/historical/2026-03-14_initial-research.md
- backlog/tasks/task-00000003_define-threat-model-and-v1-protocol-decisions.md
- backlog/plans/plan-00000001_secure-channel-foundation.md

## Implementation Notes

- Completed by writing `backlog/docs/historical/2026-03-14_device-enrollment-and-known-device-policy.md`.
- Locked v1 around device continuity and known-device reauthentication, while leaving stronger new-device gating as a policy layer above the protocol.
- Defined post-Noise enrollment and reconnect challenge flows, including channel binding and freshness requirements.
- Marked App Attest as optional enrollment-time enhancement in v1 rather than a mandatory protocol requirement.
- Later doc-set governance work preserved that output as a historical baseline
  and moved the active normative successor to
  `backlog/docs/v1-device-enrollment-and-known-device-policy.md`.
