# Repository Agent Instructions (EqnTide)

Workspace crates: `eqntide` (`src/`), `eqntide-editor` (`crates/eqntide-editor/`), and `eqntide-desktop` (`desktop/`).

## Always-On Code Rules

- Preserve `#![forbid(unsafe_code)]` at crate roots (`src/lib.rs`, `crates/eqntide-editor/src/lib.rs`, `desktop/src/main.rs`).
- Keep `pub` APIs narrow (`pub(super)` / `pub(crate)` for internals), use standard Rust modules (never `#[path = "..."]`), and split oversized files.
- Follow the 7-layer top-down order in non-test `.rs` files (`//!` -> imports -> `const`/`type` -> shared types/APIs -> private types -> private helpers -> `#[cfg(test)] mod tests`).
- Never use single-letter local variables or closure parameters (except `i`/`j`/`k`, `x`/`y`, `r`/`g`/`b`/`a`, `u`/`v`).
- Write all `//!`, `///`, and `//` comments in concise English, and avoid `[...]` intra-doc links from `pub` items to private/`pub(super)` items.
- Use `tempfile` in the same parent directory for atomic file writes and `tempfile::tempdir()` in filesystem tests.

## Required Skills & Workflows

- **Before finishing code edits or committing:** Read and follow `.agents/skills/eqntide-precommit/SKILL.md` and verify all checks pass with zero warnings:
  `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --all-targets --locked`, and `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`.
- **When bumping versions, packaging locally, or preparing a release/beta:** Read and follow `.agents/skills/eqntide-release/SKILL.md`. Keep clean `X.Y.Z` versions synced across `Cargo.toml`, `desktop/Cargo.toml`, and `crates/eqntide-editor/Cargo.toml` (`vX.Y.Z` tag = stable; `vX.Y.Z-beta.1` tag = prerelease).

See `CODING_CONVENTIONS.md` and `README.md` for detailed architecture and test instructions.
