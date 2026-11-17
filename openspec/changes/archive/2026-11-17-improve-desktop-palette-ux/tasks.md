# Tasks

## 1. Consolidated UI Category Mapping

- [x] 1.1 Define `UiGroup` and the 12 consolidated desktop categories (`Recent` plus 11 mapped groups covering all 20 `catalog::GROUPS`) with representative preview templates and helper lookup functions (`group_contains`, `items_for_ui_group`) in `desktop/src/palette.rs`; verify with unit tests in `desktop/src/palette.rs` that every canonical catalog group is mapped and every icon preview expression renders non-empty visible pixels.
- [x] 1.2 Initialize `App::category` to `"Fractions"` and update matrix contextual checks (`+ row`, `− row`, `+ col`, `− col` in `editor_ui` and `⋯` custom matrix trigger in `palette_ui`) to query `palette::group_contains` in `desktop/src/app.rs`; verify with `cargo test -p eqntide-desktop custom_matrix_dialog_cancels_without_changes_and_inserts_one_transaction`.

## 2. Sidebar 2-Column Math-Icon Grid & Recent History

- [x] 2.1 Add a DPI-aware `sidebar_atlas` texture cache in `desktop/src/app.rs` using the white-mask rasterization pipeline so sidebar category icons render crisply in both light and dark themes; verify via a desktop unit test that icon textures regenerate on DPI change and tint cleanly across themes.
- [x] 2.2 Replace the vertical text category list in `App::sidebar` (`desktop/src/app.rs`) with a non-scrolling 2-column grid of math-icon buttons with hover tooltips and accessibility labels, keeping `Batch DOCX`, `Settings`, and `About` visible below; update and pass sidebar layout tests at `MIN_WINDOW_SIZE` (`1040x560`) across DPI scales confirming all 12 categories and `Batch DOCX` stay visible without vertical scrolling.
- [x] 2.3 Add session-scoped `recent_palette` tracking in `desktop/src/app.rs` that prepends deduplicated items on palette click and populates the `Recent` category (with an empty-state hint when unused); add and pass a unit test verifying MRU ordering, deduplication, and `Recent` rendering.

## 3. Wheel-Driven Horizontal Palette Scrolling & Integration Check

- [x] 3.1 Update `App::palette_ui` in `desktop/src/app.rs` so vertical mouse-wheel deltas while hovering over a horizontally overflowing 3-row palette pan the palette horizontally while preserving zero vertical scroll and native horizontal trackpad deltas; add and pass a headless `ctx.run_ui` test sending `Event::MouseWheel` over an overflowing category and a non-overflowing category.
- [x] 3.2 Run the full workspace verification (`cargo test -p eqntide-desktop`, `cargo test --test export_verification`, and `cargo check --workspace`) to confirm no regressions in desktop UI tests or shared catalog behavior.
