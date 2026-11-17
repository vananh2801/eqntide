# Design

## Context

- `src/catalog.rs` defines 20 canonical `GROUPS` shared across LaTeX parsing, CLI, desktop, and `tests/export_verification.rs` (which asserts `GROUPS.len() == 20`).
- `desktop/src/app.rs` currently renders `palette::GROUPS` directly as 20 full-width (`150px` sidebar, `42px` button height) vertical buttons inside a `ScrollArea::vertical()`, above a `142px` bottom panel containing `Batch DOCX`, `Settings`, and `About`. At `MIN_WINDOW_SIZE` (`1040x560`), the sidebar cannot show all 20 categories without scrolling.
- `desktop/src/app.rs` (`palette_ui` and `build_palette_atlas`) already renders math templates into white-tinted texture atlases using `eqntide::render::render_preview`, tinting them dynamically for light/dark themes.
- `palette_ui` wraps the 3-row `egui::Grid` in `egui::ScrollArea::horizontal()`, which in `egui` ignores vertical mouse-wheel deltas (`smooth_scroll_delta.y`) unless horizontal scroll events or `Shift+Wheel` are sent.

## Goals / Non-Goals

**Goals:**
- Keep `eqntide::catalog::GROUPS` untouched while consolidating the desktop UI sidebar into 12 compact categories (1 `Recent` category + 11 grouped categories) in a 2-column math-icon grid that fits within `MIN_WINDOW_SIZE` (`1040x560`) with zero vertical scrolling.
- Render crisp, theme-aware mathematical icons for category buttons via the existing math preview rasterizer, paired with hover tooltips and accessibility labels.
- Track recently inserted palette templates/symbols in-session under the `Recent` category.
- Translate vertical mouse-wheel scrolling over an overflowing 3-row palette into horizontal scroll movement.

**Non-Goals:**
- Persisting `Recent` history across application restarts (in-memory session tracking keeps scope small and avoids introducing config serialization).
- Changing `eqntide::catalog::GROUPS`, LaTeX parser commands, keyboard shortcuts, or the bottom `Batch DOCX / Settings / About` navigation structure.

## Decisions

### 1. Desktop-Layer Category Mapping (`desktop/src/palette.rs`)
Define a UI-facing `UiGroup` table in `desktop/src/palette.rs` mapping 12 compact categories to the 20 underlying `catalog::GROUPS`:

| UI Category ID | Tooltip Label | Icon Preview Source | Mapped `catalog::GROUPS` |
| --- | --- | --- | --- |
| `Recent` | Recent | `★` (text/glyph icon) | *(session MRU list)* |
| `Fractions` | Fractions & Radicals | `\frac{#?}{#?}` | `Fractions` |
| `Scripts` | Scripts | `x^{#?}` | `Scripts` |
| `Fences` | Fences | `\left( #? \right)` | `Fences` |
| `Integrals` | Integrals | `\int #?` | `Integrals` |
| `SumsAndBigOps` | Sums & Big Operators | `\sum_{#?}^{#?}` | `Sums`, `BigOperators` |
| `MatricesAndBoxes` | Matrices & Boxes | `\begin{pmatrix} #? & #? \\ #? & #? \end{pmatrix}` | `Matrices`, `Boxes` |
| `AccentsAndBars` | Accents & Bars | `\bar{#?}` | `Embellishments`, `Bars` |
| `OperatorsAndRelations` | Operators & Relations | `\pm \leq` (or `a \leq b` / `\pm`) | `Operators`, `Relations` |
| `Arrows` | Arrows | `\rightarrow\Rightarrow` | `Arrows`, `LabeledArrows` |
| `LogicSetsSymbols` | Logic, Sets & Symbols | `\forall \in` (or `\forall`) | `Logic`, `Sets`, `Symbols`, `Spaces` |
| `Greek` | Greek Letters | `\alpha \Omega` | `GreekLower`, `GreekUpper` |

- **Default startup category**: `"Fractions"` (populated immediately on launch without needing prior insertions).
- **Alternative considered**: Modifying `eqntide::catalog::GROUPS` directly. Rejected because `src/catalog.rs` unit tests and `tests/export_verification.rs` assert exact canonical group counts and IDs.
- **Matrix context check**: Update `editor_ui` and `palette_ui` checks from `self.category == "Matrices"` to `palette::group_contains(&self.category, "Matrices")` so `"MatricesAndBoxes"` keeps `+ row / − row / + col / − col` and the `⋯` custom matrix button.

### 2. Two-Column Math-Icon Grid & Atlas Caching (`desktop/src/app.rs`)
- Render the 12 `UiGroup` buttons in a 2-column `egui::Grid` (6 rows) inside the `150px` left sidebar.
- Target button size of `~63x44px` with `6px` spacing uses `2 * 63 + 6 = 132px` horizontally (fitting inside `150px` with panel margins) and `6 * 44 + 5 * 6 = 294px` vertically, fitting well within the `~376px` available above the `142px` bottom panel at `560px` window height without a vertical `ScrollArea`.
- Build a cached `SidebarAtlas` (reusing the same DPI-keyed `build_palette_atlas` white-mask rasterization pipeline) for the 11 math-rendered icons so icons stay sharp across DPI changes and invert cleanly via `.tint(icon_color)` in light/dark mode.
- Attach `.on_hover_text(group.label)` and `WidgetInfo::labeled(Role::Button, ..., group.label)` to every button.

### 3. Session Recent Tracking (`App::recent_palette`)
- Store `recent_palette: Vec<&'static str>` (palette item `source` strings, capped at e.g. 24 items, deduplicated with most recently clicked first).
- Record an item whenever a user clicks a template/symbol button in `palette_ui` (including items clicked from `Recent` itself, moving them to front).
- When `Recent` is active and empty, show a concise inline hint inside the palette area instead of an empty blank strip.

### 4. Vertical Mouse-Wheel to Horizontal Scroll (`palette_ui`)
- Capture the palette area's `ScrollAreaOutput` / `inner_rect` and check `ui.rect_contains_pointer(palette_rect)`.
- When the pointer is over the palette, `ScrollArea` content width exceeds viewport width (`content_size.x > inner_rect.width()`), and the frame has a vertical scroll delta (`smooth_scroll_delta.y != 0.0`) with zero horizontal delta (`smooth_scroll_delta.x == 0.0`):
  - Apply `scroll_delta = Vec2::new(-raw_delta.y, 0.0)`? Note on `egui` coordinate convention: in `egui`, scrolling the wheel downward produces a negative `delta.y`; to pan the viewport rightward (advancing further into the palette), `ScrollArea::scroll_with_delta` expects a negative `delta.x` (moving content left relative to viewport), so `delta.x = smooth_scroll_delta.y` (negative wheel-down -> negative `x` delta -> content moves left / viewport scrolls right). Verify this sign convention in unit test with `RawInput` `Event::MouseWheel`.
  - Consume or zero out vertical delta only inside the palette `ScrollArea` (by running `.vertical_scroll_offset(0.0)` and `enable_scrolling(true)` or applying `state.offset.x = (state.offset.x - delta.y).clamp(0.0, max_offset)` directly via `ScrollArea::horizontal_scroll_offset` / `ScrollAreaState::load` & `store`), ensuring deterministic behavior both interactively and in headless `ctx.run_ui` tests.
- Using `ScrollAreaState` (or `ui.input_mut` converting vertical `MouseWheel` events inside the hovered palette rect into horizontal `MouseWheel` events before `ScrollArea::horizontal` runs) works naturally with egui's built-in kinetic/smooth scrolling and keeps `egui::ScrollArea` clamping intact.

## Risks / Trade-offs

- **Wide merged group (`LogicSetsSymbols` = 85 items)** → At 3 rows this spans ~29 columns (~1,444px), requiring horizontal scrolling at `1040px` window width. Mitigated directly by mouse-wheel horizontal scrolling; if needed later, splitting `Spaces` or `Symbols` into a 13th tile (7th row = `344px`) still fits inside the `376px` budget without vertical scroll.
- **Test assumptions in `desktop/src/app.rs`** → Existing tests `every_category_keeps_three_palette_rows_at_compact_sizes` and `batch_stays_visible_while_sidebar_groups_scroll` assume the old 20-group vertical scrolling sidebar. These tests will be updated to validate all consolidated UI groups and assert that all 12 category buttons plus `Batch DOCX` are simultaneously visible without scrolling at `MIN_WINDOW_SIZE`.
- **Trackpad vs mouse-wheel direction** → Only remap vertical delta when `delta.x == 0.0` so native 2D trackpad horizontal gestures are preserved unchanged.
