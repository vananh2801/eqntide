# Spec Delta

## MODIFIED Requirements

### Requirement: Quiet idle status bar without onboarding hints or spinner animation
The desktop application SHALL preserve the `32px` bottom status bar height, initialize the status message to empty, vertically center all status bar items (readiness dot, buttons, and text), render the bridge readiness indicator as a static vector circle (filled when connected, stroked when disconnected) without a spinner or `"Working…"` prefix when `self.busy` is true, and SHALL NOT display persistent onboarding hints or routine editing confirmations.

#### Scenario: Launching desktop application in standalone mode shows no status message
- **WHEN** the desktop application launches without an active Office session, error, or compatibility warning
- **THEN** the bottom status bar renders at `32px` height with only the vertically centered vector bridge readiness circle (and `Retry` if disconnected) and empty message text

#### Scenario: Asynchronous busy operations do not render a spinner animation
- **WHEN** `self.busy` is true (such as loading an Office session, saving `.mathml`, or returning/canceling a session)
- **THEN** the status bar displays no `ui.spinner()` animation or `"Working…"` prefix, leaving the normal static vector bridge readiness circle in place

#### Scenario: Committing or canceling inline LaTeX commands or inserting custom matrices leaves status empty
- **WHEN** the user commits a valid `\command`, cancels the command popup with `Esc`, or inserts a custom matrix from the matrix dialog without errors or target warnings
- **THEN** `self.status` remains empty rather than displaying `"Inserted LaTeX command."`, `"Canceled LaTeX command."`, or `"Inserted R × C matrix."`

#### Scenario: Status text and readiness dot share the vertical centerline
- **WHEN** the status bar displays any status message, `LaTeX error: ...`, `Warning: ...`, or compact button (`Retry`, `Next session`) alongside the bridge readiness dot
- **THEN** the vector dot, compact buttons, and text label are vertically centered within the `32px` status bar
