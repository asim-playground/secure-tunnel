# Task `00000002` - `bootstrap repository scaffold`

## Summary

Create the initial Secure Tunnel repository scaffold from the Rust copier template and wire the repository for GitHub, Jujutsu, backlog tracking, and local verification.

## Motivation

The repository started as an empty directory with only an initial research note. It needed a reproducible baseline before protocol and implementation work could begin.

## Detailed Requirements / Acceptance Criteria

### A) Repository scaffold exists

- [x] Generate the project from `~/workplace/copier_rust_template`.
- [x] Keep Rust code only for product surface, with Python and Go bindings enabled.
- [x] Disable web/WASM frontend generation.

### B) Repository workflow is ready

- [x] Create the GitHub repository at `https://github.com/asim-playground/secure-tunnel.git`.
- [x] Initialize local Git and Jujutsu state on `main`.
- [x] Bootstrap tracked backlog directories and templates without changing `mise.toml`.

### C) Baseline verification passes

- [x] Ensure `cargo build --workspace` succeeds.
- [x] Ensure `cargo test --workspace` succeeds.
- [x] Ensure `mise run go:test` succeeds.
- [x] Ensure `mise run go-wasm:test` succeeds.

## Task Dependencies

- backlog/docs/historical/2026-03-14_initial-research.md

## Implementation Notes

- Generated the repo in-place with Copier while preserving the existing research note.
- Added `AGENTS.md`, backlog templates, and a tracked starter-crates follow-up task.
- Fixed template issues found during verification and review:
  - FFI crates needed crate-local `unsafe_code = "allow"` instead of the workspace-wide forbid.
  - Core parser now returns overflow errors instead of panicking or wrapping.
  - Native Go bindings reject embedded NUL bytes.
  - Go/WASM allocation handles zero-length buffers safely.
  - Go/WASM singleton initialization no longer depends on request-scoped contexts.
  - Windows CI now runs Rust workspace tests instead of unsupported Go bridge tasks.
  - Generated Go cgo linker path and Go/WASM `.gitignore` output path were corrected.
