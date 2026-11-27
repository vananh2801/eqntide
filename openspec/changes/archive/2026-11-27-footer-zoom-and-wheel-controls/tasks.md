# Tasks

## 1. Settings Sanitization & Soft-Accent Theme Token

- [x] 1.1 Update [`Settings::sanitize`](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/src/settings.rs#L292-L297) to snap `zoom` to `0.01` (`1%`) increments within `ZOOM_RANGE`, and update unit tests in [`desktop/src/settings.rs`](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/src/settings.rs) verifying off-step `zoom` values snap cleanly (`cargo test -p eqntide-desktop settings::tests`).
- [x] 1.2 Add a soft-accent fill helper on [`ThemeTokens`](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/src/theme.rs#L36) (blending `accent` over `control_bg`/`control_hover_bg` when enabled and `disabled_accent()` over `control_disabled_bg` when disabled) and add unit coverage in [`desktop/src/theme.rs`](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/src/theme.rs) checking Light, Dark, and adaptive accent contrast (`cargo test -p eqntide-desktop theme::tests`).

## 2. Footer Level-Fill Controls, Zoom Relocation & Wheel Behavior

- [x] 2.1 Remove the `"Canvas zoom"` card (`Slider`, percentage `DragValue`, and `Reset (200%)`) from [`App::draw_settings_tab`](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/src/app/settings_ui.rs#L315-L373) and update `settings_cards_span_available_width` in [`batch_settings_tests.rs`](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/src/app/tests/batch_settings_tests.rs#L393-L402) to reflect the remaining Settings cards (`cargo test -p eqntide-desktop settings_cards_span_available_width`).
- [x] 2.2 Implement a shared Krita-style left-filled `DragValue` helper in [`desktop/src/app/mod.rs`](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/src/app/mod.rs#L888-L920) (transparent idle/hover widget background over a `6px`-rounded track, proportional soft-accent left fill, centered text without spinner arrows, `1px tokens.accent` border on text-edit focus, hover wheel stepping by `1 pt` / `10%` without altering editor focus, and immediate disk persistence on wheel/text commit) and place **Zoom** beside **Font size** inside the `!self.draft_dirty` footer group.
- [x] 2.3 Update and expand UI tests in [`batch_settings_tests.rs`](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/src/app/tests/batch_settings_tests.rs) and [`layout_theme_tests.rs`](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/src/app/tests/layout_theme_tests.rs) to verify footer **Zoom** drag/text/persistence at `1056x560`+, hover mouse-wheel stepping (`1 pt` for Font size, `10%` for Zoom), `draft_dirty` disabling of both controls, Krita-style left-fill geometry/colors in Light and Dark modes, and `DragValue` text-edit focus stroke (`cargo test -p eqntide-desktop`).

## 3. Workspace Verification

- [x] 3.1 Run the full pre-commit verification suite (`cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --all-targets --locked`, and `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`) and confirm zero warnings or failures.
