# Design

## Context

See `proposal.md` for motivation and scope. Currently in [app.rs](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/src/app.rs) and [theme.rs](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/src/theme.rs):

- [`App::shell`](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/src/app.rs#L2234-L2332) paints opaque fills on `app-status` (`panel_bg`), `app-sidebar` (`panel_bg`), `app-footer` (`panel_bg`), and `CentralPanel` (`app_bg`).
- Secondary views (**Batch DOCX**, **Settings**, **About**) wrap sections in `full_width_card` filled with `card_bg`.
- The **Equation Canvas** and **LaTeX source** box paint using [`CanvasPalette`](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/src/theme.rs#L254-L269), which is selected independently for Light and Dark modes.
- Desktop renders via `wgpu`, `glow`, or the CPU-based [`SoftwarePainter`](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/src/software_painter.rs).
- [settings.rs](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/src/settings.rs) stores `settings.json` atomically inside `ProjectDirs::from("com", "eqntide", "EqnTide").config_dir()`.

## Goals / Non-Goals

**Goals:**
- Render a user-selected background image across the entire desktop viewport (visible across Editor, Batch DOCX, Settings, and About).
- Keep UI labels, Tabler icons, palette glyphs, and especially equations on the **Equation Canvas** and **LaTeX source** box consistently legible in both Light and Dark modes.
- Ensure imported background images survive external file deletion/moves and decode quickly on startup, including under `SoftwarePainter` and `VirtualGpu`.

**Non-Goals:**
- Per-mode (separate Light/Dark) background image slots or multi-image slideshows/galleries.
- Animated GIF/video wallpapers or real-time Gaussian blur shaders (which would penalize `SoftwarePainter` CPU rendering).
- Making the **Equation Canvas** or **LaTeX source** surface transparent by default.

## Decisions

### 1. Full-window background layer + translucent chrome/cards, opaque Equation Canvas
- **Decision**: When a custom background is active, paint the background image + theme-tinted scrim onto `egui::LayerId::background()` covering the full viewport (`ui.ctx().content_rect()`) before panels render (or directly via [`SoftwarePainter`](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/src/software_painter.rs) when running the CPU software backend), make `app_bg` transparent (since the scrim already sits over the image), and derive translucent `panel_bg` and `card_bg` from the configured `background_opacity` (`0.0..=1.0`). Keep `CanvasPalette::bg` opaque inside [`App::canvas`](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/src/app.rs#L2495-L2507) and [`App::latex_ui`](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/src/app.rs#L2445-L2452).
- **Rationale**: In `egui`, `Panel` and `CentralPanel` fills are drawn sequentially; painting onto the background layer guarantees the image spans behind `app-status`, `app-sidebar`, `app-footer`, and `CentralPanel` uniformly across all tabs. Keeping `CanvasPalette` opaque preserves high contrast for math ink, placeholders, selection highlights, and caret regardless of image brightness or Light/Dark mode mismatch (e.g., `SoftPaper` in Dark mode).

### 2. Managed image import into config directory with max-dimension normalization
- **Decision**: When the user picks an image via `rfd::FileDialog`, decode it with `image`, resize proportionally using `FilterType::Triangle` only if `max(width, height) > 2560`, encode as fast PNG (`CompressionType::Fast`, `PngFilterType::Sub`) to a managed file inside the parent directory of `settings_path` (written atomically via `tempfile::NamedTempFile` and persisted in `Settings::background_image: Option<String>`), and upload an `egui::TextureHandle` (plus `SoftwareBackgroundImage` when on `--software`). Store only a safe relative filename (no path traversal) in `settings.json`.
- **Rationale**: Protects against broken backgrounds when users clean up `Downloads`, avoids repeating expensive full-resolution decode on every launch, bounds texture memory for `SoftwarePainter`, and keeps `settings.json` clean and cross-platform.

### 3. Cover UV layout math, non-blocking background import, & multi-row Settings layout
- **Decision**: Always render the background image using aspect-preserving `Cover` UV cropping centered in `[0..1, 0..1]` without a redundant `Fit` selector. Run image decoding, resizing, fast PNG encoding, and `ColorImage` construction on a worker thread with sequence tracking (`background_import_seq`) so the UI never freezes. In Settings **Appearance**, stack **App background** across two rows (rounded `72×48` thumbnail beside `Choose image…`, `Clear`, and left-aligned status text on row 1; `Overlay opacity` slider + `%` on row 2) and keep **Light mode canvas** and **Dark mode canvas** stacked vertically below.
- **Rationale**: Non-blocking worker import avoids multi-second UI freezes on large files, `Cover`-only framing simplifies Settings, and multi-row layout prevents horizontal crowding.

### 4. Fast native background blit in `SoftwarePainter` & `opt-level = 3` for `hdrshot-egui-backend`
- **Decision**: On `--software`, bypass `egui_software_backend`'s per-pixel barycentric/bilinear fallback (`draw_tri` + `sample_bilinear`) for the full-screen background quad by caching a `Cover`-cropped, LUT-scrimmed `0RGB` framebuffer inside [`SoftwarePainter`](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/src/software_painter.rs) and copying it directly into the `softbuffer` slice before rendering foreground UI primitives, and enable `opt-level = 3` for `hdrshot-egui-backend` in `[profile.dev.package.hdrshot-egui-backend]`.
- **Rationale**: Eliminates single-threaded CPU bottlenecks during window resize under `--software`.

### 5. Image format support & `deny.toml` compliance
- **Decision**: Enable `jpeg` and `webp` features on `image` in [desktop/Cargo.toml](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/Cargo.toml#L22) (in addition to `png`) and verify `cargo deny check` passes against [deny.toml](file:///home/vananh2801/Documents/mathtype-convert/eqntide/deny.toml).

## Risks / Trade-offs

- **Low text contrast at low overlay opacity** → Default `background_opacity` to `0.70` and keep `panel_bg`/`card_bg` alpha floored (`0.38..=0.92`) so text and icons remain legible even at minimum slider position.
- **CPU / memory cost on `SoftwarePainter`** → Resize imported images above `2560px` on the long edge during import, cache the decoded texture / `SoftwareBackgroundImage` once in `App`, and cache the viewport-sized `0RGB` cover+scrim buffer in `SoftwarePainter`.
- **Orphaned or missing managed files / path traversal in `settings.json`** → Sanitize `background_image` so only a bare filename (single `Component::Normal`) is accepted, and treat missing/unreadable files as "no active background" without mutating unrelated settings.
- **Test isolation** → Tests using `Settings::with_settings_path` in temporary directories naturally isolate managed background files alongside each test's temporary `settings.json`.
