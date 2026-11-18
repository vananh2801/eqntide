# Tasks

## 1. Settings Persistence and Startup Focus Option

- [x] 1.1 Add `StartupFocus` (`Canvas` default, `Latex`) and `startup_focus` field to `Settings` in `desktop/src/settings.rs` with JSON round-trip and fallback sanitization tests passing via `cargo test -p eqntide-desktop settings`
- [x] 1.2 Expose the `Startup focus` selector (`Equation canvas` / `LaTeX source`) in `App::settings_ui` in `desktop/src/app.rs`, persisting changes immediately and verifying via `cargo test -p eqntide-desktop`

## 2. Buffered Raw LaTeX Command Input on Equation Canvas

- [x] 2.1 Update `App::canvas_text`, `App::command_key`, `App::commit_command`, and `App::command_popup` in `desktop/src/app.rs` so backslash (`\`) expressions buffer `{`, `}`, `_`, `^`, operators, and spaces until `Enter` commits, `Tab` only completes the suggested command name into the buffer, `Space` remains literal buffer text, and standalone template command names fall back cleanly on `Enter`
- [x] 2.2 Update existing completion tests and add unit tests in `desktop/src/app.rs` covering `\mathbb{R}`, `\math` + `{` + `bb{R}`, `\int_0^1 x dx`, `\lim_{x \to 0}`, `Tab` name autocomplete without commit, `Space` buffering, and `Enter`-only commit via `cargo test -p eqntide-desktop completion`

## 3. Initial Focus and Active Surface Focus Retention

- [x] 3.1 Track active editor focus (`EditorFocus::Canvas` vs `EditorFocus::Latex`), request initial focus based on `startup_focus` on launch/session activation/return to Editor tab, and restore focus to `active_editor_focus` after clicking sidebar categories, palette tiles, matrix controls, footer actions (`Undo`/`Redo`/target), status buttons, and closing dialogs in `desktop/src/app.rs`
- [x] 3.2 Add desktop UI tests in `desktop/src/app.rs` verifying startup focus under both `StartupFocus` settings and focus restoration to both `equation-canvas` and `latex-source` after sidebar, palette, matrix, and footer interactions via `cargo test -p eqntide-desktop`
- [x] 3.3 Blink the focused Equation Canvas caret using `visuals().text_cursor` timing with activity reset and unit test verification (`canvas_caret_blinks_when_idle_and_resets_on_activity`)

## 4. Final Verification

- [x] 4.1 Run `cargo fmt --check`, `cargo clippy -p eqntide-desktop --all-targets -- -D warnings`, and `cargo test -p eqntide-desktop` with all checks passing
