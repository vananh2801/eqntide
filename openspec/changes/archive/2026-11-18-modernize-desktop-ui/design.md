# Design

## Context

- See `proposal.md` for motivation and capability scope.
- `desktop/src/app.rs` currently sets only `selection.bg_fill`, `selection.stroke`, and `UI_CORNER_RADIUS = 6` in `App::new_with_state`, leaving widget colors, hover expansion, and panel surfaces at `egui` defaults.
- `desktop/src/app.rs` renders equations via `render_interactive_alpha_scaled` into an alpha texture and paints it on the canvas using `.tint(Color32::BLACK)`, while `paint_placeholder` masks out the synthetic placeholder box with `Color32::WHITE`.
- `desktop/src/settings.rs` already handles atomic JSON persistence in `settings.json` with `#[serde(default)]` and per-field sanitization.
- Headless UI tests in `desktop/src/app.rs` enforce layout invariants at `1040x560` (no sidebar vertical scrolling, max 3 palette rows, visible Save/LaTeX source) and currently include a few color assertions against legacy `Color32::WHITE` / `LIGHT_GRAY` / `BLACK` values.

## Goals / Non-Goals

**Goals:**
- Centralize all visual design tokens (colors, spacing, corner radii, hover/active/selected rules, and canvas presets) in `desktop/src/theme.rs`.
- Unify hover and press behavior across every interactive widget (`expansion = 0.0`, `PointingHand` hover cursor, consistent primary/secondary/tinted-selected styling).
- Support curated per-mode Equation Canvas presets (`canvas_light`, `canvas_dark`) that default to soft light paper and adapt formula ink and all canvas overlays automatically.

**Non-Goals:**
- Arbitrary RGB/HSV color pickers for the Equation Canvas (curated presets guarantee contrast and legibility).
- Changing equation rendering output for Word/MathType export (EMF/OMML export remains unaffected).
- Altering the `1040x560` minimum window layout geometry or adding vertical scrolling to the sidebar.

## Decisions

### 1. Dedicated Theme Tokens Module (`desktop/src/theme.rs`)
Introduce `ThemeTokens` (constructed via `ThemeTokens::for_mode(dark_mode: bool)`) and `CanvasPreset` (serializable kebab-case enum):

- **Corner radii**:
  - `RADIUS_CONTROL = 6.0` (`CornerRadius::same(6)`): buttons, inputs, sliders, drag values, combo boxes, checkboxes, palette tiles, sidebar category/tab buttons, and canvas preset swatches.
  - `RADIUS_SURFACE = 8.0` (`CornerRadius::same(8)`): Equation Canvas frame, Settings/Batch group cards, dialogs, modals, and popups.
  - Remove the `4px` override on the Settings zoom slider.
- **Spacing scale (`4px` base)**:
  - `SPACE_XS = 4.0`, `SPACE_SM = 6.0` (compact grid gap to keep sidebar fit within `1040x560`), `SPACE_MD = 8.0`, `SPACE_LG = 12.0`, `SPACE_XL = 16.0`.
  - Apply consistent `Frame::NONE.fill(tokens.panel_bg)` across `status_bar`, `app-footer`, `app-sidebar`, and `CentralPanel` so panel seams and padding align cleanly.
- **Modern Slate Palette**:
  - **Light**: Base `#EFF2F6`, Panel `#F8FAFC`, Card/Canvas border `#D4DCE6`, Control fill `#FFFFFF` / `#E9EEF5`, Text `#1E293B` (primary) / `#526075` (muted), Accent `#1D78B8` (hover `#16649B`, active `#125380`), Tinted selected fill `#E2F0FA` with `#1D78B8` border and `#145B8E` foreground/icon.
  - **Dark**: Base `#161A20`, Panel `#1D222B`, Card/Elevated `#242A35`, Control fill `#2A313E`, Border `#353E4D`, Text `#E5EAF2` (primary) / `#9AA6B8` (muted), Accent `#4CA8E8` (hover `#64B6EE`, active `#3591D1`), Tinted selected fill `#1E3448` with `#4CA8E8` border and `#7CC4F5` foreground/icon.

### 2. Unified Hover, Active, and Tinted-Accent Selection
- Set `expansion = 0.0` across `widgets.{noninteractive, inactive, hovered, active, open}` in both Light and Dark `egui::Style` so controls never jitter or grow by `1px` on hover.
- Provide small UI helper methods on `ThemeTokens` / `App` so every interactive control shares consistent behavior:
  - `selectable_button` / sidebar category & tab buttons: **Tinted Accent** when selected (tinted fill + `1px` accent border + matching accent text/icon tint), and unified hover fill/border transitions when unselected.
  - Primary CTA buttons (`Save`, `Convert`, `Insert matrix`): explicit `Resting / Hover / Active / Disabled` fills computed from `tokens` and pointer hover state instead of a static `.fill(...)` override.
  - All clickable controls (buttons, palette tiles, sidebar items, radio buttons, checkboxes, combo boxes, preset swatches) set `on_hover_cursor(egui::CursorIcon::PointingHand)` when enabled.
  - `icon_tint` for sidebar/palette glyphs adapts to state (`selected` uses `tokens.accent_fg`, `hovered` uses crisp primary text, `resting` uses theme foreground, `disabled` uses muted text).

### 3. Curated Per-Mode Equation Canvas Presets (`CanvasPreset` + `CanvasPalette`)
Define `CanvasPreset`:
- **Light mode presets**:
  - `SoftPaper` (default): `#FAF9F5` background, `#D4DCE6` border, `#1A202C` ink
  - `PureWhite`: `#FFFFFF` background, `#D4DCE6` border, `#111827` ink
  - `CoolSlate`: `#EDF2F7` background, `#CBD5E1` border, `#1E293B` ink
  - `WarmCream`: `#F6F1E4` background, `#DFD5BE` border, `#292524` ink
- **Dark mode presets**:
  - `SoftPaper` (default — preserves soft paper look even in Dark mode): `#FAF9F5` background, `#353E4D` border, `#1A202C` ink
  - `DimSlate`: `#1E242E` background, `#353E4D` border, `#E6ECF5` ink
  - `SoftCharcoal`: `#252932` background, `#3C4352` border, `#ECEFF4` ink
  - `DeepInk`: `#141820` background, `#2E3746` border, `#DCE6F5` ink

Each `CanvasPreset::palette(dark_mode)` returns a `CanvasPalette` containing `bg`, `border`, `ink`, `placeholder_bg`, `placeholder_stroke`, `caret`, `selection_bg`, `ime_fg`, and scrollbar colors (`scroll_inactive`, `scroll_hover`, `scroll_active`, `scroll_track`). Both `canvas` and `latex_ui` share `active_canvas_palette(dark_mode)` so the LaTeX source editor matches the Equation Canvas surface, border, ink, cursor, and selection colors. `paint_placeholder` uses `palette.bg` to mask the synthetic placeholder glyph and `palette.placeholder_bg` / `palette.placeholder_stroke` for the dashed slot box.

**Settings persistence integration**:
- Add `canvas_light: CanvasPreset` and `canvas_dark: CanvasPreset` to `Settings` in `desktop/src/settings.rs` (`#[serde(default)]`).
- Use a serde helper (or deserialize via string fallback) so an unknown preset string in `settings.json` falls back cleanly to `CanvasPreset::SoftPaper` without discarding other valid settings; also constrain `canvas_light` to light-suitable presets (`SoftPaper`, `PureWhite`, `CoolSlate`, `WarmCream`) and `canvas_dark` to dark-mode presets (`SoftPaper`, `DimSlate`, `SoftCharcoal`, `DeepInk`) in `Settings::sanitize`.

### 4. Headless Test Alignment
Update the existing headless UI tests in `desktop/src/app.rs` (`new_with_state` helper / `apply_theme` in tests) so tests exercise the new theme tokens and update assertions that previously hardcoded `Color32::WHITE` / `Color32::LIGHT_GRAY` / fixed icon tints to match `ThemeTokens` and `CanvasPalette` while keeping all layout, overflow, and persistence guarantees intact.

## Risks / Trade-offs

- **Existing color assertions in `app.rs` tests** → Update legacy hardcoded `WHITE`/`LIGHT_GRAY` assertions in the test suite to assert against `ThemeTokens` / `CanvasPalette` rather than disabling tests.
- **Sidebar vertical budget at `1040x560`** → Keep `sidebar-batch` and `sidebar-categories` heights and gaps compact within the `4px`/`6px` scale so `all_sidebar_categories_and_batch_stay_visible_without_scrolling` continues to pass on every DPI.
- **Unrecognized preset IDs in `settings.json`** → Deserializing `CanvasPreset` safely via `#[serde(other)]` or a custom deserializer (`Unknown` -> `SoftPaper` in `sanitize`) prevents breaking startup on hand-edited or future config files.
