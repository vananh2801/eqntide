# desktop-settings-persistence Specification

## Purpose

Persists user preferences and recent palette history across desktop application restarts on Linux and Windows while keeping Office add-in session parameters isolated from saved defaults.

## Requirements

### Requirement: Cross-platform preferences persistence location
The desktop application SHALL store user preferences in a per-user JSON file inside the operating system's standard application configuration directory on both Linux and Windows.

#### Scenario: Preferences persist across application restarts
- **WHEN** the user modifies preferences and restarts the desktop application
- **THEN** the saved preferences file is loaded from the platform configuration directory and the previous values are restored

### Requirement: Persisted user settings scope and validation
The desktop application SHALL persist and restore canvas zoom, appearance theme, Light and Dark mode Equation Canvas presets, font size, equation output target, active palette category, recent palette items, batch conversion target, and batch overwrite option, clamping or defaulting invalid stored values safely.

#### Scenario: Valid persisted settings restore on launch
- **WHEN** the preferences file contains valid values for zoom, theme, canvas presets, font size, output target, category, recent items, batch target, and batch overwrite
- **THEN** the desktop application launches with those values applied

#### Scenario: Corrupt or out-of-range preferences fall back without failing
- **WHEN** the preferences file is missing, contains invalid JSON, or contains out-of-range numbers or unknown identifiers
- **THEN** the application starts normally with safe clamped or default values without reporting a startup crash

#### Scenario: Empty Recent category does not trap startup
- **WHEN** the saved category is `Recent` but the saved recent list is empty or contains no valid palette items
- **THEN** the application starts on the default populated structural category instead of an empty `Recent` strip

#### Scenario: Per-mode canvas color presets persist and sanitize
- **WHEN** the user selects custom Equation Canvas presets for Light mode or Dark mode, or when the stored preset identifier is unrecognized
- **THEN** valid presets persist across restarts and unrecognized preset identifiers fall back safely to the default soft light paper preset

### Requirement: Immediate save on user changes
The desktop application SHALL persist updated preferences atomically whenever the user changes a persisted setting or inserts a palette item in the UI.

#### Scenario: User setting edits write safely to disk
- **WHEN** the user adjusts zoom, toggles theme, changes font size or output target, switches palette category, changes batch options, or inserts a palette item
- **THEN** the updated preferences are written atomically to the preferences file without corrupting existing content on failure

### Requirement: Office session isolation
Values supplied transiently by an Office add-in session (`font_pt` and `target_mode`) SHALL apply only to that editing session and SHALL NOT overwrite the user's persisted standalone defaults.

#### Scenario: Opening an Office equation leaves saved defaults intact
- **WHEN** an Office session loads an equation with its own `font_pt` or `target_mode` and the user closes without manually changing footer controls
- **THEN** the saved preferences file retains the user's previously saved `font_pt` and `target`
