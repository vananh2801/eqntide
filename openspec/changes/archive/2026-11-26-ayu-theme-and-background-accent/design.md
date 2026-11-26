# Design

## Context

In the desktop app (`desktop/src/theme.rs`, `desktop/src/settings.rs`, and `desktop/src/app/`), visual styling previously relied on two separate paths:
1. `ThemeTokens::apply_style(&egui_ctx)` configured Light and Dark `egui::Style` visuals once at startup from `ThemeTokens::for_mode(dark_mode)`.
2. Helpers such as `ThemeTokens::from_ui(ui)` (used in `palette_ui.rs`, `settings_ui.rs`, and `keyboard.rs`) recomputed `ThemeTokens::for_mode(ui.visuals().dark_mode)` without access to `App` state, while `App::theme_tokens(&self, ui)` only modified `app_bg`, `panel_bg`, and `card_bg` alpha for custom backgrounds.

Meanwhile, `Settings` stored `background_image: Option<String>`, `background_opacity: f32`, and multi-preset `canvas_light` / `canvas_dark` options. Clicking **Clear** in `settings_ui.rs` deleted the managed PNG file and cleared `background_image`, forcing re-import whenever a user temporarily wanted the solid theme back.

See `proposal.md` for motivation and `specs/` for behavioral requirements.

## Goals / Non-Goals

**Goals:**
- Replace the slate UI palette with Zed Editor–inspired Ayu Light and Ayu Dark neutral surfaces and an Ayu amber/orange default accent while keeping `theme.rs` WCAG contrast tests (`>= 7.0` primary text, `>= 4.5` selected/primary button text) passing in both opaque and translucent states.
- Standardize the Equation Canvas and synchronized LaTeX source box directly on `AyuLight` (`#fcfcfc`, Light mode) and `AyuDark` (`#0d1016`, Dark mode), removing the other canvas preset options and preset picker UI from Settings while ignoring legacy keys in existing `settings.json` files.
- Allow instant switching between `Solid` and `Image` background modes while preserving the managed background PNG and extracted accent seed.
- Extract a single compact RGB accent seed from any imported background image, derive accessible Light-mode (deeper) and Dark-mode (brighter) accent palettes, and propagate the active accent consistently through `ThemeTokens`, `egui::Style` (including subtle `control_hover_border`, `DragValue` focus stroke, checkboxes, footer radios, `ComboBox` and inline `\command` popup selections, and the Batch DOCX progress bar), `CanvasPalette`, the inline `\command` popup (`\` prefix + focus border), and the status-bar readiness circle.

**Non-Goals:**
- User-editable custom color pickers or arbitrary theme JSON loading.
- Modifying Office add-in web/ribbon colors or equation export colors (OMML/MathType output ink remains unaffected).
- Changing status bar text colors to the accent color (status messages remain `text_primary` / `error_fg`).
- Tinting Equation Canvas / LaTeX source box borders (`pal.border`) or Batch folder tree connector lines (`border_hover`) with the accent color — both intentionally remain neutral.

## Decisions

### 1. Ayu Neutral Surfaces, Amber/Orange Default Accent, and Dynamic `primary_fg`
- **Ayu Dark**: `app_bg` `#0d1016`, `panel_bg` `#1f2127`, `card_bg` `#252830`, `extreme_bg` `#090c10`, `control_bg` `#252830`, `control_hover_bg` `#2d2f34`, `control_active_bg` `#3e4043`, `control_disabled_bg` `#1b1d22`, `border` `#3f4043`, `border_hover` `#5c5f66`, `text_primary` `#dcdad2`, `text_muted` `#8a8986`, `text_disabled` `#696a6a`, `error_fg` `#ef7177`.
- **Ayu Light**: `app_bg` `#e4e6e8`, `panel_bg` `#ececed`, `card_bg` `#fcfcfc`, `extreme_bg` `#ffffff`, `control_bg` `#fcfcfc`, `control_hover_bg` `#dfe0e1`, `control_active_bg` `#cfd0d2`, `control_disabled_bg` `#e5e6e7`, `border` `#cfd1d2`, `border_hover` `#9ea2a6`, `text_primary` `#2e3238`, `text_muted` `#6b7076`, `text_disabled` `#9ea2a6`, `error_fg` `#cc3a3a`.
  - *Rationale*: Tuning Light `text_primary` slightly to `#2e3238` and Dark `text_primary` to `#dcdad2` keeps Ayu's crisp neutral aesthetic while exceeding `>= 7.0` WCAG AAA contrast even over translucent surfaces blended over pure white/black wallpapers.
- **Default Accent (Amber/Orange)**:
  - Dark mode accent `#ffb454`, `primary_bg` `#f5a53a`, `primary_fg` `#0d1016` (dark ink on bright amber exceeds 9:1 contrast).
  - Light mode accent `#ba5e06`, `primary_bg` `#b05604`, `primary_fg` `#ffffff` (deep rich orange exceeds 4.5:1 with white text).
- **Automatic `primary_fg` selection**: For any derived `primary_bg`, pick `#ffffff` or `#0d1016` whichever maximizes WCAG contrast ratio against `primary_bg` so vivid yellow/cyan wallpapers also remain legible.

*Alternative considered*: Keeping blue `#5ac1fe` / `#3b9ee5` as default accent or forcing white `primary_fg` with muddy brown-orange buttons. Rejected per user preference for signature Ayu amber/orange and high-contrast crisp buttons.

### 2. Fixed Per-Mode `CanvasPalette` (Ayu Light & Ayu Dark Only)
- Remove `CanvasPreset`, `LIGHT_CANVAS_PRESETS`, `DARK_CANVAS_PRESETS`, `canvas_light` / `canvas_dark` settings fields, and the `Equation canvas` preset picker rows from Settings.
- Resolve `CanvasPalette::for_mode_with_accent(dark_mode, accent_seed)` directly:
  - Light mode (`dark_mode == false`): Ayu Light (`#fcfcfc` bg, `#cfd1d2` border, `#35393e` ink).
  - Dark mode (`dark_mode == true`): Ayu Dark (`#0d1016` bg, `#3f4043` border, `#d6d4cc` ink).
- Because `Settings` deserializes via `RawSettings` without `#[serde(deny_unknown_fields)]`, legacy `canvas_light` / `canvas_dark` keys in existing `settings.json` files are ignored cleanly without breaking startup.

### 3. Background Mode & Persistence Schema
- Extend `Settings` (`desktop/src/settings.rs`) with:
  - `background_mode: Option<BackgroundMode>`: `Solid` vs `Image`. When loading legacy JSON where `background_mode` is omitted, infer `Image` if `background_image.is_some()` else `Solid`; serialize cleanly as `"solid"` / `"image"`.
  - `background_accent_enabled: bool` (defaults to `true`).
  - `background_accent: Option<[u8; 3]>` (persisted RGB seed extracted on import).
- **Toggle behavior**:
  - Clicking **Solid** sets `background_mode = BackgroundMode::Solid` and saves immediately without touching `background_image`, `background_accent`, or the managed PNG file on disk.
  - Clicking **Image** sets `background_mode = BackgroundMode::Image` when an image exists (and triggers `ensure_background_texture` if not yet uploaded); if no image is saved yet, clicking **Image** launches the file dialog directly so the user is never stuck on an inactive toggle.
  - Clicking **Clear** remains the destructive reset: deletes the managed PNG, clears `background_image` and `background_accent`, and resets `background_mode` to `Solid`.

### 4. Background Accent Extraction & Dual-Tone Derivation
- **Extraction (`extract_background_accent(rgba, width, height) -> Option<[u8; 3]>`)**:
  - Run inside `prepare_background_import` on the worker thread (and as a lazy backfill if loading a pre-existing managed background with `background_accent == None`).
  - Sample pixels (strided down to a bounded grid, ignoring transparent pixels `a < 128`).
  - Convert each sample to HSL; filter out near-neutral/achromatic pixels (`S < 0.14`, `L < 0.08`, or `L > 0.92`) and score hue buckets (`36` buckets of `10°`) weighted by `saturation^1.5 * chroma_balance`.
  - Average the winning hue bucket's RGB to produce a clean chromatic `[u8; 3]` seed; return `None` only for purely grayscale/monochrome images (falling back gracefully to the default Ayu accent).
- **Dual-tone token derivation (`ThemeTokens::for_mode_with_accent(dark_mode, accent_seed)`)**:
  - Convert `[u8; 3]` seed to HSL `(H, S_raw, _)`, clamping saturation to `[0.45, 0.88]`.
  - Dark mode applies brighter tones (`accent` `L >= 0.66`, `primary_bg` with `best_contrast_fg`), while Light mode applies deeper tones (`accent` `L <= 0.39`, `primary_bg` ensured `>= 4.5` contrast).
- **CanvasPalette accent integration**:
  - `CanvasPalette::for_mode_with_accent(dark_mode, accent_seed)` derives `placeholder_stroke`, `caret`, `selection_bg`, `ime_fg`, `scroll_inactive`, `scroll_hover`, and `scroll_active` directly from the active mode's `ThemeTokens`.

### 5. Full Accent Propagation Without Per-Frame Thrashing
- Store `applied_accent_seed: Option<[u8; 3]>` on `App` and sync `ThemeTokens::apply_accent_to_style(ctx, seed)` only when `active_accent_seed()` changes, preserving `GpuProfile` styling.
- Pass `self.theme_tokens(ui)` explicitly to `palette_ui.rs`, `settings_ui.rs`, `batch_ui.rs`, `session.rs`, and `keyboard.rs` (`draw_command_popup`), and paint the status-bar readiness circle using `self.theme_tokens(ui).accent` while leaving status text neutral/error-colored.
- In `draw_command_popup` (`desktop/src/app/keyboard.rs`), use `self.theme_tokens(ui)` so both the focused popup border and the clickable leading `\` prefix reflect the active `tokens.accent` across Ayu default and adaptive image modes, and set `.align(egui::Align2::LEFT_CENTER)` on `egui::TextEdit::singleline` so the `\` label and typed command text share the same vertical baseline/center line instead of top-aligning the `TextEdit` galley.

### 6. Settings UI Cards, Thumbnail Alignment, Opaque Disabled Slider, and Status Bar Importing
Split Settings into four `full_width_card` blocks (`Canvas zoom`, `Appearance`, `App background`, `Startup focus`) preserving `30px` button heights and `4px`/`8px` spacing:
1. **Appearance** card: **Light | Dark** button row.
2. **App background** card:
   - **Solid | Image** button row (`with_selectable_style`).
   - Vertically centered row (`egui::Layout::left_to_right(egui::Align::Center)`) containing the `72×48` rounded thumbnail preview, `Choose image…`, and `Clear` without displaying filename/mode text alongside the thumbnail. (Error text renders on its own line below only when an import/load error occurs.)
   - Adaptive accent checkbox row (`ui.add_enabled_ui(has_active_image_bg, ...)` with `Self::accent_checkbox(ui, tokens, &mut self.background_accent_enabled, false, "Match accent color to background")`).
   - `Overlay opacity` slider row (`ui.add_enabled_ui(has_active_image_bg, ...)`): uses `.step_by(0.01)` so `100%` maps cleanly to `1.0`; when disabled, restore `ui.set_opacity(1.0)` and apply `ThemeTokens::disabled_accent()` + opaque `noninteractive` handle fill (`a == 255`) so the trailing fill never shows through the handle while labels and tracks stay visibly muted (`text_disabled` / `control_disabled_bg`).
3. **Status bar importing progress**: While `self.background_importing` is `true`, render `"Importing background image…"` in `draw_status_bar` (`desktop/src/app/mod.rs`) instead of inside Settings.

### 7. Full-Range Panel/Card Opacity (`0%`..`100%`) With `65%` Max Scrim
- `with_background_opacity` maps `surface_alpha` across `0.38..=1.00` (`((0.38 + 0.62 * opacity) * 255.0).round() as u8`) so `panel_bg` and `card_bg` (sidebar, footer, status bar, Settings/Batch/About cards) reach full `255` opacity with zero panel bleed at `100%`.
- `app_bg` stays `Color32::TRANSPARENT` while `Image` mode is active, and `background_scrim` scales linearly across `0.00..=0.65` (`(opacity * 0.65 * 255.0).round() as u8`). Thus open central backdrop regions and the Settings thumbnail always retain the wallpaper (dimmed by at most `65%` at `100%` overlay opacity instead of turning solid black/solid surface), while `0%` leaves the scrim transparent (`0`) and keeps panels at the readable `0.38` floor.

### 8. Control-Level Accent Coverage (Progress Bar, Selections, Checkboxes/Radios, Hover & Focus Borders)
- **Batch progress bar (`draw_batch_progress_bar` in `desktop/src/app/batch_ui.rs`)**: Paint the track in `tokens.extreme_bg` with `tokens.border` outline, fill progress using `tokens.primary_bg` (skipping the fill rect completely at `0%` so there is no partial stub), and paint a single `Color32::PLACEHOLDER`-colored galley twice through clipped regions (`bar_rect.intersect(rect)` in `tokens.primary_fg`, remaining track rect in `tokens.text_primary`) so text straddling the bar edge stays legible.
- **Popup & dropdown selections (`App::with_selectable_style`)**: Wrap `\command` popup `SelectableLabel` rows (`desktop/src/app/keyboard.rs`) and `ComboBox` rows (`batch-target` in `batch_ui.rs`; `matrix-alignment` / `matrix-fence` in `settings_ui.rs`) with `with_selectable_style` so selected items adopt `selected_bg`, `selected_border`, and `selected_fg`.
- **Checkboxes & footer radio buttons (`App::accent_checkbox`, `App::accent_radio_value` in `palette_ui.rs`)**:
  - Keep captions neutral (`tokens.text_primary` when enabled, `tokens.text_disabled` when disabled) so controls stay calm rather than visually noisy.
  - When enabled and marked/selected, tint the indicator fill/border/checkmark with `selected_bg`, `selected_border`, and `selected_fg`.
  - When disabled, call `ui.set_opacity(1.0)` and populate all four widget visual states (`noninteractive`, `inactive`, `hovered`, `active`) with `control_disabled_bg` (`weak_bg_fill` + `bg_fill`) and `disabled_accent()` (when marked/selected) or `border` (when unmarked/unselected) so disabled states render crisply without painter double-dimming.
- **Global hover borders & `DragValue` focus stroke (`ThemeTokens::control_hover_border` in `theme.rs`)**:
  - Blend `border_hover` toward `accent` (`blend_rgb(self.border_hover, self.accent, if self.dark_mode { 0.28 } else { 0.24 })`) for `widgets.hovered.bg_stroke`, wire `widgets.active.bg_stroke` and `widgets.open.bg_stroke` to `self.accent`, and wire `selection.stroke` to `self.selected_border` so interactive `DragValue` text edits also pick up the active `accent` focus ring.
  - Intentionally leave Equation Canvas / LaTeX source box borders (`pal.border`) and Batch folder tree connector `LineSegment`s (`tokens.border_hover`) neutral.

## Risks / Trade-offs

- **Desaturated or near-monochrome wallpapers** → If no chromatic hue bucket passes the saturation threshold, `extract_background_accent` returns `None` and the app cleanly falls back to the default Ayu amber/orange accent.
- **Legacy `settings.json` with old `canvas_light` / `canvas_dark` keys** → Removing those fields from `Settings` while keeping `RawSettings` permissive ensures old config files load without error and transition automatically to Ayu Light / Ayu Dark.
