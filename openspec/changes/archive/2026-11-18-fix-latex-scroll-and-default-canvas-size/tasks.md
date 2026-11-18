# Tasks

## 1. Bounded Scrollable LaTeX Source Box

- [x] 1.1 Update `editor_ui` and `latex_ui` in `desktop/src/app.rs` to increase the `latex-panel` height to `88.0` (`80.0` input area below `CONTENT_PADDING`), replace `ui.add_sized` with a fixed-height outer `Frame` (`CONTROL_CORNER_RADIUS`, `pal.bg`, focus/inactive stroke) wrapping `egui::ScrollArea::vertical()` and a frameless `TextEdit::multiline` keeping `egui::Id::unique("latex-source")` and `pal.scroll_*` scrollbar styling, and verify the box aligns left/right with the canvas and renders focus borders cleanly.
- [x] 1.2 Add a UI regression test in `desktop/src/app.rs` verifying that multi-line LaTeX input (e.g. 6+ lines) keeps the `latex-source` container bounded below `canvas.bottom()` and above the footer without overlapping or clipping, preserves click-to-focus, and passes `cargo test -p eqntide-desktop`.

## 2. Compact Default Window and Canvas Size

- [x] 2.1 Update `desktop_viewport` in `desktop/src/main.rs` to launch with `.with_inner_size(app::MIN_WINDOW_SIZE)` (`1040x560`) instead of `[1100.0, 650.0]`, and verify via `cargo test -p eqntide-desktop` that all sidebar, three-row palette, and minimum-size layout tests continue to pass.
