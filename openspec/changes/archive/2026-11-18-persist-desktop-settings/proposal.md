# Proposal

## Why

The desktop equation editor currently resets all user settings and the `Recent` palette list on every launch because nothing is persisted to disk. Saving user preferences and recent palette selections across sessions on both Linux and Windows eliminates repetitive daily setup.

## What Changes

- Persist desktop user preferences (`zoom`, `theme`, `font_pt`, `target`, `category`, `recent_palette`, `batch_target`, and `batch_overwrite`) in a per-user JSON configuration file resolved via the platform's standard config directory on Linux and Windows.
- Load and validate saved preferences at startup, falling back safely to defaults on missing, corrupt, or out-of-range fields without crashing.
- Persist user-initiated setting changes immediately using atomic writes while keeping transient Office (`Word`/`WPS`) session overrides (`font_pt`, `target_mode`) isolated from saved defaults.
- Update `Recent` palette tracking so recently inserted items survive across application restarts while still avoiding launching on an empty `Recent` strip.

## Capabilities

### New Capabilities
- `desktop-settings-persistence`: Cross-platform persistence, validation, and Office-session isolation for desktop application settings and preferences on Linux and Windows.

### Modified Capabilities
- `desktop-palette-navigation`: Extend `Recent category tracking` so recorded `Recent` palette items persist across application restarts while preserving default startup behavior when `Recent` is empty.

## Impact

- `desktop/src/app.rs`: Load/save settings on startup and user changes, apply persisted theme/zoom/defaults, keep headless tests isolated from the host config file, and add persistence unit tests.
- `desktop/src/palette.rs`: Helper to resolve/validate persisted `recent_palette` source strings and category IDs against `PALETTE` and `UI_GROUPS`.
- `desktop/src/` (optional `settings.rs` module): Cross-platform settings struct, path resolution (`directories::ProjectDirs`), clamping/validation, and atomic file save/load.
- No new Cargo dependencies required (`directories`, `serde`, `serde_json`, and `tempfile` are already in `desktop/Cargo.toml`).
