# Proposal

## Why

`egui` (`0.37.0`) enables several hidden global and widget-level keyboard shortcuts by default—most notably global UI zoom (`Ctrl++`, `Ctrl+=`, `Ctrl+-`, `Ctrl+0`, which shrinks or enlarges the entire `egui` `zoom_factor()` and breaks EqnTide's fixed desktop layout), global quit (`Ctrl+Q` and `Shift`/`Alt` variants via `Modifiers::matches_logically`), and Emacs/Readline mutating keys inside `egui::TextEdit` (`Ctrl+H`, `Ctrl+K`, `Ctrl+U`, `Ctrl+W`). None of these are listed in Settings or reserved in `RESERVED_SYSTEM_CHORDS`, so pressing them accidentally distorts the UI, closes the window unexpectedly, or deletes text in `LaTeX source` / text fields when users try to use `Ctrl+H`, `Ctrl+K`, `Ctrl+U`, `Ctrl+W`, `Ctrl+Q`, or `Ctrl+0`/`-`/`=`/`+` as fast palette shortcuts.

At the same time, standard text-editing, clipboard, and navigation shortcuts that remain enabled (`Ctrl+Arrow*`, `Ctrl+Backspace/Delete`, `Ctrl+Home/End`, `Ctrl/Shift+Insert`, `Shift+Delete`, `Tab`/`Shift+Tab`/`Enter`/`Space`/`Esc`) must remain unassignable in Settings shortcut customization, with clear feedback if a user tries to bind them.

## What Changes

- Disable `egui`'s built-in global keyboard UI zoom (`zoom_with_keyboard = false`) across all desktop backends (`wgpu`, `software`, and `glow`) so `Ctrl++`, `Ctrl+=`, `Ctrl+-`, and `Ctrl+0` (including `Shift`/`Alt` variants) never alter `egui::Context::zoom_factor()`.
- Clear `egui`'s built-in `quit_shortcuts` (`quit_shortcuts.clear()`) so `Ctrl+Q` (and `Ctrl+Shift+Q` / `Ctrl+Alt+Q`) does not silently trigger `ViewportCommand::Close` behind Settings and custom palette bindings.
- Suppress `egui::TextEdit`'s built-in Emacs/Readline mutating shortcuts (`Ctrl+H`, `Ctrl+K`, `Ctrl+U`, `Ctrl+W`) when `TextEdit`-backed inputs (`LaTeX source`, shortcut search, inline `\command` popup input, or numeric `DragValue` editing) are active, while keeping `Ctrl+H`, `Ctrl+K`, `Ctrl+U`, and `Ctrl+W` fully functional for Equation Canvas palette shortcuts and Settings shortcut recording.
- Preserve standard `egui::TextEdit` navigation/word-editing shortcuts (`Ctrl+Arrow*`, `Ctrl+Backspace/Delete`, `Ctrl+Home/End`, `Tab` in multiline `LaTeX source`), OS clipboard shortcuts (`Ctrl/Shift+Insert`, `Shift+Delete`, Linux middle-click paste), and non-canvas focus traversal (`Tab`, `Shift+Tab`, `Arrow*`, `Enter`, `Space`, `Esc`).
- Reject all preserved text-editing, OS clipboard, and navigation keys in Settings shortcut recording and config parsing (with explicit reserved-key feedback when pressed during recording), ensuring they can never be bound to palette entries.

## Capabilities

### New Capabilities

### Modified Capabilities
- `desktop-custom-shortcuts`: Suppress `egui` global keyboard UI zoom (`zoom_with_keyboard`), `egui` default quit shortcut (`Ctrl+Q`), and `egui::TextEdit` Emacs/Readline mutating chords (`Ctrl+H`, `Ctrl+K`, `Ctrl+U`, `Ctrl+W`) so customizable palette chords execute cleanly without corrupting UI scale, closing the app, or mutating text inputs; and explicitly forbid assigning preserved text-editing, clipboard, and navigation shortcuts in Settings.

## Impact

- `desktop/src/app/mod.rs`: Configure `egui_ctx.options_mut(...)` during app initialization (`App::new_with_state`) and `App::draw_ui()`, and invoke `suppress_text_edit_readline_chords` before widgets render.
- `desktop/src/app/keyboard.rs`: Filter `Ctrl+H/K/U/W` key events before non-canvas `TextEdit` widgets consume them (preserving shortcut capture in Settings and canvas dispatch).
- `desktop/src/app/shortcuts_ui.rs` & `desktop/src/shortcuts.rs`: Distinguish reserved editing/clipboard/navigation keys in shortcut capture with clear feedback and cover config-parse / UI rejection in tests.
- `desktop/src/app/tests/shortcut_tests.rs`: Regression tests verifying global zoom keys, `Ctrl+Q`, `Ctrl+H/K/U/W`, and rejection of reserved editing/clipboard/navigation shortcuts in Settings.
- No changes to `eqntide` core library, `eqntide-editor`, `settings.json` schema, or existing `SYSTEM_SHORTCUTS` entries.
