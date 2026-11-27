# Tasks

## 1. Layered Neutral Tokens & Global Borderless Style

- [x] 1.1 Refine Light and Dark `ThemeTokens` surface/control fills (`app_bg`, `panel_bg`, `card_bg`, `control_bg`, `control_hover_bg`, `control_active_bg`) in `desktop/src/theme.rs` so `card_bg != control_bg` in both modes and Dark `app_bg != CanvasPalette::bg`, keeping WCAG AA/AAA text contrast invariants and translucent `Image`-mode alpha behavior intact; update and run `desktop::theme::tests` to verify.
- [x] 1.2 Remove border strokes from `ThemeTokens::group_frame`, `ThemeTokens::apply_style_with_accent` (`window_stroke`, `widgets.noninteractive/inactive/hovered/open.bg_stroke`), and `ThemeTokens::apply_accent_to_style` in `desktop/src/theme.rs` (keeping `widgets.active.bg_stroke` and `selection.stroke` for keyboard focus and selected text legibility, and cleaning up dead `control_hover_border` / `CanvasPalette::border`); verify via `cargo test -p eqntide-desktop theme::tests`.

## 2. Shell Region Dividers, Borderless Canvas, LaTeX Box, and Command Popup

- [x] 2.1 Render horizontal and vertical region dividers (`status` top `hline`, `sidebar` `vline`, `footer` `hline`) over opaque fills while keeping the status bar free of a four-sided outer border, and restore `draw_section_separator` in `draw_footer` in `desktop/src/app/mod.rs`; verify shell layout in `cargo test -p eqntide-desktop layout_theme_tests`.
- [x] 2.2 Remove outer `pal.border` strokes from `App::draw_canvas` and `App::draw_latex_panel` in `desktop/src/app/canvas.rs` while preserving `paint_placeholder` dashed `placeholder_stroke`, caret, and selection highlight; update canvas frame assertions in `desktop/src/app/tests/layout_theme_tests.rs` and verify with `cargo test -p eqntide-desktop`.
- [x] 2.3 Remove the border stroke from the inline `\command` popup input frame in `desktop/src/app/keyboard.rs` and give the input box a borderless `control_bg` / `extreme_bg` surface fill; update popup assertions in `desktop/src/app/tests/canvas_input_tests.rs` and verify with `cargo test -p eqntide-desktop canvas_input_tests`.

## 3. Borderless Controls & Secondary Tabs with Preserved Section & Tree Lines

- [x] 3.1 Update `App::with_selectable_style` and `App::primary_button_with` in `desktop/src/app/palette_ui.rs` to render borderless inactive, hovered, and selected states (`Stroke::NONE`), while preserving `bg_stroke` indicator outlines inside `App::accent_checkbox` and `App::accent_radio_value` and modal keyboard focus rings; update `layout_theme_tests.rs` and verify with `cargo test -p eqntide-desktop`.
- [x] 3.2 Restore `draw_section_separator` in `desktop/src/app/batch_ui.rs` and remove the progress-bar outer border stroke while keeping `App::paint_batch_row_chrome` (`tree_stroke` hierarchy lines and row `divider_stroke`) untouched across **Folder tree** and **File tree**; verify tree-line and progress-bar assertions in `desktop/src/app/tests/layout_theme_tests.rs` and `batch_settings_tests.rs`.
- [x] 3.3 Remove `draw_background_thumbnail` border strokes in `desktop/src/app/settings_ui.rs`, restore `draw_section_separator` dividers in `desktop/src/app/about_ui.rs`, and ensure popups/dropdowns remain visually separated on shadowless `Software`/`VirtualGpu` backends; verify with `cargo test -p eqntide-desktop`.

## 4. Workspace Verification

- [x] 4.1 Run full pre-commit verification (`cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --all-targets --locked`, and `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`) and confirm zero warnings or failures.
