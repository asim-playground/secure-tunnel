# Backlog Docs

Use this directory for reference docs that support backlog plans, tasks, and
implementation work.

## Conventions

- Stable `v1-*.md` files are the current active design docs.
- Dated files such as `2026-03-14_*.md` are research snapshots or historical
  baselines unless their front matter says otherwise.
- Major docs use front matter to declare:
  - `status`
  - `normative`
  - `supersedes`
  - `superseded_by`

## Status Model

- Directory location controls archive lifecycle.
  - Top-level `backlog/docs/` is current working surface.
  - `backlog/docs/historical/` is archival baseline material.
- Front matter controls semantic role.
  - `active` means current source of truth.
  - `historical` means informative archive.
  - `draft` means current but not yet ratified.
- Task and plan status track backlog workflow, not just whether a draft doc
  exists. A proposed task may still have an active draft doc attached to it.

## Active V1 Docs

- `v1-threat-model-and-transport-decisions.md`
- `v1-transport-selection-and-fallback-policy.md`
- `v1-service-descriptor-and-bootstrap-config.md`
- `v1-core-protocol-quic-and-wss-bindings.md`
- `v1-device-enrollment-and-known-device-policy.md`
- `v1-glossary.md`

## Historical Baseline

- `historical/2026-03-14_initial-research.md`
- `historical/2026-03-14_v1-threat-model-and-decisions.md`
- `historical/2026-03-14_v1-core-protocol-and-wss-binding.md`
- `historical/2026-03-14_device-enrollment-and-known-device-policy.md`

## Working Notes

- `2026-03-15_doc-set-consistency-review.md`
- `2026-03-15_plan-phase-tracking-refresh.md`
- `2026-03-15_starter-crate-recommendations.md`
