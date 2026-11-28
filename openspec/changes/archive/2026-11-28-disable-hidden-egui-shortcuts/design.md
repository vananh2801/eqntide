# Design

## Context

See [proposal.md](file:///home/vananh2801/Documents/mathtype-convert/eqntide/openspec/changes/disable-hidden-egui-shortcuts/proposal.md) for motivation and affected files.

In `desktop/src/`:
- All three runtime paths (`wgpu_runner::RunnerBackend::Wgpu`, `wgpu_runner::RunnerBackend::Software`, and the `eframe::run_native` Glow fallback) initialize `App` through [`App::new_with_state`](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/src/app/mod.rs#L382-L463).
- `egui::Context::end_pass()` runs `gui_zoom::zoom_with_keyboard` when `options.zoom_with_keyboard` is `true` (default) and checks `options.quit_shortcuts` (default `[Cmd/Ctrl+Q]`) using `Modifiers::matches_logically` (which ignores extra `Shift` and `Alt` flags).
- `egui::TextEdit` evaluates `input.filtered_events(&event_filter)` inside widget update and maps `Ctrl+H`, `Ctrl+K`, `Ctrl+U`, and `Ctrl+W` to destructive character/line/word deletions via `check_for_mutating_key_press`, regardless of `egui::Options`.
- `App::draw_ui()` runs every frame before `draw_shell()` renders `LaTeX source`, `shortcut_search`, `DragValue`, or `draw_command_popup()`, and also runs `process_shortcut_capture()` inside `draw_keyboard_shortcuts_card()` when `self.shortcut_capture.is_some()`.
- `ShortcutKey` in `desktop/src/shortcuts.rs` only permits `A..Z`, `0..9`, `F1..F12`, and ASCII punctuation, so navigation/editing/clipboard keys (`Arrow*`, `Backspace`, `Delete`, `Home`, `End`, `Insert`, `Tab`, `Enter`, `Space`, `Escape`) are already excluded from `ShortcutChord` and `settings.json` parsing. However, `process_shortcut_capture()` previously fell back to the generic `"Press Ctrl, Alt, or F1–F12..."` hint even when a user explicitly held `Ctrl` with an editing/navigation key (e.g. `Ctrl+Backspace`, `Ctrl+Home`, `Ctrl+ArrowLeft`).

## Goals / Non-Goals

**Goals:**
- Disable `egui` global `zoom_with_keyboard` and `quit_shortcuts` in a single place shared by all graphics backends (`wgpu`, `software`, `glow`).
- Prevent `Ctrl+H`, `Ctrl+K`, `Ctrl+U`, and `Ctrl+W` from mutating any `TextEdit`-backed input (`LaTeX source`, shortcut search, inline `\command` input, numeric `DragValue` text mode) without breaking `Equation Canvas` palette shortcuts or Settings shortcut recording.
- Keep standard editing/navigation (`Ctrl+Arrow*`, `Ctrl+Backspace/Delete`, `Ctrl+Home/End`, multiline `Tab`), OS clipboard chords (`Ctrl/Shift+Insert`, `Shift+Delete`, Linux middle-click paste), and non-canvas `egui` focus traversal intact.
- Guarantee Settings shortcut customization rejects all preserved text-editing, clipboard, and navigation keys with accurate reserved-key feedback, while keeping bare `ArrowLeft` compatibility with the existing `"Press Ctrl, Alt, or F1–F12..."` hint.

**Non-Goals:**
- Changing `SYSTEM_SHORTCUTS` or `RESERVED_SYSTEM_CHORDS` in `desktop/src/shortcuts.rs`.
- Allowing customizable palette shortcuts to fire while `LaTeX source` or another text input has focus.
- Disabling `Ctrl+Wheel` zoom on the Equation Canvas or `LaTeX source` box.

## Decisions

### 1. Disable `zoom_with_keyboard` and `quit_shortcuts` in `App::new_with_state` and `App::draw_ui`
- **Choice**: Extract an idempotent `App::configure_egui_options(ctx: &egui::Context)` helper setting `options.zoom_with_keyboard = false` and `options.quit_shortcuts.clear()`, and call it in both `App::new_with_state` and `App::draw_ui`.
- **Rationale**: Covers `wgpu`, `software`, `glow`, and headless `from_parts` unit tests identically without patching `egui`.
- **Alternative considered**: Adding `Ctrl+Q`, `Ctrl+=`, `Ctrl+-`, `Ctrl++`, and `Ctrl+0` to `RESERVED_SYSTEM_CHORDS`. Rejected because EqnTide intentionally avoids global UI scaling and users need `Ctrl+Q`/`=`/`-`/`0` available as customizable symbol/template chords.

### 2. Strip `Ctrl+H/K/U/W` events before `TextEdit` sees them when non-canvas inputs are active
- **Choice**: Add `App::suppress_text_edit_readline_chords(&self, ctx: &egui::Context)` in `desktop/src/app/keyboard.rs` and invoke it in `App::draw_ui()` (before `draw_shell()` and modals) whenever `self.shortcut_capture.is_none()` and focus is not exclusively on `equation-canvas` (`!canvas_focused || command_input_focused`).
- **Rationale**: Because `egui::TextEdit` hardcodes `Ctrl+H/K/U/W` inside `check_for_mutating_key_press` with no `Options` toggle, removing those key-press events from `ctx.input_mut(|input| input.events)` before widgets render cleanly neutralizes them across `LaTeX source`, `shortcut_search`, `canvas-command-input`, and `DragValue` text entry.
- **Alternative considered**: Wrapping every `TextEdit` individually with pre/post snapshots or custom event filters. Rejected as brittle and easy to miss on `DragValue` or future `TextEdit` widgets.

### 3. Explicit reserved editing/clipboard/navigation feedback in Settings shortcut capture
- **Choice**: Keep `ShortcutKey` restricted to letters, digits, `F1..F12`, and ASCII punctuation (blocking Group 3 chords at both type and parser levels), and update `CaptureOutcome` in `desktop/src/app/shortcuts_ui.rs` with `CaptureOutcome::Reserved` for modified editing/navigation/clipboard keys and non-bindable control keys (`Tab`, `Enter`, `Space`, `Home`, `End`, `Insert`, `Ctrl/Alt/Shift+Backspace|Delete`, `Ctrl/Alt+Arrow*`), displaying `"Reserved for text editing, clipboard, or navigation · Backspace clears · Esc cancels"` while preserving `SHORTCUT_CAPTURE_HINT` for bare `ArrowLeft`/`F`/`/` etc. Note that `Event::Copy`/`Cut`/`Paste` (including `egui-winit`-translated `Ctrl/Shift+Insert` and `Shift+Delete`) continue to trigger `ShortcutConflict::System("Copy"/"Cut"/"Paste")`.
- **Rationale**: Gives immediate, accurate feedback when pressing `Ctrl+ArrowLeft`, `Ctrl+Backspace`, `Ctrl+Home`, or `Ctrl+Insert` during recording without breaking existing `ArrowLeft` validation expectations or introducing non-chord variants into `ShortcutChord`.

## Risks / Trade-offs

- **[Risk]** A user on Linux/macOS who expects `Ctrl+W`/`Ctrl+U`/`Ctrl+K` inside `LaTeX source` loses Emacs line-editing bindings → **Mitigation**: Intentionally accepted per user requirement so `Ctrl+H/K/U/W` remain clean for equation shortcuts; standard `Ctrl+Backspace`, `Ctrl+Delete`, `Ctrl+Arrow*`, `Home`/`End`, and `Shift+Home`/`End` continue working.
- **[Risk]** `Tab` or `Shift+Tab` pressed while `shortcut_capture` is active moves focus during `egui::Context::begin_pass()` → **Mitigation**: Either `process_shortcut_capture` rejects `Tab` with `CaptureOutcome::Reserved` or focus moves to `shortcut_search` which cancels recording; in neither case can `Tab` or `Shift+Tab` be bound.
