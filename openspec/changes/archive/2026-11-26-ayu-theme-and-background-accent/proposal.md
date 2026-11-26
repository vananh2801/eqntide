# Proposal

## Why

The desktop application's current slate palette and blue accent feel disconnected from the warm, crisp neutral contrast of Zed Editor's Ayu Light and Ayu Dark themes, and toggling between a custom background image and the solid theme currently forces the user to delete and re-import the image every time. In addition, custom background images currently keep a fixed blue accent instead of adapting UI highlights, interactive controls, and the status-bar readiness indicator to harmonious light and dark tones derived from the wallpaper. Meanwhile, multi-preset Equation Canvas pickers (`Soft paper`, `Pure white`, `Cool slate`, `Warm cream`, `Dim slate`, `Soft charcoal`, `Deep ink`) add visual clutter in Settings when **Ayu Light** and **Ayu Dark** can directly serve each theme mode by default.

## What Changes

- Replace the desktop application's default slate color palette with Zed Editor–inspired **Ayu Light** and **Ayu Dark** neutral surfaces paired with Ayu's signature **amber/orange** default accent color, while preserving accessible WCAG contrast invariants.
- Standardize the Equation Canvas and synchronized LaTeX source box directly on **Ayu Light** (`#fcfcfc`) in Light mode and **Ayu Dark** (`#0d1016`) in Dark mode, removing the other canvas preset options and their picker controls from Settings while safely ignoring legacy preset keys in existing `settings.json` files.
- Add a **Solid / Image** selectable mode toggle in Settings **App background** (matching the **Light / Dark** button group) so users can switch between solid and image backgrounds instantaneously without deleting or re-importing their saved managed background image; keep **Clear** as an explicit removal action.
- Extract a dominant hue/saturation accent seed during background image import, persist it alongside a **Match accent color to background** checkbox (active/enabled when **Image** mode has a valid background, disabled/dimmed in **Solid** mode), and derive accessible **Light** (deeper/richer) and **Dark** (brighter/lighter) accent variants.
- Apply the active accent color consistently across UI chrome (`ThemeTokens`, dynamic `egui::Style`, primary actions, selections, sliders, text cursor, links, focus borders, and the inline `\command` popup focus border and vertically aligned `\` prefix), `CanvasPalette` highlights/scrollbars, and the bottom status bar's bridge readiness circle (leaving status text neutral/error-colored).

## Capabilities

### New Capabilities

- None

### Modified Capabilities

- `desktop-ui-theme`: Replace the slate Light/Dark chrome palette with Ayu Light/Ayu Dark neutral surfaces and amber/orange default accent, standardize the Equation Canvas on Ayu Light (Light mode) and Ayu Dark (Dark mode) while removing all other canvas preset options and pickers from Settings, add the Solid/Image background toggle and adaptive accent checkbox in Settings, and apply derived Light/Dark background accent tokens across UI controls, `egui::Style`, the inline `\command` popup (`\` prefix + focus border), and `CanvasPalette`.
- `desktop-settings-persistence`: Persist and sanitize `background_mode` (`solid` / `image`), `background_accent_enabled` (`bool`), and the extracted background accent seed (`background_accent`), keep managed background files intact when switching to `solid` mode, and remove `canvas_light` / `canvas_dark` persistence while safely ignoring legacy keys during JSON deserialization.
- `desktop-status-bar`: Color the static vector bridge readiness circle in the status bar using the active theme accent color while preserving quiet idle status text and error/warning colors.

## Impact

- Desktop theme & style engine: `desktop/src/theme.rs`, `desktop/src/app/mod.rs`, `desktop/src/app/palette_ui.rs`, `desktop/src/app/canvas.rs`, `desktop/src/app/keyboard.rs`.
- Desktop settings persistence & image import: `desktop/src/settings.rs`.
- Desktop Settings UI & layout: `desktop/src/app/settings_ui.rs`.
- Desktop test suites: `desktop/src/theme.rs` unit tests, `desktop/src/settings.rs` unit tests, `desktop/src/app/tests/layout_theme_tests.rs`, `desktop/src/app/tests/batch_settings_tests.rs`, `desktop/src/app/tests/canvas_input_tests.rs`.
