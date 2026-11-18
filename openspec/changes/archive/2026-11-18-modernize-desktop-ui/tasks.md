# Tasks

## 1. Design Tokens & Canvas Presets

- [x] 1.1 Create `desktop/src/theme.rs` (`mod theme;` in `desktop/src/main.rs`) defining `ThemeTokens` (Light/Dark Modern Slate palette, `RADIUS_CONTROL = 6.0`, `RADIUS_SURFACE = 8.0`, `4px` spacing constants, zero hover expansion, tinted-accent selected colors, and `apply_style(ctx)`), plus `CanvasPreset` and `CanvasPalette` for Light (`SoftPaper`, `PureWhite`, `CoolSlate`, `WarmCream`) and Dark (`SoftPaper`, `DimSlate`, `SoftCharcoal`, `DeepInk`); verify with unit tests covering Light/Dark token contrast, zero hover expansion, and all preset palettes.
- [x] 1.2 Extend `Settings` in `desktop/src/settings.rs` with `canvas_light` and `canvas_dark` (both defaulting to `SoftPaper`), safe deserialization/sanitization of unknown or mismatched-mode preset values, and unit tests verifying round-trip persistence and fallback behavior.

## 2. Unified UI Styling, Hover States & Canvas Rendering

- [x] 2.1 Apply `ThemeTokens::apply_style` in `App::new_with_state` and test initialization, standardize panel framing (`status_bar`, `app-footer`, `app-sidebar`, `CentralPanel`), `RADIUS_CONTROL` / `RADIUS_SURFACE` (removing the `4px` zoom slider override), and `4px`-scale spacing across `editor_ui`, `palette_ui`, `footer`, `batch_ui`, `settings_ui`, `about_ui`, and dialogs while verifying `1040x560` non-scrolling sidebar and 3-row palette tests pass.
- [x] 2.2 Standardize hover, active, disabled, and tinted-accent selected states across all buttons (including `Save`, `Convert`, and matrix controls), sidebar categories/tabs, palette tiles, radio buttons, checkboxes, combo boxes, and `PointingHand` hover cursors, updating `icon_tint` so selected sidebar icons use contrasting accent colors in Light and Dark modes; verify with headless UI tests checking zero hover expansion, tinted-accent icon contrast, and `Save` hover/disabled states.
- [x] 2.3 Wire `canvas_light` / `canvas_dark` presets into `App` (`desktop/src/app.rs`), update `canvas` and `paint_placeholder` to render with the active `CanvasPalette` (background, border, ink tint, placeholder mask/box, caret, selection, IME text, and scrollbars), and add curated preset swatch selectors in `settings_ui` that persist changes; verify with unit tests covering preset switching, dark-canvas ink adaptation, persistence round-trip, and updated canvas color assertions.

## 3. Workspace Verification

- [x] 3.1 Run `cargo fmt --check`, `cargo clippy -p eqntide-desktop --all-targets -- -D warnings`, `cargo test -p eqntide-desktop`, and `cargo check --workspace` to confirm clean formatting, zero warnings, and a full test pass.
