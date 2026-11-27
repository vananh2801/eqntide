# Design

## Context

See `proposal.md` for motivation and scope. Currently:
- [`App::draw_footer`](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/src/app/mod.rs#L851-L963) renders `Font size` as an `egui::DragValue` (`96px x 30px`, `speed = 4.0 / 96.0`) inside `ui.add_enabled_ui(!self.draft_dirty, ...)`. At `MIN_WINDOW_SIZE` (`1056px`), the footer has ~`384px` of horizontal slack, which easily fits a `Zoom` label and a matching `96px`-wide control on the same row.
- [`App::draw_settings_tab`](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/src/app/settings_ui.rs#L315-L373) renders the `Canvas zoom` card with a `220px` slider, a `60px` percentage `DragValue`, and `Reset (200%)`.
- [`App::draw_batch_progress_bar`](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/src/app/batch_ui.rs#L338-L398) already demonstrates custom rounded left-filled track painting under `CONTROL_CORNER_RADIUS`.

## Goals / Non-Goals

**Goals:**
- Relocate canvas zoom from Settings into the editor footer beside **Font size**, matching its interaction model and disabled gating (`!self.draft_dirty` and modal/busy gating).
- Provide a unified Krita-style left-filled visual presentation (soft blended accent fill on the left, neutral track on the right, centered readable text, no right-side spinner arrows, `1px` `tokens.accent` border only during text-edit focus).
- Support hover mouse-wheel stepping (`1 pt` for Font size, `10%` for Zoom) without stealing or losing canvas/LaTeX focus.
- Keep existing `egui::DragValue` interactions intact (horizontal relative drag, click-to-type exact numbers, `Enter`/`Escape`, `WidgetInfo` accessibility, and `layout_theme_tests` focus-border expectations).

**Non-Goals:**
- Adding a Reset button for Zoom in the footer or keeping a duplicate Zoom control in Settings.
- Adding Krita-style right-side up/down arrow buttons.
- Changing canvas mouse-wheel scrolling behavior on the Equation Canvas itself.

## Decisions

### 1. Keep `egui::DragValue` interaction engine with pre-painted soft-accent track
- **Choice**: Keep `egui::DragValue::from_get_set` as the underlying interactive widget inside a shared footer helper (`draw_level_drag_value`), making `DragValue`'s non-editing background fill transparent while painting the rounded track + proportional left soft-accent fill underneath, and letting `DragValue`'s `TextEdit` draw with `tokens.control_bg` / `1px tokens.accent` stroke only when actively editing text.
- **Alternatives considered**:
  - *Fully custom `allocate_rect` + manual `TextEdit` state machine*: Duplicates `DragValue`'s drag accumulator, click-vs-drag threshold, `TextEdit` focus/Escape/Enter handling, and `WidgetInfo` semantics, risking subtle regressions in existing `DragValue` focus tests.
- **Rationale**: Reusing `DragValue` preserves click-to-edit, relative drag, keyboard navigation, and existing test invariants while letting custom shapes behind it paint the exact Krita-style left fill. During non-editing states, setting `widgets.{noninteractive, inactive, hovered, active}.bg_fill = Color32::TRANSPARENT` ensures `DragValue` does not paint an opaque box over the left-side fill while keeping `TextEdit` focus borders (`extreme_bg_color = tokens.control_bg`, `1px tokens.accent` border) intact.

### 2. Soft-accent fill color derivation
- **Choice**: Add a helper on [`ThemeTokens`](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/src/theme.rs#L36) (`soft_accent_fill(self, hovered: bool, enabled: bool) -> Color32`) that blends `self.accent` over `self.control_bg` / `self.control_hover_bg` when enabled (stepping down weight if needed to preserve `>= 4.5` WCAG contrast against `self.text_primary`) and blends `self.disabled_accent()` over `self.control_disabled_bg` when disabled.
- **Alternatives considered**:
  - *Reusing `tokens.selected_bg` directly*: In Dark mode, `selected_bg` (`rgb(54, 41, 24)`) has lower luminance than `control_bg` (`rgb(47, 52, 64)`), so a left fill in `selected_bg` looks dimmer than the unfilled track rather than like a filled level bar.
  - *Using `tokens.primary_bg` with split-contrast text*: Higher contrast, closer to a progress bar than the requested soft accent ("accent dịu"), and unnecessary when `soft_accent_fill` keeps `tokens.text_primary` legible across the entire bar.
- **Rationale**: Blending `accent` toward `control_bg` works uniformly in Light mode, Dark mode, and adaptive image-accent hues while guaranteeing `text_primary` contrast without split clipping.

### 3. Clipping the left fill cleanly inside `CONTROL_CORNER_RADIUS`
- **Choice**: Paint the full track rectangle with `CONTROL_CORNER_RADIUS` (`6px`), then paint the left fill rectangle (`width = rect.width() * fraction`) using left-only `CornerRadius` (`CONTROL_CORNER_RADIUS` when `fraction >= 0.999`) and computing a vertical inset whenever `fill_width < radius` so narrow fills stay strictly inside the `6px` rounded outer corner.
- **Rationale**: Matches the outer `6px` rounded corners on the left without rounding the vertical cut line at intermediate values or leaking past the corner arc at small fractions.

### 4. Mouse-wheel handling and persistence
- **Choice**: Store `(drag_resp.id, rect)` in `ui.ctx().data_mut` per control (`ui.scope_id()`), and before rendering `DragValue` on subsequent frames when `ui.is_enabled()`, `!editing_text`, and `ui.rect_contains_pointer(widget_rect)`, sum `egui::Event::MouseWheel` events from `input.raw.events`, step by `delta.y.signum() / steps_per_unit` (`±1 pt` for `font_pt`, `±0.1` / `10%` for `zoom`), clamp and snap cleanly using `steps_per_unit`, and clear `input.smooth_scroll_delta.y = 0.0` when consumed.
- **Persistence**: When wheel adjusts a value, update `self.persisted.*`, set `self.settings_dirty = true` (and bump `self.render_key` for `font_pt`), and let the frame's existing `flush_settings()` write to disk immediately since no pointer button is down. Do **not** alter editor focus (`active_editor_focus`) on wheel-only interaction.
- **Drag speed & sanitization**: Keep `Font size` at `speed = 4.0 / width` snapped to `1.0 pt` (`steps_per_unit = 1.0`); set `Zoom` at `speed = 1.0 / width` snapped to `0.1` / `10%` (`steps_per_unit = 10.0`). Keep [`Settings::sanitize`](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/src/settings.rs#L292-L297) snapping stored `font_pt` to `0.5 pt` and `zoom` to `0.01` (`1%`) for backward compatibility with existing `settings.json` files.

## Risks / Trade-offs

- **Risk**: `egui::DragValue` text-edit focus detection vs. transparent idle fill could accidentally hide the `1px tokens.accent` border tested by `footer_radios_hover_borders_and_drag_value_focus_reflect_active_accent` in [`layout_theme_tests.rs`](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/src/app/tests/layout_theme_tests.rs#L2217-L2270).
  → **Mitigation**: Keep `extreme_bg_color = tokens.control_bg` and `selection.stroke` intact so active `TextEdit` focus still renders the `1px tokens.accent` border.
- **Risk**: Reading `smooth_scroll_delta` after `DragValue` causes repeated ticks during smooth-scroll decay frames.
  → **Mitigation**: Read discrete `egui::Event::MouseWheel` from `input.raw.events` before `DragValue` renders so each wheel event produces one exact step immediately and clears `smooth_scroll_delta.y`.
- **Risk**: Existing tests in [`batch_settings_tests.rs`](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/src/app/tests/batch_settings_tests.rs#L204-L310) expect `"Canvas zoom"`, `"200%"`, slider rail, and `"Reset (200%)"` in Settings (`app.tab == 2`).
  → **Mitigation**: Update those test sections to interact with the footer `Zoom` control on `app.tab == 0` and verify Settings no longer renders a `Canvas zoom` card.
