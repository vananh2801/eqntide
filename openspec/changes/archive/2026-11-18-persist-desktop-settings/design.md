# Design

## Context

- `desktop/Cargo.toml` disables `eframe`'s built-in `persistence` feature (`default-features = false`) and already includes `directories = "6"`, `serde`, `serde_json`, and `tempfile`.
- `desktop/src/main.rs` and `desktop/src/instance.rs` already resolve per-user directories via `directories::ProjectDirs::from("com", "eqntide", "EqnTide")`.
- `App::from_parts` in `desktop/src/app.rs` initializes all state with hardcoded defaults and is called by both production startup (`App::new_with_state`) and all 59 headless unit tests (`app()`).
- Office add-in sessions (`load_next`, `load_math`, `Event::Session`) populate `App::font_pt` and `App::target` dynamically for the duration of an Office equation edit.

## Goals / Non-Goals

**Goals:**
- Persist and restore `zoom`, `theme`, `font_pt`, `target`, `category`, `recent_palette`, `batch_target`, and `batch_overwrite` across launches on Linux and Windows.
- Isolate Office-session overrides (`font_pt`, `target_mode`) from persisted user defaults by construction.
- Keep all existing unit tests isolated from host user config files while enabling real filesystem round-trip tests with temporary directories.

**Non-Goals:**
- Persisting window geometry, open equation drafts, or batch file lists across restarts.
- Enabling `eframe`'s `persistence` feature.

## Decisions

### 1. Dedicated Settings Module (`desktop/src/settings.rs`)
Introduce `Settings` (`#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]` with `#[serde(default)]`) and `ThemeMode` (`System`, `Light`, `Dark`).

- **Storage path**: `ProjectDirs::from("com", "eqntide", "EqnTide").map(|dirs| dirs.config_dir().join("settings.json"))`
  - Linux: `~/.config/eqntide/settings.json`
  - Windows: `%APPDATA%\eqntide\EqnTide\config\settings.json`
- **Alternative considered**: Enabling `eframe` `persistence`. Rejected because `eframe` writes opaque RON only on exit, cannot preserve standalone defaults when Office sessions override `font_pt`/`target`, and would not validate `recent_palette` against `palette::PALETTE`.

### 2. Validation and Safe Fallbacks
Sanitize loaded `Settings` before applying:
- `zoom`: clamp finite values to `ZOOM_RANGE` (`1.5..=4.0`), otherwise `2.0`.
- `font_pt`: clamp finite values to `6.0..=72.0` and snap to `0.5` steps, otherwise `12.0`.
- `target`: accept `"omml"` or `"mathtype"`, otherwise `"omml"`.
- `theme`: `System`, `Light`, or `Dark` (`ThemeMode` enum, default `System`).
- `recent_palette`: map each stored string through `palette::canonical_source` (matching against `palette::PALETTE`), deduplicate while preserving MRU order, and truncate to `24`.
- `category`: validate via `palette::ui_group`; if unknown, or if `Recent` with an empty `recent_palette`, fall back to `palette::DEFAULT_GROUP` (`"Fractions"`).
- `batch_target`: reset to `0` if `>= batch::TARGETS.len()`.

If `settings.json` is missing or fails to read/parse, return `Settings::default()` silently (logging via `log::warn!` if parse fails) so startup never aborts.

### 3. Atomic Persistence and Coalesced Slider Writes
- Save using `tempfile::NamedTempFile::new_in(parent)` + `persist(path)` after creating parent directories via `fs::create_dir_all(parent)`. This guarantees atomic replacement on both Linux and Windows without leaving half-written JSON files.
- Keep `settings_path: Option<PathBuf>`, `persisted: Settings`, and `settings_dirty: bool` on `App`:
  - `App::from_parts` sets `settings_path: None` so existing unit tests never touch host user files.
  - `App::new_with_state` resolves the default path, loads/sanitizes `Settings`, applies it to `App` and `egui_ctx.set_theme(...)`, and stores `settings_path`.
  - A test helper `App::with_settings_path` allows deterministic filesystem round-trip tests using `tempfile::tempdir()`.
  - When `settings_dirty` is true, flush to disk once pointer drag ends (`!ctx.input(|i| i.pointer.any_down())`) and also flush on `Event::Finished` / window close so continuous `Slider` or `DragValue` drags do not perform disk I/O every frame while discrete clicks save immediately.

### 4. Office Session Isolation via Explicit `persisted` Snapshot
- Update `self.persisted` and set `self.settings_dirty = true` **only** from user UI interactions:
  - `settings_ui`: `zoom` slider/reset, theme toggle button (and optional System reset if desired, or Light/Dark toggle updating `persisted.theme`).
  - `footer`: user-initiated `font_pt` `DragValue` change and `target` radio button change.
  - `sidebar`: clicking a category icon updates `persisted.category`.
  - `record_recent_palette`: updates `persisted.recent_palette`.
  - `batch_ui`: changing `batch_target` or `batch_overwrite`.
- Office session handlers (`load_next`, `load_math`, `Event::Session`) mutate only the live `self.font_pt` and `self.target` fields and never touch `self.persisted`.
- Note on `load_math` when `font` is `None`: keep `self.persisted.font_pt` untouched and initialize the blank/session `self.font_pt` from `font.unwrap_or(self.persisted.font_pt)` (so standalone resets don't lose the user's preferred font size either).

## Risks / Trade-offs

- **Continuous `Slider` / `DragValue` dragging** → Flushing only when `settings_dirty && !pointer_any_down` (plus on close) avoids dozens of atomic file renames per second during slider drags while persisting immediately after release.
- **Settings file permissions or read-only config directories** → Write failures are non-fatal (`log::warn!`) so editing equations is never blocked.
