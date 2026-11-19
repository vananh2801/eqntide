# Tasks

## 1. Settings Canvas Zoom Row Layout and Handle Styling

- [x] 1.1 Update `App::settings_ui` in `desktop/src/app.rs` to lay out the **Canvas zoom** row (`Slider`, percentage `DragValue`, and `Reset (200%)` button) inside an explicit `theme::BUTTON_HEIGHT` (`30px`) left-to-right centered row (`allocate_ui_with_layout` with `Align::Center`) so all three controls share the same vertical center without inflating slider handle size, and verify `cargo test -p eqntide-desktop` compiles and passes existing Settings layout tests.
- [x] 1.2 Update the zoom slider's scoped `ui.visuals_mut().widgets.{inactive,hovered,active}.fg_stroke` in `desktop/src/app.rs` to `Stroke::new(1.0, tokens.accent)` so the rounded-square handle border matches the left-side accent fill across idle, hover, and drag states in both Light and Dark modes, and add/update UI tests in `desktop/src/app.rs` verifying both vertical alignment (slider rail/handle center vs. `DragValue` and `Reset (200%)` center) and handle border color `tokens.accent` in Light and Dark modes via `cargo test -p eqntide-desktop`.
