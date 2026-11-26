# Design

## Context

`Settings` (`desktop/src/settings.rs`) currently stores `background_mode`, `background_image`, `background_opacity` (default `0.70`), `background_accent_enabled`, and `background_accent`. In `desktop/src/app/settings_ui.rs`, `draw_app_background_card` renders a `108px`-labeled `Overlay opacity` slider (`0%..100%`, `1%` steps) enabled only when `is_image_background_active()` is true. `ThemeTokens::with_background_opacity` maps `background_opacity` to surface alpha across `0.38..=1.00` for `panel_bg` and `card_bg`.

Meanwhile, both `draw_canvas` and `draw_latex_panel` (`desktop/src/app/canvas.rs`) read `pal.bg` from `App::active_canvas_palette` -> `CanvasPalette::for_mode_with_accent`, which currently always produces an opaque `255`-alpha `Color32`. Additionally, `events.rs` injects synthetic `<mi placeholder="true">□</mi>` nodes so `render_interactive_alpha_scaled` measures exact layout boxes for empty slots, and `paint_placeholder` currently covers the rasterized `□` glyph with `painter.rect_filled(rect.expand(0.5), 0.0, pal.bg)` before drawing the dashed border.

## Goals / Non-Goals

**Goals:**
- Provide an independent `Canvas opacity` slider directly below `Overlay opacity` in **Settings → App background**, defaulting to `100%` (`1.0`), mapped across `0.38..=1.00` alpha on `CanvasPalette::bg` for both the Equation Canvas and LaTeX source box.
- Disable `Canvas opacity` (while preserving an opaque muted handle) whenever **Solid** mode is active or no background image is loaded, keeping `CanvasPalette::bg` opaque (`255` alpha) in **Solid** mode.
- Eliminate the opaque `pal.bg` cover rectangle in `paint_placeholder` by removing placeholder glyph pixels from the rendered formula alpha image so translucent canvases render dashed slots cleanly.
- Persist `canvas_opacity` in `settings.json` with safe clamping and backward-compatible `1.0` fallback.

**Non-Goals:**
- Changing `Overlay opacity` (`background_opacity`) behavior, panel/card translucency, or window scrim alpha (`0.00..=0.65`).
- Making Equation Canvas or LaTeX source transparent in **Solid** mode.
- Changing math ink, caret, selection highlight, or frame border (`pal.border`) alpha rules.

## Decisions

### 1. Reuse the `0.38..=1.00` Surface-Alpha Curve on `CanvasPalette`
- Add `CanvasPalette::with_canvas_opacity(mut self, canvas_opacity: f32) -> Self` and `CanvasPalette::for_mode_with_background_and_accent(dark_mode, has_background, canvas_opacity, accent_seed)` in `desktop/src/theme.rs`, mirroring `ThemeTokens::with_background_opacity`.
- When `has_background` is true, `pal.bg` alpha becomes `((0.38 + 0.62 * opacity) * 255.0).round() as u8`, where `opacity` clamps finite values to `0.0..=1.0` and defaults non-finite values to `1.0`. When `has_background` is false, `pal.bg` stays fully opaque (`255`).
- **Why over `0.0..=1.0` linear alpha**: Matches `Overlay opacity` exactly as requested, guarantees a readable floor (`38%` backdrop) for thin monospace LaTeX text and small math subscripts over busy wallpapers, and preserves identical `255`-alpha defaults at `100%`.
- **Shared helper for `draw_overlay_opacity_row` / `draw_canvas_opacity_row`**: Extract a private helper in `desktop/src/app/settings_ui.rs` taking `(ui, tokens, enabled, label, value: &mut f32, range)` so `Overlay opacity` and `Canvas opacity` share identical `108px` label alignment, `220px` slider width, `1%` stepping, accent handle stroke, and opaque disabled handle styling.

### 2. Clean Placeholder Transparency Without `pal.bg` Cover Patches
- In `desktop/src/app/events.rs`, right after `render_interactive_alpha_scaled` returns `(mask, preview)` and `placeholders` is collected, clear the alpha bytes in `mask.alpha` inside each placeholder `HitRegion` (converting logical point bounds to mask pixel indices via `PX_PER_PT * effective_raster_scale`, clamped to `[0, width]` / `[0, height]`, with a `1px` safety margin only on the placeholder's own region if needed, or exact `floor..ceil` bounds).
- Wait — let's ensure clearing pixel bounds cannot clip neighboring glyphs (e.g., fraction bars or radical overbars adjacent to `□`): in `layout_math`, `HitRegion` bounds for `<mi>□</mi>` (`origin_x + hit.x` .. `hit.x + hit.w`, `origin_y + hit.y - hit.asc` .. `+ hit.desc`) reflect the `□` glyph box itself, whereas fraction bars (`mfrac`) sit above/below with rule gap padding. Clearing exact `floor(x * px_scale)..ceil((x + w) * px_scale)` and `floor(y * px_scale)..ceil((y + h) * px_scale)` (matching the `□` region without expanding into surrounding bars) removes the baked `□` ink cleanly, and `paint_placeholder` in `desktop/src/app/canvas.rs` simply drops `painter.rect_filled(rect.expand(0.5), 0.0, pal.bg);`!
- **Alternative considered**: Adding a `skip_placeholder_ink` mode to `eqntide::render::render_interactive_alpha_scaled`. Clearing the `placeholders` rects in `events.rs` keeps the core `eqntide` library crate API untouched and localized to desktop preview texture generation; if edge anti-aliasing around `□` leaves stray 1px ink in any font/scale, we can verify with unit/UI pixel tests during implementation and adjust rounding (`floor`/`ceil`) or filter in `render`.

### 3. Settings Persistence and Gating
- Add `DEFAULT_CANVAS_OPACITY: f32 = 1.0` and `CANVAS_OPACITY_RANGE: std::ops::RangeInclusive<f32> = 0.0..=1.0` to `desktop/src/settings.rs`.
- Add `pub(crate) canvas_opacity: f32` to `Settings` and `RawSettings` (defaulting to `1.0` via `#[serde(default)]` on `RawSettings` when absent in older `settings.json` files), sanitized via `is_finite()` + `.clamp(0.0, 1.0)` with `DEFAULT_CANVAS_OPACITY` fallback.
- Gate `Canvas opacity` in `settings_ui.rs` with `has_active_image_bg` (`self.is_image_background_active()`), identical to `Overlay opacity`. In **Solid** mode, changing `canvas_opacity` is disabled and `active_canvas_palette` passes `has_background = false` so both editors remain 100% opaque while preserving the user's saved `canvas_opacity` value for when **Image** mode is re-enabled.

## Risks / Trade-offs

- **Clearing placeholder pixels vs. adjacent glyph overlap** → Use exact `floor..ceil` pixel bounds of each placeholder `HitRegion` in `mask.alpha` (no outward `0.5pt` expansion) and add a test checking both zero ink left inside an empty slot and intact surrounding fraction/root strokes.
- **Software renderer (`SoftwarePainter`) compositing** → `SoftwarePainter` composites `egui` primitives (`egui_software_backend`) over the cached cover-cropped wallpaper + scrim buffer, so translucent `pal.bg` renders identically on both GPU (`wgpu`) and CPU (`softbuffer`) backends.
- **Scrollbar track visibility at low canvas opacity** → `configure_canvas_scroll_style` sets `extreme_bg_color = pal.scroll_track` only on the inner `ScrollArea` style (used for scrollbar track when scrolling is active), while `TextEdit::multiline` uses `Frame::NONE` and `ScrollArea` fills nothing on its viewport background, so the outer `Frame::new().fill(pal.bg)` remains the single translucent fill layer for both Equation Canvas and LaTeX box.
