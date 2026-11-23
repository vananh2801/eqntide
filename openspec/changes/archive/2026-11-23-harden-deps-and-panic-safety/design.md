# Design

## Context

See `proposal.md` for motivation and `specs/bridge-and-parser-security/spec.md` for the behavioral contract. Key codebase constraints shaping this change:

- **Workspace configuration:** All crates (`eqntide`, `eqntide-editor`, `eqntide-desktop`) target Rust `edition = "2021"`, and `deny.toml` enforces a strict license allow-list (`MIT`, `Apache-2.0`, `BSD-2-Clause`, `BSD-3-Clause`, `ISC`, `OFL-1.1`, `CC0-1.0`, `Unicode-DFS-2016`, `Unicode-3.0`) across four target triples (`x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`, `x86_64-apple-darwin`, `aarch64-apple-darwin`).
- **XML parsing containment:** `quick-xml` is isolated to `src/xml.rs` and already uses `Reader` (not `NsReader`) with custom stack-based namespace resolution.
- **DOCX packaging:** `zip` is used in production by `src/error.rs` and `src/docx/package.rs` (lazy `Package::open`, `read_bounded_zip_entry`, parallel `rayon` chunk compression, `raw_copy_file`, `write_to_file_atomic`), plus `[dev-dependencies]` in `desktop/Cargo.toml` and test builders in `src/cli.rs`, `src/docx/`, `desktop/src/`, and `tests/`.
- **Bridge and UI panic audit:** `cargo clippy --workspace --lib --bins -- -W clippy::unwrap_used -W clippy::expect_used -W clippy::panic` identifies 17 mutex/condvar `.unwrap()` calls in `src/bridge/session.rs`, 14 `.unwrap()` calls in `crates/eqntide-editor/src/lib.rs`, 10 sites in `desktop/src/app.rs`, and a smaller set of sites across `src/` (`src/docx/`, `src/omml/`, `src/latex_extensions.rs`, `src/service.rs`, `src/xml.rs`, `src/mtef/`, `src/ole.rs`, `src/render/`).

## Goals / Non-Goals

**Goals:**
- Achieve clean `cargo deny check` (advisories, licenses, bans, sources) on all configured targets.
- Eliminate mutex-poisoning panic propagation in `SessionManager` (`src/bridge/session.rs`) while keeping the HTTP bridge API responsive.
- Remove reachable runtime `.unwrap()` / `panic!` crash paths in `desktop/src/app.rs`, `crates/eqntide-editor/src/lib.rs`, and core `src/` parsers.

**Non-Goals:**
- Mechanically rewriting verified compile-time or static invariants (e.g., `include_bytes!` icon/font embeds in `src/render/font.rs` and `desktop/src/app.rs`, `OnceLock` static XML templates, or `desktop/build.rs` build-script `expect()`).
- Changing OOXML, Flat OPC, MTEF, or MathType `Equation.DSMT4` output bytes.

## Decisions

### 1. Upgrade `quick-xml` (`0.31` -> `>= 0.41.0`)
- **Choice:** Bump `quick-xml` to `>= 0.41.0` in `Cargo.toml` and adapt `src/xml.rs` for any API differences (`Reader`, `BytesStart::attributes()`, `escape::unescape`).
- **Alternative considered:** Disabling duplicate attribute checks via `.attributes().with_checks(false)` on `0.31.0`. Rejected because keeping `quick-xml` on `0.31.0` would still leave `RUSTSEC-2026-0195` (`NsReader` advisory) flagged in `Cargo.lock` and weaken duplicate-attribute rejection on malformed XML.

### 2. Upgrade `zip` (`0.6` -> `>= 9.0.5`) with feature & license verification
- **Choice:** Bump `zip` to `>= 9.0.5` in `Cargo.toml` (`default-features = false`, minimal deflate feature compatible with existing `flate2` `rust_backend`) and `desktop/Cargo.toml` `[dev-dependencies]`. Update `src/docx/package.rs` (`SimpleFileOptions` replacing `FileOptions`, `f.last_modified()` handling `Option<zip::DateTime>`) and test builders across `src/`, `desktop/src/`, and `tests/`. Retain all existing `MAX_DOCX_ENTRY_BYTES` and `MAX_DOCX_TOTAL_BYTES` enforcement.
- **Alternative considered:** Adding `ignore` entries in `deny.toml` for `RUSTSEC-2026-0367`/`0368`/`0369`/`0429`/`0545`/`0548` on `zip 0.6.6` (since `eqntide` never calls `ZipArchive::extract`, `ZipStreamReader`, or `aes-crypto`). Keep this only as an explicit fallback if `zip >= 9.0.5` cannot build under `edition = "2021"` / workspace MSRV or introduces a transitive license outside `deny.toml`; even then, do not relax `deny.toml`'s `[licenses]` allow-list.

### 3. Lock poisoning recovery in `SessionManager`
- **Choice:** Centralize lock/condvar access in `src/bridge/session.rs` using `unwrap_or_else(PoisonError::into_inner)` for `sessions` map and `SessionEntry.data` / `Condvar::wait_timeout`, combined with defensive cleanup or state validation where an in-flight mutation could have been interrupted. This prevents a panic in one thread from causing permanent panic cascades across `get_latest_session_id`, `create_session_with_target`, `get_initial_mathml`, `get_session`, `metadata`, `update_session`, `cancel_session`, `wait_session`, `remove_session`, and `cleanup_stale` without breaking public method signatures.
- **Alternative considered:** Switching every `SessionManager` method to return `Result<..., BridgePoisonedError>` and changing `std::sync::Mutex` to `parking_lot::Mutex`. Rejected for now because `parking_lot` adds a new workspace dependency and changing all signatures ripples into `desktop` and tests unnecessarily; `PoisonError::into_inner()` solves poisoning cleanly with zero new dependencies and preserves API compatibility.

### 4. Targeted panic/unwrap hardening in `eqntide-editor`, `eqntide-desktop`, and `eqntide`
- **Choice:** Replace unguarded `.unwrap()` calls that operate on mutable UI/editor state or parser inputs:
  - In `desktop/src/app.rs`: replace `is_some()` + `.unwrap()` (`latex_error`, `command_input`), `FENCES.iter().find(...).unwrap()` (fallback to `"parentheses"` label), and node-lookup `.unwrap()` calls in `move_vertical` / `enter_subtree` / structural navigation with `let ... else` / `if let Some(...)`. Leave `tabler_icon_bytes` asset lookup (`include_bytes!`) or `expect()` on embedded PNGs untouched unless converting `tabler_icon_bytes` to return `Option<&'static [u8]>` is cleaner for callers.
  - In `crates/eqntide-editor/src/lib.rs`: replace `.unwrap()` after `find_mut`, `id_at_path`, `node`, and `max_by_key` in AST editing/caret movement with defensive `let ... else { return; }` or `unwrap_or` fallbacks.
  - In `src/`: inspect remaining `unwrap()`/`expect()` sites (`src/docx/job.rs`, `src/omml/encode/normalization.rs`, `src/omml/encode.rs`, `src/latex_extensions.rs`, `src/service.rs`, `src/xml.rs`, `src/mtef/decode.rs`, `src/ole.rs`, `src/render/layout/bigop.rs`) and replace any path reachable from malformed input or dynamic state with `Result`/`Option` propagation; leave verified compile-time/static invariants untouched.

## Risks / Trade-offs

- **Transitive dependency / license fallout from `zip >= 9.0.5`** → Verify `cargo deny check` across all 4 targets immediately after updating `Cargo.toml` / `Cargo.lock`; minimize enabled `zip` features (`default-features = false`) to avoid pulling `bzip2`/`zstd`/`aes` crates with unwanted licenses or build scripts.
- **Silent state corruption after `PoisonError::into_inner()`** → Ensure `SessionManager` locks hold no partial multi-step invariants across panic points (mutations on `SessionData` are single-field/state-enum assignments, and `cleanup_stale` safely skips or drops entries whose state cannot be trusted) and add unit tests exercising poisoned locks.
- **Behavioral drift in `eqntide-editor` when replacing `.unwrap()` with early returns** → Keep all existing `eqntide-editor` unit tests passing and add regression tests for boundary/empty-tree edge cases rather than silently masking bugs.
