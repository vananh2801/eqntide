//! Persisted desktop application settings, atomic JSON save, and managed background images.
//!
//! [`Settings`] are stored in the platform config directory (`settings.json`) and sanitized on load
//! so out-of-range numeric preferences or invalid palette entries fall back
//! safely to defaults. User-selected background images are downscaled and re-encoded as managed
//! PNG files beside `settings.json` via temporary files so crashes never corrupt stored assets.

use crate::{batch, palette, shortcuts, theme};
use directories::ProjectDirs;
use eframe::egui;
use image::{
    codecs::png::{CompressionType, FilterType as PngFilterType, PngEncoder},
    imageops::FilterType,
    ExtendedColorType, ImageEncoder, ImageReader,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
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
/// Default opacity for the Equation Canvas and LaTeX source box over a custom background image.
pub(crate) const DEFAULT_CANVAS_OPACITY: f32 = 1.0;
/// Allowed range for the Equation Canvas and LaTeX source box opacity.
pub(crate) const CANVAS_OPACITY_RANGE: std::ops::RangeInclusive<f32> = 0.0..=1.0;
/// Maximum edge length (in pixels) retained when importing a custom background image.
pub(crate) const MAX_BACKGROUND_EDGE: u32 = 2560;
const MANAGED_BACKGROUND_PREFIX: &str = "app-background-";

/// Background image loaded for immediate display before attaching a settings file.
pub struct PreparedBackground {
    /// Sanitized file name for the managed background asset.
    pub filename: String,
    /// Unmultiplied RGBA pixel image ready for upload to egui.
    pub image: egui::ColorImage,
    /// Dominant chromatic accent RGB seed extracted from the image, if chromatic.
    pub accent: Option<[u8; 3]>,
}

/// Newly imported managed background PNG, decoded RGBA dimensions, and extracted accent seed.
pub(crate) struct ImportedBackground {
    /// Managed PNG filename stored beside `settings.json`.
    pub(crate) filename: String,
    /// Decoded unmultiplied RGBA pixel buffer.
    pub(crate) rgba: Vec<u8>,
    /// Decoded image width in pixels.
    pub(crate) width: u32,
    /// Decoded image height in pixels.
    pub(crate) height: u32,
    /// Dominant chromatic accent RGB seed extracted from the image, if chromatic.
    pub(crate) accent: Option<[u8; 3]>,
}

/// Decoded RGBA pixels from an existing managed background PNG plus its extracted accent seed.
pub(crate) struct LoadedBackground {
    /// Decoded unmultiplied RGBA pixel buffer.
    pub(crate) rgba: Vec<u8>,
    /// Decoded image width in pixels.
    pub(crate) width: u32,
    /// Decoded image height in pixels.
    pub(crate) height: u32,
    /// Dominant chromatic accent RGB seed extracted from the image, if chromatic.
    pub(crate) accent: Option<[u8; 3]>,
}

/// Application background display mode persisted to disk.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum BackgroundMode {
    /// Standard opaque Ayu Light/Dark neutral background.
    #[default]
    Solid,
    /// Custom imported wallpaper image with adjustable overlay scrim.
    Image,
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
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct Settings {
    pub(crate) zoom: f32,
    pub(crate) theme: ThemeMode,
    /// Active application background display mode (`solid` or `image`).
    pub(crate) background_mode: BackgroundMode,
    pub(crate) background_image: Option<String>,
    pub(crate) background_opacity: f32,
    /// Opacity applied to Equation Canvas and LaTeX source surfaces over a custom background image.
    pub(crate) canvas_opacity: f32,
    /// Whether UI and canvas accents adapt to the imported background image seed.
    pub(crate) background_accent_enabled: bool,
    /// Cached RGB accent seed extracted from the managed background image.
    pub(crate) background_accent: Option<[u8; 3]>,
    pub(crate) startup_focus: StartupFocus,
    pub(crate) font_pt: f32,
    pub(crate) target: String,
    pub(crate) category: String,
    pub(crate) recent_palette: Vec<String>,
    /// Sparse overrides for customizable palette keyboard shortcuts (`source -> Some(chord) | None`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) palette_shortcuts: BTreeMap<String, Option<String>>,
    pub(crate) batch_target: usize,
    pub(crate) batch_overwrite: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            zoom: DEFAULT_ZOOM,
            theme: ThemeMode::System,
            background_mode: BackgroundMode::Solid,
            background_image: None,
            background_opacity: DEFAULT_BACKGROUND_OPACITY,
            canvas_opacity: DEFAULT_CANVAS_OPACITY,
            background_accent_enabled: true,
            background_accent: None,
            startup_focus: StartupFocus::Canvas,
            font_pt: DEFAULT_FONT_PT,
            target: DEFAULT_TARGET.into(),
            category: palette::DEFAULT_GROUP.into(),
            recent_palette: Vec::new(),
            palette_shortcuts: BTreeMap::new(),
            batch_target: 0,
            batch_overwrite: false,
        }
    }
}

impl<'de> Deserialize<'de> for Settings {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(default)]
        struct RawSettings {
            zoom: f32,
            theme: ThemeMode,
            #[serde(deserialize_with = "deserialize_optional_background_mode")]
            background_mode: Option<BackgroundMode>,
            background_image: Option<String>,
            background_opacity: f32,
            canvas_opacity: f32,
            background_accent_enabled: bool,
            background_accent: Option<[u8; 3]>,
            startup_focus: StartupFocus,
            font_pt: f32,
            target: String,
            category: String,
            recent_palette: Vec<String>,
            #[serde(deserialize_with = "deserialize_palette_shortcuts")]
            palette_shortcuts: BTreeMap<String, Option<String>>,
            batch_target: usize,
            batch_overwrite: bool,
        }

        impl Default for RawSettings {
            fn default() -> Self {
                let defaults = Settings::default();
                Self {
                    zoom: defaults.zoom,
                    theme: defaults.theme,
                    background_mode: None,
                    background_image: defaults.background_image,
                    background_opacity: defaults.background_opacity,
                    canvas_opacity: defaults.canvas_opacity,
                    background_accent_enabled: defaults.background_accent_enabled,
                    background_accent: defaults.background_accent,
                    startup_focus: defaults.startup_focus,
                    font_pt: defaults.font_pt,
                    target: defaults.target,
                    category: defaults.category,
                    recent_palette: defaults.recent_palette,
                    palette_shortcuts: defaults.palette_shortcuts,
                    batch_target: defaults.batch_target,
                    batch_overwrite: defaults.batch_overwrite,
                }
            }
        }

        let raw = RawSettings::deserialize(deserializer)?;
        let sanitized_bg = raw
            .background_image
            .as_deref()
            .and_then(sanitize_background_filename);
        let background_mode = match raw.background_mode {
            Some(mode) => mode,
            None => {
                if sanitized_bg.is_some() {
                    BackgroundMode::Image
                } else {
                    BackgroundMode::Solid
                }
            }
        };

        Ok(Self {
            zoom: raw.zoom,
            theme: raw.theme,
            background_mode,
            background_image: sanitized_bg,
            background_opacity: raw.background_opacity,
            canvas_opacity: raw.canvas_opacity,
            background_accent_enabled: raw.background_accent_enabled,
            background_accent: raw.background_accent,
            startup_focus: raw.startup_focus,
            font_pt: raw.font_pt,
            target: raw.target,
            category: raw.category,
            recent_palette: raw.recent_palette,
            palette_shortcuts: raw.palette_shortcuts,
            batch_target: raw.batch_target,
            batch_overwrite: raw.batch_overwrite,
        })
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
            let clamped = self.zoom.clamp(*ZOOM_RANGE.start(), *ZOOM_RANGE.end());
            (clamped * 100.0).round() / 100.0
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

        self.palette_shortcuts = shortcuts::sanitize_overrides(&self.palette_shortcuts);

        let valid_category = palette::ui_group(&self.category).is_some()
            && (self.category != palette::RECENT_GROUP || !self.recent_palette.is_empty());
        if !valid_category {
            self.category = palette::DEFAULT_GROUP.into();
        }

        if self.batch_target >= batch::TARGETS.len() {
            self.batch_target = 0;
        }

        self.background_image = self
            .background_image
            .as_deref()
            .and_then(sanitize_background_filename);
        if self.background_image.is_none() {
            self.background_mode = BackgroundMode::Solid;
            self.background_accent = None;
        }
        self.background_opacity = if self.background_opacity.is_finite() {
            self.background_opacity.clamp(
                *BACKGROUND_OPACITY_RANGE.start(),
                *BACKGROUND_OPACITY_RANGE.end(),
            )
        } else {
            DEFAULT_BACKGROUND_OPACITY
        };
        self.canvas_opacity = if self.canvas_opacity.is_finite() {
            self.canvas_opacity
                .clamp(*CANVAS_OPACITY_RANGE.start(), *CANVAS_OPACITY_RANGE.end())
        } else {
            DEFAULT_CANVAS_OPACITY
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

/// Extract a dominant chromatic RGB accent seed from an RGBA image, or `None` if grayscale.
pub(crate) fn extract_background_accent(rgba: &[u8], width: u32, height: u32) -> Option<[u8; 3]> {
    if width == 0 || height == 0 || rgba.len() < (width as usize) * (height as usize) * 4 {
        return None;
    }
    const BUCKETS: usize = 36;
    let mut bucket_weights = [0.0_f32; BUCKETS];
    let mut bucket_rgb = [[0.0_f32; 3]; BUCKETS];

    let step_x = (width / 64).max(1) as usize;
    let step_y = (height / 64).max(1) as usize;
    let stride_w = width as usize;

    for y in (0..height as usize).step_by(step_y) {
        for x in (0..stride_w).step_by(step_x) {
            let offset = (y * stride_w + x) * 4;
            let alpha = rgba[offset + 3];
            if alpha < 128 {
                continue;
            }
            let r = rgba[offset];
            let g = rgba[offset + 1];
            let b = rgba[offset + 2];
            let (hue, sat, lum) = theme::rgb_to_hsl(r, g, b);
            if sat < 0.14 || lum < 0.08 || lum > 0.92 {
                continue;
            }
            let lum_balance = 1.0 - (2.0 * lum - 1.0).abs() * 0.5;
            let weight = sat * sat * lum_balance * (alpha as f32 / 255.0);
            let bucket = ((hue / 10.0).floor() as usize) % BUCKETS;
            bucket_weights[bucket] += weight;
            bucket_rgb[bucket][0] += r as f32 * weight;
            bucket_rgb[bucket][1] += g as f32 * weight;
            bucket_rgb[bucket][2] += b as f32 * weight;
        }
    }

    let mut best_bucket = 0;
    let mut best_score = 0.0_f32;
    for bucket in 0..BUCKETS {
        let prev = (bucket + BUCKETS - 1) % BUCKETS;
        let next = (bucket + 1) % BUCKETS;
        let smoothed =
            bucket_weights[bucket] + 0.35 * (bucket_weights[prev] + bucket_weights[next]);
        if smoothed > best_score {
            best_score = smoothed;
            best_bucket = bucket;
        }
    }

    if best_score < 0.05 {
        return None;
    }

    let prev = (best_bucket + BUCKETS - 1) % BUCKETS;
    let next = (best_bucket + 1) % BUCKETS;
    let mut total_w = 0.0_f32;
    let mut sum_rgb = [0.0_f32; 3];
    for idx in [prev, best_bucket, next] {
        let neighbor_scale = if idx == best_bucket { 1.0 } else { 0.5 };
        total_w += bucket_weights[idx] * neighbor_scale;
        for channel in 0..3 {
            sum_rgb[channel] += bucket_rgb[idx][channel] * neighbor_scale;
        }
    }
    if total_w <= 1e-6 {
        return None;
    }

    Some([
        (sum_rgb[0] / total_w).round().clamp(0.0, 255.0) as u8,
        (sum_rgb[1] / total_w).round().clamp(0.0, 255.0) as u8,
        (sum_rgb[2] / total_w).round().clamp(0.0, 255.0) as u8,
    ])
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
    let accent = extract_background_accent(rgba.as_raw(), width, height);

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
        accent,
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
            accent: imported.accent,
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
    let accent = extract_background_accent(rgba.as_raw(), width, height);
    let filename = source_path
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(sanitize_background_filename)
        .unwrap_or_else(|| "app-background.png".into());
    let image =
        egui::ColorImage::from_rgba_unmultiplied([width as usize, height as usize], rgba.as_raw());
    Ok(PreparedBackground {
        filename,
        image,
        accent,
    })
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
    let accent = extract_background_accent(rgba.as_raw(), width, height);
    Ok(LoadedBackground {
        rgba: rgba.into_raw(),
        width,
        height,
        accent,
    })
}

fn deserialize_optional_background_mode<'de, D>(
    deserializer: D,
) -> Result<Option<BackgroundMode>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = Option::<String>::deserialize(deserializer)?;
    Ok(match raw.as_deref() {
        Some("solid") => Some(BackgroundMode::Solid),
        Some("image") => Some(BackgroundMode::Image),
        _ => None,
    })
}

fn deserialize_palette_shortcuts<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<String, Option<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<BTreeMap<String, Option<String>>>::deserialize(deserializer)?.unwrap_or_default())
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
        let mut palette_shortcuts = BTreeMap::new();
        palette_shortcuts.insert(r"\frac{#?}{#?}".into(), Some("Ctrl+Shift+F".into()));
        palette_shortcuts.insert(r"\overrightarrow{#?}".into(), Some("Ctrl+Alt+V".into()));
        palette_shortcuts.insert(r"\not_in_palette".into(), Some("Ctrl+K".into()));
        palette_shortcuts.insert(r"\binom{#?}{#?}".into(), Some("S".into()));
        palette_shortcuts.insert(r"\alpha".into(), Some("Ctrl+S".into()));
        palette_shortcuts.insert(r"\beta".into(), Some("Ctrl+Alt+V".into()));
        palette_shortcuts.insert(r"\gamma".into(), None);

        let raw = Settings {
            zoom: 10.0,
            theme: ThemeMode::Dark,
            background_mode: BackgroundMode::Image,
            background_image: Some("../escape.png".into()),
            background_opacity: 1.5,
            canvas_opacity: 1.8,
            background_accent_enabled: false,
            background_accent: Some([40, 140, 220]),
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
            palette_shortcuts,
            batch_target: 99,
            batch_overwrite: true,
        }
        .sanitized();

        assert_eq!(raw.zoom, *ZOOM_RANGE.end());
        assert_eq!(raw.theme, ThemeMode::Dark);
        assert_eq!(raw.background_mode, BackgroundMode::Solid);
        assert_eq!(raw.background_image, None);
        assert_eq!(raw.background_accent, None);
        assert!(!raw.background_accent_enabled);
        assert_eq!(raw.background_opacity, 1.0);
        assert_eq!(raw.canvas_opacity, 1.0);
        assert_eq!(raw.font_pt, *FONT_PT_RANGE.end());
        assert_eq!(raw.target, "omml");
        assert_eq!(raw.category, palette::DEFAULT_GROUP);
        assert_eq!(
            raw.recent_palette,
            vec![r"\alpha".to_string(), r"\frac{#?}{#?}".to_string()]
        );
        assert_eq!(
            raw.palette_shortcuts,
            BTreeMap::from([
                (r"\frac{#?}{#?}".to_owned(), Some("Ctrl+Shift+F".to_owned())),
                (r"\beta".to_owned(), Some("Ctrl+Alt+V".to_owned())),
            ])
        );
        assert_eq!(raw.batch_target, 0);
        assert!(raw.batch_overwrite);

        let non_finite = Settings {
            zoom: f32::NAN,
            font_pt: f32::INFINITY,
            background_opacity: f32::NAN,
            canvas_opacity: f32::NEG_INFINITY,
            ..Settings::default()
        }
        .sanitized();
        assert_eq!(non_finite.zoom, DEFAULT_ZOOM);
        assert_eq!(non_finite.font_pt, DEFAULT_FONT_PT);
        assert_eq!(non_finite.background_opacity, DEFAULT_BACKGROUND_OPACITY);
        assert_eq!(non_finite.canvas_opacity, DEFAULT_CANVAS_OPACITY);

        let snapped_font = Settings {
            font_pt: 13.3,
            zoom: 0.5,
            background_opacity: -0.2,
            canvas_opacity: -0.4,
            background_image: Some("nested/bg.png".into()),
            ..Settings::default()
        }
        .sanitized();
        assert_eq!(snapped_font.font_pt, 13.5);
        assert_eq!(snapped_font.zoom, *ZOOM_RANGE.start());
        assert_eq!(snapped_font.background_opacity, 0.0);
        assert_eq!(snapped_font.canvas_opacity, 0.0);
        assert_eq!(snapped_font.background_image, None);
        assert_eq!(snapped_font.background_mode, BackgroundMode::Solid);

        let snapped_zoom = Settings {
            zoom: 2.346,
            ..Settings::default()
        }
        .sanitized();
        assert_eq!(snapped_zoom.zoom, 2.35);
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
    fn legacy_settings_migrate_background_mode_and_accent_enabled() {
        let dir = tempfile::tempdir().unwrap();
        let with_bg = dir.path().join("legacy_bg.json");
        std::fs::write(
            &with_bg,
            r#"{
                "background_image": "app-background-legacy.png",
                "background_opacity": 0.65
            }"#,
        )
        .unwrap();
        let loaded_bg = Settings::load_from_path(&with_bg);
        assert_eq!(loaded_bg.background_mode, BackgroundMode::Image);
        assert_eq!(
            loaded_bg.background_image.as_deref(),
            Some("app-background-legacy.png")
        );
        assert!(loaded_bg.background_accent_enabled);
        assert_eq!(loaded_bg.canvas_opacity, DEFAULT_CANVAS_OPACITY);

        let solid_with_saved = dir.path().join("solid_with_saved.json");
        std::fs::write(
            &solid_with_saved,
            r#"{
                "background_mode": "solid",
                "background_image": "app-background-saved.png",
                "background_accent_enabled": false,
                "background_accent": [52, 168, 235]
            }"#,
        )
        .unwrap();
        let loaded_solid = Settings::load_from_path(&solid_with_saved);
        assert_eq!(loaded_solid.background_mode, BackgroundMode::Solid);
        assert_eq!(
            loaded_solid.background_image.as_deref(),
            Some("app-background-saved.png")
        );
        assert!(!loaded_solid.background_accent_enabled);
        assert_eq!(loaded_solid.background_accent, Some([52, 168, 235]));
        assert_eq!(loaded_solid.canvas_opacity, DEFAULT_CANVAS_OPACITY);
    }

    #[test]
    fn atomic_round_trip_saves_and_reloads_all_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("settings.json");

        let original = Settings {
            zoom: 3.25,
            theme: ThemeMode::Light,
            background_mode: BackgroundMode::Image,
            background_image: Some("app-background-123.png".into()),
            background_opacity: 0.55,
            canvas_opacity: 0.62,
            background_accent_enabled: false,
            background_accent: Some([44, 152, 224]),
            startup_focus: StartupFocus::Latex,
            font_pt: 16.5,
            target: "mathtype".into(),
            category: palette::RECENT_GROUP.into(),
            recent_palette: vec![r"\frac{#?}{#?}".into(), r"\alpha".into()],
            palette_shortcuts: BTreeMap::from([
                (r"\frac{#?}{#?}".into(), None),
                (r"\overrightarrow{#?}".into(), Some("Ctrl+Alt+V".into())),
            ]),
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
    fn palette_shortcuts_persist_explicit_null_and_sanitize_duplicates() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(
            &path,
            r#"{
                "palette_shortcuts": {
                    "\\frac{#?}{#?}": null,
                    "\\sqrt{#?}": "Ctrl+F",
                    "\\unknown": "Ctrl+Q",
                    "\\overrightarrow{#?}": "F4",
                    "\\binom{#?}{#?}": "f4",
                    "\\alpha": "Ctrl+Z",
                    "\\beta": "K"
                }
            }"#,
        )
        .unwrap();

        let loaded = Settings::load_from_path(&path);
        let map = shortcuts::ShortcutMap::from_overrides(&loaded.palette_shortcuts);
        // Explicit null keeps \frac{#?}{#?} unassigned across restarts.
        assert_eq!(map.binding_for(r"\frac{#?}{#?}"), None);
        // \sqrt{#?} took Ctrl+F cleanly after \frac{#?}{#?} was cleared.
        assert_eq!(
            map.binding_for(r"\sqrt{#?}"),
            shortcuts::ShortcutChord::parse("Ctrl+F")
        );
        // One of \binom or \overrightarrow wins F4 (BTreeMap visits \binom first); the duplicate drops.
        assert_eq!(
            map.source_for(shortcuts::ShortcutChord::parse("F4").unwrap()),
            Some(r"\binom{#?}{#?}")
        );
        assert_eq!(map.binding_for(r"\overrightarrow{#?}"), None);
        // System-reserved and bare keys are rejected.
        assert_eq!(map.binding_for(r"\alpha"), None);
        assert_eq!(map.binding_for(r"\beta"), None);
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
        assert!((loaded.background_opacity - 0.42).abs() < 1e-6);
        assert_eq!(loaded.canvas_opacity, 1.0);
        assert_eq!(loaded.startup_focus, StartupFocus::Canvas);
        assert!(loaded.palette_shortcuts.is_empty());
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
    fn background_accent_extraction_handles_chromatic_and_grayscale_images() {
        let teal = RgbaImage::from_pixel(32, 32, Rgba([32, 180, 170, 255]));
        let extracted = extract_background_accent(teal.as_raw(), 32, 32)
            .expect("chromatic teal image should yield accent seed");
        assert!(extracted[1] > extracted[0] && extracted[2] > extracted[0]);

        let gray = RgbaImage::from_pixel(32, 32, Rgba([128, 128, 128, 255]));
        assert_eq!(extract_background_accent(gray.as_raw(), 32, 32), None);
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
        assert!(first.accent.is_some());
        assert!(first_path.exists());

        // Deleting external source does not break loading the managed copy.
        std::fs::remove_file(&src_jpg).unwrap();
        let loaded = load_managed_background(&settings_path, &first.filename).unwrap();
        assert_eq!(loaded.width, first.width);
        assert_eq!(loaded.height, first.height);
        assert!(loaded.accent.is_some());

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
        assert!(second.accent.is_some());
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
