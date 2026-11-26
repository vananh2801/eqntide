# Proposal

## Why

When a custom wallpaper image is active in EqnTide Desktop, `Overlay opacity` controls the translucency of surrounding chrome panels and cards, while the Equation Canvas and LaTeX source box remain 100% opaque. Adding a dedicated `Canvas opacity` slider lets users blend the Equation Canvas and LaTeX source box into their custom background image while preserving readable contrast and keeping the default opaque look intact.

## What Changes

- Add a `Canvas opacity` slider (`0%`..`100%`, `1%` steps, defaulting to `100%`) inside Settings → **App background** directly below `Overlay opacity`, enabled only when **Image** mode has an active custom background and disabled (with an opaque muted handle) in **Solid** mode.
- Scale the background fill alpha of both the **Equation Canvas** and **LaTeX source** box across `0.38..=1.00` (matching `Overlay opacity`'s readable surface-alpha mapping) when **Image** background mode is active, while keeping both surfaces opaque (`255` alpha) in **Solid** mode.
- Clear rasterized placeholder glyph ink (`□`) from the equation alpha texture so translucent Equation Canvas backgrounds reveal the background cleanly inside dashed placeholders without opaque cover patches or double alpha blending.
- Persist `canvas_opacity` in `settings.json` with safe clamping (`0.0..=1.0`, defaulting to `1.0` when missing or non-finite).

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- `desktop-ui-theme`: Add `Canvas opacity` control layout/gating below `Overlay opacity` in **App background**, apply `0.38..=1.00` translucent `CanvasPalette::bg` to the Equation Canvas and LaTeX source box when **Image** mode is active, and keep unfilled dashed placeholders clean over translucent canvas fills.
- `desktop-settings-persistence`: Persist, sanitize, and default `canvas_opacity` (`1.0` default, clamped to `0.0..=1.0`) alongside existing custom app background settings.

## Impact

- `desktop/src/settings.rs`: `Settings::canvas_opacity`, `DEFAULT_CANVAS_OPACITY`, `CANVAS_OPACITY_RANGE`, deserialization default, and sanitization.
- `desktop/src/theme.rs`: `CanvasPalette::with_canvas_opacity` (and helper constructor accepting canvas opacity).
- `desktop/src/app/mod.rs`, `desktop/src/app/settings_ui.rs`: `App::canvas_opacity` state, `active_canvas_palette` / `apply_settings`, and `draw_canvas_opacity_row` below `draw_overlay_opacity_row`.
- `desktop/src/app/events.rs`, `desktop/src/app/canvas.rs`: Clear placeholder region pixels from the rendered equation mask/texture and remove the opaque `pal.bg` cover rectangle in `paint_placeholder`.
- `desktop/src/app/tests/`: Unit and UI integration tests for `Canvas opacity` layout, disabled solid-mode behavior, translucent canvas/LaTeX rendering, clean translucent placeholders, and settings persistence.
