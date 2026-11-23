# Tasks

## 1. Dependency Upgrades & Advisory Compliance

- [x] 1.1 Upgrade `quick-xml` to `>=0.41.0` in `Cargo.toml`, update `src/xml.rs` if any parser/attribute/unescape API signatures differ, and verify `cargo test --lib xml` passes with `RUSTSEC-2026-0194` and `RUSTSEC-2026-0195` cleared.
- [x] 1.2 Upgrade `zip` to `>=9.0.5` (with minimal `default-features = false` deflate features) in `Cargo.toml` and `desktop/Cargo.toml`, migrate `src/error.rs`, `src/docx/package.rs` (`SimpleFileOptions`, `last_modified`), and test ZIP builders (`src/docx/`, `src/cli.rs`, `desktop/src/`, `tests/`), and verify `cargo test --workspace` compiles and passes DOCX round-trip and zip-bomb size-limit tests.
- [x] 1.3 Run `cargo deny check` across all configured targets to verify advisories, licenses, bans, and sources pass cleanly (or record an explicitly justified fallback in `deny.toml` only if `zip >=9.0.5` cannot satisfy `edition = "2021"` / `deny.toml` license constraints).

## 2. Bridge Session Poisoning Resilience

- [x] 2.1 Refactor `SessionManager` lock and `Condvar::wait_timeout` handling in `src/bridge/session.rs` to recover safely from `PoisonError` instead of panicking on `.unwrap()`, preserving all public method contracts and stale EMF cleanup behavior.
- [x] 2.2 Add unit tests in `src/bridge/session.rs` that intentionally poison the `sessions` and `SessionEntry.data` mutexes and verify `create_session_with_target`, `get_latest_session_id`, `get_initial_mathml`, `get_session`, `metadata`, `update_session`, `cancel_session`, `wait_session`, `remove_session`, and `cleanup_stale` complete without panicking under `cargo test --lib bridge::session`.

## 3. Runtime Panic Hardening Across Desktop, Editor, and Core Parsers

- [x] 3.1 Replace reachable runtime `.unwrap()` calls in `crates/eqntide-editor/src/lib.rs` (AST node lookup, `find_mut`, `id_at_path`, `canonical_stops`) with defensive early returns or fallbacks, add unit tests covering missing/boundary node paths, and verify `cargo test -p eqntide-editor` passes.
- [x] 3.2 Replace reachable runtime `.unwrap()` / `panic!` paths in `desktop/src/app.rs` (`FENCES` lookup, `latex_error`, `command_input`, `move_vertical` / navigation helpers) without altering verified compile-time asset invariants, and verify `cargo test -p eqntide-desktop` passes.
- [x] 3.3 Audit and harden non-static `.unwrap()` / `expect()` sites in `src/` (`src/docx/job.rs`, `src/omml/encode*`, `src/latex_extensions.rs`, `src/service.rs`, `src/xml.rs`, `src/mtef/decode.rs`, `src/ole.rs`, `src/render/layout/bigop.rs`) so malformed or unexpected runtime inputs return errors or safe fallbacks instead of panicking, and verify via `cargo clippy --workspace --lib --bins -- -W clippy::unwrap_used -W clippy::expect_used -W clippy::panic` and `cargo test --lib`.

## 4. Workspace Verification

- [x] 4.1 Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -W clippy::all`, `cargo deny check`, and `./tests/run_all.sh` to verify zero warnings, zero advisories, and full conversion/bridge/editor test compatibility.
