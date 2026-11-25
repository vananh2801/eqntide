//! Settings/About tabs, custom matrix & confirmation dialogs, canvas presets, and app backgrounds.

use super::{App, MatrixDialog, UI_ICON_SIZE};
use crate::{
    settings::{self, Settings, StartupFocus, ThemeMode, ZOOM_RANGE},
    theme::{self, CanvasPalette, CanvasPreset, ThemeTokens},
    Event,
};
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use std::{
    path::Path,
    sync::{atomic::Ordering, Arc},
};

impl App {
    /// Renders the Settings tab covering canvas zoom, theme, background image, canvas presets, and startup focus.
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
                            if Self::with_selectable_style(ui, selected, |ui| {
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
                    ui.add_space(theme::SPACE_SM);
                    ui.label(egui::RichText::new("App background").strong());
                    ui.add_space(theme::SPACE_XS);
                    let has_bg_setting =
                        self.background_image.is_some() || self.background_texture.is_some();
                    let has_bg_texture = self.background_texture.is_some();
                    let thumb_size = Vec2::new(72.0, 48.0);
                    ui.allocate_ui_with_layout(
                        Vec2::new(ui.available_size_before_wrap().x, thumb_size.y),
                        egui::Layout::left_to_right(egui::Align::Min),
                        |ui| {
                            let (thumb_rect, _) =
                                ui.allocate_exact_size(thumb_size, egui::Sense::hover());
                            if ui.is_rect_visible(thumb_rect) {
                                ui.painter().rect(
                                    thumb_rect,
                                    theme::CONTROL_CORNER_RADIUS,
                                    tokens.control_disabled_bg,
                                    Stroke::new(1.0, tokens.border),
                                    egui::StrokeKind::Inside,
                                );
                                if let Some(tex) = &self.background_texture {
                                    let inner = thumb_rect.shrink(1.0);
                                    let (dest, uv) =
                                        Self::background_image_layout(inner, tex.size_vec2());
                                    egui::Image::from_texture((tex.id(), dest.size()))
                                        .uv(uv)
                                        .corner_radius(theme::CONTROL_CORNER_RADIUS)
                                        .paint_at(ui, dest);
                                    let scrim = ThemeTokens::background_scrim(
                                        is_dark,
                                        self.background_opacity,
                                    );
                                    if scrim.a() > 0 {
                                        ui.painter().rect_filled(
                                            inner,
                                            theme::CONTROL_CORNER_RADIUS,
                                            scrim,
                                        );
                                    }
                                    ui.painter().rect_stroke(
                                        thumb_rect,
                                        theme::CONTROL_CORNER_RADIUS,
                                        Stroke::new(1.0, tokens.border),
                                        egui::StrokeKind::Inside,
                                    );
                                }
                            }
                            ui.allocate_ui_with_layout(
                                Vec2::new(ui.available_width(), thumb_size.y),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    ui.spacing_mut().item_spacing.y = theme::SPACE_XS;
                                    ui.allocate_ui_with_layout(
                                        Vec2::new(ui.available_width(), theme::BUTTON_HEIGHT),
                                        egui::Layout::left_to_right(egui::Align::Center),
                                        |ui| {
                                            if ui
                                                .add_enabled_ui(!self.background_importing, |ui| {
                                                    self.icon_standard_button(
                                                        ui,
                                                        "folder-open",
                                                        "Choose image…",
                                                    )
                                                })
                                                .inner
                                                .clicked()
                                            {
                                                self.background_import_seq =
                                                    self.background_import_seq.wrapping_add(1);
                                                let seq = self.background_import_seq;
                                                let tx = self.tx.clone();
                                                let settings_path = self.settings_path.clone();
                                                std::thread::spawn(move || {
                                                    if let Some(path) = rfd::FileDialog::new()
                                                        .add_filter(
                                                            "Images",
                                                            &["png", "jpg", "jpeg", "webp"],
                                                        )
                                                        .pick_file()
                                                    {
                                                        let _ = tx.send(
                                                            Event::BackgroundImportStarted(seq),
                                                        );
                                                        let result =
                                                            settings::prepare_background_import(
                                                                &path,
                                                                settings_path.as_deref(),
                                                            );
                                                        let _ = tx.send(
                                                            Event::BackgroundImported(seq, result),
                                                        );
                                                    }
                                                });
                                            }
                                            if ui
                                                .add_enabled(
                                                    has_bg_setting || self.background_importing,
                                                    egui::Button::new("Clear").min_size(Vec2::new(
                                                        0.0,
                                                        theme::BUTTON_HEIGHT,
                                                    )),
                                                )
                                                .clicked()
                                            {
                                                self.clear_background_image();
                                            }
                                        },
                                    );
                                    ui.allocate_ui_with_layout(
                                        Vec2::new(ui.available_width(), 14.0),
                                        egui::Layout::left_to_right(egui::Align::Center),
                                        |ui| {
                                            if self.background_importing {
                                                ui.label(
                                                    egui::RichText::new(
                                                        "Importing background image…",
                                                    )
                                                    .small()
                                                    .color(tokens.text_muted),
                                                );
                                            } else if let Some(err) = &self.background_error {
                                                ui.label(
                                                    egui::RichText::new(err)
                                                        .small()
                                                        .color(tokens.error_fg),
                                                );
                                            } else if let Some(name) = &self.background_image {
                                                if self.background_texture.is_some() {
                                                    ui.label(
                                                        egui::RichText::new(format!(
                                                            "Active: {name}"
                                                        ))
                                                        .small()
                                                        .color(tokens.text_muted),
                                                    );
                                                } else {
                                                    ui.label(
                                                        egui::RichText::new(
                                                            "Saved background image is missing or unreadable",
                                                        )
                                                        .small()
                                                        .color(tokens.error_fg),
                                                    );
                                                }
                                            } else {
                                                ui.label(
                                                    egui::RichText::new("Default solid theme")
                                                        .small()
                                                        .color(tokens.text_muted),
                                                );
                                            }
                                        },
                                    );
                                },
                            );
                        },
                    );
                    ui.add_space(theme::SPACE_XS);
                    ui.add_enabled_ui(has_bg_texture, |ui| {
                        let label_width = 108.0;
                        ui.allocate_ui_with_layout(
                            Vec2::new(ui.available_size_before_wrap().x, theme::BUTTON_HEIGHT),
                            egui::Layout::left_to_right(egui::Align::Center),
                            |ui| {
                                ui.allocate_ui_with_layout(
                                    Vec2::new(label_width, theme::BUTTON_HEIGHT),
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            egui::RichText::new("Overlay opacity")
                                                .color(tokens.text_muted),
                                        );
                                    },
                                );
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
                                            egui::Slider::new(
                                                &mut self.background_opacity,
                                                settings::BACKGROUND_OPACITY_RANGE,
                                            )
                                            .show_value(false)
                                            .trailing_fill(true)
                                            .handle_shape(egui::style::HandleShape::Rect {
                                                aspect_ratio: 1.0,
                                            }),
                                        )
                                        .changed()
                                    })
                                    .inner;
                                if slider_changed {
                                    self.persisted.background_opacity = self.background_opacity;
                                    self.settings_dirty = true;
                                }
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.0}%",
                                        self.background_opacity * 100.0
                                    ))
                                    .color(tokens.text_muted),
                                );
                            },
                        );
                    });
                    ui.add_space(theme::SPACE_SM);
                    ui.label(egui::RichText::new("Equation canvas").strong());
                    ui.add_space(theme::SPACE_XS);
                    if Self::draw_canvas_preset_picker(
                        ui,
                        "Light mode canvas",
                        theme::LIGHT_CANVAS_PRESETS,
                        false,
                        &mut self.canvas_light,
                    ) {
                        self.persisted.canvas_light = self.canvas_light;
                        self.settings_dirty = true;
                    }
                    ui.add_space(theme::SPACE_SM);
                    if Self::draw_canvas_preset_picker(
                        ui,
                        "Dark mode canvas",
                        theme::DARK_CANVAS_PRESETS,
                        true,
                        &mut self.canvas_dark,
                    ) {
                        self.persisted.canvas_dark = self.canvas_dark;
                        self.settings_dirty = true;
                    }
                });
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
                            if Self::with_selectable_style(ui, selected, |ui| {
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
            });
    }

    /// Renders the About tab showing version, logo, author, license, and repository link.
    pub(super) fn draw_about_tab(&mut self, ui: &mut egui::Ui) {
        let tokens = self.theme_tokens(ui);
        self.draw_back_to_editor_button(ui);
        let author_icon = self.ui_icon(ui.ctx(), "user", UI_ICON_SIZE, tokens.text_muted);
        let license_icon = self.ui_icon(ui.ctx(), "license", UI_ICON_SIZE, tokens.text_muted);
        let github_icon = self.ui_icon(ui.ctx(), "brand-github", UI_ICON_SIZE, tokens.text_muted);
        let logo_id = self
            .logo_texture
            .get_or_insert_with(|| {
                let bytes = include_bytes!("../../../assets/icons/icon.png");
                let pixels = image::load_from_memory(bytes)
                    .expect("bundled icon is valid")
                    .to_rgba8();
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [pixels.width() as usize, pixels.height() as usize],
                    pixels.as_raw(),
                );
                ui.ctx()
                    .load_texture("eqntide-logo", image, egui::TextureOptions::LINEAR)
            })
            .id();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                Self::full_width_card(ui, tokens, |ui| {
                    const ABOUT_LOGO_SIZE: f32 = 72.0;
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::Image::new((logo_id, Vec2::splat(ABOUT_LOGO_SIZE)))
                                .corner_radius(theme::SURFACE_CORNER_RADIUS),
                        );
                        ui.add_space(theme::SPACE_LG);
                        let title = concat!("EqnTide ", env!("CARGO_PKG_VERSION"));
                        let desc = env!("CARGO_PKG_DESCRIPTION");
                        let desc = desc.strip_prefix("EqnTide: ").unwrap_or(desc);
                        let avail_w = ui.available_width().max(1.0);
                        let heading_font = egui::TextStyle::Heading.resolve(ui.style());
                        let body_font = egui::TextStyle::Body.resolve(ui.style());
                        let title_h = ui.fonts_mut(|fonts| {
                            fonts
                                .layout(
                                    title.to_owned(),
                                    heading_font,
                                    tokens.text_primary,
                                    avail_w,
                                )
                                .size()
                                .y
                        });
                        let desc_h = ui.fonts_mut(|fonts| {
                            fonts
                                .layout(desc.to_owned(), body_font, tokens.text_primary, avail_w)
                                .size()
                                .y
                        });
                        let content_h = title_h + ui.spacing().item_spacing.y + desc_h;
                        let top_pad = ((ABOUT_LOGO_SIZE - content_h) * 0.5).max(0.0);
                        ui.vertical(|ui| {
                            ui.add_space(top_pad);
                            ui.heading(title);
                            ui.label(desc);
                        });
                    });
                    ui.add_space(theme::SPACE_XL);
                    ui.separator();
                    ui.add_space(theme::SPACE_MD);
                    egui::Grid::new("about-info")
                        .num_columns(2)
                        .spacing([theme::SPACE_XL, theme::SPACE_MD])
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.add(author_icon);
                                ui.strong("Author:");
                            });
                            ui.label(env!("CARGO_PKG_AUTHORS"));
                            ui.end_row();

                            ui.horizontal(|ui| {
                                ui.add(license_icon);
                                ui.strong("License:");
                            });
                            ui.label(env!("CARGO_PKG_LICENSE"));
                            ui.end_row();

                            ui.horizontal(|ui| {
                                ui.add(github_icon);
                                ui.strong("GitHub:");
                            });
                            ui.hyperlink(env!("CARGO_PKG_REPOSITORY"));
                            ui.end_row();
                        });
                });
            });
    }

    /// Renders the modal custom matrix configuration dialog when open.
    pub(super) fn draw_matrix_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut settings) = self.matrix_dialog.take() else {
            return;
        };
        if self.pending.is_some() || self.tab != 0 {
            return;
        }
        let mut insert = false;
        let response = egui::Modal::new(egui::Id::unique("custom-matrix-dialog")).show(ctx, |ui| {
            ui.set_min_width(330.0);
            ui.heading("Custom matrix");
            ui.add_space(6.0);
            egui::Grid::new("matrix-settings")
                .spacing([16.0, 10.0])
                .show(ui, |ui| {
                    ui.label("Rows");
                    ui.add(egui::DragValue::new(&mut settings.rows).range(1..=20));
                    ui.end_row();
                    ui.label("Columns");
                    ui.add(egui::DragValue::new(&mut settings.columns).range(1..=20));
                    ui.end_row();
                    ui.label("Alignment");
                    egui::ComboBox::from_id_salt("matrix-alignment")
                        .selected_text(match settings.alignment {
                            "left" => "Left",
                            "right" => "Right",
                            _ => "Center",
                        })
                        .show_ui(ui, |ui| {
                            for (value, label) in
                                [("left", "Left"), ("center", "Center"), ("right", "Right")]
                            {
                                ui.selectable_value(&mut settings.alignment, value, label);
                            }
                        });
                    ui.end_row();
                    ui.label("Delimiters");
                    const FENCES: &[(&str, &str)] = &[
                        ("none", "None"),
                        ("parentheses", "Parentheses ( )"),
                        ("brackets", "Brackets [ ]"),
                        ("braces", "Braces { }"),
                        ("bars", "Bars | |"),
                        ("double-bars", "Double bars ‖ ‖"),
                        ("left-brace", "Left brace {"),
                        ("left-bracket", "Left bracket ["),
                    ];
                    egui::ComboBox::from_id_salt("matrix-fence")
                        .selected_text(
                            FENCES
                                .iter()
                                .find(|(value, _)| *value == settings.fence)
                                .map_or("Parentheses ( )", |&(_, label)| label),
                        )
                        .show_ui(ui, |ui| {
                            for &(value, label) in FENCES {
                                ui.selectable_value(&mut settings.fence, value, label);
                            }
                        });
                    ui.end_row();
                });
            ui.add_space(theme::SPACE_MD);
            ui.horizontal(|ui| {
                insert = Self::primary_button(ui, true, "Insert matrix").clicked();
                if Self::standard_button(ui, "Cancel").clicked() {
                    ui.close();
                }
            });
        });
        if insert {
            self.insert_custom_matrix(&settings);
            self.pending_focus_restore = Some(self.active_editor_focus);
        } else if !response.should_close() {
            self.matrix_dialog = Some(settings);
        } else {
            self.pending_focus_restore = Some(self.active_editor_focus);
        }
    }

    pub(super) fn insert_custom_matrix(&mut self, settings: &MatrixDialog) {
        let matrix = eqntide::catalog::matrix(
            settings.rows,
            settings.columns,
            settings.alignment,
            settings.fence,
        );
        self.command_input = None;
        self.latex_groups.clear();
        self.expects_latex_group = false;
        self.editor.insert_template(&matrix);
        self.mark_canvas_changed();
    }

    /// Renders unsaved-changes / active-batch confirmation modals or auto-cancels untouched sessions.
    pub(super) fn draw_dialogs(&mut self, ctx: &egui::Context) {
        // An Office session alone is not an unsaved change. Cancel an
        // untouched session before closing/switching so its add-in wait ends
        // without replacing the equation in the document.
        if self.pending.is_some() && !self.is_dirty() && !self.batch_running {
            if !self.busy {
                self.finish_session(false);
            }
            return;
        }
        if self.pending.is_some() {
            egui::Window::new("Finish current equation")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    if self.is_dirty() {
                        ui.label("Save your changes, discard them, or continue editing.");
                    }
                    if self.batch_running {
                        ui.label("A document is being converted. Stop the batch and wait before closing.");
                        if Self::standard_button(ui, "Stop batch").clicked() {
                            self.stop.store(true, Ordering::Relaxed);
                        }
                    }
                    ui.add_enabled_ui(!self.busy && !self.batch_running, |ui| {
                        ui.horizontal(|ui| {
                            if Self::primary_button(ui, true, "Save").clicked() {
                                self.finish_session(true);
                            }
                            if Self::standard_button(ui, "Discard / cancel session").clicked() {
                                self.finish_session(false);
                            }
                            if Self::standard_button(ui, "Continue editing").clicked() {
                                self.pending = None;
                            }
                        });
                    });
                });
        }
    }

    pub(super) fn draw_canvas_preset_picker(
        ui: &mut egui::Ui,
        title: &str,
        presets: &[CanvasPreset],
        dark_mode_preview: bool,
        current: &mut CanvasPreset,
    ) -> bool {
        let tokens = ThemeTokens::from_ui(ui);
        ui.label(egui::RichText::new(title).color(tokens.text_muted));
        let mut changed = false;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(theme::SPACE_MD);
            for &preset in presets {
                let selected = *current == preset;
                let pal = preset.palette(dark_mode_preview);
                let response = Self::with_selectable_style(ui, selected, |ui| {
                    let (rect, response) =
                        ui.allocate_exact_size(Vec2::new(116.0, 32.0), egui::Sense::click());
                    if ui.is_rect_visible(rect) {
                        let visuals = ui.style().interact(&response);
                        let bg = if selected {
                            if response.hovered() {
                                tokens.selected_hover_bg
                            } else {
                                tokens.selected_bg
                            }
                        } else {
                            visuals.bg_fill
                        };
                        let stroke = if selected {
                            Stroke::new(1.0, tokens.selected_border)
                        } else {
                            visuals.bg_stroke
                        };
                        let fg = if selected {
                            tokens.selected_fg
                        } else {
                            visuals.text_color()
                        };
                        ui.painter().rect(
                            rect,
                            theme::CONTROL_CORNER_RADIUS,
                            bg,
                            stroke,
                            egui::StrokeKind::Inside,
                        );
                        let swatch = Rect::from_min_size(
                            Pos2::new(rect.left() + theme::SPACE_MD, rect.center().y - 9.0),
                            Vec2::splat(18.0),
                        );
                        ui.painter().rect(
                            swatch,
                            theme::CONTROL_CORNER_RADIUS,
                            pal.bg,
                            Stroke::new(1.0, pal.border),
                            egui::StrokeKind::Inside,
                        );
                        ui.painter().text(
                            swatch.center(),
                            egui::Align2::CENTER_CENTER,
                            "x",
                            egui::FontId::proportional(10.0),
                            pal.ink,
                        );
                        ui.painter().text(
                            Pos2::new(swatch.right() + theme::SPACE_SM, rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            preset.label(),
                            egui::TextStyle::Body.resolve(ui.style()),
                            fg,
                        );
                    }
                    response
                });
                response.widget_info(|| {
                    egui::WidgetInfo::selected(
                        egui::Role::RadioButton,
                        ui.is_enabled(),
                        selected,
                        preset.label(),
                    )
                });
                if response.clicked() && *current != preset {
                    *current = preset;
                    changed = true;
                }
            }
        });
        changed
    }

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
        self.canvas_light = settings.canvas_light;
        self.canvas_dark = settings.canvas_dark;
        self.background_image = settings.background_image.clone();
        self.background_opacity = settings.background_opacity;
        self.background_texture = None;
        self.software_bg = None;
        self.background_load_pending = self.background_image.is_some();
        self.background_error = None;
        self.startup_focus = settings.startup_focus;
        self.active_editor_focus = settings.startup_focus;
        self.pending_focus_restore = Some(settings.startup_focus);
        if let Some(ctx) = egui_ctx {
            ctx.set_theme(settings.theme.to_egui());
        }
        self.persisted = settings;
        self.settings_dirty = false;
    }

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

    pub(super) fn theme_tokens(&self, ui: &egui::Ui) -> ThemeTokens {
        ThemeTokens::for_mode_with_background(
            ui.visuals().dark_mode,
            self.background_texture.is_some(),
            self.background_opacity,
        )
    }

    pub(super) fn active_canvas_palette(&self, dark_mode: bool) -> CanvasPalette {
        let preset = if dark_mode {
            self.canvas_dark
        } else {
            self.canvas_light
        };
        preset.palette(dark_mode)
    }

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

    /// Returns the CPU background bitmap and scrim parameters for [`crate::software_painter::SoftwarePainter`].
    pub(crate) fn software_background(
        &self,
        visuals: &egui::Visuals,
    ) -> Option<crate::software_painter::SoftwareBackgroundLayer> {
        if !self.native_software_background {
            return None;
        }
        let image = self.software_bg.as_ref()?;
        Some(crate::software_painter::SoftwareBackgroundLayer {
            image: Arc::clone(image),
            scrim: ThemeTokens::background_scrim(visuals.dark_mode, self.background_opacity),
            base_bg: ThemeTokens::for_mode(visuals.dark_mode).app_bg,
        })
    }

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
            }
            Err(err) => {
                eprintln!("Failed to load managed background {filename}: {err}");
                self.background_texture = None;
                self.software_bg = None;
            }
        }
    }

    pub(super) fn apply_prepared_background(
        &mut self,
        bg: settings::PreparedBackground,
        ctx: &egui::Context,
    ) {
        let prev = self.background_image.take();
        self.upload_background_image(ctx, bg.image);
        self.background_image = Some(bg.filename.clone());
        self.persisted.background_image = Some(bg.filename.clone());
        self.background_load_pending = false;
        self.background_error = None;
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

    pub(super) fn clear_background_image(&mut self) {
        self.background_import_seq = self.background_import_seq.wrapping_add(1);
        self.background_importing = false;
        if let (Some(settings_path), Some(filename)) = (
            self.settings_path.as_deref(),
            self.background_image.as_deref(),
        ) {
            settings::remove_managed_background_file(settings_path, filename);
        }
        self.background_image = None;
        self.background_texture = None;
        self.software_bg = None;
        self.background_load_pending = false;
        self.background_error = None;
        self.persisted.background_image = None;
        self.settings_dirty = true;
        self.flush_settings();
    }

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

    pub(super) fn paint_app_background(&self, ui: &egui::Ui) {
        if self.native_software_background {
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
