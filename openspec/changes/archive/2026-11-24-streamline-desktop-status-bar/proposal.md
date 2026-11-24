# Proposal

## Why

The desktop application's bottom status bar currently opens with a verbose onboarding hint (`"Click the canvas to type. Use the palette or Ctrl+F for a fraction."`), emits redundant confirmation messages after routine canvas operations (`"Inserted LaTeX command."`, `"Canceled LaTeX command."`, `"Inserted R × C matrix."`), renders a spinning `ui.spinner()` (or `"Working…"`) busy indicator beside the bridge dot during asynchronous operations, and never clears `self.status` when editing continues—leaving stale messages visible indefinitely even when nothing requires user attention. Streamlining these elements keeps the status bar quiet and static during idle and busy states while preserving `32px` panel height, actionable errors, compatibility warnings, and key session/save feedback.

## What Changes

- Initialize `self.status` as empty (`""`) instead of the onboarding hint string so the status bar shows no idle text on launch.
- Remove the `ui.spinner()` / `"Working…"` busy indicator in `status_bar()` (and remove `self.busy` from `has_active_animation()`), leaving only the static bridge readiness dot (`●` / `○`) while keeping the `32px` status bar panel height intact.
- Remove redundant confirmation messages for routine canvas actions: `"Inserted LaTeX command."`, `"Canceled LaTeX command."`, and `"Inserted R × C matrix."`.
- Shorten the Office session active notice to `"Office session — Save to return"`.
- Clear `self.status` automatically when the user resumes editing (`changed()`, `source_changed()`, `load_math()`, or opening a new inline `\command` buffer) so stale status or prior error messages do not linger once editing moves on.
- Preserve all high-priority errors, target compatibility warnings, active progress states (`"Updating equation…"`, `"Returning the result to Office…"`), blocking save/matrix validations, and major outcome notifications (`"Saved."`, `"Batch finished."`, `"Batch stopped."`).

## Capabilities

### New Capabilities
- `desktop-status-bar`: Defines idle quietness, removal of the status-bar spinner animation, message filtering, concise session notices, and automatic clearing of stale status text on subsequent edits in the desktop status bar, complementing existing `LaTeX error: ...` and compact button rules.

### Modified Capabilities
<!-- No existing capability requirements are altered; `desktop-latex-typing` and `desktop-ui-theme` status-bar requirements remain intact. -->

## Impact

- `desktop/src/app.rs`: `EqnTideApp::from_parts`, `has_active_animation`, `status_bar`, `changed`, `source_changed`, `load_math`, `insert_custom_matrix`, `commit_command`, `canvas_text`, `command_key`, `command_popup`, `events` (`Event::Session`), and unit tests verifying status-bar behavior.
