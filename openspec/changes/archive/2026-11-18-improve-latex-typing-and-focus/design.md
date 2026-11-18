# Design

## Context

See `proposal.md` for motivation. Currently in `desktop/src/app.rs`:
1. `App::canvas_text` stores only `commands::Input { name, selected }` and commits as soon as a non-ASCII-alphabetic character arrives when `name` contains only ASCII letters (`!ch.is_ascii_alphabetic()`), then re-dispatches `{`, `_`, `^` into structural placeholders.
2. `App::command_key` commits on `Enter`, `Tab`, and pointer click, while `Space` falls through as non-alphabetic commit in `canvas_text`.
3. `App::from_parts`, `App::load_next`, and `App::ui_pass` do not request focus for either `equation-canvas` or `latex-source`, and most non-canvas controls (`sidebar`, `footer` Undo/Redo/target/DragValue, matrix `+ row`/`− row`/`+ col`/`− col`, status buttons) leave focus on the clicked button instead of returning it to the active editor surface.

## Goals / Non-Goals

**Goals:**
- Allow full inline LaTeX expressions starting with `\` (e.g. `\mathbb{R}`, `\int_0^1 x dx`, `\lim_{x \to 0}`) to be typed continuously in a single buffer and committed once via `Enter`.
- Keep command suggestions working while typing the leading command token (`\ma` -> `\mathbb`), with `Tab` filling only the suggested command name into the buffer and `Space` treated as literal buffer text.
- Track active editor focus (`EditorFocus::Canvas` vs `EditorFocus::Latex`), focus the configured startup surface on launch/session load/return to Editor tab, and restore focus to the active editor surface after button interactions.
- Add `StartupFocus` (`Canvas` default, `Latex`) to `Settings` and `settings_ui`.

**Non-Goals:**
- Replacing the structural Equation Canvas with a plain-text LaTeX editor; structural navigation outside `\` mode remains unchanged.
- Live re-parsing on every keystroke inside the `\` buffer; parsing occurs on `Enter` commit via `eqntide::service::parse_latex_to_mathml` (with standalone template fallback).

## Decisions

1. **Buffered `commands::Input` structure and leading token split**
   - `commands::Input` stores the raw buffer `text: String` (everything after `\`) and `selected: usize`.
   - When computing `suggestions`, extract the leading command name (either the entire `text` if all ASCII alphabetic, or the prefix before the first non-alphabetic character). Suggestions are shown while typing the leading command token; once arguments begin (`{`, `_`, `^`, space, etc.), `suggestions` stops overriding the buffer and the popup displays the raw `\text` buffer plus `Enter to insert · Backspace to edit · Esc to cancel`.
   - *Alternative considered*: Hybrid placeholder deferral (`Option B`), rejected because keeping nested `{}`/`_`/`^` synchronized across partial MathML subtrees adds fragile edge cases for `\int_0^1`, `\limits`, and `\mathbb{R}`.

2. **Key semantics in buffered mode (`Enter`-only commit)**
   - `Enter`:
     - If `text` has no non-alphabetic argument suffix and `commands::suggestions(&text)` / `commands::exact` matches a `#?` template (or selected suggestion when user typed a bare command prefix like `\frac`), keep existing behavior: insert template `#?` (wrapping selection if present) unless `text` already contains arguments or is a non-template command. Wait—careful distinction: if the user typed `\alp` and hit `Enter` directly without `Tab`, `commands::suggestions("alp")[0]` (`\alpha`) can still be committed as before; if the user typed `\math` and hit `Enter`, wait: `\math` is not exact, whereas if `\mathbb` is highlighted in suggestions, `palette::template("\\mathbb{#?}")` or `parse_latex_to_mathml` runs. Specifically: if `text` is strictly ASCII alphabetic and has a matching suggestion (exact match preferred, else `suggestions(&text).get(selected)`), resolve via `palette::template(command.source)` (supporting selection wrapping for `\frac` etc.). Otherwise (when `text` contains `{`, `_`, `^`, spaces, etc.), parse `format!("\\{text}")` directly via `eqntide::service::parse_latex_to_mathml` and insert the resulting MathML in one edit (`self.editor.insert_math` / `insert_template`).
   - `Tab`:
     - If `text` is strictly ASCII alphabetic (still typing command name) and a suggestion exists at `selected`, replace `text` with `suggestion.name` and reset `selected = 0`, keeping `command_input` open without committing. Otherwise ignore (`true` consumed so egui focus does not leave canvas).
   - `Space`:
     - Treated as normal text in `canvas_text`, appended to `input.name` / `input.text` inside `command_input` so expressions like `\lim_{x \to 0}` or `\alpha + \beta` are never cut off.
   - `Backspace` / `Esc` / `Ctrl+Z`:
     - `Backspace` pops from buffer (closing `command_input` when empty); `Esc` discards buffer; `Ctrl+Z` cancels active buffer before touching editor undo stack.

3. **`EditorFocus` tracking and focus restoration**
   - Define `enum EditorFocus { Canvas, Latex }` derived from `StartupFocus`.
   - Maintain `active_editor_focus: EditorFocus` (initialized from `settings.startup_focus`) and `pending_focus_restore: Option<EditorFocus>` (initialized to `Some(settings.startup_focus)` on startup, set on `load_next` / `tab` transitions back to `0`, and set when closing `matrix_dialog`).
   - During `shell` / `editor_ui`:
     - If `equation-canvas` currently has egui focus, set `active_editor_focus = EditorFocus::Canvas`; if `latex-source` has egui focus, set `active_editor_focus = EditorFocus::Latex`.
     - Avoid calling `request_focus` while `matrix_dialog` or modal `dialogs` are open, or while the user is actively interacting with a `DragValue` text-edit or combo popup.
     - Whenever an editor button is clicked (`sidebar` category button, `palette_ui` button, matrix `+ row`/`− row`/`+ col`/`− col`, `footer` `Undo`/`Redo`/target radio, `status_bar` `Retry`/`Next session`), schedule `pending_focus_restore = Some(self.active_editor_focus)`.
     - Apply `pending_focus_restore` via `ctx.memory_mut(|m| m.request_focus(target_id))` at the end of the editor pass (when `tab == 0` and no modal overlay is active) so the focused surface is ready on the very next keystroke without stealing focus from modal dialogs.
   - Note on `egui::DragValue`: dragging `Font size` updates `font_pt` without wanting text focus permanently trapped, while double-clicking `DragValue` opens inline text editing (`has_focus`). Only restore focus after discrete button/radio clicks or pointer-release after drag, never while `DragValue` is actively being edited via keyboard.

4. **Persisted `StartupFocus` setting**
   - Add `#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "lowercase")] pub(crate) enum StartupFocus { #[default] Canvas, Latex }` to `desktop/src/settings.rs`.
   - Add `#[serde(default)]` on `Settings` (already present) so existing `settings.json` files deserialize seamlessly to `StartupFocus::Canvas`.
   - Expose in `settings_ui` under a clear `Startup focus` section/row using selectable controls (`Equation canvas`, `LaTeX source`) matching existing Settings typography and button heights (`30px`).

## Risks / Trade-offs

- **Risk**: `Tab` in buffered mode no longer commits `\alpha` directly -> **Mitigation**: Document clearly in popup footer hint (`↑↓ select · Tab autocomplete · Enter insert · Esc cancel`) and update `completion_accept_keys_work_through_the_focused_canvas` test to verify `Enter` commits, `Tab` completes command name without committing, and `Space` stays in buffer.
- **Risk**: Restoring focus to `latex-source` after `palette_ui` or `Undo`/`Redo` runs `self.changed()` (which updates `self.latex`) -> **Mitigation**: Only restore widget focus (`request_focus(Id::unique("latex-source"))`) without clearing text selection unnecessarily, and ensure `tab == 0` / no-modal guards prevent fighting `matrix_dialog`, `Finish current equation` dialog, or `Settings` inputs.
- **Risk**: Clicking `Font size` `DragValue` vs dragging it -> **Mitigation**: Do not steal keyboard focus while user is typing a number into `DragValue`; restore `active_editor_focus` only on discrete clicks (`sidebar`, `palette`, matrix controls, `Undo`/`Redo`, target radios, status buttons) and tab/dialog return.
