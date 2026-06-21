# Task `00000017` - `decompose core modules before sdk expansion`

## Final Summary

Task `00000017` is complete. The oversized core modules were split without
changing public exports or behavior: selector tests, Noise tests, prototype
transport tests, and prototype scripted-responder fixtures now live in smaller
sibling modules. All non-Markdown code files are at or below the repo's
500-line review threshold, `mise run dev` passes, and independent review found
no high/medium issues.

## Summary

Split oversized Rust core modules before the product SDK and generated binding
surface expand around them.

## Motivation

The current `selector.rs`, `noise.rs`, and `prototype_transport.rs` files are
useful proving-slice code, but each exceeded the repo's 500-line non-Markdown
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

- [x] `crates/core/src/selector.rs` is split into smaller modules with each
      non-Markdown code file at or below 500 lines.
- [x] `crates/core/src/noise.rs` is split into smaller modules with each
      non-Markdown code file at or below 500 lines.
- [x] `crates/core/src/prototype_transport.rs` is split into smaller test
      modules with each non-Markdown code file at or below 500 lines.

### B) Behavior and public API are preserved

- [x] Existing public exports from `secure-tunnel-core` continue to compile
      unless the task records a deliberate, reviewed API adjustment.
- [x] Existing selector, Noise, trust, and prototype transport tests continue
      to cover the same behavior after the split.
- [x] Rustdoc remains present on public contracts introduced or moved by the
      decomposition.

### C) Reviewability improves before SDK work

- [x] The resulting module layout has clear ownership for selection policy,
      attempt classification, Noise handshake, Noise transport mode, and
      prototype fixtures.
- [x] `mise run dev` passes.
- [x] Independent review finds no unresolved high/medium issues.

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

- [x] Implementation notes added with command evidence.
- Split `selector.rs` by moving its test module into
  `crates/core/src/selector/tests.rs`. The production selector module is now
  478 lines and owns selection policy plus attempt classification.
- Split `noise.rs` by moving its test module into
  `crates/core/src/noise/tests.rs`. The production Noise module is now 210
  lines and owns secure-ready evaluation plus Noise transport-mode framing.
- Split `prototype_transport.rs` by moving:
  - prototype transport tests into
    `crates/core/src/prototype_transport/tests.rs`
  - scripted responder fixtures into
    `crates/core/src/prototype_transport/scripted_responder.rs`
- Post-split line-count check:
  - `selector.rs`: 478 lines
  - `selector/tests.rs`: 415 lines
  - `noise/tests.rs`: 410 lines
  - `prototype_transport.rs`: 361 lines
  - `prototype_transport/tests.rs`: 250 lines
  - `prototype_transport/scripted_responder.rs`: 218 lines
- Focused verification:
  - `cargo fmt --all --check` passed.
  - `cargo test -p secure-tunnel-core` passed with 30 tests.
  - `find crates -name '*.rs' -not -path '*/target/*' -print0 | xargs -0 wc
    -l | awk '$1 > 500 {print}'` printed no file paths.
  - `mise run copyright-check` passed after adding headers to split test files.
- Full verification:
  - `mise run dev` passed, including format, lint, strict Clippy, Rust
    nextest, doctests, Python tests, Go tests, and Go/WASM tests.

## Implementation Plan

1. Measure current module sizes and identify natural ownership splits.
2. Move selector helpers, Noise handshake/transport helpers, and prototype
   fixtures into smaller modules without changing behavior.
3. Run focused Rust tests, then `mise run dev`.
4. Complete independent review and re-review.

## Review Notes

- Independent review found no high/medium findings.
- Low review finding: the newly split Rust test files initially missed the
  repo's MPL/SPDX header. Fixed by adding the standard headers and rerunning
  `cargo fmt --all --check`, `cargo test -p secure-tunnel-core`, the file-size
  scan, and `mise run copyright-check`.

## Acceptance Closure

- [x] All acceptance criteria are satisfied and marked.
- [x] Verification commands and outcomes are recorded.
- [x] No unresolved high/medium findings remain.
