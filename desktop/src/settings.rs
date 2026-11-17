use crate::{batch, palette};
use directories::ProjectDirs;
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::{Path, PathBuf},
};

pub(crate) const ZOOM_RANGE: std::ops::RangeInclusive<f32> = 1.5..=4.0;
pub(crate) const FONT_PT_RANGE: std::ops::RangeInclusive<f32> = 6.0..=72.0;
pub(crate) const DEFAULT_ZOOM: f32 = 2.0;
pub(crate) const DEFAULT_FONT_PT: f32 = 12.0;
pub(crate) const DEFAULT_TARGET: &str = "omml";
pub(crate) const MAX_RECENT_PALETTE: usize = 24;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}

impl ThemeMode {
    pub(crate) fn to_egui(self) -> egui::ThemePreference {
        match self {
            Self::System => egui::ThemePreference::System,
            Self::Light => egui::ThemePreference::Light,
            Self::Dark => egui::ThemePreference::Dark,
        }
    }

    pub(crate) fn from_egui_theme(theme: egui::Theme) -> Self {
        match theme {
            egui::Theme::Light => Self::Light,
            egui::Theme::Dark => Self::Dark,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Settings {
    pub(crate) zoom: f32,
    pub(crate) theme: ThemeMode,
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
    pub(crate) fn default_path() -> Option<PathBuf> {
        ProjectDirs::from("com", "eqntide", "EqnTide")
            .map(|dirs| dirs.config_dir().join("settings.json"))
    }

    pub(crate) fn sanitized(mut self) -> Self {
        self.sanitize();
        self
    }

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
    }

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

    pub(crate) fn save_to_path(&self, path: &Path) -> std::io::Result<()> {
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        std::fs::create_dir_all(parent)?;
        let mut temp = tempfile::NamedTempFile::new_in(parent)?;
        serde_json::to_writer_pretty(temp.as_file_mut(), self).map_err(std::io::Error::other)?;
        temp.as_file_mut().write_all(b"\n")?;
        temp.as_file_mut().sync_all()?;
        temp.persist(path).map_err(|e| e.error)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_path_points_to_settings_json_in_eqntide_config_dir() {
        let path = Settings::default_path().expect("platform config directory");
        assert_eq!(
            path.file_name().and_then(|n| n.to_str()),
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
            ..Settings::default()
        }
        .sanitized();
        assert_eq!(non_finite.zoom, DEFAULT_ZOOM);
        assert_eq!(non_finite.font_pt, DEFAULT_FONT_PT);

        let snapped_font = Settings {
            font_pt: 13.3,
            zoom: 0.5,
            ..Settings::default()
        }
        .sanitized();
        assert_eq!(snapped_font.font_pt, 13.5);
        assert_eq!(snapped_font.zoom, *ZOOM_RANGE.start());
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
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(entries, vec!["settings.json"]);
    }
}
