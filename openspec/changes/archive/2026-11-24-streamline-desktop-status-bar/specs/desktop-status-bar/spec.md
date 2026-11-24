# Spec Delta

## Purpose

Defines the idle-quiet message policy, static indicator presentation without spinner animation, noise filtering, concise session notice, and automatic stale-status clearing behavior for the desktop application's bottom status bar.

## ADDED Requirements

### Requirement: Quiet idle status bar without onboarding hints or spinner animation
The desktop application SHALL preserve the `32px` bottom status bar height, initialize the status bar message to empty, render only the static bridge readiness dot (`●`/`○`) without a spinning busy indicator or `"Working…"` prefix when `self.busy` is true, and SHALL NOT display persistent onboarding hints or routine canvas confirmation messages during normal editing.

#### Scenario: Launching desktop application in standalone mode shows no status message
- **WHEN** the desktop application launches without an active Office session, error, or compatibility warning
- **THEN** the bottom status bar renders at `32px` height with only the bridge readiness indicator (and `Retry` if disconnected) and empty message text

#### Scenario: Asynchronous busy operations do not render a spinner animation
- **WHEN** `self.busy` is true (such as loading an Office session, saving `.mathml`, or returning/canceling a session)
- **THEN** the status bar displays no `ui.spinner()` animation or `"Working…"` prefix, leaving the normal bridge readiness dot (`●`/`○`) in place

#### Scenario: Committing or canceling inline LaTeX commands or inserting custom matrices leaves status empty
- **WHEN** the user commits a valid `\command`, cancels the command popup with `Esc`, or inserts a custom matrix from the matrix dialog without errors or target warnings
- **THEN** `self.status` remains empty rather than displaying `"Inserted LaTeX command."`, `"Canceled LaTeX command."`, or `"Inserted R × C matrix."`

### Requirement: Automatic clearing of stale status messages on subsequent edits
When the user modifies the equation, edits `LaTeX source`, loads a new equation, opens an inline `\command` buffer, or edits/cancels/dismisses an active command buffer after an invalid command error, the desktop application SHALL clear any prior `self.status` message so transient notices or resolved/canceled errors do not persist.

#### Scenario: Editing the equation clears a prior save or validation notice
- **WHEN** `self.status` holds a prior message such as `"Saved."`, `"Wait for valid LaTeX before saving."`, or `"Place the caret inside a matrix first."` and the user types on the Equation Canvas, clicks a palette item, or edits `LaTeX source`
- **THEN** `self.status` is cleared immediately upon the edit

#### Scenario: Editing, backspacing out of, or canceling the command popup clears its prior invalid-command error from status
- **WHEN** `commit_command` fails (e.g. `\text` followed by `Enter`) and sets `command_error` / `status`, and the user then presses `Backspace` (including backspacing until the popup closes), presses `Esc`, types/edits in the popup, or clicks away from the popup
- **THEN** both `command_error` and the matching `self.status` error message clear immediately so no stale plain-text error remains in the status bar

#### Scenario: Loading an Office session displays concise session context after load
- **WHEN** the desktop application activates an Office editing session
- **THEN** the status bar displays `"Office session — Save to return"`, and clears that notice once the user begins editing the equation

### Requirement: Preservation of actionable errors, warnings, and major outcomes
Streamlining idle and routine messages SHALL preserve `LaTeX error: ...` diagnostics, OMML/MathType compatibility `Warning: ...` banners, active progress states (`"Updating equation…"`, `"Returning the result to Office…"`), blocking save/matrix validation messages, operational I/O or bridge errors, and final outcomes (`"Saved."`, `"Batch finished."`, `"Batch stopped."`).

#### Scenario: Actionable errors and compatibility warnings remain visible until resolved
- **WHEN** an invalid LaTeX input, unsupported OMML/MathType construct, blocked save attempt, or session/bridge failure occurs
- **THEN** the status bar continues to display the corresponding error, warning, or blocking reason according to the existing priority order
