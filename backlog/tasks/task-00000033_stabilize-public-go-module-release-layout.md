# Task `00000033` - `stabilize public go module release layout`

## Summary

Stabilize the public native Go SDK module path, source layout, and native
library distribution contract so Go consumers can use released artifacts outside
the monorepo checkout.

## Motivation

Task `00000027` records the native Go SDK as an internal source artifact during
the dry-run release lane. The current module path is `secure_tunnel`, and the
cgo package relies on generated C headers and native libraries that live outside
the Go module root in the monorepo. That is sufficient for repo-local smoke
tests, but it is not a durable public Go release shape.

## Read-Write Repository

- Primary read-write repository: `/home/ubuntu/workplace/secure-tunnel`
- Secondary read-write repository/repositories (if applicable): none
- State explicitly:
  - code changes are expected to land in Go package layout, C ABI artifact
    packaging, release automation, docs, and tests.
  - this repository itself is expected to change.

## Read-Only Reference Repository

- Read-only reference repository/repositories:
  - Go cgo/native package release examples may be cloned under
    `/home/ubuntu/Downloads/references` if needed.
- State explicitly which repositories may be inspected only for reference or
  legacy behavior:
  - examples are read-only; implementation changes land in `secure-tunnel`.

## Detailed Requirements / Acceptance Criteria

### A) Public module identity is explicit

- [ ] Choose and document the Go module path for public or internal
      distribution.
- [ ] Decide whether the Go SDK is released from a subdirectory module, a
      generated source bundle, or a split repository.
- [ ] Update package metadata and release docs to match the chosen identity.

### B) Native dependencies are release-consumable

- [ ] Ensure generated C headers required by cgo are inside the released module
      or artifact layout.
- [ ] Define how native `secure_tunnel_ffi` libraries are built, named,
      versioned, checksummed, and loaded by consumers.
- [ ] Add checks that fail if the released Go artifact omits required headers,
      module files, or native library metadata.

### C) Consumer smoke runs outside the monorepo layout

- [ ] Add a smoke test that installs or unpacks the Go artifact into a temporary
      consumer project outside `crates/go`.
- [ ] Prove the consumer can compile and run a fixture-backed session smoke.
- [ ] Record rollback and compatibility notes for Go consumers.

## Cross-Repo Boundaries

- Primary implementation boundary: repo-local Go SDK package layout, C ABI
  artifacts, release scripts, tests, and docs.
- Parser / upstream dependency boundary: no parser work expected.
- Downstream integration boundary: no downstream consumer repository is modified
  by this task.
- External asset / catalog / fixture boundary: release artifacts only.
- If another repository is read-write, state what is implemented there versus
  what is implemented in this repository.
  - none.

## Task Dependencies

- backlog/tasks/completed/task-00000027_add-sdk-release-ci-and-versioning.md
- backlog/tasks/completed/task-00000029_package-go-sdk-over-stable-c-abi.md
- backlog/tasks/completed/task-00000032_harden-go-sdk-cache-and-failure-reporting.md
- backlog/plans/plan-00000002_product-secure-tunnel-sdk-and-bindings.md

## Reference Tasks

- backlog/tasks/completed/task-00000015_stabilize-ci-portability-and-add-docker-repro.md

## Backlog Bookkeeping

- `backlog/` is local planning state and is not version controlled by default.
- Moving a task to `backlog/tasks/completed/` is separate from pushing code.
- Commit history for work tracked by this task may live in a different
  read-write repository than this backlog entry.

## Implementation Notes

- [ ] Implementation notes added with command evidence.
- Created during task `00000027` after the release dry-run documented native Go
  as an internal source artifact only.

## Implementation Plan

1. Decide the module path and artifact topology.
2. Move or generate headers/native metadata into the release-consumable layout.
3. Add external-consumer Go smoke coverage and manifest checks.
4. Update release docs and run the SDK release dry-run.

## Review Notes

## Acceptance Closure

- [ ] All acceptance criteria are satisfied and marked.
- [ ] Verification commands and outcomes are recorded.
- [ ] No unresolved high/medium findings remain.
