# Design

## Context

- See `proposal.md` for motivation and scope.
- `src/render/mod.rs::render_alpha_fit` currently computes `scale = (avail_w / box_w).min(avail_h / box_h)` from a `12.0pt` (`96px` em) layout with no upper bound. Short symbols (`★`, `αΩ`, `∀ ∈`, `± ≤`, `□^□`) get scaled up up to `2.4x`, while tall/wide expressions (`\lim_{#? \to #?}`, `\int`, `\sum`, matrices) get scaled down below `0.8x`, producing stark glyph-size mismatches across adjacent buttons.
- `desktop/src/app.rs` currently uses `46 x 28` inside `64 x 42` sidebar buttons and `34 x 22` inside `44 x 31` palette buttons with `PALETTE_GAP = 6.0`.
- `desktop/src/catalog.rs` has `49` items in the `Symbols` category (`17` columns at 3 rows). At the `1040x560` minimum window width, the central editor width is `1040 - 158 - 24 = 858px`. With `PALETTE_GAP = 4.0`, `17 * 46 + 16 * 4 = 846px <= 858px`, allowing every palette button to grow uniformly to `46 x 32` while keeping all `Symbols` visible in 3 rows without clipping.

## Goals / Non-Goals

**Goals:**
- Add capped typographic icon rendering (`render_alpha_at`) so math button icons render at a uniform base em size and only downscale if an expression overflows the tile bounds.
- Enlarge all palette buttons uniformly (`46 x 32`, `40 x 26` icon budget, `4px` gap) and utilize `54 x 32` inside `64 x 42` sidebar category tiles so multi-character operators (`max`, `min`, `\lim`, `\int`, `x^{#?}`) look crisp and proportionate.
- Define explicit UI `text_styles` and consistent button height tiers (`30px` standard, `34px` sidebar tabs, `24px` compact) across `theme.rs` and `app.rs`.

**Non-Goals:**
- Changing per-tile dynamic widths (all palette tiles remain strictly uniform in size).
- Modifying `render_preview`, `render_interactive`, or Word/MathType export output.

## Decisions

### 1. Capped Alpha-Mask Icon Renderer (`src/render/mod.rs`)
Extract a shared helper `render_alpha_scaled(math: &MathNode, target_em_px: Option<f32>, max_w_px: f32, max_h_px: f32)` used by both:
- `pub fn render_alpha_fit(math: &MathNode, max_w_px: f32, max_h_px: f32) -> Result<AlphaMask>` (existing behavior when `target_em_px` is `None`)
- `pub fn render_alpha_at(math: &MathNode, target_em_px: f32, max_w_px: f32, max_h_px: f32) -> Result<AlphaMask>` (caps `scale = fit_scale.min(target_em_px / em)`)

Re-export `render_alpha_at` from `src/lib.rs` alongside `render_alpha_fit`.

- **Alternative considered**: Modifying `render_alpha_fit` directly. Rejected to preserve backward-compatible semantics for any external caller expecting unconstrained fit-to-box scaling.

### 2. Uniform Palette & Sidebar Icon Budgets (`desktop/src/app.rs`)
- **Sidebar categories** (`build_sidebar_atlas`):
  - Tile size remains `~64 x 42` (`SIDEBAR_BUTTON_HEIGHT = 42.0`, `button_padding = (4, 4)`), giving `56 x 34` inner area.
  - Render via `render_alpha_at(&math, 18.0 * ppp, 54.0 * ppp, 32.0 * ppp)`.
  - At `18.0px` em, single-line icons (`★`, `± ≤`, `∀ ∈`, `αΩ`, `(□)`, `∫□`, `x^□`) share the exact `18.0px` typographic em instead of blowing up to `~26px`.
- **Palette buttons** (`build_palette_atlas`):
  - `PALETTE_BUTTON_SIZE = Vec2::new(46.0, 32.0)` and `PALETTE_GAP = theme::SPACE_XS` (`4.0`).
  - `palette_ui` button padding `Vec2::splat(3.0)` (`40 x 26` inner area).
  - Render via `render_alpha_at(&math, 17.0 * ppp, 40.0 * ppp, 26.0 * ppp)`.
  - `PALETTE_HEIGHT = 3.0 * 32.0 + 2.0 * 4.0 + 8.0 = 112.0` (only `+3px` compared to `109px` before, leaving ample vertical space at `560px` window height).
  - `"⋯"` custom matrix button uses `TextStyle::Button` inside `PALETTE_BUTTON_SIZE` (or clean centered dot label).

### 3. Explicit Typography & Button Height Tiers (`desktop/src/theme.rs` & `desktop/src/app.rs`)
- In `ThemeTokens::apply_style`:
  - Explicitly set `style.text_styles` (`Small = 11.0`, `Body = 13.0`, `Button = 13.0`, `Heading = 18.0`, `Monospace = 13.0`) and `style.drag_value_text_style = TextStyle::Button`.
  - Define `BUTTON_HEIGHT = 30.0`, `SIDEBAR_TAB_HEIGHT = 34.0`, `COMPACT_BUTTON_HEIGHT = 24.0`, and `style.spacing.interact_size.y = BUTTON_HEIGHT`? Wait: setting `interact_size.y = 24.0` (or applying explicit `min_size(Vec2::new(0.0, BUTTON_HEIGHT))` in `primary_button` and `standard_button` helpers) ensures `RadioButton`/`Checkbox` don't inflate vertically while `Save`, `Undo`, `Redo`, `DragValue`, and standard action buttons (`Reset`, `Switch theme`, `Back`, `Convert`, etc.) all render cleanly at `30.0px`, sidebar tabs render at `34.0px`, and compact buttons (`+ row`, `− row`, `+ col`, `− col`, `Retry`, `Next session`) use a shared `compact_button` helper (`24.0px` high with `Margin::symmetric(8, 3)` instead of `ui.small_button`'s collapsed `0px` vertical margin).
- For `Undo` / `Redo` (`"↶"`, `"↷"`), keep `BUTTON_HEIGHT` (`30.0px`) and use `RichText::new("↶").size(15.0)` (same for `"↷"`) so the STIX fallback arrow glyph matches the optical weight of `13.0px` Nunito button labels.

## Risks / Trade-offs

- **`Symbols` 17-column fit at `1040px`** → `17 * 46 + 16 * 4 = 846px <= 858px` leaves `12px` headroom; verified directly by `symbol_buttons_use_at_most_three_rows_even_during_startup_resize`.
- **`Fractions` non-overflow vs `LogicSetsSymbols` overflow** → `Fractions` (14 items = 5 columns = `246px`) does not overflow; `LogicSetsSymbols` (> 20 columns > `1000px`) overflows as expected by `palette_scrolls_horizontally_on_vertical_mouse_wheel`.
- **`Undo`/`Redo` glyph optical sizing** → Explicitly sizing `"↶"`/`"↷"` at `15.0` keeps optical parity without changing font files or introducing custom SVG assets.
