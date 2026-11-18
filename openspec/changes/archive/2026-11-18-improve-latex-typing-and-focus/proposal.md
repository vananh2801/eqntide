# Proposal

## Why

Typing LaTeX directly on the desktop Equation Canvas currently commits command names prematurely whenever non-alphabetic characters (`{`, `_`, `^`, spaces) are typed, breaking flow on expressions like `\mathbb{R}`, `\int_0^1`, and `\lim_{x \to 0}` by jumping unexpectedly into structural placeholder boxes. In addition, the application starts without focusing either input surface, and toolbar, sidebar, matrix, or footer controls steal keyboard focus from whichever surface the user was typing in.

## What Changes

- Switch Equation Canvas backslash (`\`) entry to buffered raw LaTeX command input that keeps `{`, `}`, `_`, `^`, spaces, and operator arguments in the expression buffer until committed.
- **BREAKING** Change canvas command completion key behavior so `Enter` is the dedicated commit key, `Space` stays as literal expression spacing inside the buffer, and `Tab` only fills the highlighted command name suggestion into the buffer without committing.
- Preserve existing command-name suggestions while typing the leading command token and keep template `#?` fallback when committing a standalone template command name with `Enter`.
- Track active editor focus between `Equation canvas` and `LaTeX source`, restoring focus to the previously active editor surface after interacting with palette items, sidebar category buttons, matrix row/column controls, undo/redo, target toggles, status actions, dialogs, or tab navigation back to the editor.
- Automatically focus the configured default input surface on window launch and session activation, and add a persisted Settings option (`Startup focus`) to choose between `Equation canvas` (default) and `LaTeX source`.

## Capabilities

### New Capabilities
- `desktop-latex-typing`: Buffered raw LaTeX command input on the Equation Canvas (`\` through `Enter`), suggestion completion without premature commit, startup focus targeting, and active editor focus preservation across UI interactions.

### Modified Capabilities
- `desktop-settings-persistence`: Persist and restore the user's `startup_focus` preference (`canvas` or `latex`, defaulting to `canvas`) in the JSON settings file and expose it in Settings.

## Impact

- `desktop/src/app.rs`: Replace premature non-alphabetic `commit_command` splitting in `App::canvas_text` / `App::command_key` / `App::command_popup`, add buffered command state, track `EditorFocus`, request startup/restoration focus across editor controls, and render the `Startup focus` control in `settings_ui`; update affected canvas completion unit tests.
- `desktop/src/settings.rs`: Add `StartupFocus` enum and `startup_focus` field to `Settings` with safe serde default/fallback validation.
- No changes required to core conversion, MathML/OMML/MathType export, or Office add-in protocol.
