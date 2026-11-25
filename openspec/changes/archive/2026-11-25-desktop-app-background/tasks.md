# Tasks

## 1. Dependencies & Image Format Support

- [x] 1.1 Enable `jpeg` and `webp` features on the `image` crate in `desktop/Cargo.toml` and verify `cargo check -p eqntide-desktop` and `cargo deny check` (if installed) succeed without license or advisory violations.

## 2. Settings Persistence & Managed Background Storage

- [x] 2.1 Add `background_image` and `background_opacity` fields with sanitization/clamping (including rejecting path traversal in `background_image`) in `desktop/src/settings.rs`, and verify unit tests cover round-trip persistence, clamping, and invalid/missing JSON fallback.
- [x] 2.2 Implement managed image import/normalization (decode `PNG`/`JPEG`/`WebP`, resize long edge > 2560px, atomic write to the config directory beside `settings.json`) and cleanup on clear/replace, and verify unit tests cover import, external file deletion resilience, corrupt image rejection, and cleanup.

## 3. Theme Surface Translucency & Background Rendering

- [x] 3.1 Extend `ThemeTokens` (and/or helper methods) in `desktop/src/theme.rs` to compute theme-aware scrim and translucent `panel_bg`/`card_bg` colors from overlay opacity while preserving default opaque tokens when no background is active, and verify unit tests confirm accessible contrast in Light and Dark modes.
- [x] 3.2 Render the active background image (`Cover` UV layout math) and theme-tinted scrim onto the full viewport background layer in `desktop/src/app.rs` (`App::shell`), making `CentralPanel`, `Sidebar`, `Footer`, `Status bar`, and `full_width_card` surfaces translucent when a background is active while keeping `Canvas` and `LaTeX source` opaque via `CanvasPalette`, and verify UI tests cover rendering across Editor, Batch DOCX, Settings, and About plus missing-file fallback.

## 4. Settings UI Controls

- [x] 4.1 Add the **App background** section inside the **Appearance** card of `App::settings_ui` (`Choose image…` via `rfd::FileDialog` with non-blocking worker import, thumbnail preview + status/filename, `Overlay opacity` slider, and `Clear` button using `30px` controls and `6px`/`8px` theme radii), and verify UI tests confirm immediate updates, error reporting on invalid images, and persistence on change.

## 5. Cross-Backend Verification

- [x] 5.1 Run `cargo test -p eqntide-desktop` and manually verify the background feature in Light and Dark modes across Editor, Batch DOCX, Settings, and About (including software rendering via `EQNTIDE_GPU_PROFILE=software` or `--software` if available).
