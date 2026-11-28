//! Palette sprite atlas, template insertion, and recent-item tracking.

use super::{App, PaletteAtlas, PaletteAtlasEntry};
use crate::{palette, settings::MAX_RECENT_PALETTE};
use eframe::egui::{self, Pos2, Rect, Vec2};
use std::collections::HashMap;

const PALETTE_ATLAS_PNG: &[u8] =
    include_bytes!("../../../assets/icons/generated/palette-atlas/atlas.png");

impl App {
    /// Parses a LaTeX template snippet and inserts it at the current caret/selection.
    pub(super) fn insert_template(&mut self, source: &str) {
        match palette::template(source) {
            Ok(template) => {
                if palette::attaches_to_previous(source) {
                    self.editor.insert_template_wrapping_previous(&template);
                } else {
                    self.editor.insert_template(&template);
                }
                self.mark_canvas_changed();
            }
            Err(err) => self.status = err,
        }
    }

    /// Moves `source` to the front of the MRU palette list and marks settings dirty.
    pub(super) fn record_recent_palette(&mut self, source: &'static str) {
        self.recent_palette.retain(|&existing| existing != source);
        self.recent_palette.insert(0, source);
        self.recent_palette.truncate(MAX_RECENT_PALETTE);
        self.persisted.recent_palette = self
            .recent_palette
            .iter()
            .map(|&item_src| item_src.to_owned())
            .collect();
        self.settings_dirty = true;
    }

    /// Lazily decodes the bundled pre-rasterized palette sprite atlas and uploads it once.
    pub(super) fn ensure_palette_atlas(&mut self, ctx: &egui::Context) -> &PaletteAtlas {
        self.palette_atlas.get_or_insert_with(|| {
            let meta = palette::AtlasMetadata::load_embedded();
            let pixels = image::load_from_memory(PALETTE_ATLAS_PNG)
                .expect("bundled palette atlas PNG is valid")
                .to_rgba8();
            let width = pixels.width() as usize;
            let height = pixels.height() as usize;
            assert_eq!(
                (width as u32, height as u32),
                (meta.atlas_width, meta.atlas_height),
                "bundled palette atlas PNG dimensions must match atlas.json"
            );
            let image = egui::ColorImage::from_rgba_unmultiplied([width, height], pixels.as_raw());
            let texture = ctx.load_texture("palette-atlas", image, egui::TextureOptions::LINEAR);
            let atlas_width = meta.atlas_width as f32;
            let atlas_height = meta.atlas_height as f32;
            let display_size = Vec2::new(meta.icon_width, meta.icon_height);
            let mut entries = HashMap::with_capacity(meta.entries.len());
            for item in meta.entries {
                if let Some(canonical) = palette::canonical_source(&item.source) {
                    let uv = Rect::from_min_max(
                        Pos2::new(item.x as f32 / atlas_width, item.y as f32 / atlas_height),
                        Pos2::new(
                            (item.x + item.width) as f32 / atlas_width,
                            (item.y + item.height) as f32 / atlas_height,
                        ),
                    );
                    entries.insert(canonical, PaletteAtlasEntry { uv, display_size });
                }
            }
            PaletteAtlas { texture, entries }
        })
    }

    /// Returns the active GPU texture IDs used for palette icons.
    #[cfg(test)]
    pub(super) fn palette_texture_ids(&self) -> std::collections::HashSet<egui::TextureId> {
        self.palette_atlas
            .iter()
            .map(|atlas| atlas.texture.id())
            .collect()
    }

    /// Returns the cached GPU texture ID for a Tabler UI icon in tests.
    #[cfg(test)]
    pub(super) fn ui_icon_texture_id(&self, name: &str) -> Option<egui::TextureId> {
        self.ui_icons.texture_id(name)
    }
}
