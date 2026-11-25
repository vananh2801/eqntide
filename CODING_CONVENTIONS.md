# Coding Conventions & Repository Structure (EqnTide)

This document defines the architectural boundaries, module layout, declaration ordering, naming rules, documentation standards, and verification gates established by the `refactor-pre-1-0-readability` refactor across the workspace (`eqntide`, `eqntide-editor`, `eqntide-desktop`).

---

## 1. Core Principles

1. **Memory Safety**: Preserve `#![forbid(unsafe_code)]` at the root of all three crates ([`src/lib.rs`](src/lib.rs), [`crates/eqntide-editor/src/lib.rs`](crates/eqntide-editor/src/lib.rs), and [`desktop/src/main.rs`](desktop/src/main.rs)). Never introduce `unsafe` blocks.
2. **Minimal Visibility**: Keep crate-root `pub` APIs intentionally narrow. Share internal helpers and fields only via `pub(super)` or `pub(crate)`.
3. **Single Responsibility per File**: Prevent multi-thousand-line monoliths from returning. When a file grows past ~600–800 lines or accumulates unrelated responsibilities, split it into cohesive submodules.
4. **Standard Rust Module Layout**: Use standard `mod.rs` / directory modules. **Never** use `#[path = "..."]` attributes to stitch unrelated files together.
5. **Separate Large Test Suites**: Small test suites stay in a trailing `#[cfg(test)] mod tests` block; large suites (~200+ lines) belong in a dedicated `tests.rs` file or `tests/` submodule directory.

---

## 2. Workspace Architecture & Module Layout

The workspace is organized into three crates with clear boundaries:

```text
eqntide/
├── crates/eqntide-editor/src/   # UI-agnostic structural equation editor
│   ├── lib.rs                   # //! Architecture diagram + crate-root re-exports
│   ├── node.rs                  # Node, NodeId, is_opaque_tag, to_math_node
│   ├── stops.rs                 # StopKind, StopEntry, collect_stops, canonical_stops
│   ├── navigation.rs            # Caret, Move, cursor & selection movement
│   ├── editing.rs               # Editor, Snapshot, transact(), undo/redo, splice
│   ├── deletion.rs              # Backspace/Delete & empty-slot pruning rules
│   ├── structures.rs            # Templates, matrix resize, fence/row promotion
│   └── tests.rs                 # Unit & regression test suite
│
├── src/                         # Core `eqntide` crate: codecs, render, DOCX, bridge, CLI
│   ├── lib.rs                   # Pipeline overview & public API
│   ├── bridge/server/           # mod, auth, http, session_routes, convert_routes, addin_assets, tests
│   ├── catalog/                 # mod.rs (catalog! tables & builders), tests.rs
│   ├── cli/                     # conversion, dispatch, paths
│   ├── docx/                    # consts, discovery, flat_opc, job, ole_xml, options, package, pipeline, to_omml, upgrade, perf_tests
│   ├── mtef/
│   │   ├── decode/              # mod, consts, stream, entry, parse, v5, v3, templates, characters, embellishment, tests
│   │   ├── encode/              # mod, dispatch, fences, limits, records, structural, tokens, tests
│   │   └── templates/           # mod, consts, basic, composite, slots, tests
│   ├── omml/encode/             # mod, fences, scripts, normalization, nary, accents, font, tests
│   ├── render/                  # mod, canvas, emf, png, svg, font/{metadata,selection,geometry}, layout/*.rs
│   └── service/                 # mod.rs, latex_extensions.rs, tests.rs
│
└── desktop/src/                 # `eqntide-desktop` crate: native egui/eframe application
    ├── main.rs                  # Entry point, CLI/worker dispatch, EventSender<Event>
    ├── app/
    │   ├── mod.rs               # App struct (grouped pub(super) fields), lifecycle diagram, draw_ui, draw_shell
    │   ├── events.rs            # Event loop & async render/LaTeX/background workers
    │   ├── session.rs           # Office session lifecycle, dirty checks, save/cancel/close guards
    │   ├── canvas.rs            # Canvas paint, placeholder boxes, hit-testing, vertical cursor nav
    │   ├── keyboard.rs          # Shortcuts, canvas text input, IME, \command popup
    │   ├── palette_ui.rs        # Palette grid, DPI glyph atlas rasterization, icon caching
    │   ├── batch_ui.rs          # Batch DOCX tab UI & folder tree controls
    │   ├── settings_ui.rs       # Settings/About tabs, matrix dialog, background picker, modals
    │   └── tests/               # mod, helpers, session_tests, canvas_input_tests, layout_theme_tests, batch_settings_tests
    └── {batch,commands,instance,palette,runtime,settings,software_painter,theme,wgpu_runner}.rs
```

### Preserving `desktop/src/app/` Structure

- Keep shared window state in `struct App` inside `desktop/src/app/mod.rs` with `pub(super)` fields grouped and documented by subsystem (`Instance & Event Channel`, `Office Bridge & Session Queue`, `Editor & Document Snapshot`, `LaTeX Source & Command Buffer`, `Navigation, Palette & Icons`, `Canvas Render & Display`, `Batch DOCX Conversion`, `Persisted Settings, Appearance & Focus`).
- Place each subsystem's `impl App` methods in its matching submodule under `desktop/src/app/`.

---

## 3. Top-Down File Declaration Order

Every non-test `.rs` file **must** follow this 7-layer top-down order so readers see high-level contracts before low-level details:

1. **`//!` Module Doc**: Purpose of the module (plus an ASCII workflow diagram on major orchestration/pipeline modules).
2. **`#![...]` & `mod` / `use`**: Crate attributes (root only), submodule declarations, and `use` imports.
3. **`const` & `type`**: Constants and type aliases (`pub` → `pub(crate)`/`pub(super)` → private).
4. **Public APIs (`pub` / `pub(crate)` / `pub(super)`)**:
   - Exported `struct`, `enum`, and `trait` definitions with their `impl` blocks.
   - Inside each `impl`, order methods as: constructors (`new`, `from_*`) → queries/predicates (`is_*`, `has_*`, `can_*`) → mutations/actions/UI draw.
   - Module-level public functions.
5. **Private Types & Impls**: Internal `struct` / `enum` definitions (e.g., `DirNode`, `WorkerRequest`, `Scanner`) and their `impl` blocks.
6. **Private Helpers**: High-level orchestrator helpers first, leaf utility functions below.
7. **Tests**: `#[cfg(test)] mod tests { ... }` or `#[cfg(test)] mod tests;` always at the very end of the file.

---

## 4. Naming Rules

### 4.1. Files, Modules, Types, and Constants

| Item | Convention | Rule | Good Examples | Avoid |
| --- | --- | --- | --- | --- |
| File / Module | `snake_case` | Reflects a single domain/responsibility | `session_routes.rs`, `latex_extensions.rs` | `utils2.rs`, `misc_stuff.rs` |
| `struct` / `enum` / `trait` | `UpperCamelCase` | Clear noun or noun phrase | `BatchRowOutput`, `ThemeTokens`, `GpuProfile` | `Data`, `Manager`, `Ctx2` |
| `const` / `static` | `SCREAMING_SNAKE_CASE` | Include units or bounds where helpful | `MAX_BACKGROUND_EDGE`, `FONT_PT_RANGE` | `MAX`, `LIMIT_1` |

### 4.2. Functions and Methods

All functions and methods use `snake_case` and follow consistent semantic prefixes:

1. **Boolean Predicates (`is_*`, `has_*`, `can_*`, `should_*`)**:
   - Always prefix boolean queries instead of bare nouns/adjectives.
   - Workspace examples:
     - [`Node::is_token`](crates/eqntide-editor/src/node.rs), [`Node::is_row`](crates/eqntide-editor/src/node.rs), [`Node::is_opaque`](crates/eqntide-editor/src/node.rs) *(never `token()`, `row()`, `opaque()`)*
     - `is_dirty()`, `can_save()`, `has_selection()`, `should_blink()`, `is_software_adapter_name()`
2. **Type Conversions (`as_*`, `to_*`, `into_*`, `from_*`)**:
   - `as_*`: Cheap reference/view cast (`O(1)`, no allocation), e.g., `as_str`.
   - `to_*`: Allocates or reconstructs a new value without consuming `self`, e.g., [`Node::to_math_node`](crates/eqntide-editor/src/node.rs), `to_xml`, `to_latex`, `to_egui`.
   - `into_*` / `from_*`: Consumes `self` or constructs from an input, e.g., `from_parts`, `from_conversion`, `into_raw`.
3. **UI, Rendering, and Event Handlers (`desktop` and `render`) — Verb-First**:
   - `draw_*`: Render `egui` UI chrome/panels/widgets (`draw_ui`, `draw_shell`, `draw_sidebar`, `draw_footer`, `draw_status_bar`, `draw_editor`, `draw_canvas`, `draw_batch_tab`, `draw_settings_tab`, `draw_dialogs`).
   - `handle_*`: Process user input, IPC/bridge requests, or lifecycle actions (`handle_canvas_keyboard`, `handle_window_request`).
   - `process_*`: Drain and dispatch queued events or work items (`process_events`, `process_items_sequential`).
   - `sync_*`: Synchronize state between models/renders (`sync_render`, `sync_latex_from_editor`).
   - `hit_test_*`: Resolve pointer coordinates against canvas geometry (`hit_test_caret`, `hit_test_placeholder`).
   - `render_*` / `rasterize_*`: Generate previews, EMF/SVG/PNG output, or CPU bitmaps (`render_preview`, `render_emf`, `rasterize_cover_background`).
   - `sanitize_*` / `resolve_*`: Validate inputs and resolve safe filesystem paths (`sanitize_background_filename`, `resolve_managed_background_path`).

### 4.3. Local Variables and Closure Parameters

- **No single-letter local variables or closure parameters** (`a`, `b`, `c`, `d`, `e`, `f`, `n`, `p`, `r`, `s`, `t`, `w`, `z`, etc.). Use descriptive domain names:
  - `app` instead of `a`
  - `layout_box` instead of `b`
  - `font_set` instead of `f`
  - `err` instead of `e`
  - `entry` / `row` / `item` / `node` instead of `x` / `r` / `it`
  - `path` / `parent_path` instead of `p`
  - `name` / `text` / `source` instead of `n` / `s`
  - `bytes_read` instead of `n`
  - `|first, second|` instead of `|a, b|` in comparators
  - `|arg|`, `|val|`, `|duration|`, `|depth|`, `|flag|` instead of `|a|`, `|v|`, `|d|`, `|s|`
- **Permitted 1-letter exceptions**:
  1. Loop indices: `i`, `j`, `k` (prefer `idx` / `row_idx` when referenced across larger blocks).
  2. 2D coordinates: `x`, `y`.
  3. RGBA color channels: `r`, `g`, `b`, `a` (prefer `alpha` or `alpha_norm` when standalone in blending math).
  4. Texture UV coordinates: `u`, `v`.
- **Avoid helper shadowing in tests**: In `desktop/src/app/tests/*.rs`, initialize test fixtures explicitly via `let mut app = super::helpers::app();` so the local `app` does not shadow the helper function inside the same scope.

---

## 5. Documentation & Comments (`//!`, `///`, `//`)

1. **English Only**: All `//!` module headers, `///` docstrings, and `//` inline comments must be written in concise, clear **English** *(preserve intentional Unicode math symbols, Greek characters, or multilingual strings only when they represent domain/test data)*.
2. **Module Headers (`//!`)**:
   - Every non-test `.rs` file starts with a `//!` summary explaining its role.
   - Major orchestration and pipeline modules (`src/lib.rs`, `crates/eqntide-editor/src/lib.rs`, `src/service/mod.rs`, `src/bridge/server/mod.rs`, `src/mtef/decode/mod.rs`, `src/omml/encode/mod.rs`, `src/render/mod.rs`, `src/docx/pipeline.rs`, `desktop/src/app/mod.rs`) include an ASCII data-flow diagram (`+`, `-`, `|`, `-->` or `──▶`).
3. **API & Field Docs (`///`)**:
   - Document all `pub`, `pub(crate)`, and `pub(super)` structs, enums, variants, constants, key fields, and core functions.
4. **Clean Intra-Doc Links**:
   - Do **not** link from a `pub` item's `///` comment to a private or `pub(super)` item using `[...]` (triggers `rustdoc::private-intra-doc-links`). Use plain code backticks (`` `helper_name` ``) for private items.
   - Disambiguate names shared by a function and a module (e.g., `` [`mathml_to_latex`](crate::mathml_to_latex()) `` or `mod@crate::mathml_to_latex`).

---

## 6. Safety Invariants & Engineering Practices

- **Atomic File Writes**: When writing `.docx` outputs, `settings.json`, or managed background images, stage into a `tempfile` inside the **same parent directory**, flush/`sync_all()` where required, and commit via `persist` or `persist_noclobber`. Never mutate destination or source files partially on failure or cancellation.
- **Cooperative Cancellation**: Background batch and conversion jobs accept `Option<&AtomicBool>` / `&Arc<AtomicBool>` (`*_with_cancel`) and check the flag before and between expensive steps.
- **UI Thread Communication**: Background threads in `desktop/` report back solely via [`EventSender<Event>`](desktop/src/main.rs), which requests an `egui` repaint without sharing mutable UI locks.
- **Isolated Tests**: Filesystem tests must use `tempfile::tempdir()` and leave zero artifacts outside temporary directories.

---

## 7. Mandatory Verification Gate

Before committing or finishing any code change, all four workspace checks must pass with zero warnings:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```
