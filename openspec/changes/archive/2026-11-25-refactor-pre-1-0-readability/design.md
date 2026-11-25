# Design: Pre-1.0.0 Readability and Maintainability Refactor

## Context

See `proposal.md` for motivation and scope. Baseline verification passes cleanly (`cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, and workspace tests). The workspace spans ~55.3k lines of Rust across `eqntide` (`src/`), `eqntide-editor` (`crates/eqntide-editor/`), and `eqntide-desktop` (`desktop/`), with several multi-thousand-line files and partially split submodules.

## Goals / Non-Goals

**Goals:**
- Break monolithic files (`desktop/src/app.rs`, `src/mtef/decode.rs`, `crates/eqntide-editor/src/lib.rs`, `src/bridge/server.rs`, `src/omml/encode.rs`, etc.) into focused modules where each file has one clear responsibility.
- Separate multi-thousand-line inline `mod tests` blocks from production code into dedicated `tests.rs` or `tests/` submodules.
- Apply consistent English naming, top-down item ordering, and `//!` workflow + `///` API/helper comments across all three crates.
- Keep `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --all-targets --locked`, and `cargo doc --workspace --no-deps` green after every phase.

**Non-Goals:**
- Changing runtime behavior, equation conversion semantics, CLI options, HTTP bridge protocol, or desktop UI layout/features.
- Restructuring `desktop`'s `App` struct into separate sub-state structs (e.g., `EditorState`, `BatchState`) — `App` keeps its fields for now while its `impl App` methods and tests are split into cohesive submodules (lower risk before `1.0.0`).
- Over-splitting static lookup tables (`src/catalog.rs`, `src/mtef/charmap.rs`, `src/symbols/*`) beyond separating inline tests and organizing helper functions.

## Decisions

### 1. Module Split Architecture Across All Three Crates

- **Decision**: Decompose oversized files into domain-aligned submodules while preserving crate-root re-exports so callers see minimal disruption.
- **Alternative considered**: Keeping single files and only adding section banner comments. Rejected because 2,000–10,700 line files remain difficult to navigate, review, and test in isolation.
- **Target layouts**:
  - `crates/eqntide-editor/src/`:
    - `lib.rs`: module overview (`//!`), re-exports
    - `node.rs`: `Node`, `NodeId`, `is_opaque_tag`, tree lookup & `to_math_node`
    - `stops.rs`: `StopKind`, `StopEntry`, `collect_stops`, `canonical_stops`
    - `navigation.rs`: `Caret`, `Move`, `set_caret`, `move_caret`, `select_all`, `select_node`
    - `editing.rs`: `Editor`, `Snapshot`, `transact`, `undo`/`redo`, `insert_*`, `splice`, `erase_selection`
    - `deletion.rs`: `delete`, `delete_from_row_caret`, `prune_empty_token`, `handle_empty_slot_delete`
    - `structures.rs`: template insertion, `grow_matrix`/`shrink_matrix`, fence/row promotion & removal
    - `tests.rs`: unit and regression tests (~1,000 lines)
  - `src/mtef/decode/`: consolidate the remaining code in `src/mtef/decode.rs` (3,017 lines) into `mod.rs` (overview + public entrypoints `mtef_to_mathml`, `ole_to_mathml`), `consts.rs`, `stream.rs`, `entry.rs`, `parse.rs`, `v5.rs`, `v3.rs`, `templates.rs`, `characters.rs`, `embellishment.rs`, and `tests.rs`.
  - `src/omml/encode/`: consolidate `src/omml/encode.rs` (1,801 lines) alongside existing `fences.rs`, `scripts.rs`, `normalization.rs` by extracting `nary.rs`, `accents.rs`, `font.rs` (`apply_math_font`), `tests.rs`, and keeping `MathmlEncoder` orchestration in `mod.rs`.
  - `src/mtef/encode/` & `src/mtef/templates.rs`: finish splitting encoder helpers in `encode/` and group constants separately from template builders in `templates.rs`.
  - `src/service/`: replace `#[path = "latex_extensions.rs"]` in `src/service.rs` with a standard `src/service/mod.rs` + `src/service/latex_extensions.rs` (separating `ScriptRewriter`, `Scanner`, and lookup tables cleanly).
  - `src/bridge/server/`: split `src/bridge/server.rs` (2,052 lines) into `mod.rs` (server startup & request router), `auth.rs` (bridge token & CORS/origin validation), `http.rs` (request parsing & response helpers), `session_routes.rs`, `convert_routes.rs`, `addin_assets.rs`, and `tests.rs`.
  - `desktop/src/app/`: split `desktop/src/app.rs` (10,695 lines) into `mod.rs` (`App` struct grouped by domain + constructors + `ui_pass` + `eframe::App`), `events.rs` (event loop + async render/LaTeX/background import), `session.rs` (launch args, window requests, save/finish, close), `canvas.rs` (painting, placeholders, hit-testing, caret rect, vertical nav), `keyboard.rs` (key handling, canvas text/IME, command popup), `palette_ui.rs` (sidebar, palette, DPI atlas, footer, status bar), `batch_ui.rs` (batch tab & tree rows), `settings_ui.rs` (settings, about, matrix dialog, background picker, dialogs), and `tests/` (`helpers.rs`, `session_tests.rs`, `canvas_input_tests.rs`, `layout_theme_tests.rs`, `batch_settings_tests.rs`).

### 2. Keep `App` Struct Intact Across `desktop/src/app/*.rs`

- **Decision**: Keep `App` as a single struct defined in `desktop/src/app/mod.rs` with `pub(super)` fields grouped and documented by subsystem, and distribute `impl App` blocks across `desktop/src/app/*.rs`.
- **Alternative considered**: Extracting sub-structs (`EditorState`, `BatchState`, `SessionState`) immediately. Rejected because cross-cutting handlers (e.g., dirty checks, status messages, focus restoration, and 5,500 lines of unit tests directly accessing `app.*` fields) would require heavy churn and risk subtle regressions right before `1.0.0`.

### 3. Standardized English Naming, Ordering, and Flow Documentation

- **Decision**: Enforce uniform rules across all three crates:
  - **Naming**:
    - Boolean predicates use `is_`, `has_`, `can_`, `should_` (including pre-1.0.0 `eqntide_editor::Node` renames: `token` -> `is_token`, `row` -> `is_row`, `opaque` -> `is_opaque`, `math` -> `to_math_node`).
    - Verb-first naming for handlers and UI draw methods: `draw_*` (egui UI), `rasterize_*` / `render_*` (image/preview generation), `handle_*`, `process_*`, `sync_*`, `hit_test_*`.
    - Descriptive local variable names (replace 1-letter abbreviations like `b`, `f`, `a`, `n`, `r` outside loop indices `i`, `j` or coordinates `x`, `y`).
  - **File ordering (top-down)**:
    1. `//!` module doc + ASCII workflow diagram (English only)
    2. `mod` and `use` declarations
    3. `const` and `type` aliases
    4. `pub struct` / `pub enum` and `impl` blocks (constructors -> queries -> mutations)
    5. Private `struct` / `enum` and `impl` blocks
    6. Private helpers (high-level orchestrators first, leaf helpers below)
    7. `#[cfg(test)] mod tests` (or `mod tests;` pointing to `tests.rs` / `tests/`)
  - **Documentation & Language**:
    - All module headers, struct/field docs, function docs, and inline comments are written in concise English.
    - Major pipelines (`desktop/src/app/mod.rs`, `eqntide-editor/src/lib.rs`, `src/bridge/server/mod.rs`, `src/docx/pipeline.rs`, `src/service/mod.rs`, `src/render/mod.rs`, `src/mtef/decode/mod.rs`, `src/omml/encode/mod.rs`) include an ASCII data-flow diagram (`+`, `-`, `|`, `-->`) explaining how inputs flow through the subsystem.

### 4. Phased Execution Order

- **Decision**: Execute bottom-up by dependency (`eqntide-editor` -> `eqntide` core -> `desktop`), validating fmt, clippy, and tests after every phase.
- **Rationale**: Updating `eqntide-editor` first locks down the `Node` API renames before `desktop` is split; cleaning `eqntide` core modules next ensures `desktop` calls stable internal APIs; splitting `desktop/src/app.rs` last isolates the largest test refactor cleanly.

## Risks / Trade-offs

- [Visibility leaks when splitting files into submodules] -> Keep new submodule items `pub(super)` or `pub(crate)` rather than widening crate `pub` APIs; verify with `cargo doc --workspace --no-deps`.
- [Large diff in `desktop/src/app.rs` (~5,500 lines of tests moved)] -> Move tests verbatim into `desktop/src/app/tests/` first before renaming internal `App` methods and test helpers (`a` -> `app`), running `cargo test -p eqntide-desktop` at each step.
- [Broken intra-doc links or clippy warnings after moving symbols] -> Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, and `cargo doc --workspace --no-deps` at the end of every phase.
- [Behavioral drift during helper extraction] -> Keep function logic equivalent without algorithmic rewrites; rely on existing `conversion_matrix`, `cross_conversion`, `export_verification`, `eqntide-editor`, and `desktop` UI tests as regression guards.

## Migration Plan

1. **Phase 1**: Refactor `crates/eqntide-editor` + update `Node` callers in `desktop/`. Verify workspace fmt/clippy/tests.
2. **Phase 2**: Complete `src/mtef/decode/`, `src/omml/encode/`, `src/mtef/encode/`, `src/mtef/templates.rs`, and `src/service/`. Verify workspace fmt/clippy/tests.
3. **Phase 3**: Refactor `src/bridge/server/`, `src/docx/`, `src/render/`, `src/catalog.rs`, and `src/mathml_to_latex.rs`. Verify workspace fmt/clippy/tests.
4. **Phase 4**: Split `desktop/src/app.rs` into `desktop/src/app/*` + clean `desktop/src/batch.rs` and sibling desktop modules. Verify workspace fmt/clippy/tests.
5. **Phase 5**: Workspace-wide audit of English comments, doc links, `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test --workspace --all-targets --locked`, and `cargo doc --workspace --no-deps`.
