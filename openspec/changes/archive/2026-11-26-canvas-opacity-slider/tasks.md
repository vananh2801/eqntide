# Tasks

## 1. Settings Persistence and Theme Tokens

- [x] 1.1 Add `DEFAULT_CANVAS_OPACITY` (`1.0`), `CANVAS_OPACITY_RANGE` (`0.0..=1.0`), and `Settings::canvas_opacity` in `desktop/src/settings.rs` with backward-compatible deserialization default (`1.0`), clamping/sanitization, and unit tests in `desktop/src/settings.rs` verifying default, clamping, non-finite fallback, and round-trip JSON persistence.
- [x] 1.2 Add `CanvasPalette::with_canvas_opacity` and `CanvasPalette::for_mode_with_background_and_accent` in `desktop/src/theme.rs` scaling `pal.bg` alpha across `0.38..=1.00` when `has_background` is true (keeping `pal.bg` opaque at `1.0` and in Solid mode), and add unit tests in `desktop/src/theme.rs` verifying `0.0`, `0.5`, `1.0`, non-finite fallback, and Solid-mode opacity.

## 2. Settings UI Control and App Wiring

- [x] 2.1 Add `App::canvas_opacity` in `desktop/src/app/mod.rs` and `desktop/src/app/settings_ui.rs`, wire `apply_settings` and `active_canvas_palette` to pass `is_image_background_active()` and `self.canvas_opacity`, extract a shared opacity slider row helper in `desktop/src/app/settings_ui.rs`, and render `Canvas opacity` directly below `Overlay opacity` (disabled in `Solid` mode with opaque muted handle, enabled in `Image` mode with immediate persistence); verify with UI tests in `desktop/src/app/tests/batch_settings_tests.rs` checking row ordering, Solid-mode disabled handle opacity, and live disk persistence.

## 3. Translucent Canvas & Placeholder Rendering

- [x] 3.1 Clear placeholder region pixels from the rendered formula alpha mask in `desktop/src/app/events.rs` and remove the opaque `pal.bg` cover rectangle from `paint_placeholder` in `desktop/src/app/canvas.rs` so translucent Equation Canvas and LaTeX source fills render cleanly with unfilled dashed placeholders; verify with tests in `desktop/src/app/tests/layout_theme_tests.rs` and `desktop/src/app/tests/canvas_input_tests.rs` covering translucent canvas/LaTeX fill alphas, Solid-mode opaque fallback, and zero solid placeholder cover fill.

## 4. Workspace Verification

- [x] 4.1 Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --all-targets --locked`, and `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` and verify zero warnings or test failures.
