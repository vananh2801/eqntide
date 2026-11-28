# Tasks

## 1. Settings Persistence for LaTeX Source Font Size

- [x] 1.1 Add `LATEX_FONT_PT_RANGE` (`8.0..=32.0`), `DEFAULT_LATEX_FONT_PT` (`theme::UI_FONT_SIZE` = `13.0`), and `latex_font_pt: f32` to `Settings` / `RawSettings` and `Settings::sanitize()` (`desktop/src/settings.rs`), clamping to `8.0..=32.0` and snapping to `0.5 pt` increments while defaulting omitted or non-finite values to `13.0`; verify existing and new unit tests in `desktop/src/settings.rs` pass (`cargo test -p eqntide-desktop settings::tests --locked`).
- [x] 1.2 Wire `App::latex_font_pt` in `desktop/src/app/mod.rs` and `App::apply_settings()` (`desktop/src/app/settings_ui.rs`) so `latex_font_pt` initializes from `Settings` and stays isolated from `load_math()` Office sessions; verify with `batch_settings_tests` (`cargo test -p eqntide-desktop batch_settings_tests --locked`).

## 2. Equation Canvas `Ctrl+Wheel` Cursor-Anchored Zoom

- [x] 2.1 Add a helper in `desktop/src/app/canvas.rs` to extract discrete `Ctrl+Wheel` (`modifiers.ctrl || modifiers.command`) steps from `input.raw.events` (supporting `Line`/`Page` notches and accumulated `Point` deltas) while clearing residual `smooth_scroll_delta`.
- [x] 2.2 Implement Equation Canvas `Ctrl+Wheel` zoom in `App::draw_canvas` (`desktop/src/app/canvas.rs`) gated on `ui.is_enabled()` and `ui.rect_contains_pointer(...)`: clamp/snap `self.zoom` across `150%..=400%` in `10%` steps, update `self.persisted.zoom` and `self.settings_dirty = true`, and compute the same-frame cursor-anchored `ScrollArea` offset passed via `.scroll_offset(new_offset)`.
- [x] 2.3 Add UI integration tests in `desktop/src/app/tests/layout_theme_tests.rs` verifying Equation Canvas `Ctrl+Wheel` syncs with footer Zoom and `settings.json`, preserves the equation point under the cursor when scrolled, does not affect `LaTeX source` font size, and is suppressed when `draft_dirty`, a modal, or the `\command` popup under the pointer is active (`cargo test -p eqntide-desktop layout_theme_tests --locked`).

## 3. `LaTeX source` Box `Ctrl+Wheel` Font-Size Zoom

- [x] 3.1 Update `App::draw_latex_panel` (`desktop/src/app/canvas.rs`) to render `TextEdit::multiline` using `egui::FontId::monospace(self.latex_font_pt)` and handle `Ctrl+Wheel` over the enabled `LaTeX source` box (`8.0..=32.0 pt`, `0.5 pt` steps), updating `self.latex_font_pt`, `self.persisted.latex_font_pt`, and `self.settings_dirty = true` without altering footer **Zoom** / **Font size** or the fixed `88px` / `80px` panel height.
- [x] 3.2 Add UI integration tests in `desktop/src/app/tests/layout_theme_tests.rs` and `desktop/src/app/tests/batch_settings_tests.rs` verifying `Ctrl+Wheel` over `LaTeX source` scales monospace font size across `8.0..=32.0 pt` in `0.5 pt` steps (including while `draft_dirty` is true), ignores footer **Zoom** / **Font size** changes, keeps Equation Canvas zoom unchanged, and restores the saved `latex_font_pt` in a fresh `App` instance (`cargo test -p eqntide-desktop --locked`).

## 4. Workspace Verification

- [x] 4.1 Run all pre-commit checks (`cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --all-targets --locked`, and `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`) and confirm zero warnings or failures.
