# Tasks

## 1. Command Buffer Cursor Model (`desktop/src/commands.rs`)

- [x] 1.1 Extend `commands::Input` with `cursor: usize` (character index), `needs_focus: bool`, and `sync_cursor: bool`, plus UTF-8-safe cursor-aware editing helpers (`insert_char`, `backspace`, `delete`, `move_left`, `move_right`, `move_home`, `move_end`, `set_name_end`) and verify with unit tests in `desktop/src/commands.rs` (`cargo test -p eqntide-desktop commands::tests`).
- [x] 1.2 Update `App::canvas_text` and `App::command_key` in `desktop/src/app.rs` to route character insertion, `Backspace`, `Delete`, `ArrowLeft`, `ArrowRight`, `Home`, `End`, and `Tab` through the cursor-aware `commands::Input` helpers, and verify existing + new headless `command_key`/`type_canvas` tests pass (`cargo test -p eqntide-desktop completion_`).

## 2. Interactive Popup Textbox (`desktop/src/app.rs`)

- [x] 2.1 Replace the static `ui.monospace(format!("\\{name}"))` in `App::command_popup` with a themed single-line input box containing the fixed `\` prefix and an `egui::TextEdit::singleline(&mut input.name)` (`Id::unique("canvas-command-input")`), synchronizing `CCursor` via `TextEdit::State` and requesting focus when the popup opens or when the input row is clicked; verify the popup renders and accepts text/cursor updates cleanly in UI frame tests.
- [x] 2.2 Intercept and consume popup-specific keys (`Enter`, `Escape`, `Tab`, `ArrowUp`/`ArrowDown` on prefix suggestions, empty-buffer `Backspace`, and bare-prefix `Space`) before `TextEdit` processes input, set `focus_lock_filter` on `canvas-command-input`, restore focus to `Id::unique("equation-canvas")` on commit/cancel/suggestion click, and clear `command_input` when `latex-source` receives focus in `sync_editor_focus`; verify with UI tests in `desktop/src/app.rs`.

## 3. Verification & Regression Suite

- [x] 3.1 Add and update UI integration tests in `desktop/src/app.rs` covering `ArrowLeft`/`ArrowRight`/`Home`/`End`/`Delete`/`Backspace` mid-buffer editing, `Tab` completion with trailing `{R}` insertion, pointer click inside the popup textbox repositioning the cursor without dismissing the popup, clicking outside onto `equation-canvas` or `latex-source` closing the popup, and Equation Canvas focus restoration; run `cargo test -p eqntide-desktop` to verify all desktop tests pass.
- [x] 3.2 Run `cargo clippy -p eqntide-desktop --all-targets` and `cargo test --workspace` to confirm no regressions or lint warnings across the workspace.
