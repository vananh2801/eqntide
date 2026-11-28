# Design

## Context

See `proposal.md` for motivation and scope. Currently:
- `desktop/src/app/mod.rs` (`draw_footer`) renders `MathType` and `OMML` using `App::accent_radio_value` (`desktop/src/app/palette_ui.rs`) with an extra `theme::SPACE_MD` spacer between them, while `Font size`, `Zoom`, `Undo`/`Redo`, and `Save` use `30px` (`FOOTER_CONTROL_HEIGHT = theme::BUTTON_HEIGHT`) controls aligned via `ui.horizontal_centered`.
- `desktop/src/app/settings_ui.rs` (`draw_app_background_card`) renders `Light`/`Dark`, `Solid`/`Image`, and `Equation canvas`/`LaTeX source` using `Self::with_selectable_style(ui, tokens, selected, |ui| Self::standard_button(ui, label))`.
- `desktop/src/app/settings_ui.rs` (`draw_opacity_slider_row`) renders `Overlay opacity` and `Canvas opacity` using `egui::Slider` (`slider_width = 220.0`, rounded-square handle) plus a separate `%` text label on the right.
- `desktop/src/settings.rs` defines `BACKGROUND_OPACITY_RANGE = 0.0..=1.0` (`DEFAULT_BACKGROUND_OPACITY = 0.70`) and `CANVAS_OPACITY_RANGE = 0.0..=1.0` (`DEFAULT_CANVAS_OPACITY = 1.0`).
- `desktop/src/theme.rs` (`MIN_SURFACE_ALPHA = 0.38`, `MAX_BACKGROUND_SCRIM_ALPHA = 0.65`) maps `0.0..=1.0` onto `0.38..=1.00` for `panel_bg`, `card_bg`, and `CanvasPalette::bg`, and onto `0.00..=0.65` for `background_scrim`.

## Goals / Non-Goals

**Goals:**
- Unify footer `MathType` / `OMML` buttons with Settings selectable option buttons (`30px`, borderless, `8px` spacing, vertically centered with adjacent footer controls).
- Unify Settings `Overlay opacity` and `Canvas opacity` controls with the footer `Font size` and `Zoom` level-fill `DragValue` widget (`220px × 30px`, centered `%` text, no handle, no external `%` label), without mouse-wheel capture inside the scrollable Settings view.
- Eliminate surface-alpha remapping so `background_opacity` and `canvas_opacity` store, display, and directly apply `0.38..=1.00` (`38%..=100%`) surface alpha, while `background_scrim` maps `[0.38, 1.00]` linearly to `[0.00, 0.65]`.

**Non-Goals:**
- Changing `Font size` or `Zoom` ranges, widths (`96px`), or hover mouse-wheel behaviors in the editor footer.
- Exposing a separate scrim percentage readout or third slider in Settings.
- Changing canvas placeholder rendering or adaptive accent extraction logic.

## Decisions

### 1. Footer Target Buttons (`MathType` / `OMML`)
- Replace the two `Self::accent_radio_value(...)` calls and the extra `ui.add_space(theme::SPACE_MD)` in `draw_footer` (`desktop/src/app/mod.rs`) with a loop over `[("mathtype", "MathType"), ("omml", "OMML")]` calling `Self::with_selectable_style(ui, tokens, selected, |ui| Self::standard_button(ui, label))`.
- Clicking a non-selected target sets `self.target`, updates `self.persisted.target`, sets `self.settings_dirty = true`, and sets `self.pending_focus_restore = Some(self.active_editor_focus)` (even clicking the already-selected button restores editor focus, preserving existing UX).
- Keep `selected` passed to `with_selectable_style` (matching `settings_ui.rs` and `draw_sidebar`) so disabled dimming from `ui.add_enabled_ui(!self.draft_dirty, ...)` and outer modal gating fades the selected button cleanly without losing visual indication of the active target.
- Remove `App::accent_radio_value` from `desktop/src/app/palette_ui.rs` because no other call site exists.
- **Alternative considered**: Keeping `selected && ui.is_enabled()` on the footer buttons. Rejected because Settings option buttons (`Light`/`Dark`, `Solid`/`Image`, `Startup focus`) pass `selected` directly and rely on egui disabled dimming; matching Settings keeps behavior uniform.

### 2. Shared Level-Fill `DragValue` Helper for Settings Opacity Controls
- Promote `App::draw_level_drag_value` (`desktop/src/app/mod.rs`) to `pub(super)` and parameterize mouse-wheel stepping as `wheel_steps_per_unit: Option<f64>` so `settings_ui.rs` reuses the exact same Krita-style track/fill painter, `1px` `tokens.accent` text-edit focus border, and `soft_accent_fill` while keeping wheel stepping `Some(...)` for footer `Font size` / `Zoom` and `None` for Settings opacity bars.
- Because the **Settings** tab sits inside `egui::ScrollArea::vertical()`, disabling mouse-wheel stepping (`wheel_steps_per_unit = None`) on **Overlay opacity** and **Canvas opacity** avoids fighting vertical page scrolling—wheeling over the bars scrolls the Settings page cleanly without mutating opacity.
- In `settings_ui.rs`, replace `egui::Slider` and the trailing `format!("{:.0}%", *value * 100.0)` label in `draw_opacity_slider_row` with `App::draw_level_drag_value`:
  - Row height: `theme::BUTTON_HEIGHT` (`30.0`), left label width: `108.0`, control width: `220.0`, `Layout::left_to_right(egui::Align::Center)`.
  - `range`: `settings::BACKGROUND_OPACITY_RANGE` (`0.38..=1.00`) and `settings::CANVAS_OPACITY_RANGE` (`0.38..=1.00`).
  - `wheel_steps_per_unit`: `None` (horizontal drag and click-to-type numeric entry remain active; tooltip reads `"Click and drag left/right, or click to type a percentage"`).
  - `speed`: `f64::from((*range.end() - *range.start()) / 220.0)` (`0.62 / 220.0`) so dragging across the `220px` bar traverses the full `38%..=100%` span smoothly.
  - `DragValue::from_get_set` snaps to `1%` (`(value * 100.0).round() / 100.0`), formats as `format!("{:.0}%", (value * 100.0).round())`, and parses percentage strings (`"85%"` or `"85"` -> `pct / 100.0`).
  - Disable `ui.add_enabled_ui(enabled, ...)` around `draw_level_drag_value` so `Solid` mode disables both bars identically to how `draft_dirty` disables footer `Font size` and `Zoom` (painting `control_disabled_bg`, `soft_accent_fill(false, false)`, and `text_disabled` text, and ignoring drag/click).
- **Alternative considered**: Consuming `raw.events` / `smooth_scroll_delta` inside `draw_level_drag_value` even in Settings. Rejected because `egui::ScrollArea` inside `CentralPanel` still scrolls on wheel events and user explicitly prefers Settings opacity bars not to respond to mouse-wheel scrolling.

### 3. Direct `0.38..=1.00` Opacity Range Without Surface-Alpha Remapping
- In `desktop/src/settings.rs`:
  - Update `BACKGROUND_OPACITY_RANGE` and `CANVAS_OPACITY_RANGE` to `0.38..=1.00`.
  - Keep `DEFAULT_CANVAS_OPACITY = 1.0`.
  - Update `DEFAULT_BACKGROUND_OPACITY` to `0.81` (closest `1%` integer step to the previous visual default `0.38 + 0.62 * 0.70 = 0.814`, keeping the out-of-the-box appearance and contrast margins nearly identical).
  - In `Settings::sanitize()`, clamp `background_opacity` and `canvas_opacity` to `0.38..=1.00` and snap to `0.01` (`(clamped * 100.0).round() / 100.0`) so stored values always align with `1%` steps and legacy values below `0.38` clamp safely to `0.38`.
- In `desktop/src/theme.rs`:
  - Replace `surface_alpha_for_opacity(raw, default)` with direct clamping `direct_surface_alpha(raw, default)`: clamp finite values to `MIN_SURFACE_ALPHA..=1.0` (`0.38..=1.00`) and convert directly `(alpha * 255.0).round() as u8`.
  - Update `ThemeTokens::with_background_opacity(self, overlay_opacity)`: `panel_bg` and `card_bg` use `direct_surface_alpha(overlay_opacity, 0.81)` directly.
  - Update `CanvasPalette::with_canvas_opacity(self, canvas_opacity)`: `self.bg` uses `direct_surface_alpha(canvas_opacity, 1.0)` directly.
  - Update `ThemeTokens::background_scrim(dark_mode, overlay_opacity)`: normalize `t = ((clamped - MIN_SURFACE_ALPHA) / (1.0 - MIN_SURFACE_ALPHA)).clamp(0.0, 1.0)` so `38%` yields `0%` scrim and `100%` yields `65%` (`MAX_BACKGROUND_SCRIM_ALPHA`) scrim, using `0.81` as the non-finite fallback opacity.

## Risks / Trade-offs

- **Legacy `settings.json` values below `0.38`** → Clamping to `0.38..=1.00` in `Settings::sanitize()` safely upgrades older configs without errors; users who previously had e.g. `background_opacity: 0.20` (which used to render at ~`50%` surface alpha) will see it start at `38%` (`0.38`), within the valid readable floor.
- **DragValue text color when opaque/translucent in Settings** → `draw_level_drag_value` uses `tokens.text_disabled` on `noninteractive` when disabled and `tokens.text_primary` when enabled, with `tokens.soft_accent_fill(hovered, enabled)` explicitly verified to preserve `>= 4.5` contrast against `tokens.text_primary` in `theme.rs` unit tests.
- **Existing UI tests inspecting radio circles and slider handles** → Tests in `layout_theme_tests.rs`, `batch_settings_tests.rs`, `canvas_input_tests.rs`, `theme.rs`, and `settings.rs` explicitly assert old radio-circle counts/colors, slider handle rects, and `0.38 + 0.62 * opacity` curves; those tests must be updated alongside the implementation to assert selectable footer buttons, `220×30px` level-fill bars, no wheel mutation in Settings, and direct `0.38..=1.00` alphas.
