# Task `00000033` - `stabilize public go module release layout`

## Summary

Stabilize the public native Go SDK module path, source layout, and native
library distribution contract so Go consumers can use released artifacts outside
the monorepo checkout.

## Motivation

Task `00000027` recorded the native Go SDK as an internal source artifact during
the dry-run release lane. The task started with module path `secure_tunnel`, Go
sources nested below the module root, and cgo relying on generated C headers and
native libraries that lived outside the release artifact. That was sufficient
for repo-local smoke tests, but it was not a durable public Go release shape.

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

- [x] Choose and document the Go module path for public or internal
      distribution.
- [x] Decide whether the Go SDK is released from a subdirectory module, a
      generated source bundle, or a split repository.
- [x] Update package metadata and release docs to match the chosen identity.

### B) Native dependencies are release-consumable

- [x] Ensure generated C headers required by cgo are inside the released module
      or artifact layout.
- [x] Define how native `secure_tunnel_ffi` libraries are built, named,
      versioned, checksummed, and loaded by consumers.
- [x] Add checks that fail if the released Go artifact omits required headers,
      module files, or native library metadata.

### C) Consumer smoke runs outside the monorepo layout

- [x] Add a smoke test that installs or unpacks the Go artifact into a temporary
      consumer project outside `crates/go`.
- [x] Prove the consumer can compile and run a fixture-backed session smoke.
- [x] Record rollback and compatibility notes for Go consumers.

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

- [x] Implementation notes added with command evidence.
- Chose `github.com/asim-playground/secure-tunnel/crates/go` as the Go module
  path because it is VCS-resolvable inside the current repository without a
  split repository.
- Moved the public Go package sources to the module root `crates/go` and named
  the package `securetunnel`.
- Updated cgo flags to include the module-root `binding.h` and platform native
  library folders under `native/<goos>-<goarch>/`.
- Updated `scripts/sdk_release.py` to stage a clean Go release module artifact
  with `go.mod`, Go sources, `binding.h`, `README.md`, `LICENSE`,
  `native.json`, and the host `secure_tunnel_ffi` dynamic library.
- Added `mise run sdk:go:smoke-release`, which unpacks the Go release artifact
  outside the monorepo, creates a fresh consumer module with a `replace` to the
  unpacked artifact, sets the documented native loader path, and runs a
  fixture-backed session smoke.
- Validation evidence:
  - `mise run sdk:release:check-metadata` passed with module path
    `github.com/asim-playground/secure-tunnel/crates/go` and package
    `securetunnel`.
  - `mise run sdk:go:check-package` passed.
  - `mise run sdk:go:smoke-release` passed and generated a Go artifact with
    module files plus `native/linux-arm64/libsecure_tunnel_ffi.so`.
  - `mise run sdk:release:dry-run` passed on Linux with the external Go
    release smoke integrated into the release lane.
  - `mise run lint-all` passed.
  - `mise run dev` passed: 97 Rust tests, Python tests/smokes, FastAPI smoke,
    and Go tests.

## Implementation Plan

1. [x] Decide the module path and artifact topology.
2. [x] Move or generate headers/native metadata into the release-consumable layout.
3. [x] Add external-consumer Go smoke coverage and manifest checks.
4. [x] Update release docs and run the SDK release dry-run.

## Review Notes

- Independent review by Copernicus found no unresolved high or medium findings.
- Residual risk: local validation proved the host Linux ARM64 native artifact.
  macOS release behavior is covered by release CI when run on macOS, while
  Windows dry-run remains disabled. Direct `go get` from VCS does not by itself
  deliver generated native bundle artifacts; publication remains dry-run/manual
  per the release policy.

## Acceptance Closure

- [x] All acceptance criteria are satisfied and marked.
- [x] Verification commands and outcomes are recorded.
- [x] No unresolved high/medium findings remain.
