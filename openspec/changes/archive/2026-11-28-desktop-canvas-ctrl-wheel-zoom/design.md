# Design

## Context

See `proposal.md` for motivation and scope, and the delta specs in `specs/` for behavioral requirements.

In the desktop app (`desktop/src/`):
- **Equation Canvas** ([`App::draw_canvas`](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/src/app/canvas.rs#L36-L244)) renders a rasterized equation alpha mask scaled by `scale = self.zoom * 96.0 / 72.0` inside `egui::ScrollArea::new([scroll_x, scroll_y]).id_salt("formula-scroll")`. `draw_canvas` is wrapped in `ui.add_enabled_ui(!self.draft_dirty, ...)` inside [`App::draw_editor`](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/src/app/canvas.rs#L28-L30), while `draw_shell` disables the editor tab during `busy`, `pending`, `matrix_dialog`, and `shortcut_conflict`.
- **Footer Zoom** ([`App::draw_footer`](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/src/app/mod.rs#L921-L962)) binds directly to `self.zoom` (`1.5..=4.0`, `0.1` step on wheel), updates `self.persisted.zoom`, and sets `self.settings_dirty = true` for end-of-frame atomic flush in `draw_ui`.
- **LaTeX source box** ([`App::draw_latex_panel`](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/src/app/canvas.rs#L247-L288)) renders an `egui::TextEdit::multiline(&mut self.latex)` inside a fixed-height `88px` panel (`80px` input height) with `ScrollArea::vertical().id_salt("latex-scroll")` and currently hardcodes `.font(egui::TextStyle::Monospace)` (`UI_FONT_SIZE = 13.0`).
- **egui 0.37 wheel semantics**: When `wheel.modifiers` matches `options.zoom_modifier` (default `Modifiers::COMMAND`, i.e., `Ctrl` on Windows/Linux), `egui` zeroes `smooth_scroll_delta` so `ScrollArea` does not scroll, while preserving raw `egui::Event::MouseWheel { unit, delta, modifiers, .. }` entries in `input.raw.events`.

## Goals / Non-Goals

**Goals:**
- Support `Ctrl+Wheel` zoom on both the Equation Canvas and `LaTeX source` box without triggering unwanted `ScrollArea` scrolling.
- Keep Equation Canvas zoom cursor-anchored and synchronized two-way with the footer **Zoom** bar in a single frame (zero 1-frame scroll jump).
- Persist and sanitize `latex_font_pt` cleanly in `Settings` (`settings.json`) while keeping existing `settings.json` files backward-compatible.

**Non-Goals:**
- Do not change the scope of the footer **Zoom** or **Font size** controls.
- Do not add a new UI slider/input for `LaTeX source` font size or resize the `88px` / `80px` LaTeX panel.
- Do not zoom the inline `\command` popup (`draw_command_popup`) or global `egui::Context::zoom_factor()`.

## Decisions

### 1. Discrete `Ctrl+Wheel` step extraction from `input.raw.events`
- **Choice**: Extract vertical wheel delta from `ui.ctx().input(|input| ...)` over `input.raw.events` filtering `egui::Event::MouseWheel { unit, delta, modifiers, .. }` where `modifiers.ctrl || modifiers.command` (supporting both Ctrl on Linux/Windows and Cmd/Ctrl on macOS), and derive discrete step direction cleanly:
  - Sum `delta.y` (or fallback `delta.x` only if needed; standard vertical wheel uses `delta.y`).
  - For `MouseWheelUnit::Line` / `Page`, each event notch steps by `±1` step (`delta.y.signum()`). For smooth `MouseWheelUnit::Point` trackpad streams, accumulate per-widget residual delta in `egui` temp memory (`Id::with("ctrl-wheel-accum")`) and emit a step per threshold (e.g., `18.0` points) so trackpads do not jump across the full range in a few frames while discrete `Line` wheels respond immediately on every notch.
- **Alternative considered**: Reading `input.zoom_delta()`. Rejected because `zoom_delta()` is exponential/smoothed across frames over `smooth_wheel_delta` and would not align cleanly with discrete `10%` (`0.1`) and `0.5 pt` increments or instant unit-test injection of single `MouseWheel` events.

### 2. Same-frame cursor-anchored `ScrollArea` offset update on Equation Canvas
- **Choice**: Handle Equation Canvas `Ctrl+Wheel` at the start of `draw_canvas` (after `ui.is_enabled()` and palette lookup, before computing `scale`, `content_size`, `scroll_x`, and `scroll_y`):
  1. Compute the canvas viewport rect (matching the inner area inside `Frame::inner_margin(theme::SPACE_LG)` clamped to `height`, or reading the stored prior-frame `inner_rect` from `ctx` temp data and falling back to the outer rect) and check `ui.rect_contains_pointer(canvas_rect)` so overlapping foreground layers such as the inline `\command` popup (`Order::Foreground`) naturally block canvas pointer hits.
  2. When a non-zero step changes `self.zoom` (`old_zoom -> new_zoom`, clamped to `ZOOM_RANGE` and snapped to `0.1` steps):
     - Read the prior `ScrollArea` offset `old_offset` via `egui::scroll_area::State::load(ui.ctx(), scroll_id).map_or(Vec2::ZERO, |s| s.clamped_offset())`, where `scroll_id = ui.make_persistent_id("formula-scroll")`.
     - Compute `cursor_in_viewport = (pointer_pos - viewport_min).clamp(Vec2::ZERO, viewport_size)`.
     - With the constant content-to-equation padding `pad = Vec2::new(4.0, 8.0)` (`origin = rect.min + pad`), the content point under the cursor scales around `pad` by `ratio = new_zoom / old_zoom`:
       `new_offset = (pad + (old_offset + cursor_in_viewport - pad) * ratio - cursor_in_viewport).max(Vec2::ZERO)`.
     - Clamp `new_offset` to `(new_content_size - new_viewport_size).max(Vec2::ZERO)` (computed using the newly updated `scale` and gutter-aware `scroll_x`/`scroll_y`) and pass `.scroll_offset(new_offset)` to `ScrollArea` in this same frame.
     - Update `self.persisted.zoom = self.zoom` and `self.settings_dirty = true`.
- **Rationale**: Applying the new zoom and `.scroll_offset(new_offset)` before `ScrollArea::show_viewport` ensures `content_size`, `scroll_x`/`scroll_y` scrollbar visibility, texture bounds, caret, selection, and scroll position all reflect the anchored zoom on the exact same frame without a 1-frame jump.

### 3. `LaTeX source` font sizing and persistence
- **Choice**:
  - Add `LATEX_FONT_PT_RANGE: RangeInclusive<f32> = 8.0..=32.0` and `DEFAULT_LATEX_FONT_PT: f32 = theme::UI_FONT_SIZE` (`13.0`) in `desktop/src/settings.rs`.
  - Add `pub(crate) latex_font_pt: f32` to `Settings` and `RawSettings` (defaulting to `DEFAULT_LATEX_FONT_PT` via `#[serde(default)]` when omitted in existing `settings.json`), and sanitize it in `Settings::sanitize()` by clamping to `8.0..=32.0` and snapping to `0.5 pt` (`(clamped * 2.0).round() / 2.0`), falling back to `DEFAULT_LATEX_FONT_PT` if non-finite.
  - Store `pub(super) latex_font_pt: f32` on `App`, loaded in `apply_settings()` and untouched by `load_math()` / Office sessions.
  - In `draw_latex_panel`, check `ui.is_enabled()` and `ui.rect_contains_pointer(panel_rect)` for `Ctrl+Wheel` (`±0.5 pt`, clamped/snapped to `8.0..=32.0`), update `self.latex_font_pt`, `self.persisted.latex_font_pt`, and `self.settings_dirty = true`, and pass `.font(egui::FontId::monospace(self.latex_font_pt))` to `TextEdit::multiline`.
- **Alternative considered**: Modifying `TextStyle::Monospace` in `ui.style_mut()`. Rejected because the inline `\command` popup (`draw_command_popup`) also uses `TextStyle::Monospace` and must stay at `UI_FONT_SIZE`.

### 4. Disabled state parity with footer **Zoom**
- **Choice**: Equation Canvas `Ctrl+Wheel` runs only when `ui.is_enabled()` inside `draw_canvas`, which is already gated by `!self.draft_dirty` (in `draw_editor`) and `!self.busy && self.pending.is_none() && self.matrix_dialog.is_none() && self.shortcut_conflict.is_none()` (in `draw_shell`) — identical to the footer **Zoom** control. `draw_latex_panel` is outside `!self.draft_dirty`, so `LaTeX source` `Ctrl+Wheel` continues working while editing a LaTeX draft and disables only when modals or `busy` disable the central panel.

## Risks / Trade-offs

- **Pointer-over-frame vs pointer-over-scrollbar**: Using the inner `ScrollArea` viewport rect (recorded as `inner_rect` from `ScrollAreaOutput` or derived from the frame) keeps the cursor-anchor math exact relative to `viewport_min`; if the pointer hovers the `Frame` padding (`SPACE_LG`) just inside the rounded canvas box, clamping `cursor_in_viewport` into `[0, viewport_size]` still anchors cleanly at the nearest viewport edge without NaNs or negative coordinates.
- **Residual scroll suppression**: `egui` suppresses `smooth_scroll_delta` whenever `zoom_modifier` (`COMMAND`) is active; if a platform sends `Ctrl` without `COMMAND` (none on Windows/Linux, possible only if someone maps `Ctrl` separately on macOS), also clearing `smooth_scroll_delta` when handling `Ctrl+Wheel` prevents accidental scroll.
- **Large LaTeX font sizes in `80px` box**: At `32 pt`, line height exceeds half the `80px` viewport; `ScrollArea::vertical()` ("latex-scroll") remains intact so users can still scroll through lines cleanly.
