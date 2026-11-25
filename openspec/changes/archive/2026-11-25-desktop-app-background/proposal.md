# Proposal

## Why

The EqnTide desktop application currently renders only flat slate neutral surfaces across Light and Dark modes, giving users no way to personalize the overall look of the workspace. Allowing users to choose a custom background image from the Settings tab makes the editor, Batch DOCX, Settings, and About views feel personal while keeping equations and controls crisp and readable.

## What Changes

- Add an **App background** section inside the **Appearance** card of the **Settings** tab allowing users to choose an image (`PNG`, `JPEG`, `WebP`), preview a thumbnail and status, adjust an overlay dimming/opacity slider, or clear the background back to the default solid theme.
- Import and normalize (copy and cap oversized dimensions) the selected image into the application configuration directory alongside `settings.json` so moving or deleting the source file does not break the configured background, and remove/replace the managed file when cleared or changed.
- Render the configured background image across the full desktop window behind all primary views (**Editor**, **Batch DOCX**, **Settings**, and **About**), applying a theme-aware tint/overlay and translucent chrome/cards (`app_bg`, `panel_bg`, `card_bg`) while keeping the **Equation Canvas** and **LaTeX source** surface opaque/crisp using their active `CanvasPreset`.
- Persist background configuration fields (enabled/imported image filename, overlay opacity) in `settings.json` with safe fallback when the image file is missing, unreadable, or corrupt.

## Capabilities

### New Capabilities

### Modified Capabilities
- `desktop-ui-theme`: Add custom full-window application background image rendering with translucent chrome and cards, theme-aware overlay opacity, Settings Appearance controls, and preserved opaque Equation Canvas / LaTeX source readability.
- `desktop-settings-persistence`: Persist, validate, and safely restore custom app background image settings and managed background image storage alongside `settings.json`.

## Impact

- Affected crate: `eqntide-desktop` ([desktop/Cargo.toml](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/Cargo.toml), [desktop/src/settings.rs](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/src/settings.rs), [desktop/src/theme.rs](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/src/theme.rs), [desktop/src/app.rs](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/src/app.rs)).
- Dependency impact: Enable `jpeg` and `webp` features on the existing `image` dependency in `desktop/Cargo.toml` (subject to `deny.toml` license/advisory check).
- Rendering backends: Must work consistently across `wgpu`, `glow`, and [`SoftwarePainter`](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/src/software_painter.rs).
