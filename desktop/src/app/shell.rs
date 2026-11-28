//! Main window chrome: shell layout, navigation sidebar, footer controls, status bar, and window sizing.

use super::{App, Intent};
use crate::{
    palette, runtime, settings, theme,
    ui::{self, icons::UI_ICON_SIZE},
};
use eframe::egui::{self, Stroke, Vec2};

/// Minimum logical window size (width, height in DIP) enforced on native and Wayland viewports.
pub(crate) const MIN_WINDOW_SIZE: [f32; 2] = [1056.0, 560.0];
/// Shared vertical content margin (in DIP) applied across shell panels.
pub(super) const CONTENT_PADDING: i8 = theme::SPACE_MD as i8;
/// Logical dimensions of a single palette template button tile.
pub(super) const PALETTE_BUTTON_SIZE: Vec2 = Vec2::new(46.0, 32.0);
/// Spacing between adjacent palette tiles and action buttons.
pub(super) const PALETTE_GAP: f32 = theme::SPACE_XS;
/// Fixed number of template rows displayed inside the top palette ribbon.
pub(super) const PALETTE_ROWS: usize = 3;
/// Logical DIP size for category button icons in the left sidebar.
pub(super) const SIDEBAR_CATEGORY_ICON_SIZE: f32 = 22.0;
/// Total vertical height of the top palette ribbon panel including padding.
pub(super) const PALETTE_HEIGHT: f32 = PALETTE_ROWS as f32 * PALETTE_BUTTON_SIZE.y
    + (PALETTE_ROWS - 1) as f32 * PALETTE_GAP
    + 2.0 * CONTENT_PADDING as f32;
const FONT_SIZE_CONTROL_WIDTH: f32 = 96.0;
const ZOOM_CONTROL_WIDTH: f32 = 96.0;
const FOOTER_CONTROL_HEIGHT: f32 = theme::BUTTON_HEIGHT;

impl App {
    /// Renders the window chrome (status bar, sidebar, palette ribbon, footer) and active tab content.
    pub(super) fn draw_shell(&mut self, ui: &mut egui::Ui) {
        self.save_clicked = false;
        self.keyboard_handled = false;
        if self.tab == 0
            && !self.draft_dirty
            && !self.busy
            && self.pending.is_none()
            && self.matrix_dialog.is_none()
            && self.shortcut_conflict.is_none()
            && ui.is_enabled()
            && ui.memory(|mem| mem.has_focus(egui::Id::unique("equation-canvas")))
            && !ui.input(|input| {
                input.pointer.any_pressed()
                    || input.pointer.any_released()
                    || input.pointer.any_down()
            })
        {
            self.handle_canvas_keyboard(ui);
            self.keyboard_handled = true;
        }
        self.sync_render(ui.ctx());
        self.ensure_background_texture(ui.ctx());
        self.sync_accent_style(ui.ctx());
        self.paint_app_background(ui);
        let tokens = self.theme_tokens(ui);
        let status_rect =
            egui::Panel::bottom("app-status")
                .exact_size(32.0)
                .frame(egui::Frame::NONE.fill(tokens.panel_bg).inner_margin(
                    egui::Margin::symmetric(theme::SPACE_LG as i8, theme::SPACE_XS as i8),
                ))
                .show_separator_line(false)
                .show(ui, |ui| self.draw_status_bar(ui))
                .response
                .rect;
        let sidebar_rect = if self.tab == 0 {
            Some(
                egui::Panel::left("app-sidebar")
                    .exact_size(174.0)
                    .resizable(false)
                    .frame(egui::Frame::NONE.fill(tokens.panel_bg).inner_margin(
                        egui::Margin::symmetric(theme::SPACE_LG as i8, CONTENT_PADDING),
                    ))
                    .show_separator_line(false)
                    .show(ui, |ui| {
                        ui.add_enabled_ui(
                            !self.busy
                                && self.pending.is_none()
                                && self.matrix_dialog.is_none()
                                && self.shortcut_conflict.is_none(),
                            |ui| self.draw_sidebar(ui),
                        );
                    })
                    .response
                    .rect,
            )
        } else {
            None
        };
        let footer_rect = if self.tab == 0 || self.session.is_some() {
            Some(
                egui::Panel::bottom("app-footer")
                    .exact_size(56.0)
                    .frame(egui::Frame::NONE.fill(tokens.panel_bg).inner_margin(
                        egui::Margin::symmetric(theme::SPACE_LG as i8, CONTENT_PADDING),
                    ))
                    .show_separator_line(false)
                    .show(ui, |ui| self.draw_footer(ui))
                    .response
                    .rect,
            )
        } else {
            None
        };
        let palette_rect = if self.tab == 0 {
            Some(
                egui::Panel::top("templates-panel")
                    .exact_size(PALETTE_HEIGHT)
                    .frame(egui::Frame::NONE.fill(tokens.panel_bg).inner_margin(
                        egui::Margin::symmetric(theme::SPACE_LG as i8, CONTENT_PADDING),
                    ))
                    .show_separator_line(false)
                    .show(ui, |ui| {
                        ui.add_enabled_ui(
                            !self.busy
                                && self.pending.is_none()
                                && self.matrix_dialog.is_none()
                                && self.shortcut_conflict.is_none(),
                            |ui| self.draw_palette(ui),
                        );
                    })
                    .response
                    .rect,
            )
        } else {
            None
        };
        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(tokens.app_bg)
                    .inner_margin(egui::Margin::symmetric(
                        theme::SPACE_LG as i8,
                        CONTENT_PADDING,
                    )),
            )
            .show(ui, |ui| {
                ui.add_enabled_ui(
                    !self.busy
                        && self.pending.is_none()
                        && self.matrix_dialog.is_none()
                        && self.shortcut_conflict.is_none(),
                    |ui| match self.tab {
                        0 => self.draw_editor(ui),
                        1 => self.draw_batch_tab(ui),
                        2 => self.draw_settings_tab(ui),
                        3 => self.draw_about_tab(ui),
                        _ => self.draw_editor(ui),
                    },
                );
            });
        let divider_stroke = Stroke::new(1.0, tokens.border);
        ui.painter().hline(
            status_rect.x_range(),
            status_rect.min.y + 0.5,
            divider_stroke,
        );
        if let Some(rect) = sidebar_rect {
            ui.painter()
                .vline(rect.max.x - 0.5, rect.y_range(), divider_stroke);
        }
        if let Some(rect) = palette_rect {
            ui.painter()
                .hline(rect.x_range(), rect.max.y - 0.5, divider_stroke);
        }
        if let Some(rect) = footer_rect {
            ui.painter()
                .hline(rect.x_range(), rect.min.y + 0.5, divider_stroke);
        }
    }

    /// Renders the left navigation sidebar with palette categories and secondary view tabs.
    pub(super) fn draw_sidebar(&mut self, ui: &mut egui::Ui) {
        const SIDEBAR_GAP: f32 = theme::SPACE_SM;
        const SIDEBAR_BUTTON_HEIGHT: f32 = 42.0;
        let tokens = self.theme_tokens(ui);
        let enabled = ui.is_enabled();
        egui::Panel::bottom("sidebar-batch")
            .exact_size(120.0)
            .frame(egui::Frame::NONE.inner_margin(egui::Margin {
                left: 0,
                right: 0,
                top: theme::SPACE_SM as i8,
                bottom: 0,
            }))
            .show_separator_line(false)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = theme::SPACE_SM;
                for (tab, icon, label) in [
                    (1, "file-type-docx", "Batch DOCX"),
                    (2, "settings", "Settings"),
                    (3, "info-circle", "About"),
                ] {
                    let selected = self.tab == tab;
                    let icon_tint = tokens.icon_tint(selected, enabled);
                    let icon_img = self.ui_icons.image(ui.ctx(), icon, UI_ICON_SIZE, icon_tint);
                    if ui::widgets::with_selectable_style(ui, tokens, selected, |ui| {
                        ui.add_sized(
                            [ui.available_width(), theme::SIDEBAR_TAB_HEIGHT],
                            egui::Button::image_and_text(icon_img, label),
                        )
                    })
                    .clicked()
                    {
                        self.tab = tab;
                    }
                }
            });
        let button_width = ((ui.available_width() - SIDEBAR_GAP) / 2.0)
            .floor()
            .max(48.0);
        let button_size = Vec2::new(button_width, SIDEBAR_BUTTON_HEIGHT);
        let mut clicked_group = None;
        egui::Grid::new("sidebar-categories")
            .num_columns(2)
            .spacing(Vec2::splat(SIDEBAR_GAP))
            .show(ui, |ui| {
                ui.spacing_mut().button_padding = Vec2::splat(theme::SPACE_XS);
                for (index, group) in palette::UI_GROUPS.iter().enumerate() {
                    let selected = self.category == group.id;
                    let icon_color = tokens.icon_tint(selected, enabled);
                    let button = match group.icons {
                        [first, second] => {
                            let left = self.ui_icons.image(
                                ui.ctx(),
                                first,
                                SIDEBAR_CATEGORY_ICON_SIZE,
                                icon_color,
                            );
                            let right = self.ui_icons.image(
                                ui.ctx(),
                                second,
                                SIDEBAR_CATEGORY_ICON_SIZE,
                                icon_color,
                            );
                            egui::Button::new((left, right)).gap(0.0)
                        }
                        [single] => {
                            let image = self.ui_icons.image(
                                ui.ctx(),
                                single,
                                SIDEBAR_CATEGORY_ICON_SIZE,
                                icon_color,
                            );
                            egui::Button::new(image)
                        }
                        _ => unreachable!("sidebar category must have 1 or 2 icons"),
                    };
                    let response = ui::widgets::with_selectable_style(ui, tokens, selected, |ui| {
                        ui.add_sized(button_size, button)
                    });
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::Role::Button, enabled, group.label)
                    });
                    if response.on_hover_text(group.label).clicked() {
                        clicked_group = Some(group.id);
                    }
                    if (index + 1) % 2 == 0 {
                        ui.end_row();
                    }
                }
            });
        if let Some(group_id) = clicked_group {
            self.category = group_id.into();
            self.persisted.category = self.category.clone();
            self.settings_dirty = true;
            self.pending_focus_restore = Some(self.active_editor_focus);
        }
    }

    /// Renders the bottom action bar with target format buttons, font size, zoom, undo/redo, and save.
    pub(super) fn draw_footer(&mut self, ui: &mut egui::Ui) {
        let tokens = self.theme_tokens(ui);
        ui.add_enabled_ui(
            !self.busy
                && self.pending.is_none()
                && self.matrix_dialog.is_none()
                && self.shortcut_conflict.is_none(),
            |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = theme::SPACE_MD;
                    ui.add_enabled_ui(!self.draft_dirty, |ui| {
                        let font = egui::TextStyle::Body.resolve(ui.style());
                        let color = ui.visuals().widgets.inactive.fg_stroke.color;
                        let caption =
                            |text: &str| egui::RichText::new(text).font(font.clone()).color(color);
                        for (choice, label) in [("mathtype", "MathType"), ("omml", "OMML")] {
                            let selected = self.target == choice;
                            let resp =
                                ui::widgets::with_selectable_style(ui, tokens, selected, |ui| {
                                    ui::widgets::standard_button(ui, label)
                                });
                            if resp.clicked() {
                                if !selected {
                                    self.target = choice.into();
                                    self.persisted.target = self.target.clone();
                                    self.settings_dirty = true;
                                }
                                self.pending_focus_restore = Some(self.active_editor_focus);
                            }
                        }
                        ui::widgets::section_separator(ui, tokens);
                        ui.label(caption("Font size"));
                        let (font_resp, font_wheel_changed) = ui::widgets::level_drag_value(
                            ui,
                            tokens,
                            "footer-font-size",
                            f64::from(4.0 / FONT_SIZE_CONTROL_WIDTH),
                            Some(1.0),
                            settings::FONT_PT_RANGE,
                            &mut self.font_pt,
                            "Click and drag left/right, or scroll to adjust",
                            |ui, font_pt, speed| {
                                ui.add(
                                    egui::DragValue::from_get_set(|value| {
                                        if let Some(value) = value {
                                            *font_pt = value.round() as f32;
                                        }
                                        f64::from(*font_pt)
                                    })
                                    .range(settings::FONT_PT_RANGE)
                                    .min_size(Vec2::new(
                                        FONT_SIZE_CONTROL_WIDTH,
                                        FOOTER_CONTROL_HEIGHT,
                                    ))
                                    .speed(speed)
                                    .max_decimals(1)
                                    .suffix(" pt"),
                                )
                            },
                        );
                        if font_resp.changed() || font_wheel_changed {
                            self.render_key += 1;
                            self.persisted.font_pt = self.font_pt;
                            self.settings_dirty = true;
                        }
                        if font_resp.drag_stopped() && !font_resp.has_focus() {
                            self.pending_focus_restore = Some(self.active_editor_focus);
                        }
                        ui.label(caption("Zoom"));
                        let (zoom_resp, zoom_wheel_changed) = ui::widgets::level_drag_value(
                            ui,
                            tokens,
                            "footer-zoom",
                            f64::from(1.0 / ZOOM_CONTROL_WIDTH),
                            Some(10.0),
                            settings::ZOOM_RANGE,
                            &mut self.zoom,
                            "Click and drag left/right, or scroll to adjust",
                            |ui, zoom, speed| {
                                ui.add(
                                    egui::DragValue::from_get_set(|value| {
                                        if let Some(value) = value {
                                            *zoom = ((value * 10.0).round() / 10.0) as f32;
                                        }
                                        f64::from(*zoom)
                                    })
                                    .range(settings::ZOOM_RANGE)
                                    .min_size(Vec2::new(ZOOM_CONTROL_WIDTH, FOOTER_CONTROL_HEIGHT))
                                    .speed(speed)
                                    .custom_formatter(|value, _| {
                                        format!("{:.0}%", (value * 100.0).round())
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
                            },
                        );
                        if zoom_resp.changed() || zoom_wheel_changed {
                            self.persisted.zoom = self.zoom;
                            self.settings_dirty = true;
                        }
                        if zoom_resp.drag_stopped() && !zoom_resp.has_focus() {
                            self.pending_focus_restore = Some(self.active_editor_focus);
                        }
                        ui::widgets::section_separator(ui, tokens);
                        let history_tint = self.theme_tokens(ui).icon_tint(false, ui.is_enabled());
                        let undo_img = self.ui_icons.image(
                            ui.ctx(),
                            "arrow-back-up",
                            UI_ICON_SIZE,
                            history_tint,
                        );
                        let undo_resp = ui.add(
                            egui::Button::new(undo_img)
                                .min_size(Vec2::new(32.0, FOOTER_CONTROL_HEIGHT)),
                        );
                        undo_resp.widget_info(|| {
                            egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), "Undo")
                        });
                        if undo_resp.on_hover_text("Undo (Ctrl+Z)").clicked() {
                            self.command_input = None;
                            self.latex_groups.clear();
                            self.expects_latex_group = false;
                            self.editor.undo();
                            self.mark_canvas_changed();
                            self.pending_focus_restore = Some(self.active_editor_focus);
                        }
                        let redo_img = self.ui_icons.image(
                            ui.ctx(),
                            "arrow-forward-up",
                            UI_ICON_SIZE,
                            history_tint,
                        );
                        let redo_resp = ui.add(
                            egui::Button::new(redo_img)
                                .min_size(Vec2::new(32.0, FOOTER_CONTROL_HEIGHT)),
                        );
                        redo_resp.widget_info(|| {
                            egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), "Redo")
                        });
                        if redo_resp.on_hover_text("Redo (Ctrl+Y)").clicked() {
                            self.command_input = None;
                            self.latex_groups.clear();
                            self.expects_latex_group = false;
                            self.editor.redo();
                            self.mark_canvas_changed();
                            self.pending_focus_restore = Some(self.active_editor_focus);
                        }
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        self.draw_save_button(ui);
                    });
                });
            },
        );
    }

    /// Renders the bottom status strip showing bridge state, queued sessions, errors, and warnings.
    pub(super) fn draw_status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_centered(|ui| {
            let dot_color = self.theme_tokens(ui).accent;
            let (dot_rect, dot_response) =
                ui.allocate_exact_size(Vec2::splat(12.0), egui::Sense::hover());
            if ui.is_rect_visible(dot_rect) {
                let center = dot_rect.center();
                if self.bridge_ready {
                    ui.painter().circle_filled(center, 4.0, dot_color);
                } else {
                    ui.painter()
                        .circle_stroke(center, 3.5, Stroke::new(1.5, dot_color));
                }
            }
            dot_response.on_hover_text(&self.bridge_status);
            if !self.bridge_ready && ui::widgets::compact_button(ui, "Retry").clicked() {
                runtime::start_bridge(self.tx.clone());
                self.pending_focus_restore = Some(self.active_editor_focus);
            }
            if !self.queue.is_empty()
                && ui::widgets::compact_button(ui, &format!("Next session ({})", self.queue.len()))
                    .clicked()
            {
                self.pending = Some(Intent::Next);
                self.pending_focus_restore = Some(self.active_editor_focus);
            }
            let target_warning = if self.tab == 0 {
                let cache_hit = self.warning_cache.as_ref().is_some_and(
                    |(cached_key, cached_gen, cached_target, _)| {
                        *cached_key == self.render_key
                            && *cached_gen == self.latex_generation
                            && *cached_target == self.target
                    },
                );
                if !cache_hit {
                    let math = self.editor.math();
                    let computed = if self.target == "mathtype" {
                        eqntide::check_mathtype_unsupported(&math)
                            .or_else(|| eqntide::check_mathtype_unsupported_latex(&self.latex))
                            .map(|unsupported| {
                                format!("Warning: {unsupported} is not supported in MathType.")
                            })
                            .or_else(|| {
                                eqntide::check_omml_unsupported(&math)
                                    .or_else(|| eqntide::check_omml_unsupported_latex(&self.latex))
                                    .map(|unsupported| {
                                        format!(
                                            "Warning: {unsupported} is not supported in OMML (MathType target active)."
                                        )
                                    })
                            })
                    } else {
                        eqntide::check_omml_unsupported(&math)
                            .or_else(|| eqntide::check_omml_unsupported_latex(&self.latex))
                            .map(|unsupported| {
                                if eqntide::check_mathtype_unsupported(&math)
                                    .or_else(|| eqntide::check_mathtype_unsupported_latex(&self.latex))
                                    .is_some()
                                {
                                    format!("Warning: {unsupported} is not supported in OMML or MathType.")
                                } else {
                                    format!(
                                        "Warning: {unsupported} is not supported in OMML. Switch target to MathType to insert."
                                    )
                                }
                            })
                    };
                    self.warning_cache = Some((
                        self.render_key,
                        self.latex_generation,
                        self.target.clone(),
                        computed,
                    ));
                }
                self.warning_cache
                    .as_ref()
                    .and_then(|(_, _, _, cached_msg)| cached_msg.clone())
            } else {
                None
            };
            let active_latex_error = if self.tab == 0 {
                self.command_error
                    .as_deref()
                    .filter(|_| self.command_input.is_some())
                    .or(self.latex_error.as_deref())
            } else {
                None
            };
            if let Some(latex_error) = active_latex_error {
                let error = format!("LaTeX error: {latex_error}");
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(&error).color(ui.visuals().error_fg_color),
                    )
                    .truncate(),
                )
                .on_hover_text(error);
            } else if self.tab == 0 && self.draft_dirty {
                ui.label("Updating equation…");
            } else if self.background_importing {
                ui.label("Importing background image…");
            } else if let Some(warning) = target_warning {
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(&warning).color(ui.visuals().error_fg_color),
                    )
                    .truncate(),
                )
                .on_hover_text(warning);
            } else if !self.status.is_empty() {
                ui.add(egui::Label::new(&self.status).truncate())
                    .on_hover_text(&self.status);
            }
        });
    }

    /// Enforces [`MIN_WINDOW_SIZE`] when viewport size or maximize/fullscreen state changes.
    pub(super) fn enforce_window_minimum_if_needed(&mut self, ctx: &egui::Context) {
        let state = ctx.input(|input| {
            let viewport = input.viewport();
            let size = viewport.inner_rect.unwrap_or(input.viewport_rect()).size();
            let fullscreen_or_max =
                viewport.maximized == Some(true) || viewport.fullscreen == Some(true);
            (size, fullscreen_or_max)
        });
        if self
            .last_enforced_viewport
            .is_some_and(|(prev_size, prev_flag)| {
                prev_flag == state.1
                    && (prev_size.x - state.0.x).abs() < 0.5
                    && (prev_size.y - state.0.y).abs() < 0.5
            })
        {
            return;
        }
        self.last_enforced_viewport = Some(state);
        Self::enforce_window_minimum(ctx);
    }

    /// Disables built-in global `egui` UI zoom and quit shortcuts across all backends.
    pub(super) fn configure_egui_options(ctx: &egui::Context) {
        ctx.options_mut(|options| {
            options.zoom_with_keyboard = false;
            options.quit_shortcuts.clear();
        });
    }

    /// Applies [`MIN_WINDOW_SIZE`] viewport commands on non-maximized windows.
    pub(super) fn enforce_window_minimum(ctx: &egui::Context) {
        let request = ctx.input(|input| {
            let viewport = input.viewport();
            if viewport.maximized == Some(true) || viewport.fullscreen == Some(true) {
                return None;
            }
            // Wayland does not provide native inner_rect; the UI viewport still has its size.
            let size = viewport.inner_rect.unwrap_or(input.viewport_rect()).size();
            // Respect a physically smaller display, avoiding resize loops with the compositor.
            let minimum =
                Vec2::from(MIN_WINDOW_SIZE).min(viewport.monitor_size.unwrap_or(Vec2::INFINITY));
            let required = size.max(minimum);
            (required.x > size.x + 0.5 || required.y > size.y + 0.5).then_some((minimum, required))
        });
        if let Some((minimum, required)) = request {
            ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(minimum));
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(required));
        }
    }
}
