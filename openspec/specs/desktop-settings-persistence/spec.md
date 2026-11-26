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
The desktop application SHALL persist and restore canvas zoom, appearance theme, custom app background settings (`background_mode`, `background_image`, `background_opacity`, `background_accent_enabled`, `background_accent`), startup focus target, font size, equation output target, active palette category, recent palette items, batch conversion target, and batch overwrite option, clamping or defaulting invalid stored values safely while ignoring legacy canvas preset keys (`canvas_light`, `canvas_dark`) in existing JSON files.

#### Scenario: Valid persisted settings restore on launch
- **WHEN** the preferences file contains valid values for zoom, theme, custom app background settings, startup focus target, font size, output target, category, recent items, batch target, and batch overwrite
- **THEN** the desktop application launches with those values applied

#### Scenario: Corrupt or out-of-range preferences fall back without failing
- **WHEN** the preferences file is missing, contains invalid JSON, or contains out-of-range numbers or unknown identifiers
- **THEN** the application starts normally with safe clamped or default values without reporting a startup crash

#### Scenario: Empty Recent category does not trap startup
- **WHEN** the saved category is `Recent` but the saved recent list is empty or contains no valid palette items
- **THEN** the application starts on the default populated structural category instead of an empty `Recent` strip

#### Scenario: Per-mode canvas color presets persist and sanitize
- **WHEN** an older preferences file contains legacy `canvas_light` or `canvas_dark` keys
- **THEN** the application ignores those legacy keys without failing and uses `Ayu light` in Light mode and `Ayu dark` in Dark mode while preserving all other preferences

#### Scenario: Startup focus target defaults safely when missing or invalid
- **WHEN** the preferences file omits `startup_focus` or stores an unrecognized value
- **THEN** the application safely defaults `startup_focus` to `Equation canvas`

#### Scenario: Background opacity clamps and defaults safely
- **WHEN** `background_opacity` is non-finite or out of range
- **THEN** `background_opacity` clamps safely into the valid range (`0.0..=1.0`, defaulting to a readable default when non-finite)

#### Scenario: Legacy settings without background mode migrate seamlessly
- **WHEN** an existing `settings.json` specifies `background_image` without `background_mode`
- **THEN** the desktop application defaults `background_mode` to `image` when `background_image` is present and `solid` when `background_image` is absent, and defaults `background_accent_enabled` to `true`

#### Scenario: Switching to solid mode preserves saved background image and accent seed
- **WHEN** the user switches `background_mode` from `image` to `solid`
- **THEN** `background_mode` persists as `solid`, `background_image` and `background_accent` remain saved in `settings.json`, and the managed PNG file remains on disk for instant re-enabling

### Requirement: Immediate save on user changes
The desktop application SHALL persist updated preferences atomically whenever the user changes a persisted setting (including selecting, adjusting, or clearing the app background) or inserts a palette item in the UI.

#### Scenario: User setting edits write safely to disk
- **WHEN** the user adjusts zoom, toggles theme, changes startup focus, modifies app background settings, changes font size or output target, switches palette category, changes batch options, or inserts a palette item
- **THEN** the updated preferences are written atomically to the preferences file without corrupting existing content on failure

### Requirement: Configurable startup focus target in Settings
The Settings view SHALL provide a clear control allowing the user to choose whether opening the window focuses `Equation canvas` (default) or `LaTeX source`.

#### Scenario: Selecting LaTeX source in Settings updates persisted startup focus
- **WHEN** the user switches the `Startup focus` setting in Settings from `Equation canvas` to `LaTeX source`
- **THEN** the preference is saved immediately and used when opening subsequent editor windows or sessions

### Requirement: Office session isolation
Values supplied transiently by an Office add-in session (`font_pt` and `target_mode`) SHALL apply only to that editing session and SHALL NOT overwrite the user's persisted standalone defaults.

#### Scenario: Opening an Office equation leaves saved defaults intact
- **WHEN** an Office session loads an equation with its own `font_pt` or `target_mode` and the user closes without manually changing footer controls
- **THEN** the saved preferences file retains the user's previously saved `font_pt` and `target`

### Requirement: Managed app background image storage and fallback
When the user selects a custom background image, the desktop application SHALL import and store a normalized copy inside the application configuration directory alongside `settings.json` (rather than relying on the original file staying in place), clean up replaced or cleared managed files, and fall back safely to the default Ayu solid background if the managed image file is missing or unreadable at runtime.

#### Scenario: Deleting or moving the original image file does not break the app background
- **WHEN** the user selects an image from an external folder and later moves or deletes that original file before restarting the application
- **THEN** the application restores the custom background from the managed copy inside its configuration directory when `background_mode` is `image`

#### Scenario: Clearing the background removes the managed image reference and file
- **WHEN** the user clicks **Clear** in Settings
- **THEN** the persisted `background_image` and `background_accent` fields are cleared immediately, `background_mode` resets to `solid`, and the managed background image file in the configuration directory is removed if present

#### Scenario: Missing or corrupt managed background file fails gracefully
- **WHEN** `settings.json` references a managed background file that no longer exists or cannot be decoded
- **THEN** the application ignores the missing image, renders the standard Ayu solid theme without crashing, and preserves all other user preferences
