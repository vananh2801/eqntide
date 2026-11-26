//! Desktop matrix configuration and session confirmation dialogs.

use super::{App, MatrixDialog};
use crate::theme::{self, ThemeTokens};
use eframe::egui::{self, Vec2};
use std::sync::atomic::Ordering;

impl App {
    /// Inserts a custom matrix template into the editor and marks the canvas dirty.
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

    /// Renders the modal custom matrix configuration dialog when open.
    pub(super) fn draw_matrix_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut settings) = self.matrix_dialog.take() else {
            return;
        };
        if self.pending.is_some() || self.tab != 0 {
            return;
        }
        let tokens = ThemeTokens::for_mode_with_background_and_accent(
            ctx.theme() == egui::Theme::Dark,
            self.is_image_background_active(),
            self.background_opacity,
            self.active_accent_seed(),
        );
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
                                let selected = settings.alignment == value;
                                Self::with_selectable_style(ui, tokens, selected, |ui| {
                                    ui.selectable_value(&mut settings.alignment, value, label)
                                });
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
                                let selected = settings.fence == value;
                                Self::with_selectable_style(ui, tokens, selected, |ui| {
                                    ui.selectable_value(&mut settings.fence, value, label)
                                });
                            }
                        });
                    ui.end_row();
                });
            ui.add_space(theme::SPACE_MD);
            ui.horizontal(|ui| {
                insert = Self::primary_button_with(ui, tokens, true, |ui| {
                    ui.add(
                        egui::Button::new("Insert matrix")
                            .min_size(Vec2::new(0.0, theme::BUTTON_HEIGHT)),
                    )
                })
                .clicked();
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
            let tokens = ThemeTokens::for_mode_with_background_and_accent(
                ctx.theme() == egui::Theme::Dark,
                self.is_image_background_active(),
                self.background_opacity,
                self.active_accent_seed(),
            );
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
                            if Self::primary_button_with(ui, tokens, true, |ui| {
                                ui.add(
                                    egui::Button::new("Save")
                                        .min_size(Vec2::new(0.0, theme::BUTTON_HEIGHT)),
                                )
                            })
                            .clicked()
                            {
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
}
