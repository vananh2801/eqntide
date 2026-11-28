# Proposal

## Why

Currently, the top 3-row palette ribbon in the desktop editor renders inside the central panel with `Frame::NONE` and no surface fill, leaving its buttons floating directly over the transparent backdrop when a custom **Image** background is enabled in Settings. Giving the palette bar a dedicated chrome panel background matching the sidebar and footer restores clear visual separation and button legibility across both Light/Dark themes and custom wallpapers.

## What Changes

- Promote the editor's top palette ribbon (`templates-panel`) to a full-width chrome panel (`Panel::top` to the right of the left sidebar) filled with `ThemeTokens::panel_bg`, matching the sidebar, footer, and status bar.
- Add `8px` top and bottom inner padding inside the palette panel (`120px` total height for 3 tile rows plus symmetric padding) and draw a `1px` `ThemeTokens::border` horizontal divider along its bottom edge.
- Reduce the Equation Canvas vertical allocation at the default/minimum `1056x560` window size (absorbing the `8px` vertical padding added for the palette panel + divider gap) and match its inner margin deduction (`2 * SPACE_LG = 24px` instead of `26px`) so the palette-to-canvas, canvas-to-LaTeX, and LaTeX-to-footer gaps are all uniformly `8px` without clipping or enlarging the window.
- Automatically scale the palette panel's opacity (`0.38..=1.00`, `255` alpha at `100%`) using the existing **Overlay opacity** slider in Settings via `ThemeTokens::panel_bg`.

## Capabilities

### New Capabilities

### Modified Capabilities
- `desktop-ui-theme`: Extend chrome panel background (`panel_bg`), **Overlay opacity** translucency scaling, `1px` region divider rules, and compact `1056x560` Equation Canvas vertical sizing to include the top palette ribbon panel.

## Impact

- Desktop UI layout and chrome rendering in `desktop/src/app/mod.rs`, `desktop/src/app/palette_ribbon.rs`, and `desktop/src/app/canvas.rs`.
- Desktop layout and theme unit tests in `desktop/src/app/tests/layout_theme_tests.rs`.
- No changes to core math parsing/conversion crates (`eqntide`, `eqntide-editor`), settings schema (`settings.json`), or Office bridge protocols.
