# Tasks

## 1. Direct Opacity Ranges & Alpha Calculations (`38%..=100%`)

- [x] 1.1 Update `BACKGROUND_OPACITY_RANGE`, `CANVAS_OPACITY_RANGE` (`0.38..=1.00`), `DEFAULT_BACKGROUND_OPACITY` (`0.81`), and `Settings::sanitize()` clamping/1%-snapping in `desktop/src/settings.rs`, and update `settings::tests` to verify `0.38..=1.00` clamping, `0.01` snapping, and legacy `< 0.38` migration.
- [x] 1.2 Update `ThemeTokens::with_background_opacity`, `ThemeTokens::background_scrim`, and `CanvasPalette::with_canvas_opacity` in `desktop/src/theme.rs` to apply `0.38..=1.00` alpha directly without surface-alpha remapping while scaling `background_scrim` across `0%..=65%` over `0.38..=1.00`, and update `theme::tests` to verify direct surface alpha and `0%..=65%` scrim scaling.

## 2. Footer `MathType` / `OMML` Selectable Buttons

- [x] 2.1 Replace `Self::accent_radio_value` and the extra `SPACE_MD` spacer in `draw_footer` (`desktop/src/app/mod.rs`) with `Self::with_selectable_style` + `Self::standard_button` (`30px` height, `8px` gap, centered with `draw_footer` row) and remove unused `accent_radio_value` from `desktop/src/app/palette_ui.rs`; verify `cargo check -p eqntide-desktop` passes without dead-code warnings.
- [x] 2.2 Update footer target tests in `desktop/src/app/tests/layout_theme_tests.rs` (including `footer_radios_hover_borders_and_drag_value_focus_reflect_active_accent` and `button_height_tiers_align_footer_tabs_and_compact_controls`) to verify `MathType`/`OMML` selectable button fills, `30px` height, `8px` spacing, vertical center alignment, enabled/disabled states, and focus restoration.

## 3. Settings Opacity Level-Fill `DragValue` Controls

- [x] 3.1 Expose `App::draw_level_drag_value` to `desktop/src/app/settings_ui.rs` (`pub(super)`) with `wheel_steps_per_unit: Option<f64>` and update `draw_opacity_slider_row` to render a `220px × 30px` `6px`-rounded level-fill `DragValue` box (`38%..=100%`, `1%` steps, centered `%` readout, no slider handle or external `%` label, horizontal drag, numeric entry, mouse-wheel scrolling ignored so Settings scroll is unaffected, and crisp disabled state in **Solid** mode), ensuring `cargo check -p eqntide-desktop` compiles cleanly.
- [x] 3.2 Update opacity UI and persistence tests in `desktop/src/app/tests/batch_settings_tests.rs`, `desktop/src/app/tests/layout_theme_tests.rs`, and `desktop/src/app/tests/canvas_input_tests.rs` to verify `220×30px` level-fill rendering, `38%..=100%` proportional fills, direct alpha values on canvas/panels, drag / numeric edits, mouse-wheel non-mutation, disabled **Solid** gating, and `settings.json` persistence.

## 4. Workspace Verification

- [x] 4.1 Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --all-targets --locked`, and `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`, confirming zero warnings and all tests passing.
