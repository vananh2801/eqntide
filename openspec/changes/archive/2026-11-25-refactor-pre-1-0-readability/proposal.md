# Proposal: Pre-1.0.0 Readability and Maintainability Refactor

## Why

Before tagging `1.0.0`, several core modules across all three workspace crates have grown into multi-thousand-line monoliths (`desktop/src/app.rs` at 10,695 lines, `src/mtef/decode.rs` at 3,017 lines, `crates/eqntide-editor/src/lib.rs` at 2,842 lines, `src/bridge/server.rs` at 2,052 lines, `src/omml/encode.rs` at 1,801 lines) with inconsistent naming, ad-hoc function ordering, single-letter locals, and sparse architectural comments. Refactoring module boundaries, naming, ordering, and English documentation now stabilizes the `1.0.0` API surface and makes long-term maintenance safe without altering runtime behavior.

## What Changes

- Split oversized monolithic files across `eqntide`, `eqntide-editor`, and `desktop` into cohesive, single-responsibility submodules and separate large inline test suites from production code.
- Finish partially started module splits in `src/mtef/decode/`, `src/omml/encode/`, and `src/mtef/encode/`, and replace the `#[path = "latex_extensions.rs"]` include in `src/service.rs` with a standard module layout.
- **BREAKING** (pre-1.0.0 workspace crate API): Standardize `eqntide-editor::Node` method names to follow Rust conventions (`token` -> `is_token`, `row` -> `is_row`, `opaque` -> `is_opaque`, `math` -> `to_math_node`) and update all workspace callers.
- Standardize internal function and variable naming across all three crates (`is_*`/`has_*`/`can_*`/`should_*` predicates, verb-first action/UI handlers such as `draw_*`/`handle_*`/`process_*`/`sync_*`, descriptive local variables instead of single-letter abbreviations).
- Reorder declarations consistently within every refactored file (module docs -> imports -> constants/types -> public APIs -> private types -> top-down helpers -> tests).
- Add English `//!` module-level workflow summaries (with ASCII data-flow diagrams) and `///` doc comments for structs, key fields, and core functions across all three crates.
- Preserve all observable runtime behavior, CLI flags, HTTP bridge contracts, file formats, and UI interactions, verified continuously with `cargo fmt`, `cargo clippy -- -D warnings`, and the full workspace test suite.

## Capabilities

### New Capabilities

<!-- Purely internal maintainability refactor; no new user-facing capabilities. -->

### Modified Capabilities

<!-- No spec-level behavior changes (`skip_specs: true` in `.openspec.yaml`). -->

## Impact

- **Crates affected**: `eqntide` (`src/`), `eqntide-editor` (`crates/eqntide-editor/`), `eqntide-desktop` (`desktop/`).
- **Public API**: Minor pre-1.0.0 method renames on `eqntide_editor::Node`; external `.docx`/OMML/MTEF/CLI/HTTP bridge behavior remains unchanged.
- **Tooling & CI**: Must continue passing `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --all-targets --locked`, and `cargo doc --workspace --no-deps`.
