# Tasks

## 1. Ayu Theme Palette, Adaptive Accent Derivation, and Fixed Ayu Canvas Palette

- [x] 1.1 Update `ThemeTokens::for_mode` in `desktop/src/theme.rs` to use Zed-inspired Ayu Light and Ayu Dark neutral surfaces and Ayu amber/orange default accent with accessible `primary_fg` contrast, and update unit tests in `theme.rs` to verify WCAG contrast (`>= 7.0` primary text in opaque/translucent modes, `>= 4.5` selected/primary button text).
- [x] 1.2 Add `ThemeTokens::for_mode_with_accent`, `ThemeTokens::apply_style_with_accent`, `ThemeTokens::disabled_accent`, and `ThemeTokens::control_hover_border` in `desktop/src/theme.rs` to derive accessible Light (deeper) and Dark (brighter) accent palettes, opaque disabled accent colors, and subtle accent-tinted hover borders from an optional `[u8; 3]` seed; update `with_background_opacity` (`0.38..=1.00` for `panel_bg`/`card_bg`, keeping `app_bg` transparent) and `background_scrim` (`0.00..=0.65`) so `100%` overlay opacity makes panels/cards fully opaque without hiding the background image or turning the thumbnail solid black; and add unit tests verifying diverse accent hues and `0.0` / `0.70` / `1.0` opacity behavior.
- [x] 1.3 Standardize `CanvasPalette::for_mode_with_accent` in `desktop/src/theme.rs` on `Ayu light` (`#fcfcfc`) for Light mode and `Ayu dark` (`#0d1016`) for Dark mode (removing all non-Ayu presets), and update unit tests verifying high ink contrast (`>= 10.0`), neutral `pal.border`, and accent-adapted highlights (`placeholder_stroke`, `caret`, `selection_bg`, `ime_fg`, `scroll_*`).

## 2. Settings Persistence and Background Accent Extraction

- [x] 2.1 Add `BackgroundMode` (`Solid` / `Image`), `background_accent_enabled: bool`, and `background_accent: Option<[u8; 3]>` to `Settings` in `desktop/src/settings.rs` with legacy JSON migration (`background_mode` inferred from `background_image` when omitted), remove `canvas_light` / `canvas_dark` persistence while ignoring legacy preset keys on load, and update sanitization tests in `settings.rs`.
- [x] 2.2 Implement dominant chromatic accent extraction in `desktop/src/settings.rs`, attach the extracted seed to `PreparedBackground` during `prepare_background_import`, backfill missing seeds when loading pre-existing managed backgrounds, and add unit tests covering chromatic images and grayscale fallback (`None`).

## 3. Dynamic Accent Propagation Across Desktop Chrome and Status Bar

- [x] 3.1 Track `background_mode`, `background_accent_enabled`, `background_accent`, and `applied_accent_seed` on `App` in `desktop/src/app/mod.rs` and `settings_ui.rs`, re-syncing `ThemeTokens::apply_style_with_accent` whenever the active accent seed changes and keeping `App::clear_color` / `software_background` / `theme_tokens` aligned with `background_mode`.
- [x] 3.2 Update `palette_ui.rs`, `settings_ui.rs`, `canvas.rs`, `keyboard.rs` (`draw_command_popup` focus border and vertically centered leading `\` prefix aligned with `.align(egui::Align2::LEFT_CENTER)` on `TextEdit::singleline`), and `draw_status_bar` in `desktop/src/app/mod.rs` so primary buttons, selectable controls, sliders, popup chrome/prefix, canvas overlays/scrollbars, and the status-bar bridge readiness circle use the active `ThemeTokens` / `CanvasPalette` accent (while status text remains primary/error colored and `background_importing` displays `"Importing background image…"` in `draw_status_bar`), and add/update layout and canvas input tests verifying accent propagation, prefix alignment, and status-dot coloring.

## 4. Settings Appearance & App Background Cards

- [x] 4.1 Split **Appearance** (**Light** / **Dark**) and **App background** (**Solid** / **Image**, vertically centered `72×48` thumbnail + `Choose image…` / `Clear` with no filename/mode text beside the thumbnail, **Match accent color to background**, and `1%`-stepped `Overlay opacity` with fully opaque slider handles in both enabled and disabled states) into two separate full-width cards inside `draw_settings_tab` (`desktop/src/app/settings_ui.rs`), keeping importing progress in the bottom status bar and omitting canvas preset pickers.
- [x] 4.2 Add UI integration tests in `desktop/src/app/tests/batch_settings_tests.rs` and `layout_theme_tests.rs` verifying separate `Appearance` and `App background` cards, vertically centered thumbnail/button alignment without filename text, opaque disabled `Overlay opacity` handle fill, status-bar importing indicator, `Solid` <-> `Image` non-destructive switching, checkbox gating, `Clear` behavior, and absence of removed canvas preset pickers.

## 5. Control-Level Accent Coverage (Progress Bar, Selections, Checkboxes/Radios, Hover & Focus Borders)

- [x] 5.1 Update `draw_batch_progress_bar` in `desktop/src/app/batch_ui.rs` to fill progress with `tokens.primary_bg`, avoid any partial fill stub at `0%`, and render split-contrast progress text (`primary_fg` on filled portion, `text_primary` on track) using a clipped `PLACEHOLDER`-colored galley.
- [x] 5.2 Apply `App::with_selectable_style` to inline `\command` popup suggestions (`desktop/src/app/keyboard.rs`) and `ComboBox` selected rows (`batch-target` in `batch_ui.rs`, `matrix-alignment` and `matrix-fence` in `settings_ui.rs`).
- [x] 5.3 Add `App::accent_checkbox` and `App::accent_radio_value` in `desktop/src/app/palette_ui.rs` and wire them across `batch_ui.rs`, `settings_ui.rs`, and `draw_footer` (`mod.rs`), keeping labels/captions neutral while tinting enabled indicators with `selected_*` tokens and disabled indicators with opaque `control_disabled_bg` + `disabled_accent()` / `border`.
- [x] 5.4 Wire `ThemeTokens::control_hover_border()` (`widgets.hovered.bg_stroke`), `accent` (`widgets.active.bg_stroke` / `widgets.open.bg_stroke` for controls and `DragValue` focus), and `selected_border` (`selection.stroke`) in `desktop/src/theme.rs`, keeping Equation Canvas / LaTeX source box borders (`pal.border`) and Batch folder tree connector lines (`border_hover`) neutral.
- [x] 5.5 Add UI integration tests covering the Batch progress bar, `ComboBox` & `\command` popup selected rows, checkboxes & footer radio buttons (`enabled`, `indeterminate`, `disabled`), hovered control borders, neutral Batch tree lines, and `DragValue` focus stroke across Light/Dark & Solid/Image modes in `batch_settings_tests.rs`, `canvas_input_tests.rs`, and `layout_theme_tests.rs`.

## 6. End-to-End Verification

- [x] 6.1 Run `openspec validate "ayu-theme-and-background-accent" --strict`, `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`, and `cargo test --workspace --all-targets --locked` cleanly with zero warnings.
