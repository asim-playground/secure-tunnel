# Task `00000017` - `decompose core modules before sdk expansion`

## Summary

Split oversized Rust core modules before the product SDK and generated binding
surface expand around them.

## Motivation

The current `selector.rs`, `noise.rs`, and `prototype_transport.rs` files are
useful proving-slice code, but each exceeds the repo's 500-line non-Markdown
code-file review threshold. The SDK plan should not freeze those large modules
into public contracts before the responsibilities are easier to review.

## Read-Write Repository

- Primary read-write repository: `/Users/asimi/workplace/secure-tunnel`
- Secondary read-write repository/repositories (if applicable): none
- State explicitly:
  - code changes are expected to land in `crates/core/src/` and any new
    internal modules needed to preserve the current public Rust API.
  - this repository itself is expected to change.

## Read-Only Reference Repository

- Read-only reference repository/repositories: none
- State explicitly which repositories may be inspected only for reference or
  legacy behavior:
  - none.

## Detailed Requirements / Acceptance Criteria

### A) Oversized modules are split

- [ ] `crates/core/src/selector.rs` is split into smaller modules with each
      non-Markdown code file at or below 500 lines.
- [ ] `crates/core/src/noise.rs` is split into smaller modules with each
      non-Markdown code file at or below 500 lines.
- [ ] `crates/core/src/prototype_transport.rs` is split into smaller test
      modules with each non-Markdown code file at or below 500 lines.

### B) Behavior and public API are preserved

- [ ] Existing public exports from `secure-tunnel-core` continue to compile
      unless the task records a deliberate, reviewed API adjustment.
- [ ] Existing selector, Noise, trust, and prototype transport tests continue
      to cover the same behavior after the split.
- [ ] Rustdoc remains present on public contracts introduced or moved by the
      decomposition.

### C) Reviewability improves before SDK work

- [ ] The resulting module layout has clear ownership for selection policy,
      attempt classification, Noise handshake, Noise transport mode, and
      prototype fixtures.
- [ ] `mise run dev` passes.
- [ ] Independent review finds no unresolved high/medium issues.

## Cross-Repo Boundaries

- Primary implementation boundary: Rust core module organization only.
- Parser / upstream dependency boundary: no dependency upgrades are expected.
- Downstream integration boundary: do not add Swift, Kotlin, or Python APIs in
  this task.
- External asset / catalog / fixture boundary: no external assets expected.
- If another repository is read-write, state what is implemented there versus
  what is implemented in this repository.
  - none.

## Task Dependencies

- backlog/tasks/completed/task-00000016_update-runtimes-deps-and-add-swift-callable-library-surface.md
- backlog/plans/plan-00000002_product-secure-tunnel-sdk-and-bindings.md

## Reference Tasks

- backlog/tasks/completed/task-00000010_implement-framed-duplex-abstraction-and-transport-selector.md
- backlog/tasks/completed/task-00000011_prototype-server-auth-noise-handshake-and-trust-verification-on-transport-neutral-frames.md
- backlog/tasks/completed/task-00000012_prototype-quic-preferred-transport-with-wss-fallback-and-local-secure-session.md

## Backlog Bookkeeping

- `backlog/` is local planning state and is not version controlled by default.
- Moving a task to `backlog/tasks/completed/` is separate from pushing code.
- Commit history for work tracked by this task may live in a different
  read-write repository than this backlog entry.

## Implementation Notes

- [ ] Implementation notes added with command evidence.
- (fill in after completion)

## Implementation Plan

1. Measure current module sizes and identify natural ownership splits.
2. Move selector helpers, Noise handshake/transport helpers, and prototype
   fixtures into smaller modules without changing behavior.
3. Run focused Rust tests, then `mise run dev`.
4. Complete independent review and re-review.

## Review Notes

## Acceptance Closure

- [ ] All acceptance criteria are satisfied and marked.
- [ ] Verification commands and outcomes are recorded.
- [ ] No unresolved high/medium findings remain.
