//! Settings tab, preferences persistence, and appearance controls.

use super::App;
use crate::{
    settings::{self, BackgroundMode, Settings, StartupFocus, ThemeMode},
    theme::{self, ThemeTokens},
    ui,
};
use eframe::egui::{self, Color32, Vec2};

const OPACITY_CONTROL_WIDTH: f32 = 220.0;

impl App {
    /// Applies sanitized settings to in-memory app state and updates theme/accent styles when `egui_ctx` is provided.
    pub(super) fn apply_settings(&mut self, settings: Settings, egui_ctx: Option<&egui::Context>) {
        let settings = settings.sanitized();
        self.zoom = settings.zoom;
        self.font_pt = settings.font_pt;
        self.saved_font = settings.font_pt;
        self.latex_font_pt = settings.latex_font_pt;
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

    /// Renders the Settings tab covering appearance, app background, startup focus, and shortcuts.
    pub(super) fn draw_settings_tab(&mut self, ui: &mut egui::Ui) {
        let tokens = self.theme_tokens(ui);
        self.draw_back_to_editor_button(ui);
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui::widgets::full_width_card(ui, tokens, |ui| {
                    ui.label(egui::RichText::new("Appearance").strong());
                    ui.add_space(theme::SPACE_XS);
                    let is_dark = ui.visuals().dark_mode;
                    ui.horizontal(|ui| {
                        for (choice, label) in
                            [(egui::Theme::Light, "Light"), (egui::Theme::Dark, "Dark")]
                        {
                            let selected = (choice == egui::Theme::Dark) == is_dark;
                            if ui::widgets::with_selectable_style(ui, tokens, selected, |ui| {
                                ui::widgets::standard_button(ui, label)
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
                ui::widgets::full_width_card(ui, tokens, |ui| {
                    ui.label(egui::RichText::new("Startup focus").strong());
                    ui.add_space(theme::SPACE_XS);
                    ui.horizontal(|ui| {
                        let mut focus_changed = false;
                        for (choice, label) in [
                            (StartupFocus::Canvas, "Equation canvas"),
                            (StartupFocus::Latex, "LaTeX source"),
                        ] {
                            let selected = self.startup_focus == choice;
                            if ui::widgets::with_selectable_style(ui, tokens, selected, |ui| {
                                ui::widgets::standard_button(ui, label)
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

    fn draw_app_background_card(&mut self, ui: &mut egui::Ui, tokens: ThemeTokens) {
        ui::widgets::full_width_card(ui, tokens, |ui| {
            let is_dark = ui.visuals().dark_mode;
            ui.label(egui::RichText::new("App background").strong());
            ui.add_space(theme::SPACE_XS);
            ui.horizontal(|ui| {
                for (choice, label) in [
                    (BackgroundMode::Solid, "Solid"),
                    (BackgroundMode::Image, "Image"),
                ] {
                    let selected = self.background_mode == choice;
                    if ui::widgets::with_selectable_style(ui, tokens, selected, |ui| {
                        ui::widgets::standard_button(ui, label)
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
                            ui::widgets::icon_standard_button(
                                ui,
                                &mut self.ui_icons,
                                tokens,
                                "folder-open",
                                "Choose image…",
                            )
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
                let accent_checkbox = ui::widgets::accent_checkbox(
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
        ui::widgets::without_double_dimming(ui, enabled, |ui| {
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
                    let speed = f64::from((*range.end() - *range.start()) / OPACITY_CONTROL_WIDTH);
                    let (drag_resp, wheel_changed) = ui::widgets::level_drag_value(
                        ui,
                        tokens,
                        label,
                        speed,
                        None,
                        range.clone(),
                        value,
                        "Click and drag left/right, or click to type a percentage",
                        |ui, current, drag_speed| {
                            ui.add(
                                egui::DragValue::from_get_set(|next| {
                                    if let Some(next) = next {
                                        *current = ((next * 100.0).round() / 100.0) as f32;
                                    }
                                    f64::from(*current)
                                })
                                .range(range)
                                .min_size(Vec2::new(OPACITY_CONTROL_WIDTH, theme::BUTTON_HEIGHT))
                                .speed(drag_speed)
                                .custom_formatter(|val, _| format!("{:.0}%", (val * 100.0).round()))
                                .custom_parser(|text| {
                                    text.trim()
                                        .trim_end_matches('%')
                                        .trim()
                                        .parse::<f64>()
                                        .ok()
                                        .map(|pct| pct / 100.0)
                                }),
                            )
                        },
                    );
                    changed = drag_resp.changed() || wheel_changed;
                },
            );
        });
    });
    changed
}
