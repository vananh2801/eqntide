//! Persisted desktop application settings, atomic JSON save, and managed background images.
//!
//! [`Settings`] are stored in the platform config directory (`settings.json`) and sanitized on load
//! so out-of-range numeric preferences, unknown presets, or invalid palette entries fall back
//! safely to defaults. User-selected background images are downscaled and re-encoded as managed
//! PNG files beside `settings.json` via temporary files so crashes never corrupt stored assets.

use crate::{batch, palette, theme::CanvasPreset};
use directories::ProjectDirs;
use eframe::egui;
use image::{
    codecs::png::{CompressionType, FilterType as PngFilterType, PngEncoder},
    imageops::FilterType,
    ExtendedColorType, ImageEncoder, ImageReader,
};
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::{Component, Path, PathBuf},
};

/// Allowed UI zoom range.
pub(crate) const ZOOM_RANGE: std::ops::RangeInclusive<f32> = 1.5..=4.0;
/// Allowed equation export font size range in points.
pub(crate) const FONT_PT_RANGE: std::ops::RangeInclusive<f32> = 6.0..=72.0;
/// Default UI zoom factor.
pub(crate) const DEFAULT_ZOOM: f32 = 2.0;
/// Default equation export font size in points.
pub(crate) const DEFAULT_FONT_PT: f32 = 12.0;
/// Default clipboard/document conversion target identifier.
pub(crate) const DEFAULT_TARGET: &str = "omml";
/// Maximum number of recent palette symbols persisted in settings.
pub(crate) const MAX_RECENT_PALETTE: usize = 24;
/// Default opacity for the translucent overlay over a custom background image.
pub(crate) const DEFAULT_BACKGROUND_OPACITY: f32 = 0.70;
/// Allowed range for the background overlay opacity.
pub(crate) const BACKGROUND_OPACITY_RANGE: std::ops::RangeInclusive<f32> = 0.0..=1.0;
/// Maximum edge length (in pixels) retained when importing a custom background image.
pub(crate) const MAX_BACKGROUND_EDGE: u32 = 2560;
const MANAGED_BACKGROUND_PREFIX: &str = "app-background-";

/// Background image loaded for immediate display before attaching a settings file.
pub struct PreparedBackground {
    /// Sanitized file name for the managed background asset.
    pub filename: String,
    /// Unmultiplied RGBA pixel image ready for upload to egui.
    pub image: egui::ColorImage,
}

/// Application color theme preference persisted to disk.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}

impl ThemeMode {
    /// Convert this setting into egui's theme preference.
    pub(crate) fn to_egui(self) -> egui::ThemePreference {
        match self {
            Self::System => egui::ThemePreference::System,
            Self::Light => egui::ThemePreference::Light,
            Self::Dark => egui::ThemePreference::Dark,
        }
    }

    /// Derive a concrete [`ThemeMode`] from the currently resolved egui theme.
    pub(crate) fn from_egui_theme(theme: egui::Theme) -> Self {
        match theme {
            egui::Theme::Light => Self::Light,
            egui::Theme::Dark => Self::Dark,
        }
    }
}

/// Which editor pane receives initial keyboard focus when the desktop app starts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum StartupFocus {
    #[default]
    Canvas,
    Latex,
}

impl<'de> Deserialize<'de> for StartupFocus {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = Option::<String>::deserialize(deserializer)?;
        Ok(match raw.as_deref() {
            Some("latex") => Self::Latex,
            _ => Self::Canvas,
        })
    }
}

/// Persisted user preferences and workspace defaults for the desktop application.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Settings {
    pub(crate) zoom: f32,
    pub(crate) theme: ThemeMode,
    pub(crate) canvas_light: CanvasPreset,
    pub(crate) canvas_dark: CanvasPreset,
    pub(crate) background_image: Option<String>,
    pub(crate) background_opacity: f32,
    pub(crate) startup_focus: StartupFocus,
    pub(crate) font_pt: f32,
    pub(crate) target: String,
    pub(crate) category: String,
    pub(crate) recent_palette: Vec<String>,
    pub(crate) batch_target: usize,
    pub(crate) batch_overwrite: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            zoom: DEFAULT_ZOOM,
            theme: ThemeMode::System,
            canvas_light: CanvasPreset::SoftPaper,
            canvas_dark: CanvasPreset::SoftPaper,
            background_image: None,
            background_opacity: DEFAULT_BACKGROUND_OPACITY,
            startup_focus: StartupFocus::Canvas,
            font_pt: DEFAULT_FONT_PT,
            target: DEFAULT_TARGET.into(),
            category: palette::DEFAULT_GROUP.into(),
            recent_palette: Vec::new(),
            batch_target: 0,
            batch_overwrite: false,
        }
    }
}

impl Settings {
    /// Locate the platform-standard `settings.json` path under the EqnTide config directory.
    pub(crate) fn default_path() -> Option<PathBuf> {
        ProjectDirs::from("com", "eqntide", "EqnTide")
            .map(|dirs| dirs.config_dir().join("settings.json"))
    }

    /// Clamp fields to valid ranges and normalize fallbacks by value.
    pub(crate) fn sanitized(mut self) -> Self {
        self.sanitize();
        self
    }

    /// Clamp numeric settings, normalize palette selections, and drop unsafe paths in place.
    pub(crate) fn sanitize(&mut self) {
        self.zoom = if self.zoom.is_finite() {
            self.zoom.clamp(*ZOOM_RANGE.start(), *ZOOM_RANGE.end())
        } else {
            DEFAULT_ZOOM
        };

        self.font_pt = if self.font_pt.is_finite() {
            let clamped = self
                .font_pt
                .clamp(*FONT_PT_RANGE.start(), *FONT_PT_RANGE.end());
            (clamped * 2.0).round() / 2.0
        } else {
            DEFAULT_FONT_PT
        };

        if self.target != "omml" && self.target != "mathtype" {
            self.target = DEFAULT_TARGET.into();
        }

        let mut deduped: Vec<&'static str> = Vec::with_capacity(MAX_RECENT_PALETTE);
        for source in &self.recent_palette {
            if let Some(canonical) = palette::canonical_source(source) {
                if !deduped.contains(&canonical) {
                    deduped.push(canonical);
                    if deduped.len() == MAX_RECENT_PALETTE {
                        break;
                    }
                }
            }
        }
        self.recent_palette = deduped.into_iter().map(str::to_owned).collect();

        let valid_category = palette::ui_group(&self.category).is_some()
            && (self.category != palette::RECENT_GROUP || !self.recent_palette.is_empty());
        if !valid_category {
            self.category = palette::DEFAULT_GROUP.into();
        }

        if self.batch_target >= batch::TARGETS.len() {
            self.batch_target = 0;
        }

        self.canvas_light = self.canvas_light.sanitize_for_mode(false);
        self.canvas_dark = self.canvas_dark.sanitize_for_mode(true);
        self.background_image = self
            .background_image
            .as_deref()
            .and_then(sanitize_background_filename);
        self.background_opacity = if self.background_opacity.is_finite() {
            self.background_opacity.clamp(
                *BACKGROUND_OPACITY_RANGE.start(),
                *BACKGROUND_OPACITY_RANGE.end(),
            )
        } else {
            DEFAULT_BACKGROUND_OPACITY
        };
    }

    /// Return the deduplicated list of canonical palette snippets from `recent_palette`.
    pub(crate) fn resolved_recent_palette(&self) -> Vec<&'static str> {
        let mut resolved = Vec::with_capacity(MAX_RECENT_PALETTE);
        for source in &self.recent_palette {
            if let Some(canonical) = palette::canonical_source(source) {
                if !resolved.contains(&canonical) {
                    resolved.push(canonical);
                    if resolved.len() == MAX_RECENT_PALETTE {
                        break;
                    }
                }
            }
        }
        resolved
    }

    /// Load and sanitize settings from `path`, falling back to defaults when missing or invalid.
    pub(crate) fn load_from_path(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(raw) => match serde_json::from_str::<Self>(&raw) {
                Ok(settings) => settings.sanitized(),
                Err(err) => {
                    eprintln!("Failed to parse settings {}: {err}", path.display());
                    Self::default()
                }
            },
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(err) => {
                eprintln!("Failed to read settings {}: {err}", path.display());
                Self::default()
            }
        }
    }

    /// Persist settings to `path` via an fsynced sibling temporary file.
    pub(crate) fn save_to_path(&self, path: &Path) -> std::io::Result<()> {
        let parent = config_parent_dir(path);
        std::fs::create_dir_all(parent)?;
        let mut temp = tempfile::NamedTempFile::new_in(parent)?;
        serde_json::to_writer_pretty(temp.as_file_mut(), self).map_err(std::io::Error::other)?;
        temp.as_file_mut().write_all(b"\n")?;
        temp.as_file_mut().sync_all()?;
        temp.persist(path).map_err(|err| err.error)?;
        Ok(())
    }
}

/// Newly imported managed background PNG and its decoded RGBA dimensions.
pub(crate) struct ImportedBackground {
    pub(crate) filename: String,
    pub(crate) rgba: Vec<u8>,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

/// Decoded RGBA pixels from an existing managed background PNG.
pub(crate) struct LoadedBackground {
    pub(crate) rgba: Vec<u8>,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

/// Accept only single-component filenames without directory separators or traversal.
pub(crate) fn sanitize_background_filename(name: &str) -> Option<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }
    let path = Path::new(trimmed);
    let mut components = path.components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(part)), None) => {
            let text = part.to_str()?;
            if text.contains('/') || text.contains('\\') {
                None
            } else {
                Some(text.to_owned())
            }
        }
        _ => None,
    }
}

/// Resolve a sanitized managed background file name inside the directory containing `settings_path`.
pub(crate) fn resolve_managed_background_path(
    settings_path: &Path,
    filename: &str,
) -> Option<PathBuf> {
    let clean = sanitize_background_filename(filename)?;
    Some(config_parent_dir(settings_path).join(clean))
}

/// Best-effort deletion of a managed background PNG file beside `settings_path`.
pub(crate) fn remove_managed_background_file(settings_path: &Path, filename: &str) {
    if let Some(path) = resolve_managed_background_path(settings_path, filename) {
        let _ = std::fs::remove_file(path);
    }
}

/// Decode, resize, and atomically persist a background image into the config directory.
pub(crate) fn import_background_image(
    source_path: &Path,
    settings_path: &Path,
    previous_filename: Option<&str>,
) -> Result<ImportedBackground, String> {
    let reader = ImageReader::open(source_path)
        .map_err(|err| format!("Could not open background image: {err}"))?
        .with_guessed_format()
        .map_err(|err| format!("Could not read background image format: {err}"))?;
    let mut img = reader
        .decode()
        .map_err(|err| format!("Could not decode background image: {err}"))?;
    if img.width().max(img.height()) > MAX_BACKGROUND_EDGE {
        img = img.resize(
            MAX_BACKGROUND_EDGE,
            MAX_BACKGROUND_EDGE,
            FilterType::Triangle,
        );
    }
    let has_alpha = img.color().has_alpha();
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    if width == 0 || height == 0 {
        return Err("Background image has zero dimensions.".into());
    }

    let parent = config_parent_dir(settings_path);
    std::fs::create_dir_all(parent)
        .map_err(|err| format!("Could not create config directory: {err}"))?;
    let filename = format!("{MANAGED_BACKGROUND_PREFIX}{}.png", uuid::Uuid::new_v4());
    let dest_path = parent.join(&filename);
    let mut temp = tempfile::NamedTempFile::new_in(parent)
        .map_err(|err| format!("Could not create temporary background file: {err}"))?;
    {
        let encoder = PngEncoder::new_with_quality(
            temp.as_file_mut(),
            CompressionType::Fast,
            PngFilterType::Sub,
        );
        if has_alpha {
            encoder
                .write_image(rgba.as_raw(), width, height, ExtendedColorType::Rgba8)
                .map_err(|err| format!("Could not encode background image: {err}"))?;
        } else {
            let rgb = image::DynamicImage::ImageRgba8(rgba.clone()).into_rgb8();
            encoder
                .write_image(rgb.as_raw(), width, height, ExtendedColorType::Rgb8)
                .map_err(|err| format!("Could not encode background image: {err}"))?;
        }
    }
    temp.as_file_mut()
        .sync_all()
        .map_err(|err| format!("Could not flush background image: {err}"))?;
    temp.persist(&dest_path)
        .map_err(|err| format!("Could not save background image: {}", err.error))?;

    if let Some(prev) = previous_filename {
        if prev != filename {
            remove_managed_background_file(settings_path, prev);
        }
    }

    Ok(ImportedBackground {
        filename,
        rgba: rgba.into_raw(),
        width,
        height,
    })
}

/// Prepare a background image asynchronously, persisting a managed PNG when `settings_path` is known.
pub(crate) fn prepare_background_import(
    source_path: &Path,
    settings_path: Option<&Path>,
) -> Result<PreparedBackground, String> {
    if let Some(settings_path) = settings_path {
        let imported = import_background_image(source_path, settings_path, None)?;
        let image = egui::ColorImage::from_rgba_unmultiplied(
            [imported.width as usize, imported.height as usize],
            &imported.rgba,
        );
        return Ok(PreparedBackground {
            filename: imported.filename,
            image,
        });
    }

    let reader = ImageReader::open(source_path)
        .map_err(|err| format!("Could not open background image: {err}"))?
        .with_guessed_format()
        .map_err(|err| format!("Could not read background image format: {err}"))?;
    let mut img = reader
        .decode()
        .map_err(|err| format!("Could not decode background image: {err}"))?;
    if img.width().max(img.height()) > MAX_BACKGROUND_EDGE {
        img = img.resize(
            MAX_BACKGROUND_EDGE,
            MAX_BACKGROUND_EDGE,
            FilterType::Triangle,
        );
    }
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    if width == 0 || height == 0 {
        return Err("Background image has zero dimensions.".into());
    }
    let filename = source_path
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(sanitize_background_filename)
        .unwrap_or_else(|| "app-background.png".into());
    let image =
        egui::ColorImage::from_rgba_unmultiplied([width as usize, height as usize], rgba.as_raw());
    Ok(PreparedBackground { filename, image })
}

/// Decode a previously imported managed background PNG from beside `settings_path`.
pub(crate) fn load_managed_background(
    settings_path: &Path,
    filename: &str,
) -> Result<LoadedBackground, String> {
    let path = resolve_managed_background_path(settings_path, filename)
        .ok_or_else(|| "Invalid managed background filename.".to_string())?;
    let reader = ImageReader::open(&path)
        .map_err(|err| format!("Could not open managed background: {err}"))?
        .with_guessed_format()
        .map_err(|err| format!("Could not detect managed background format: {err}"))?;
    let img = reader
        .decode()
        .map_err(|err| format!("Could not decode managed background: {err}"))?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    if width == 0 || height == 0 {
        return Err("Managed background image has zero dimensions.".into());
    }
    Ok(LoadedBackground {
        rgba: rgba.into_raw(),
        width,
        height,
    })
}

fn config_parent_dir(settings_path: &Path) -> &Path {
    settings_path
        .parent()
        .filter(|parent_path| !parent_path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageFormat, Rgba, RgbaImage};

    #[test]
    fn default_path_points_to_settings_json_in_eqntide_config_dir() {
        let path = Settings::default_path().expect("platform config directory");
        assert_eq!(
            path.file_name().and_then(|name| name.to_str()),
            Some("settings.json")
        );
        let expected = ProjectDirs::from("com", "eqntide", "EqnTide")
            .unwrap()
            .config_dir()
            .join("settings.json");
        assert_eq!(path, expected);
    }

    #[test]
    fn missing_or_corrupt_json_falls_back_to_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing.json");
        assert_eq!(Settings::load_from_path(&missing), Settings::default());

        let corrupt = dir.path().join("corrupt.json");
        std::fs::write(&corrupt, "{ not valid json").unwrap();
        assert_eq!(Settings::load_from_path(&corrupt), Settings::default());
    }

    #[test]
    fn sanitize_clamps_values_deduplicates_recent_and_falls_back_safely() {
        let raw = Settings {
            zoom: 10.0,
            theme: ThemeMode::Dark,
            canvas_light: CanvasPreset::DimSlate,
            canvas_dark: CanvasPreset::PureWhite,
            background_image: Some("../escape.png".into()),
            background_opacity: 1.5,
            startup_focus: StartupFocus::Latex,
            font_pt: 81.3,
            target: "latex".into(),
            category: "DoesNotExist".into(),
            recent_palette: vec![
                r"\alpha".into(),
                r"\unknown_symbol".into(),
                r"\alpha".into(),
                r"\frac{#?}{#?}".into(),
            ],
            batch_target: 99,
            batch_overwrite: true,
        }
        .sanitized();

        assert_eq!(raw.zoom, *ZOOM_RANGE.end());
        assert_eq!(raw.theme, ThemeMode::Dark);
        assert_eq!(raw.canvas_light, CanvasPreset::SoftPaper);
        assert_eq!(raw.canvas_dark, CanvasPreset::SoftPaper);
        assert_eq!(raw.background_image, None);
        assert_eq!(raw.background_opacity, 1.0);
        assert_eq!(raw.font_pt, *FONT_PT_RANGE.end());
        assert_eq!(raw.target, "omml");
        assert_eq!(raw.category, palette::DEFAULT_GROUP);
        assert_eq!(
            raw.recent_palette,
            vec![r"\alpha".to_string(), r"\frac{#?}{#?}".to_string()]
        );
        assert_eq!(raw.batch_target, 0);
        assert!(raw.batch_overwrite);

        let non_finite = Settings {
            zoom: f32::NAN,
            font_pt: f32::INFINITY,
            background_opacity: f32::NAN,
            ..Settings::default()
        }
        .sanitized();
        assert_eq!(non_finite.zoom, DEFAULT_ZOOM);
        assert_eq!(non_finite.font_pt, DEFAULT_FONT_PT);
        assert_eq!(non_finite.background_opacity, DEFAULT_BACKGROUND_OPACITY);

        let snapped_font = Settings {
            font_pt: 13.3,
            zoom: 0.5,
            background_opacity: -0.2,
            background_image: Some("nested/bg.png".into()),
            ..Settings::default()
        }
        .sanitized();
        assert_eq!(snapped_font.font_pt, 13.5);
        assert_eq!(snapped_font.zoom, *ZOOM_RANGE.start());
        assert_eq!(snapped_font.background_opacity, 0.0);
        assert_eq!(snapped_font.background_image, None);
    }

    #[test]
    fn empty_or_invalid_recent_category_falls_back_to_default_group() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(
            &path,
            r#"{
                "category": "Recent",
                "recent_palette": ["\\not_in_palette"]
            }"#,
        )
        .unwrap();

        let loaded = Settings::load_from_path(&path);
        assert!(loaded.recent_palette.is_empty());
        assert_eq!(loaded.category, palette::DEFAULT_GROUP);
    }

    #[test]
    fn atomic_round_trip_saves_and_reloads_all_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("settings.json");

        let original = Settings {
            zoom: 3.25,
            theme: ThemeMode::Light,
            canvas_light: CanvasPreset::CoolSlate,
            canvas_dark: CanvasPreset::DimSlate,
            background_image: Some("app-background-123.png".into()),
            background_opacity: 0.55,
            startup_focus: StartupFocus::Latex,
            font_pt: 16.5,
            target: "mathtype".into(),
            category: palette::RECENT_GROUP.into(),
            recent_palette: vec![r"\frac{#?}{#?}".into(), r"\alpha".into()],
            batch_target: 2,
            batch_overwrite: true,
        };
        original.save_to_path(&path).unwrap();

        let loaded = Settings::load_from_path(&path);
        assert_eq!(loaded, original);

        // No leftover temporary files remain beside settings.json.
        let entries: Vec<_> = std::fs::read_dir(path.parent().unwrap())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(entries, vec!["settings.json"]);
    }

    #[test]
    fn unknown_canvas_presets_in_json_fall_back_without_discarding_other_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(
            &path,
            r#"{
                "zoom": 2.5,
                "canvas_light": "neon-green",
                "canvas_dark": "pure-white",
                "background_fit": "tile",
                "background_opacity": 0.42
            }"#,
        )
        .unwrap();

        let loaded = Settings::load_from_path(&path);
        assert_eq!(loaded.zoom, 2.5);
        assert_eq!(loaded.canvas_light, CanvasPreset::SoftPaper);
        assert_eq!(loaded.canvas_dark, CanvasPreset::SoftPaper);
        assert!((loaded.background_opacity - 0.42).abs() < 1e-6);
        assert_eq!(loaded.startup_focus, StartupFocus::Canvas);
    }

    #[test]
    fn unknown_startup_focus_in_json_falls_back_to_canvas() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(
            &path,
            r#"{
                "zoom": 3.0,
                "startup_focus": "sidebar"
            }"#,
        )
        .unwrap();

        let loaded = Settings::load_from_path(&path);
        assert_eq!(loaded.zoom, 3.0);
        assert_eq!(loaded.startup_focus, StartupFocus::Canvas);
    }

    #[test]
    fn managed_background_import_supports_formats_resizes_survives_source_delete_and_cleans_up() {
        let dir = tempfile::tempdir().unwrap();
        let settings_path = dir.path().join("cfg").join("settings.json");

        // Import oversized JPEG (long edge > 2560) and verify proportional downscale + JPEG decode.
        let src_jpg = dir.path().join("source.jpg");
        let large = RgbaImage::from_pixel(3200, 1600, Rgba([40, 120, 200, 255]));
        image::DynamicImage::ImageRgba8(large)
            .to_rgb8()
            .save_with_format(&src_jpg, ImageFormat::Jpeg)
            .unwrap();

        let first = import_background_image(&src_jpg, &settings_path, None).unwrap();
        let first_path = resolve_managed_background_path(&settings_path, &first.filename).unwrap();
        assert_eq!(first.width, MAX_BACKGROUND_EDGE);
        assert_eq!(first.height, MAX_BACKGROUND_EDGE / 2);
        assert!(first_path.exists());

        // Deleting external source does not break loading the managed copy.
        std::fs::remove_file(&src_jpg).unwrap();
        let loaded = load_managed_background(&settings_path, &first.filename).unwrap();
        assert_eq!(loaded.width, first.width);
        assert_eq!(loaded.height, first.height);

        // Import a WebP image and verify the previous managed PNG is cleaned up.
        let src_webp = dir.path().join("second.webp");
        let small = RgbaImage::from_pixel(64, 48, Rgba([220, 80, 60, 255]));
        image::DynamicImage::ImageRgba8(small)
            .save_with_format(&src_webp, ImageFormat::WebP)
            .unwrap();
        let second =
            import_background_image(&src_webp, &settings_path, Some(&first.filename)).unwrap();
        let second_path =
            resolve_managed_background_path(&settings_path, &second.filename).unwrap();
        assert_eq!(second.width, 64);
        assert_eq!(second.height, 48);
        assert!(!first_path.exists());
        assert!(second_path.exists());

        // Corrupt image is rejected without removing the active managed file.
        let corrupt = dir.path().join("broken.png");
        std::fs::write(&corrupt, b"not a valid image").unwrap();
        assert!(import_background_image(&corrupt, &settings_path, Some(&second.filename)).is_err());
        assert!(second_path.exists());

        // Clearing removes the managed file.
        remove_managed_background_file(&settings_path, &second.filename);
        assert!(!second_path.exists());
    }
}
