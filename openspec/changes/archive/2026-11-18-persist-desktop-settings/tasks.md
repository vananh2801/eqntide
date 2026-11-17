# Tasks

## 1. Settings Storage & Validation Module

- [x] 1.1 Add `palette::canonical_source` in `desktop/src/palette.rs` to resolve stored LaTeX source strings against `PALETTE`, and add unit tests verifying valid, unknown, and duplicate resolution.
- [x] 1.2 Create `desktop/src/settings.rs` (`mod settings;` in `desktop/src/main.rs`) defining `ThemeMode`, `Settings`, platform path resolution via `ProjectDirs::from("com", "eqntide", "EqnTide").config_dir().join("settings.json")`, value sanitization/clamping, and atomic `save_to_path` / `load_from_path` using `tempfile::NamedTempFile`; verify with unit tests covering missing/corrupt JSON, clamping/fallbacks, empty `Recent` fallback, and atomic round-trip persistence.

## 2. App Integration & Office Isolation

- [x] 2.1 Wire `Settings` into `App` (`desktop/src/app.rs`) so production startup (`App::new_with_state`) loads persisted settings and applies theme/zoom/defaults/recent items while keeping `App::from_parts` isolated for existing unit tests, and add a helper `with_settings_path` for filesystem round-trip tests; verify with a unit test restoring persisted `zoom`, `theme`, `font_pt`, `target`, `category`, `recent_palette`, `batch_target`, and `batch_overwrite` on a new `App` instance.
- [x] 2.2 Mark `settings_dirty` and update `persisted` only on user UI actions (`zoom` slider/reset, theme toggle, footer `font_pt`/`target` changes, sidebar category click, palette item click, and batch target/overwrite changes) and flush atomically once pointer drag releases and on close/finish; add and pass unit tests verifying Office session overrides (`Event::Session` / `load_math`) do not overwrite saved `font_pt`/`target` and user edits persist cleanly.

## 3. Workspace Verification

- [x] 3.1 Run `cargo fmt --check`, `cargo clippy -p eqntide-desktop --all-targets -- -D warnings`, `cargo test -p eqntide-desktop`, and `cargo check --workspace` to confirm clean formatting, lints, and full desktop test pass.
