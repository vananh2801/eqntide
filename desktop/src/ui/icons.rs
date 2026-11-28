//! Bundled Tabler UI icon lookup and lazy GPU texture caching.

use eframe::egui::{self, Color32, Vec2};
use std::collections::HashMap;

/// Default logical size (in DIP) for standard toolbar and button icons.
pub(crate) const UI_ICON_SIZE: f32 = 16.0;

/// Lazy texture cache for bundled Tabler UI icons keyed by canonical icon name.
#[derive(Default)]
pub(crate) struct IconCache {
    textures: HashMap<&'static str, egui::TextureHandle>,
}

impl IconCache {
    /// Returns a cached texture handle for `name`, if already uploaded.
    #[cfg(test)]
    pub(crate) fn get(&self, name: &str) -> Option<&egui::TextureHandle> {
        self.textures.get(name)
    }

    /// Returns the uploaded GPU texture ID for `name`, if already cached.
    #[cfg(test)]
    pub(crate) fn texture_id(&self, name: &str) -> Option<egui::TextureId> {
        self.textures.get(name).map(egui::TextureHandle::id)
    }

    /// Loads (if needed) and returns a tinted `egui::Image` for the named Tabler icon.
    pub(crate) fn image(
        &mut self,
        ctx: &egui::Context,
        name: &'static str,
        size: f32,
        tint: Color32,
    ) -> egui::Image<'static> {
        let texture_id = self
            .textures
            .entry(name)
            .or_insert_with(|| {
                let pixels = image::load_from_memory(tabler_icon_bytes(name))
                    .expect("bundled tabler icon is valid")
                    .to_rgba8();
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [pixels.width() as usize, pixels.height() as usize],
                    pixels.as_raw(),
                );
                ctx.load_texture(
                    format!("tabler-icon-{name}"),
                    image,
                    egui::TextureOptions::LINEAR,
                )
            })
            .id();
        egui::Image::new((texture_id, Vec2::splat(size))).tint(tint)
    }
}

/// Returns embedded PNG bytes for the named bundled Tabler UI icon.
pub(crate) fn tabler_icon_bytes(name: &str) -> &'static [u8] {
    match name {
        "alpha" => include_bytes!("../../../assets/icons/generated/desktop/alpha.png"),
        "arrow-autofit-right" => {
            include_bytes!("../../../assets/icons/generated/desktop/arrow-autofit-right.png")
        }
        "arrow-back-up" => {
            include_bytes!("../../../assets/icons/generated/desktop/arrow-back-up.png")
        }
        "arrow-forward-up" => {
            include_bytes!("../../../assets/icons/generated/desktop/arrow-forward-up.png")
        }
        "arrows-left-right" => {
            include_bytes!("../../../assets/icons/generated/desktop/arrows-left-right.png")
        }
        "brand-github" => {
            include_bytes!("../../../assets/icons/generated/desktop/brand-github.png")
        }
        "chevron-left" => {
            include_bytes!("../../../assets/icons/generated/desktop/chevron-left.png")
        }
        "dots" => include_bytes!("../../../assets/icons/generated/desktop/dots.png"),
        "file" => include_bytes!("../../../assets/icons/generated/desktop/file.png"),
        "file-type-docx" => {
            include_bytes!("../../../assets/icons/generated/desktop/file-type-docx.png")
        }
        "file-upload" => include_bytes!("../../../assets/icons/generated/desktop/file-upload.png"),
        "folder" => include_bytes!("../../../assets/icons/generated/desktop/folder.png"),
        "folder-open" => include_bytes!("../../../assets/icons/generated/desktop/folder-open.png"),
        "history" => include_bytes!("../../../assets/icons/generated/desktop/history.png"),
        "info-circle" => include_bytes!("../../../assets/icons/generated/desktop/info-circle.png"),
        "license" => include_bytes!("../../../assets/icons/generated/desktop/license.png"),
        "math-avg" => include_bytes!("../../../assets/icons/generated/desktop/math-avg.png"),
        "math-equal-lower" => {
            include_bytes!("../../../assets/icons/generated/desktop/math-equal-lower.png")
        }
        "math-integral" => {
            include_bytes!("../../../assets/icons/generated/desktop/math-integral.png")
        }
        "math-sin" => include_bytes!("../../../assets/icons/generated/desktop/math-sin.png"),
        "math-x-divide-y" => {
            include_bytes!("../../../assets/icons/generated/desktop/math-x-divide-y.png")
        }
        "matrix" => include_bytes!("../../../assets/icons/generated/desktop/matrix.png"),
        "omega" => include_bytes!("../../../assets/icons/generated/desktop/omega.png"),
        "overline" => include_bytes!("../../../assets/icons/generated/desktop/overline.png"),
        "parentheses" => include_bytes!("../../../assets/icons/generated/desktop/parentheses.png"),
        "settings" => include_bytes!("../../../assets/icons/generated/desktop/settings.png"),
        "square-root-2" => {
            include_bytes!("../../../assets/icons/generated/desktop/square-root-2.png")
        }
        "sum" => include_bytes!("../../../assets/icons/generated/desktop/sum.png"),
        "underline" => include_bytes!("../../../assets/icons/generated/desktop/underline.png"),
        "user" => include_bytes!("../../../assets/icons/generated/desktop/user.png"),
        "x-power-y" => include_bytes!("../../../assets/icons/generated/desktop/x-power-y.png"),
        other => panic!("unknown tabler icon: {other}"),
    }
}
