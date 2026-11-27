# Proposal

## Why

Adjusting canvas zoom currently requires switching away from the Editor tab to Settings, separating visual zoom from the live equation canvas and font size control. Moving the zoom control into the editor footer beside **Font size**, giving both controls a shared Krita-style left-filled level indicator, and supporting mouse-wheel adjustments makes both equation size and preview zoom fast, consistent, and directly observable while editing.

## What Changes

- **Move canvas zoom out of Settings into the editor footer**:
  - Remove the **Canvas zoom** card (slider, percentage `DragValue`, and `Reset (200%)` button) from the **Settings** tab.
  - Place a **Zoom** control directly beside **Font size** in the editor footer (without a Reset button), sharing the same `30px` row height, `draft_dirty` / modal disabled gating, drag-release persistence, and focus restoration behavior.
- **Unify interaction & add mouse-wheel adjustment**:
  - **Font size** (`6.0..=72.0 pt`) steps in `1 pt` increments via relative horizontal drag, direct numeric entry, and mouse wheel while hovered.
  - **Zoom** (`150%..=400%`) steps in `10%` (`0.1`) increments via relative horizontal drag (`speed = 1.0 / width`), direct numeric entry, and mouse wheel while hovered, while `Settings::sanitize()` continues sanitizing persisted `zoom` to `0.01` (`1%`) increments for backward compatibility.
- **Krita-style soft-accent left-fill appearance**:
  - Render both **Font size** and **Zoom** boxes with a borderless `6px`-rounded track and a proportional soft-accent left fill (`(value - min) / (max - min)`) behind the centered value text (without right-side up/down arrow buttons), while keeping external `Font size` and `Zoom` labels and preserving focused text-edit borders.

## Capabilities

### New Capabilities

### Modified Capabilities
- `desktop-ui-theme`: Remove the Settings **Canvas zoom** card/slider requirements and define the footer **Zoom** placement, Krita-style soft-accent left-fill rendering for both **Font size** and **Zoom** controls, and hover mouse-wheel stepping behavior (`1 pt` / `10%`).
- `desktop-settings-persistence`: Snap persisted `zoom` values to `1%` (`0.01`) increments on sanitization and persist changes made from the footer **Zoom** control (including mouse-wheel steps).

## Impact

- **UI & Theme (`desktop/src/app/mod.rs`, `desktop/src/app/settings_ui.rs`, `desktop/src/theme.rs`)**:
  - Footer layout (`draw_footer`), shared left-filled drag/wheel numeric control helper, Settings tab card list (`draw_settings_tab`), and soft-accent fill helper on `ThemeTokens`.
- **Settings Sanitization (`desktop/src/settings.rs`)**:
  - Snap `zoom` to `0.01` in `Settings::sanitize()`.
- **Tests (`desktop/src/app/tests/batch_settings_tests.rs`, `desktop/src/app/tests/layout_theme_tests.rs`, `desktop/src/settings.rs`)**:
  - Update Settings zoom card/slider/reset assertions to target the footer **Zoom** control and add coverage for wheel stepping (`1 pt` / `10%`) and soft-accent left-fill rendering.
