//! App background image import, GPU/software texture upload, adaptive accent sync, and scrim painting.

use super::App;
use crate::{
    settings::{self, BackgroundMode},
    theme::{CanvasPalette, ThemeTokens},
    Event,
};
use eframe::egui::{self, Color32, Pos2, Rect, Vec2};
use std::{path::Path, sync::Arc};

impl App {
    /// Returns `true` when `Image` background mode is selected and a background texture is loaded.
    pub(super) fn is_image_background_active(&self) -> bool {
        self.background_mode == BackgroundMode::Image && self.background_texture.is_some()
    }

    /// Returns the active adaptive RGB accent seed, or `None` when using the default Ayu accent.
    pub(super) fn active_accent_seed(&self) -> Option<[u8; 3]> {
        if self.is_image_background_active() && self.background_accent_enabled {
            self.background_accent
        } else {
            None
        }
    }

    /// Resolves effective [`ThemeTokens`] for the active theme, background opacity, and accent seed.
    pub(super) fn theme_tokens(&self, ui: &egui::Ui) -> ThemeTokens {
        ThemeTokens::for_mode_with_background_and_accent(
            ui.visuals().dark_mode,
            self.is_image_background_active(),
            self.background_opacity,
            self.active_accent_seed(),
        )
    }

    /// Resolves the [`CanvasPalette`] for the requested mode, active background state, canvas opacity, and accent seed.
    pub(super) fn active_canvas_palette(&self, dark_mode: bool) -> CanvasPalette {
        CanvasPalette::for_mode_with_background_and_accent(
            dark_mode,
            self.is_image_background_active(),
            self.canvas_opacity,
            self.active_accent_seed(),
        )
    }

    /// Returns the CPU background bitmap and scrim parameters for [`crate::software_painter::SoftwarePainter`].
    pub(crate) fn software_background(
        &self,
        visuals: &egui::Visuals,
    ) -> Option<crate::software_painter::SoftwareBackgroundLayer> {
        if !self.native_software_background || self.background_mode != BackgroundMode::Image {
            return None;
        }
        let image = self.software_bg.as_ref()?;
        Some(crate::software_painter::SoftwareBackgroundLayer {
            image: Arc::clone(image),
            scrim: ThemeTokens::background_scrim(visuals.dark_mode, self.background_opacity),
            base_bg: ThemeTokens::for_mode(visuals.dark_mode).app_bg,
        })
    }

    /// Synchronizes global `egui::Context` accent styling when the effective accent seed changes.
    pub(super) fn sync_accent_style(&mut self, ctx: &egui::Context) {
        let seed = self.active_accent_seed();
        if seed != self.applied_accent_seed {
            self.applied_accent_seed = seed;
            ThemeTokens::apply_accent_to_style(ctx, seed);
        }
    }

    /// Spawns a background file picker and worker thread to import a custom wallpaper image.
    pub(super) fn spawn_background_picker(&mut self) {
        self.background_import_seq = self.background_import_seq.wrapping_add(1);
        let seq = self.background_import_seq;
        let tx = self.tx.clone();
        let settings_path = self.settings_path.clone();
        std::thread::spawn(move || {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Images", &["png", "jpg", "jpeg", "webp"])
                .pick_file()
            {
                let _ = tx.send(Event::BackgroundImportStarted(seq));
                let result = settings::prepare_background_import(&path, settings_path.as_deref());
                let _ = tx.send(Event::BackgroundImported(seq, result));
            }
        });
    }

    /// Uploads a decoded background image to the GPU texture handle and software backdrop cache.
    pub(super) fn upload_background_image(&mut self, ctx: &egui::Context, image: egui::ColorImage) {
        if self.native_software_background {
            self.software_bg_seq = self.software_bg_seq.wrapping_add(1);
            self.software_bg = Some(Arc::new(
                crate::software_painter::SoftwareBackgroundImage::from_color_image(
                    self.software_bg_seq,
                    &image,
                ),
            ));
        }
        if let Some(texture) = &mut self.background_texture {
            texture.set(image, egui::TextureOptions::LINEAR);
        } else {
            self.background_texture =
                Some(ctx.load_texture("app-background", image, egui::TextureOptions::LINEAR));
        }
    }

    /// Loads any deferred startup background image from disk and backfills missing accent seeds.
    pub(super) fn ensure_background_texture(&mut self, ctx: &egui::Context) {
        if !self.background_load_pending {
            return;
        }
        self.background_load_pending = false;
        let (Some(settings_path), Some(filename)) = (
            self.settings_path.as_deref(),
            self.background_image.as_deref(),
        ) else {
            return;
        };
        match settings::load_managed_background(settings_path, filename) {
            Ok(loaded) => {
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [loaded.width as usize, loaded.height as usize],
                    &loaded.rgba,
                );
                self.upload_background_image(ctx, image);
                if self.background_accent.is_none() && loaded.accent.is_some() {
                    self.background_accent = loaded.accent;
                    self.persisted.background_accent = loaded.accent;
                    self.settings_dirty = true;
                }
                self.sync_accent_style(ctx);
            }
            Err(err) => {
                eprintln!("Failed to load managed background {filename}: {err}");
                self.background_texture = None;
                self.software_bg = None;
                self.sync_accent_style(ctx);
            }
        }
    }

    /// Applies an imported background image, updates persisted settings, and cleans up the prior file.
    pub(super) fn apply_prepared_background(
        &mut self,
        bg: settings::PreparedBackground,
        ctx: &egui::Context,
    ) {
        let prev = self.background_image.take();
        self.upload_background_image(ctx, bg.image);
        self.background_mode = BackgroundMode::Image;
        self.background_image = Some(bg.filename.clone());
        self.background_accent = bg.accent;
        self.persisted.background_mode = BackgroundMode::Image;
        self.persisted.background_image = Some(bg.filename.clone());
        self.persisted.background_accent = bg.accent;
        self.background_load_pending = false;
        self.background_error = None;
        self.sync_accent_style(ctx);
        self.settings_dirty = true;
        self.flush_settings();
        if let (Some(settings_path), Some(old_name)) =
            (self.settings_path.as_deref(), prev.as_deref())
        {
            if old_name != bg.filename {
                settings::remove_managed_background_file(settings_path, old_name);
            }
        }
    }

    /// Completes an async background import when its sequence matches the latest request.
    pub(super) fn finish_background_import(
        &mut self,
        seq: u64,
        result: Result<settings::PreparedBackground, String>,
        ctx: &egui::Context,
    ) {
        if seq != self.background_import_seq {
            if let (Ok(bg), Some(settings_path)) = (result, self.settings_path.as_deref()) {
                settings::remove_managed_background_file(settings_path, &bg.filename);
            }
            return;
        }
        self.background_importing = false;
        match result {
            Ok(bg) => self.apply_prepared_background(bg, ctx),
            Err(err) => {
                self.background_error = Some(err.clone());
                self.status = err;
            }
        }
    }

    /// Imports a custom background image synchronously from `source_path` (used in tests and CLI helpers).
    pub(super) fn set_background_image_from_path(
        &mut self,
        source_path: &Path,
        ctx: &egui::Context,
    ) {
        self.background_import_seq = self.background_import_seq.wrapping_add(1);
        let seq = self.background_import_seq;
        let result =
            settings::prepare_background_import(source_path, self.settings_path.as_deref());
        self.finish_background_import(seq, result, ctx);
    }

    /// Deletes any managed background asset, clears its accent seed, and switches back to `Solid` mode.
    pub(super) fn clear_background_image(&mut self) {
        self.background_import_seq = self.background_import_seq.wrapping_add(1);
        self.background_importing = false;
        if let (Some(settings_path), Some(filename)) = (
            self.settings_path.as_deref(),
            self.background_image.as_deref(),
        ) {
            settings::remove_managed_background_file(settings_path, filename);
        }
        self.background_mode = BackgroundMode::Solid;
        self.background_image = None;
        self.background_accent = None;
        self.background_texture = None;
        self.software_bg = None;
        self.background_load_pending = false;
        self.background_error = None;
        self.persisted.background_mode = BackgroundMode::Solid;
        self.persisted.background_image = None;
        self.persisted.background_accent = None;
        self.settings_dirty = true;
        self.flush_settings();
    }

    /// Computes cover-crop destination bounds and UVs for an image inside `viewport`.
    pub(super) fn background_image_layout(viewport: Rect, tex_size: Vec2) -> (Rect, Rect) {
        let full_uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
        if viewport.width() <= 0.0
            || viewport.height() <= 0.0
            || tex_size.x <= 0.0
            || tex_size.y <= 0.0
        {
            return (viewport, full_uv);
        }
        let view_aspect = viewport.width() / viewport.height();
        let tex_aspect = tex_size.x / tex_size.y;
        let uv = if tex_aspect > view_aspect {
            let uv_w = (view_aspect / tex_aspect).clamp(0.0, 1.0);
            let x0 = (1.0 - uv_w) * 0.5;
            Rect::from_min_max(Pos2::new(x0, 0.0), Pos2::new(x0 + uv_w, 1.0))
        } else {
            let uv_h = (tex_aspect / view_aspect).clamp(0.0, 1.0);
            let y0 = (1.0 - uv_h) * 0.5;
            Rect::from_min_max(Pos2::new(0.0, y0), Pos2::new(1.0, y0 + uv_h))
        };
        (viewport, uv)
    }

    /// Paints the active GPU background image and theme scrim behind all panels.
    pub(super) fn paint_app_background(&self, ui: &egui::Ui) {
        if self.native_software_background || self.background_mode != BackgroundMode::Image {
            return;
        }
        let Some(texture) = &self.background_texture else {
            return;
        };
        let viewport = ui.max_rect();
        let bg_painter = ui.ctx().layer_painter(egui::LayerId::background());
        let dark_mode = ui.visuals().dark_mode;
        let (dest, uv) = Self::background_image_layout(viewport, texture.size_vec2());
        bg_painter.image(texture.id(), dest, uv, Color32::WHITE);
        let scrim = ThemeTokens::background_scrim(dark_mode, self.background_opacity);
        if scrim.a() > 0 {
            bg_painter.rect_filled(viewport, 0.0, scrim);
        }
    }
}
