# Task `00000023` - `create uniffi sdk facade and bindgen tooling`

## Summary

Add the project-pinned UniFFI facade crate and binding-generation tooling for
Swift, Kotlin, and Python.

## Motivation

The repository needs one generated SDK surface for Swift, Kotlin, and Python,
but that surface must remain smaller and more stable than the internal Rust
implementation. UniFFI is the default path from the binding research for those
three languages, with the manual C ABI retained for Swift bootstrap and Go.
Flutter/Dart and Go are handled by later target-specific tasks over the same
Rust SDK facade.

## Read-Write Repository

- Primary read-write repository: `/Users/asimi/workplace/secure-tunnel`
- Secondary read-write repository/repositories (if applicable): none
- State explicitly:
  - code changes are expected to land in the Rust workspace, generated binding
    directories, task automation, and CI as needed.
  - this repository itself is expected to change.

## Read-Only Reference Repository

- Read-only reference repository/repositories:
  - UniFFI reference repositories may be cloned under
    `/Users/asimi/Downloads/references` if needed.
  - `/Users/asimi/Downloads/references/uniffi-rs`
  - `/Users/asimi/Downloads/references/application-services`
- State explicitly which repositories may be inspected only for reference or
  legacy behavior:
  - cloned UniFFI examples are read-only references; implementation changes
    land only in `secure-tunnel`.

## Detailed Requirements / Acceptance Criteria

### A) UniFFI tooling is pinned

- [ ] Add `uniffi` to the workspace dependency graph with an explicit pinned
      version selected during the task.
- [ ] Add a project-local `uniffi-bindgen` binary so binding generation does
      not depend on globally installed generator versions.
- [ ] Add deterministic `mise` tasks for generating and checking bindings.

### B) Facade crate is deliberately small

- [ ] Add a `secure-tunnel-sdk-ffi` crate or equivalent that depends on the
      Rust SDK facade from `task-00000018`.
- [ ] Prefer a UDL contract unless implementation evidence shows proc macros
      are materially simpler for this repo.
- [ ] Expose only approved SDK records, enums, opaque session/client objects,
      and coarse operations.

### C) Generated bindings compile or import

- [ ] Generate Swift, Kotlin, and Python binding source into stable repo paths.
- [ ] Add smoke tests that import or build each generated binding at the source
      level before native packaging begins.
- [ ] Keep the manual C ABI crate building and documented as compatibility
      during the transition.
- [ ] State explicitly that Flutter/Dart and Go are out of UniFFI scope and are
      covered by `task-00000028` and `task-00000029`.

## Cross-Repo Boundaries

- Primary implementation boundary: UniFFI facade and generated binding source.
- Parser / upstream dependency boundary: no parser work expected.
- Downstream integration boundary: native packaging is handled in later tasks.
- External asset / catalog / fixture boundary: generated bindings are repo
  artifacts if the task decides they should be tracked.
- If another repository is read-write, state what is implemented there versus
  what is implemented in this repository.
  - none.

## Task Dependencies

- backlog/tasks/task-00000018_define-product-sdk-facade-and-session-contract.md
- backlog/tasks/completed/task-00000021_build-end-to-end-tunnel-harness-and-cli-smoke-path.md
- backlog/tasks/task-00000022_add-observability-and-conformance-test-matrix.md
- backlog/plans/plan-00000002_product-secure-tunnel-sdk-and-bindings.md

## Reference Tasks

- backlog/docs/2026-06-21_sdk-reference-repositories.md
- backlog/tasks/completed/task-00000016_update-runtimes-deps-and-add-swift-callable-library-surface.md

## Backlog Bookkeeping

- `backlog/` is local planning state and is not version controlled by default.
- Moving a task to `backlog/tasks/completed/` is separate from pushing code.
- Commit history for work tracked by this task may live in a different
  read-write repository than this backlog entry.

## Implementation Notes

- [ ] Implementation notes added with command evidence.
- (fill in after completion)

## Implementation Plan

1. Verify current UniFFI release, docs, and target-language caveats against
   primary sources before selecting the pinned version.
2. Add project-local bindgen and the facade crate/UDL.
3. Generate Swift, Kotlin, and Python source and add import/build checks.
4. Document how Flutter/Dart and Go reuse the Rust SDK facade outside UniFFI.
5. Run `mise run dev`, binding checks, and independent review.

## Review Notes

## Acceptance Closure

- [ ] All acceptance criteria are satisfied and marked.
- [ ] Verification commands and outcomes are recorded.
- [ ] No unresolved high/medium findings remain.
