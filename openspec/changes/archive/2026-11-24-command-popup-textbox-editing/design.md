# Design

## Context

See `proposal.md` for motivation. In the current implementation (`desktop/src/commands.rs` and `desktop/src/app.rs`):
1. `commands::Input` holds only `name: String` and `selected: usize`, with no cursor tracking.
2. `App::canvas_text` always appends via `input.name.push(ch)`, and `App::command_key` handles `Backspace` via `input.name.pop()` while swallowing `Key::Delete | Key::ArrowLeft | Key::ArrowRight | Key::Home | Key::End => true`.
3. `App::command_popup` draws the command string as a static `ui.monospace(format!("\\{name}"))` inside an `egui::Area` (`Order::Foreground`), while keyboard focus remains on `Id::unique("equation-canvas")`.
4. `App::shell` invokes `self.keyboard(ui)` when `equation-canvas` has focus and no pointer button is active, and `App::sync_editor_focus` updates `active_editor_focus` only when `equation-canvas` or `latex-source` holds egui focus.

## Goals / Non-Goals

**Goals:**
- Provide standard single-line textbox editing in the `\` command popup (`ArrowLeft`, `ArrowRight`, `Home`, `End`, `Shift`/word selection, `Backspace`, `Delete`, `Copy`/`Cut`/`Paste`, pointer click/drag cursor placement, blinking caret, and horizontal overflow scrolling).
- Preserve immediate typing flow from the Equation Canvas when `\` is pressed (including multi-character chunks like `\alp` in a single frame) and keep headless `command_key`/`canvas_text` tests functional.
- Preserve all command-completion behaviors (`ArrowUp`/`ArrowDown` suggestion cycling, `Tab` completion without committing, `Enter` commit, `Esc` cancel, empty-buffer `Backspace` close, bare-prefix `Space` no-op, and clicking suggestions) and return focus to `equation-canvas` on close.

**Non-Goals:**
- Making the leading `\` prefix deletable or moving the cursor to the left of `\`; `\` stays a fixed visual prefix while the editable buffer holds everything after `\`.
- Live MathML preview inside the popup before `Enter`.

## Decisions

1. **Interactive `egui::TextEdit::singleline` in `command_popup` paired with cursor state in `commands::Input`**
   - Render the top row of `command_popup` inside a styled input frame (transparent fill matching the popup background, `CONTROL_CORNER_RADIUS`, `accent` border when focused / `border` when unfocused) containing a non-editable monospace `\` prefix label followed by `egui::TextEdit::singleline(&mut input.name)` (`Id::unique("canvas-command-input")`, monospace font, frameless inside the outer input frame). Clicking anywhere on the outer input row (including padding around `\`) focuses the `TextEdit`.
   - Extend `commands::Input` with `pub cursor: usize` (character index `0..=name.chars().count()`, matching `egui::text::CCursor::index`) and `pub sync_cursor: bool` (plus helper methods for UTF-8-safe char-index insert, backspace, delete, left/right, home/end).
   - *Rationale*: Reusing `egui::TextEdit::singleline` gives native egui textbox parity (pointer hit-testing, drag/double-click selection, `Shift+Arrow`/`Ctrl+Arrow`, `Ctrl+A`/`C`/`V`/`X`/`Z`, IME preedit, blinking cursor, horizontal scroll) matching `latex-source`, while `commands::Input` cursor helpers keep frame-1 canvas dispatch and headless `type_canvas`/`command_key` unit tests working cleanly.
   - *Alternative considered*: Reimplementing galley hit-testing, selection rendering, blinking cursor, and clipboard shortcuts manually on top of `ui.monospace(...)`. Rejected as fragile and prone to subtle UX gaps vs `egui::TextEdit`.

2. **Focus handoff and `shell()` / `command_popup()` key coordination**
   - When `\` starts `command_input` in `App::canvas_text`, mark `needs_focus = true` and `sync_cursor = true` on `commands::Input` (any trailing characters in the same `Event::Text`, e.g., `\alp`, insert at `cursor` and advance `cursor`).
   - In `App::command_popup`, when `needs_focus` is set, call `response.request_focus()` for `Id::unique("canvas-command-input")` and apply `sync_cursor` by writing `CCursorRange::one(CCursor::new(input.cursor))` into `egui::TextEdit::State`.
   - While `canvas-command-input` has focus:
     - `App::shell` sees `has_focus("equation-canvas") == false`, so `self.keyboard(ui)` does not run on subsequent frames and `TextEdit` receives standard textbox key/pointer events directly.
     - `App::sync_editor_focus` does not overwrite `active_editor_focus`, keeping it as `StartupFocus::Canvas` so sidebar/palette/footer focus restoration still targets `equation-canvas`.
     - Set `focus_lock_filter` on `canvas-command-input` (`tab: true`, `horizontal_arrows: true`, `vertical_arrows: true`, `escape: true`) so `Tab` and arrows never leak to surrounding UI widgets.
     - Before `TextEdit` processes input in `command_popup`, intercept and consume:
       - `Enter` (no command modifier): call `self.commit_command(true)`; if committed (`command_input` is `None`), `request_focus("equation-canvas")` and return; if invalid/incomplete, keep `command_input` open and keep `canvas-command-input` focused.
       - `Escape`: clear `self.command_input = None`, set cancel status, `request_focus("equation-canvas")`, and return.
       - `Tab` (no command/shift/alt): if `input.is_command_prefix()` and a suggestion exists at `input.selected`, set `input.name = command.name`, `input.selected = 0`, `input.cursor = input.name.chars().count()`, `input.sync_cursor = true`, and update `TextEdit::State` cursor immediately.
       - `ArrowUp` / `ArrowDown`: when `input.is_command_prefix()` and suggestions are non-empty, consume the key and cycle `input.selected` without letting `TextEdit` move its cursor to start/end.
       - `Backspace`: if `input.name.is_empty()` (empty buffer after `\`), consume the key, close `self.command_input = None`, `request_focus("equation-canvas")`, and return. (When `input.name` is non-empty, let `TextEdit` handle `Backspace` normally; at `cursor == 0`, it safely does nothing.)
       - `Space` (`Event::Text(" ")` / `Key::Space`): when `input.is_command_prefix()` is true, filter out the space event before `TextEdit` runs so bare command names stay unchanged per spec; once non-alphabetic arguments begin (`is_command_prefix() == false`), spaces pass through into `TextEdit` normally.
     - After `TextEdit` runs, if `response.changed()`, reset `input.selected = 0` so suggestions reflect the edited prefix, and sync `input.cursor` from `TextEdit::State`'s primary `CCursor`.

3. **Closing on outside clicks without breaking suggestion clicks**
   - Clicking a suggestion item inside `command_popup` sets `accept = Some(index)`, commits via `self.commit_command(true)`, and calls `request_focus("equation-canvas")`.
   - Clicking `equation-canvas` outside the popup already clears `self.command_input = None` in `canvas()`.
   - In `sync_editor_focus`, if `latex-source` gains focus (`has_latex`), clear `self.command_input = None` so clicking into `LaTeX source` dismisses any open popup cleanly.
   - Do not auto-close `command_input` on `TextEdit::lost_focus()` alone, because pointer-down on a suggestion item blurs `TextEdit` one step before `.clicked()` fires on release (and headless `command_popup` tests run without initial focus).

## Risks / Trade-offs

- **Risk**: Updating `completion_accept_keys_work_through_the_focused_canvas` expectations → **Mitigation**: In full `shell()` runs, widget focus moves from `equation-canvas` to `canvas-command-input` while the popup is open and returns to `equation-canvas` upon `Enter`/`Esc`/suggestion click. Update that test to verify `canvas-command-input` holds focus while editing the popup and `equation-canvas` regains focus after `Enter`.
- **Risk**: `TextEdit` consuming `Enter`/`Tab`/`Up`/`Down`/`Space` before popup handlers run → **Mitigation**: Intercept and consume those keys on `ui.input_mut(...)` (and strip `Event::Text(" ")` when `is_command_prefix()`) strictly *before* `ui.add(TextEdit::singleline(...))` whenever `canvas-command-input` has focus.
- **Risk**: `canvas_caret_visible` or `sync_editor_focus` losing `StartupFocus::Canvas` while popup is open → **Mitigation**: `sync_editor_focus` leaves `active_editor_focus` untouched when neither `equation-canvas` nor `latex-source` has focus, preserving `StartupFocus::Canvas` across all sidebar/palette interactions.
