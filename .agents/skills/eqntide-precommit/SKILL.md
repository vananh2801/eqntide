---
name: eqntide-precommit
description: Review and clean up modified code against EqnTide coding conventions before committing. Use when the user asks to check, clean up, lint, review, or prepare code changes for commit.
---

# EqnTide Pre-Commit Code Review & Cleanup

Apply these checks to modified files before committing. Work from `git status --short` and `git diff` first so edits stay scoped to the change.

## 1. Diff-First Scope

1. Inspect `git status --short` and `git diff` to list all new or modified files.
2. Check only touched files and directly affected callers/tests; avoid unrelated refactors.
3. For broader architectural context, consult `CODING_CONVENTIONS.md` only if needed.

## 2. Module Layout & Visibility

- **Memory safety:** Preserve `#![forbid(unsafe_code)]` at crate roots (`src/lib.rs`, `crates/eqntide-editor/src/lib.rs`, `desktop/src/main.rs`). Never introduce `unsafe`.
- **Minimal visibility:** Keep crate-root `pub` APIs narrow; prefer `pub(super)` or `pub(crate)` for internal sharing.
- **Standard Rust modules:** Use `mod.rs` or directory modules cleanly; never use `#[path = "..."]`.
- **File size:** Split non-test `.rs` files exceeding ~600–800 lines into cohesive submodules; keep large test suites (~200+ lines) in `tests.rs` or a `tests/` directory.
- **Desktop `app/` boundary:** Keep shared `App` fields grouped by subsystem in `desktop/src/app/mod.rs` and subsystem `impl App` methods in their matching submodule.

## 3. Top-Down 7-Layer Declaration Order (Non-Test `.rs` Files)

1. `//!` module summary (plus an ASCII flow diagram on major orchestration/pipeline modules)
2. `#![...]` crate attributes (roots only), `mod` declarations, and `use` imports
3. `const` & `type` (`pub` -> `pub(crate)`/`pub(super)` -> private)
4. Exported/shared `struct`, `enum`, `trait` + `impl` blocks (constructors -> `is_`/`has_`/`can_` predicates -> mutations/UI draw) + module-level shared functions
5. Private `struct` / `enum` definitions + `impl` blocks
6. Private helper functions (orchestrators first, leaf utilities below)
7. `#[cfg(test)] mod tests` / `mod tests;` at the bottom

## 4. Naming & Documentation

- **Method prefixes:**
  - Boolean predicates: `is_*`, `has_*`, `can_*`, `should_*`
  - Conversions: `as_*` (cheap borrow), `to_*` (new allocation), `into_*` / `from_*`
  - UI/render/handlers: verb-first (`draw_*`, `handle_*`, `process_*`, `sync_*`, `hit_test_*`, `render_*`, `rasterize_*`, `sanitize_*`, `resolve_*`)
- **No single-letter locals or closure params** (`a`, `b`, `c`, `d`, `e`, `f`, `n`, `p`, `r`, `s`, `t`, `w`, `z`, etc.).
  - Permitted exceptions only: loop indices (`i`, `j`, `k`), 2D coordinates (`x`, `y`), RGBA channels (`r`, `g`, `b`, `a`), texture UVs (`u`, `v`).
- **Avoid test helper shadowing:** In `desktop/src/app/tests/*.rs`, initialize via `let mut app = super::helpers::app();`.
- **English comments & docs:** All `//!`, `///`, and `//` comments must be concise English. Document all `pub`, `pub(crate)`, and `pub(super)` types, variants, constants, key fields, and core functions.
- **Clean intra-doc links:** Never link `[...]` from a `pub` item's docstring to a private or `pub(super)` item (use plain backticks instead); disambiguate module vs. function collisions (`crate::fn()` vs `mod@crate::mod`).

## 5. Safety & Test Invariants

- **Atomic file writes:** Stage `.docx`, `settings.json`, or managed assets into a `tempfile` inside the same parent directory and commit via `persist` / `persist_noclobber`.
- **UI & worker boundaries:** Background threads in `desktop/` report only via `EventSender<Event>`; batch/conversion jobs honor cancellation flags (`AtomicBool`).
- **Isolated tests:** Filesystem tests must use `tempfile::tempdir()` and leave zero artifacts outside temporary directories.

## 6. Mandatory Pre-Commit Verification Gate

Run all four workspace checks and fix any warning or failure before committing:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

Also run applicable targeted checks when matching files changed:
- `Cargo.toml`, `Cargo.lock`, or `deny.toml`: `cargo deny --all-features --workspace --locked check advisories licenses bans sources`
- `scripts/` or `tests/python/`: `python3 -m py_compile scripts/package_native.py && python3 -m unittest discover -s tests/python -p "test_*.py"`
- `wps-addin/`: `node --check wps-addin/main.js && node --check wps-addin/taskpane.js` (and `npm test --prefix tests/wps` if installed)
- `.github/workflows/*.yml`: validate YAML syntax
