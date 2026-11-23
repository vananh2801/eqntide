# Proposal

## Why

`cargo deny check` currently fails on security advisories for `quick-xml 0.31.0` (`RUSTSEC-2026-0194`, `RUSTSEC-2026-0195`) and `zip 0.6.6` (`RUSTSEC-2026-0367`, `RUSTSEC-2026-0368`, `RUSTSEC-2026-0369`, `RUSTSEC-2026-0429`, `RUSTSEC-2026-0545`, `RUSTSEC-2026-0548`). In addition, production runtime paths—most notably `SessionManager` in `src/bridge/session.rs` and interactive UI/AST navigation paths in `desktop/src/app.rs` and `crates/eqntide-editor/src/lib.rs`—contain unguarded `.lock().unwrap()`, `.wait_timeout(...).unwrap()`, and `.unwrap()`/`panic!` calls that can propagate mutex poisoning or crash the bridge and desktop editor.

## What Changes

- Upgrade `quick-xml` from `0.31` to `>=0.41.0` (and adapt `src/xml.rs` if needed) to eliminate `RUSTSEC-2026-0194` and `RUSTSEC-2026-0195`.
- Upgrade `zip` from `0.6` to `>=9.0.5` across root `Cargo.toml` and `desktop/Cargo.toml` (or, if blocked by `deny.toml` license/edition constraints, document an explicit scoped alternative before proceeding), adapting `src/error.rs`, `src/docx/package.rs`, and test/CLI helpers to the updated `zip` API while preserving OOXML/DOCX round-trip compatibility and `cargo deny check` compliance.
- Harden `SessionManager` in `src/bridge/session.rs` (and any affected callers in `src/bridge/server.rs`) against mutex poisoning by recovering/handling `PoisonError` safely instead of panicking on `.lock().unwrap()` or `Condvar::wait_timeout(...).unwrap()`.
- Replace risky runtime `.unwrap()` and `panic!` calls in `desktop/src/app.rs` and `crates/eqntide-editor/src/lib.rs` (and audit production `.unwrap()`/`expect()`/`panic!` sites in `src/`) with defensive handling or graceful fallbacks while leaving true compile-time invariants (such as `include_bytes!` assets) untouched.

## Capabilities

### New Capabilities

<!-- None: dependency/parser hardening and bridge session resilience fit within existing capability boundaries. -->

### Modified Capabilities

- `bridge-and-parser-security`: Require `SessionManager` lock/condvar operations in the local HTTP bridge to handle poisoned synchronization primitives without panicking, and require XML/DOCX dependencies (`quick-xml`, `zip`) to pass `cargo deny check` advisories and license policy without weakening archive/XML bounds.

## Impact

- **Dependencies & Tooling:** `Cargo.toml`, `Cargo.lock`, `desktop/Cargo.toml`, and `deny.toml` (only if scoped justification or policy adjustment is required and verified).
- **Library & Bridge:** `src/xml.rs`, `src/error.rs`, `src/docx/package.rs`, `src/bridge/session.rs`, and test helpers in `src/docx/` and `src/cli.rs`.
- **Desktop & Editor:** `desktop/src/app.rs`, `desktop/src/batch.rs`, `crates/eqntide-editor/src/lib.rs`, and integration tests in `tests/`.
- **Verification:** `cargo deny check`, `cargo clippy --workspace --all-targets -- -W clippy::all`, `cargo fmt --all --check`, and `./tests/run_all.sh`.
