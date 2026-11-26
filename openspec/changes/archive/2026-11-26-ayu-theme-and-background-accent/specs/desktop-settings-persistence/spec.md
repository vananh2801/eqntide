# Spec Delta

## MODIFIED Requirements

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
