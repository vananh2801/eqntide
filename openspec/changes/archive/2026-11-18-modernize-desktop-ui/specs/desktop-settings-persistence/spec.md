# Spec Delta

## MODIFIED Requirements

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
