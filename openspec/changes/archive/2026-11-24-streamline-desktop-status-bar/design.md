# Design

## Context

In `desktop/src/app.rs`, the bottom `app-status` panel (`exact_size(32.0)`) renders `status_bar()` with:
1. Optional `if self.busy` spinner (`ui.spinner()` or `"Working…"` on software GPU), paired with `has_active_animation()` returning `true` while `self.busy`.
2. Static bridge dot (`●` / `○`) and conditional compact `24px` buttons (`Retry`, `Next session`).
3. Mutual-exclusion message slot ordered by priority:
   1. `active_latex_error` (`LaTeX error: ...`)
   2. `draft_dirty` (`"Updating equation…"`)
   3. `target_warning` (`Warning: ...`)
   4. `self.status` (fallback event/hint string)
In `desktop/src/app.rs`, the bottom `app-status` panel (`exact_size(32.0)`) renders `status_bar()` with:
1. Optional `if self.busy` spinner (`ui.spinner()` or `"Working…"` on software GPU), paired with `has_active_animation()` returning `true` while `self.busy`.
2. Static bridge dot (`●` / `○`) and conditional compact `24px` buttons (`Retry`, `Next session`).
3. Mutual-exclusion message slot ordered by priority:
   1. `active_latex_error` (`LaTeX error: ...`)
   2. `draft_dirty` (`"Updating equation…"`)
   3. `target_warning` (`Warning: ...`)
   4. `self.status` (fallback event/hint string)

See `proposal.md` for why `self.status` and the busy spinner currently addadd visual clutter. Existing requirements in `desktop-latex-typing` (`LaTeX error: ...`) and `desktop-ui-theme` (`24px` compact action buttons `Retry` / `Next session` with `4px` vertical panel margin = `32px` panel height) remain unchanged.

## Goals / Non-Goals

**Goals:**
- Keep the `32px` status-bar panel height unchanged so `24px` compact buttons (`Retry`, `Next session`) retain their `4px` vertical margins without layout shifts.
- Remove the `ui.spinner()` / `"Working…"` busy indicator in `status_bar()` (and `self.busy` from `has_active_animation()`) so the bridge dot (`●`/`○`) stays static.
- Ensure the status bar displays no text when idle on startup or after routine canvas actions.
- Automatically clear stale `self.status` text whenever the user resumes editing (`changed()`, `source_changed()`, opening an inline `\command`, or editing/canceling/dismissing the command popup).
- Shorten the Office session active notice to `"Office session — Save to return"`.
- Keep all error, warning, progress, blocking validation, and major completion messages intact.

**Non-Goals:**
- Changing the `32px` status-bar panel height, bridge dot (`●`/`○`), `Retry` / `Next session` buttons, or `LaTeX error: ...` / `Warning: ...` rendering rules.
- Introducing a timed toast/auto-fade timer; clearing on the next user edit keeps state deterministic without background timers or extra repaints.

## Decisions

1. **Keep `32px` panel height and remove `ui.spinner()` / `"Working…"`**
   - **Choice**: Keep `egui::Panel::bottom("app-status").exact_size(32.0)` unchanged, remove the `if self.busy` block from `status_bar()`, and simplify `has_active_animation()` to check `self.batch_running && self.gpu_profile != GpuProfile::Software`.
   - **Rationale**: `COMPACT_BUTTON_HEIGHT` (`24px`) plus `SPACE_XS` (`4px`) top/bottom inner margin equals `32px`; keeping `32px` avoids clipping `Retry` / `Next session`. Removing `ui.spinner()` keeps the bar visually calm (just the `●`/`○` dot) and eliminates continuous repaint pressure during `self.busy`. Note that closing after an Office save still reports `"Returning the result to Office…"` via `self.status`.
   - **Alternative considered**: Reducing panel height below `32px` when buttons are absent. Rejected because dynamic panel height changes shift the canvas layout whenever `Retry` or `Next session` appears/disappears.

2. **Clear `self.status` at edit entry points rather than adding a time-based expiry**
1. **Keep `32px` panel height and remove `ui.spinner()` / `"Working…"`**
   - **Choice**: Keep `egui::Panel::bottom("app-status").exact_size(32.0)` unchanged, remove the `if self.busy` block from `status_bar()`, and simplify `has_active_animation()` to check `self.batch_running && self.gpu_profile != GpuProfile::Software`.
   - **Rationale**: `COMPACT_BUTTON_HEIGHT` (`24px`) plus `SPACE_XS` (`4px`) top/bottom inner margin equals `32px`; keeping `32px` avoids clipping `Retry` / `Next session`. Removing `ui.spinner()` keeps the bar visually calm (just the `●`/`○` dot) and eliminates continuous repaint pressure during `self.busy`. Note that closing after an Office save still reports `"Returning the result to Office…"` via `self.status`.
   - **Alternative considered**: Reducing panel height below `32px` when buttons are absent. Rejected because dynamic panel height changes shift the canvas layout whenever `Retry` or `Next session` appears/disappears.

2. **Clear `self.status` at edit entry points rather than adding a time-based expiry**
   - **Choice**: Call `self.status.clear()` in `changed()`, `source_changed()`, and when opening `self.command_input` on `\` in `canvas_text()`.
   - **Rationale**: `changed()` is already the single convergence point for canvas typing, deletion, undo/redo, palette insertion, matrix row/col mutation, and successful `load_math()`. Clearing there guarantees `"Saved."`, `"Batch finished."`, or resolved save/matrix validation notices stay visible immediately after the event and disappear as soon as the user edits again—without requiring a repaint timer (which would also conflict with software-profile idle frame capping).
   - **Ordering note**: `Event::Session` calls `load_math()` (which calls `changed()`) and *then* sets `self.status = "Office session — Save to return".into()`, so the session notice still appears upon loading and clears on the user's first edit. On failed `load_math()`, `changed()` is not called, so load error messages are preserved.
   - **Alternative considered**: Auto-clearing `"Saved."` / `"Batch finished."` after N seconds. Rejected because it adds `Instant` state and periodic repaint wakeups while still failing to clear `"Place the caret inside a matrix first."` immediately on the next keystroke.

33. **Synchronize `command_error` clearing with `self.status` (`clear_command_error` / `dismiss_command_input`)**
   - **Choice**: Add `clear_command_error()` (clears `self.command_error` and clears `self.status` if `self.status == error`) and `dismiss_command_input()` (clears `self.command_input` and calls `clear_command_error()`) and route all command popup edits, Backspace/Delete/Tab/Arrow changes, Escape cancels, and outside clicks/focus switches through them.
   - **Rationale**: `commit_command()` writes errors to both `self.command_error` and `self.status` (which existing tests assert). Previously, clearing only `self.command_error = None` on edit/Escape/empty-Backspace left `self.status` populated, causing the plain error string to leak into the status bar as soon as `command_error` became `None` or the popup closed.

4. **Remove routine action confirmations (`commit_command`, `command_key`/`command_popup` Escape, `insert_custom_matrix`)**
   - **Choice**: Drop the `self.status = ...` assignments for `"Inserted LaTeX command."`, `"Canceled LaTeX command."`, and `"Inserted {} × {} matrix."`, and initialize `from_parts` with `status: String::new()`.
   - **Rationale**: Canvas and popup changes provide immediate visual feedback directly in the equation view. None of the existing specs or unit tests assert on these three strings or the startup hint.

45. **GuardGuard empty `self.status` label renderinglabel rendering**
   - **Choice**: In `status_bar()`, only add the label and `.on_hover_text(&self.status)` when `!self.status.is_empty()` so hovering over empty space in the status bar does not show an empty tooltip box.
   - **Rationale**: Prevents an artifact empty tooltip frame on hover when `self.status` is `""`.

## Risks / Trade-offs

- **Loss of confirmation toast after saving when user immediately edits**: After `"Saved."` appears, the user's very next keystroke clears it. → Acceptable and desired; `"Saved."` remains visible until they move on to the next edit.
- **No spinning indicator during slow Office session fetch**: When loading an Office session (`self.busy = true`), the status bar shows only the static dot instead of a spinner. → Acceptable per user preference; `begin_close` still displays `"Returning the result to Office…"`, and interactive controls remain guarded by `self.busy`.
- **Standalone user discovery of `Ctrl+F` / canvas typing**: Removing the startup hint assumes the Equation Canvas placeholder / UI is self-evident. → Mitigated by the blinking canvas caret on startup (`startup_focus: Canvas`) and palette tooltips.
