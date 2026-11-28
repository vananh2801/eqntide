# Proposal

## Why

The desktop UI currently uses inconsistent control styles across the main editor footer and Settings: the footer `MathType` / `OMML` selector uses circular radio indicators instead of the selectable button style used in Settings, and the `Overlay opacity` / `Canvas opacity` controls in Settings use `egui::Slider` bars with a hidden `0%..=100%` to `38%..=100%` (and `0%..=65%` scrim) remapping instead of matching the footer's level-fill `DragValue` controls and displaying real percentages.

## What Changes

- Replace the circular `MathType` and `OMML` radio buttons in the editor footer with selectable `30px` option buttons styled identically to the option selectors in **Settings** (`with_selectable_style` + `standard_button`), using standard `8px` (`SPACE_MD`) spacing and vertical centering aligned with the other footer controls.
- Remove the now-unused `accent_radio_value` helper and radio-circle indicator expectations for the footer target selector.
- Replace the `egui::Slider` + external right-hand `%` label for **Overlay opacity** and **Canvas opacity** in **Settings → App background** with `220px`-wide, `30px`-tall level-fill `DragValue` boxes matching the footer **Font size** and **Zoom** controls (proportional soft-accent left fill, centered `%` readout inside the bar, horizontal drag, direct numeric entry, and crisp disabled state), while disabling mouse-wheel stepping on those two Settings bars so scrolling over them scrolls the Settings page cleanly instead of altering opacity.
- Remove the `0%..=100%` -> `38%..=100%` surface-alpha remapping: expose and persist both **Overlay opacity** (`background_opacity`) and **Canvas opacity** (`canvas_opacity`) directly on their true `38%..=100%` (`0.38..=1.00`) range, with `panel_bg`/`card_bg` and `CanvasPalette::bg` using that exact alpha directly, while **Overlay opacity** continues to drive the full-window `background_scrim` across `0%..=65%` (`38% -> 0%` to `100% -> 65%`) without displaying a separate scrim percentage on the bar.

## Capabilities

### New Capabilities

<!-- None -->

### Modified Capabilities

- `desktop-ui-theme`: Update footer target selector from circular radio buttons to `30px` selectable option buttons matching Settings; replace Settings opacity sliders and `Accent-matched slider handle border` expectations with `220px` level-fill `DragValue` controls displaying `38%..=100%` directly without surface-alpha remapping (while scaling `background_scrim` `0%..=65%` across `38%..=100%`) and without mouse-wheel adjustment over the Settings scroll area.
- `desktop-settings-persistence`: Update `background_opacity` and `canvas_opacity` valid clamped range from `0.0..=1.0` to `0.38..=1.00` (snapped to `1%` increments, with readable defaults preserved).

## Impact

- Desktop UI chrome and widgets: `desktop/src/app/mod.rs`, `desktop/src/app/palette_ui.rs`, `desktop/src/app/settings_ui.rs`
- Theme tokens and alpha calculation: `desktop/src/theme.rs`
- Settings ranges and sanitization: `desktop/src/settings.rs`
- Desktop UI and theme tests: `desktop/src/app/tests/layout_theme_tests.rs`, `desktop/src/app/tests/batch_settings_tests.rs`, `desktop/src/app/tests/canvas_input_tests.rs`, `desktop/src/theme.rs`, `desktop/src/settings.rs`
