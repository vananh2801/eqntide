# Proposal

## Why

The desktop equation editor currently relies largely on `egui`'s default colors, inconsistent spacing and corner radii (`4px`, `6px`, `12px`), and fragmented hover/selection feedback (such as buttons expanding by `1px` on hover, hard-coded `Save` button fill, and black sidebar icons rendered on a blue selected background in Light mode). Establishing a cohesive, modern slate design system with unified hover/interaction states and customizable Equation Canvas presets will make the UI cleaner, easier on the eyes, and consistent across every control.

## What Changes

- Introduce a unified **Modern Slate / Soft Neutral** design token system for both Light and Dark themes with a clear 3-tier surface hierarchy (`base`, `surface`, `control`), readable contrast, and a unified ocean-blue accent.
- Standardize **hover, active, focus, disabled, and selected** feedback across all interactive controls (buttons, sidebar categories and tabs, palette tiles, radio buttons, checkboxes, combo boxes, sliders, drag values, and canvas color swatches): disable jarring hover border expansion, apply consistent hover surface/border transitions and `PointingHand` cursor feedback, and style selected sidebar items with a **tinted accent** background, thin accent border, and high-contrast foreground text/icons.
- Standardize **technical corner radii** (`6px` for controls/inputs/buttons/sliders, `8px` for canvas/cards/modals/popups) and harmonize panel margins, inner padding, and spacing on a consistent `4px` scale (`4 / 8 / 12 / 16px`), including unified `Status` and `Footer` bar framing, while preserving `1040x560` non-scrolling sidebar and 3-row palette layout guarantees.
- Add curated **Equation Canvas color presets** separately for **Light mode** and **Dark mode** in the `Settings` view, defaulting to a **soft light paper** preset in both modes while offering dark-surface presets for Dark mode, automatically adapting equation ink tint, placeholder masking/styling, caret, selection, and scrollbar colors to each preset.
- Extend persisted desktop preferences to save, validate, and restore the selected Light and Dark canvas presets (`canvas_light` and `canvas_dark`).

## Capabilities

### New Capabilities
- `desktop-ui-theme`: Cohesive Light/Dark slate color tokens, technical corner-radius and spacing scale, unified hover/cursor/tinted-accent interaction states, and curated per-mode Equation Canvas presets with adaptive ink and overlay contrast.

### Modified Capabilities
- `desktop-settings-persistence`: Extend persisted user preferences scope and validation to store and restore independent Equation Canvas color presets for Light mode and Dark mode.

## Impact

- `desktop/src/theme.rs` (new module): Design tokens for Light/Dark themes, `CanvasPreset` catalog and per-preset rendering palette (background, border, ink, placeholder, caret, selection, scrollbar), and `egui::Style` configuration helper.
- `desktop/src/settings.rs`: Add `canvas_light` and `canvas_dark` fields to `Settings` with safe preset validation and default fallbacks (`SoftPaper`).
- `desktop/src/app.rs`: Apply theme tokens at startup and across UI panels (`status_bar`, `sidebar`, `footer`, `palette_ui`, `editor_ui`, `canvas`, `paint_placeholder`, `batch_ui`, `settings_ui`, `about_ui`), unify hover/cursor/selected states, add canvas preset selectors in `Settings`, and update headless UI tests that previously asserted legacy `Color32::WHITE` / `LIGHT_GRAY` / fixed icon-tint values.
- `desktop/src/main.rs`: Register `mod theme;`.
