//! Settings tab, canvas presets, and app backgrounds.

use super::App;
use crate::{
    settings::{self, BackgroundMode, Settings, StartupFocus, ThemeMode, ZOOM_RANGE},
    theme::{self, CanvasPalette, ThemeTokens},
    Event,
};
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
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

    /// Applies sanitized settings to in-memory app state and updates theme/accent styles when `egui_ctx` is provided.
    pub(super) fn apply_settings(&mut self, settings: Settings, egui_ctx: Option<&egui::Context>) {
        let settings = settings.sanitized();
        self.zoom = settings.zoom;
        self.font_pt = settings.font_pt;
        self.saved_font = settings.font_pt;
        self.target = settings.target.clone();
        self.saved_mode = settings.target.clone();
        self.category = settings.category.clone();
        self.recent_palette = settings.resolved_recent_palette();
        self.batch_target = settings.batch_target;
        self.batch_overwrite = settings.batch_overwrite;
        self.background_mode = settings.background_mode;
        self.background_image = settings.background_image.clone();
        self.background_opacity = settings.background_opacity;
        self.canvas_opacity = settings.canvas_opacity;
        self.background_accent_enabled = settings.background_accent_enabled;
        self.background_accent = settings.background_accent;
        self.background_texture = None;
        self.software_bg = None;
        self.background_load_pending = self.background_image.is_some();
        self.background_error = None;
        self.startup_focus = settings.startup_focus;
        self.active_editor_focus = settings.startup_focus;
        self.pending_focus_restore = Some(settings.startup_focus);
        self.shortcuts = crate::shortcuts::ShortcutMap::from_overrides(&settings.palette_shortcuts);
        self.shortcut_capture = None;
        self.shortcut_capture_error = None;
        self.shortcut_conflict = None;
        self.persisted = settings;
        self.settings_dirty = false;
        if let Some(ctx) = egui_ctx {
            ctx.set_theme(self.persisted.theme.to_egui());
            self.sync_accent_style(ctx);
        }
    }

    /// Persists dirty settings atomically to `settings_path` if configured.
    pub(super) fn flush_settings(&mut self) {
        if !self.settings_dirty {
            return;
        }
        self.settings_dirty = false;
        let Some(path) = self.settings_path.as_deref() else {
            return;
        };
        let to_save = self.persisted.clone().sanitized();
        if let Err(err) = to_save.save_to_path(path) {
            eprintln!("Failed to save settings {}: {err}", path.display());
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

    /// Renders the Settings tab covering canvas zoom, appearance, app background, and startup focus.
    pub(super) fn draw_settings_tab(&mut self, ui: &mut egui::Ui) {
        let tokens = self.theme_tokens(ui);
        self.draw_back_to_editor_button(ui);
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                Self::full_width_card(ui, tokens, |ui| {
                    ui.label(egui::RichText::new("Canvas zoom").strong());
                    ui.add_space(theme::SPACE_XS);
                    ui.allocate_ui_with_layout(
                        Vec2::new(ui.available_size_before_wrap().x, theme::BUTTON_HEIGHT),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            let slider_changed = ui
                                .scope(|ui| {
                                    ui.spacing_mut().slider_width = 220.0;
                                    let visuals = ui.visuals_mut();
                                    let handle_stroke = Stroke::new(1.0, tokens.accent);
                                    visuals.widgets.inactive.bg_fill = tokens.border;
                                    visuals.widgets.inactive.fg_stroke = handle_stroke;
                                    visuals.widgets.hovered.fg_stroke = handle_stroke;
                                    visuals.widgets.active.fg_stroke = handle_stroke;
                                    visuals.selection.bg_fill = tokens.accent;
                                    ui.add(
                                        egui::Slider::new(&mut self.zoom, ZOOM_RANGE)
                                            .show_value(false)
                                            .trailing_fill(true)
                                            .handle_shape(egui::style::HandleShape::Rect {
                                                aspect_ratio: 1.0,
                                            }),
                                    )
                                    .changed()
                                })
                                .inner;
                            let value_changed = ui
                                .add(
                                    egui::DragValue::new(&mut self.zoom)
                                        .range(ZOOM_RANGE)
                                        .speed(0.01)
                                        .min_size(Vec2::new(60.0, theme::BUTTON_HEIGHT))
                                        .custom_formatter(|value, _| {
                                            format!("{:.0}%", value * 100.0)
                                        })
                                        .custom_parser(|text| {
                                            text.trim()
                                                .trim_end_matches('%')
                                                .trim()
                                                .parse::<f64>()
                                                .ok()
                                                .map(|pct| pct / 100.0)
                                        }),
                                )
                                .changed();
                            if slider_changed || value_changed {
                                self.persisted.zoom = self.zoom;
                                self.settings_dirty = true;
                            }
                            if Self::standard_button(ui, "Reset (200%)").clicked() {
                                self.zoom = settings::DEFAULT_ZOOM;
                                self.persisted.zoom = self.zoom;
                                self.settings_dirty = true;
                            }
                        },
                    );
                });
                ui.add_space(theme::SPACE_MD);
                Self::full_width_card(ui, tokens, |ui| {
                    ui.label(egui::RichText::new("Appearance").strong());
                    ui.add_space(theme::SPACE_XS);
                    let is_dark = ui.visuals().dark_mode;
                    ui.horizontal(|ui| {
                        for (choice, label) in
                            [(egui::Theme::Light, "Light"), (egui::Theme::Dark, "Dark")]
                        {
                            let selected = (choice == egui::Theme::Dark) == is_dark;
                            if Self::with_selectable_style(ui, tokens, selected, |ui| {
                                Self::standard_button(ui, label)
                            })
                            .clicked()
                                && !selected
                            {
                                ui.ctx().set_theme(choice);
                                self.persisted.theme = ThemeMode::from_egui_theme(choice);
                                self.settings_dirty = true;
                            }
                        }
                    });
                });
                ui.add_space(theme::SPACE_MD);
                self.draw_app_background_card(ui, tokens);
                let tokens = self.theme_tokens(ui);
                ui.add_space(theme::SPACE_MD);
                Self::full_width_card(ui, tokens, |ui| {
                    ui.label(egui::RichText::new("Startup focus").strong());
                    ui.add_space(theme::SPACE_XS);
                    ui.horizontal(|ui| {
                        let mut focus_changed = false;
                        for (choice, label) in [
                            (StartupFocus::Canvas, "Equation canvas"),
                            (StartupFocus::Latex, "LaTeX source"),
                        ] {
                            let selected = self.startup_focus == choice;
                            if Self::with_selectable_style(ui, tokens, selected, |ui| {
                                Self::standard_button(ui, label)
                            })
                            .clicked()
                                && !selected
                            {
                                self.startup_focus = choice;
                                focus_changed = true;
                            }
                        }
                        if focus_changed {
                            self.persisted.startup_focus = self.startup_focus;
                            self.settings_dirty = true;
                        }
                    });
                });
                ui.add_space(theme::SPACE_MD);
                self.draw_keyboard_shortcuts_card(ui, tokens);
            });
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

    fn draw_app_background_card(&mut self, ui: &mut egui::Ui, tokens: ThemeTokens) {
        Self::full_width_card(ui, tokens, |ui| {
            let is_dark = ui.visuals().dark_mode;
            ui.label(egui::RichText::new("App background").strong());
            ui.add_space(theme::SPACE_XS);
            ui.horizontal(|ui| {
                for (choice, label) in [
                    (BackgroundMode::Solid, "Solid"),
                    (BackgroundMode::Image, "Image"),
                ] {
                    let selected = self.background_mode == choice;
                    if Self::with_selectable_style(ui, tokens, selected, |ui| {
                        Self::standard_button(ui, label)
                    })
                    .clicked()
                        && !selected
                    {
                        match choice {
                            BackgroundMode::Solid => {
                                self.background_mode = BackgroundMode::Solid;
                                self.persisted.background_mode = BackgroundMode::Solid;
                                self.settings_dirty = true;
                                self.sync_accent_style(ui.ctx());
                            }
                            BackgroundMode::Image => {
                                if self.background_image.is_some() {
                                    self.background_mode = BackgroundMode::Image;
                                    self.persisted.background_mode = BackgroundMode::Image;
                                    self.settings_dirty = true;
                                    if self.background_texture.is_none() {
                                        self.background_load_pending = true;
                                    }
                                    self.sync_accent_style(ui.ctx());
                                } else if !self.background_importing {
                                    self.spawn_background_picker();
                                }
                            }
                        }
                    }
                }
            });
            ui.add_space(theme::SPACE_XS);
            let has_bg_setting =
                self.background_image.is_some() || self.background_texture.is_some();
            let thumb_size = Vec2::new(72.0, 48.0);
            ui.allocate_ui_with_layout(
                Vec2::new(ui.available_size_before_wrap().x, thumb_size.y),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    self.draw_background_thumbnail(ui, tokens, is_dark, thumb_size);
                    if ui
                        .add_enabled_ui(!self.background_importing, |ui| {
                            self.icon_standard_button(ui, "folder-open", "Choose image…")
                        })
                        .inner
                        .clicked()
                    {
                        self.spawn_background_picker();
                    }
                    if ui
                        .add_enabled(
                            has_bg_setting || self.background_importing,
                            egui::Button::new("Clear")
                                .min_size(Vec2::new(0.0, theme::BUTTON_HEIGHT)),
                        )
                        .clicked()
                    {
                        self.clear_background_image();
                        self.sync_accent_style(ui.ctx());
                    }
                },
            );
            let has_active_image_bg = self.is_image_background_active();
            let tokens = self.theme_tokens(ui);
            if let Some(err) = &self.background_error {
                ui.add_space(theme::SPACE_XS);
                ui.label(egui::RichText::new(err).small().color(tokens.error_fg));
            } else if self.background_image.is_some()
                && self.background_texture.is_none()
                && !self.background_importing
            {
                ui.add_space(theme::SPACE_XS);
                ui.label(
                    egui::RichText::new("Saved background image is missing or unreadable")
                        .small()
                        .color(tokens.error_fg),
                );
            }
            ui.add_space(theme::SPACE_XS);
            ui.add_enabled_ui(has_active_image_bg, |ui| {
                let accent_checkbox = Self::accent_checkbox(
                    ui,
                    tokens,
                    &mut self.background_accent_enabled,
                    false,
                    "Match accent color to background",
                );
                if accent_checkbox.changed() {
                    self.persisted.background_accent_enabled = self.background_accent_enabled;
                    self.settings_dirty = true;
                    self.sync_accent_style(ui.ctx());
                }
            });
            let tokens = self.theme_tokens(ui);
            ui.add_space(theme::SPACE_XS);
            if draw_opacity_slider_row(
                ui,
                tokens,
                has_active_image_bg,
                "Overlay opacity",
                &mut self.background_opacity,
                settings::BACKGROUND_OPACITY_RANGE,
            ) {
                self.persisted.background_opacity = self.background_opacity;
                self.settings_dirty = true;
            }
            ui.add_space(theme::SPACE_XS);
            if draw_opacity_slider_row(
                ui,
                tokens,
                has_active_image_bg,
                "Canvas opacity",
                &mut self.canvas_opacity,
                settings::CANVAS_OPACITY_RANGE,
            ) {
                self.persisted.canvas_opacity = self.canvas_opacity;
                self.settings_dirty = true;
            }
        });
    }

    fn draw_background_thumbnail(
        &self,
        ui: &mut egui::Ui,
        tokens: ThemeTokens,
        is_dark: bool,
        thumb_size: Vec2,
    ) {
        let (thumb_rect, _) = ui.allocate_exact_size(thumb_size, egui::Sense::hover());
        if !ui.is_rect_visible(thumb_rect) {
            return;
        }
        ui.painter().rect_filled(
            thumb_rect,
            theme::CONTROL_CORNER_RADIUS,
            tokens.control_disabled_bg,
        );
        if let Some(tex) = &self.background_texture {
            let (dest, uv) = Self::background_image_layout(thumb_rect, tex.size_vec2());
            let thumb_tint = if self.background_mode == BackgroundMode::Image {
                Color32::WHITE
            } else {
                Color32::from_white_alpha(140)
            };
            egui::Image::from_texture((tex.id(), dest.size()))
                .uv(uv)
                .tint(thumb_tint)
                .corner_radius(theme::CONTROL_CORNER_RADIUS)
                .paint_at(ui, dest);
            if self.background_mode == BackgroundMode::Image {
                let scrim = ThemeTokens::background_scrim(is_dark, self.background_opacity);
                if scrim.a() > 0 {
                    ui.painter()
                        .rect_filled(thumb_rect, theme::CONTROL_CORNER_RADIUS, scrim);
                }
            }
        }
    }
}

fn draw_opacity_slider_row(
    ui: &mut egui::Ui,
    tokens: ThemeTokens,
    enabled: bool,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
) -> bool {
    let mut changed = false;
    ui.add_enabled_ui(enabled, |ui| {
        if !enabled {
            // Keep slider handle fills opaque so the trailing fill never shows through.
            ui.set_opacity(1.0);
        }
        let label_width = 108.0;
        let label_color = if enabled {
            tokens.text_muted
        } else {
            tokens.text_disabled
        };
        ui.allocate_ui_with_layout(
            Vec2::new(ui.available_size_before_wrap().x, theme::BUTTON_HEIGHT),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.allocate_ui_with_layout(
                    Vec2::new(label_width, theme::BUTTON_HEIGHT),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.label(egui::RichText::new(label).color(label_color));
                    },
                );
                changed = ui
                    .scope(|ui| {
                        ui.spacing_mut().slider_width = 220.0;
                        let visuals = ui.visuals_mut();
                        if enabled {
                            let handle_stroke = Stroke::new(1.0, tokens.accent);
                            visuals.widgets.inactive.bg_fill = tokens.border;
                            visuals.widgets.inactive.fg_stroke = handle_stroke;
                            visuals.widgets.hovered.fg_stroke = handle_stroke;
                            visuals.widgets.active.fg_stroke = handle_stroke;
                            visuals.selection.bg_fill = tokens.accent;
                        } else {
                            let opaque_tokens = ThemeTokens::for_mode(tokens.dark_mode);
                            let disabled_accent = tokens.disabled_accent();
                            let handle_stroke = Stroke::new(1.0, disabled_accent);
                            visuals.widgets.noninteractive.bg_fill = opaque_tokens.panel_bg;
                            visuals.widgets.noninteractive.weak_bg_fill = opaque_tokens.panel_bg;
                            visuals.widgets.noninteractive.bg_stroke = handle_stroke;
                            visuals.widgets.noninteractive.fg_stroke = handle_stroke;
                            visuals.widgets.inactive.bg_fill = opaque_tokens.control_disabled_bg;
                            visuals.widgets.inactive.fg_stroke = handle_stroke;
                            visuals.selection.bg_fill = disabled_accent;
                        }
                        ui.add(
                            egui::Slider::new(value, range)
                                .step_by(0.01)
                                .show_value(false)
                                .trailing_fill(true)
                                .handle_shape(egui::style::HandleShape::Rect { aspect_ratio: 1.0 }),
                        )
                        .changed()
                    })
                    .inner;
                ui.label(egui::RichText::new(format!("{:.0}%", *value * 100.0)).color(label_color));
            },
        );
    });
    changed
}
