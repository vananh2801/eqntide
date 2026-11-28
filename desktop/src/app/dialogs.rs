//! Desktop matrix configuration, shortcut conflict, and session confirmation dialogs.

use super::{App, MatrixDialog};
use crate::{
    shortcuts::{self, ShortcutConflict},
    theme::{self, ThemeTokens},
    ui::{modal, widgets},
};
use eframe::egui;
use std::sync::atomic::Ordering;

const MATRIX_MIN_INNER_WIDTH: f32 = 260.0;
const MATRIX_COMBO_WIDTH: f32 = 156.0;
const CONFLICT_MIN_INNER_WIDTH: f32 = 340.0;

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
        let modal_id = egui::Id::unique("custom-matrix-dialog");
        let Some(mut settings) = self.matrix_dialog.take() else {
            modal::clear_focus_state(ctx, modal_id);
            return;
        };
        if self.pending.is_some() || self.tab != 0 {
            modal::clear_focus_state(ctx, modal_id);
            return;
        }
        let previous_state = modal::read_focus_state(ctx, modal_id);
        if modal::handle_escape(ctx, previous_state.as_ref(), true) {
            modal::clear_focus_state(ctx, modal_id);
            self.pending_focus_restore = Some(self.active_editor_focus);
            return;
        }
        modal::apply_focus_navigation(ctx, previous_state.as_ref());

        let tokens = ThemeTokens::for_mode_with_background_and_accent(
            ctx.theme() == egui::Theme::Dark,
            self.is_image_background_active(),
            self.background_opacity,
            self.active_accent_seed(),
        );
        let frame = modal::modal_frame(&ctx.style_of(ctx.theme()));
        let default_outer_width =
            MATRIX_MIN_INNER_WIDTH + f32::from(modal::MODAL_INNER_MARGIN) * 2.0;
        let mut insert = false;
        let mut cancel = false;
        let mut rows_id = egui::Id::NULL;
        let mut cols_id = egui::Id::NULL;
        let mut align_id = egui::Id::NULL;
        let mut fence_id = egui::Id::NULL;
        let mut insert_id = egui::Id::NULL;
        let mut cancel_id = egui::Id::NULL;

        let response = egui::Modal::new(modal_id)
            .frame(frame)
            .area(egui::Modal::default_area(modal_id).default_width(default_outer_width))
            .show(ctx, |ui| {
                ui.set_min_width(MATRIX_MIN_INNER_WIDTH);
                ui.vertical_centered(|ui| {
                    ui.heading("Custom matrix");
                });
                ui.add_space(theme::SPACE_LG);
                let grid_resp = egui::Grid::new("matrix-settings")
                    .spacing([32.0, 10.0])
                    .show(ui, |ui| {
                        ui.label("Rows");
                        rows_id = ui
                            .add(egui::DragValue::new(&mut settings.rows).range(1..=20))
                            .id;
                        ui.end_row();
                        ui.label("Columns");
                        cols_id = ui
                            .add(egui::DragValue::new(&mut settings.columns).range(1..=20))
                            .id;
                        ui.end_row();
                        ui.label("Alignment");
                        align_id = egui::ComboBox::from_id_salt("matrix-alignment")
                            .width(MATRIX_COMBO_WIDTH)
                            .selected_text(match settings.alignment {
                                "left" => "Left",
                                "right" => "Right",
                                _ => "Center",
                            })
                            .popup_style(tokens.combo_popup_style())
                            .show_ui(ui, |ui| {
                                for (value, label) in
                                    [("left", "Left"), ("center", "Center"), ("right", "Right")]
                                {
                                    let selected = settings.alignment == value;
                                    widgets::with_selectable_style(ui, tokens, selected, |ui| {
                                        ui.selectable_value(&mut settings.alignment, value, label)
                                    });
                                }
                            })
                            .response
                            .id;
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
                        fence_id = egui::ComboBox::from_id_salt("matrix-fence")
                            .width(MATRIX_COMBO_WIDTH)
                            .selected_text(
                                FENCES
                                    .iter()
                                    .find(|(value, _)| *value == settings.fence)
                                    .map_or("Parentheses ( )", |&(_, label)| label),
                            )
                            .popup_style(tokens.combo_popup_style())
                            .show_ui(ui, |ui| {
                                for &(value, label) in FENCES {
                                    let selected = settings.fence == value;
                                    widgets::with_selectable_style(ui, tokens, selected, |ui| {
                                        ui.selectable_value(&mut settings.fence, value, label)
                                    });
                                }
                            })
                            .response
                            .id;
                        ui.end_row();
                    });
                ui.add_space(theme::SPACE_LG);
                let button_row_width = grid_resp.response.rect.width().max(MATRIX_MIN_INNER_WIDTH);
                modal::button_row(
                    ui,
                    button_row_width,
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        let cancel_resp = widgets::standard_button(ui, "Cancel");
                        cancel_id = cancel_resp.id;
                        cancel = cancel_resp.clicked();

                        let insert_resp =
                            widgets::primary_button(ui, tokens, true, "Insert matrix");
                        insert_id = insert_resp.id;
                        insert = insert_resp.clicked();
                    },
                );
            });

        if insert {
            self.insert_custom_matrix(&settings);
            modal::clear_focus_state(ctx, modal_id);
            self.pending_focus_restore = Some(self.active_editor_focus);
        } else if cancel {
            modal::clear_focus_state(ctx, modal_id);
            self.pending_focus_restore = Some(self.active_editor_focus);
        } else {
            let next_state = modal::ModalFocusState {
                layer_id: Some(response.response.layer_id),
                tab_ids: vec![rows_id, cols_id, align_id, fence_id, insert_id, cancel_id],
                button_ids: vec![insert_id, cancel_id],
                default_id: Some(insert_id),
            };
            modal::store_focus_state(ctx, modal_id, previous_state.is_none(), next_state);
            self.matrix_dialog = Some(settings);
        }
    }

    /// Renders the modal dialog for resolving a shortcut collision with a system or palette binding.
    pub(super) fn draw_shortcut_conflict_dialog(&mut self, ctx: &egui::Context) {
        let modal_id = egui::Id::unique("shortcut-conflict-dialog");
        let Some(dialog) = self.shortcut_conflict.take() else {
            modal::clear_focus_state(ctx, modal_id);
            return;
        };
        if self.pending.is_some() || self.tab != 2 {
            modal::clear_focus_state(ctx, modal_id);
            return;
        }

        let previous_state = modal::read_focus_state(ctx, modal_id);
        if modal::handle_escape(ctx, previous_state.as_ref(), true) {
            modal::clear_focus_state(ctx, modal_id);
            ctx.request_repaint();
            return;
        }
        modal::apply_focus_navigation(ctx, previous_state.as_ref());

        let tokens = ThemeTokens::for_mode_with_background_and_accent(
            ctx.theme() == egui::Theme::Dark,
            self.is_image_background_active(),
            self.background_opacity,
            self.active_accent_seed(),
        );
        let frame = modal::modal_frame(&ctx.style_of(ctx.theme()));
        let default_outer_width =
            CONFLICT_MIN_INNER_WIDTH + f32::from(modal::MODAL_INNER_MARGIN) * 2.0;
        let mut chosen_action = None;
        let mut tab_ids = Vec::new();
        let mut button_ids = Vec::new();
        let mut default_id = None;

        let response = egui::Modal::new(modal_id)
            .frame(frame)
            .area(egui::Modal::default_area(modal_id).default_width(default_outer_width))
            .show(ctx, |ui| {
                ui.set_min_width(CONFLICT_MIN_INNER_WIDTH);
                ui.set_max_width(420.0);
                let heading = match dialog.conflict {
                    ShortcutConflict::System(_) => "Shortcut reserved",
                    ShortcutConflict::Palette(_) => "Shortcut already in use",
                };
                ui.vertical_centered(|ui| {
                    ui.heading(heading);
                });
                ui.add_space(theme::SPACE_MD);

                let target_label = shortcuts::palette_label(dialog.target_source);
                match dialog.conflict {
                    ShortcutConflict::System(system_action) => {
                        ui.label(format!(
                            "{} is reserved for system command \"{}\".",
                            dialog.attempted_chord, system_action
                        ));
                    }
                    ShortcutConflict::Palette(existing_source) => {
                        let existing_label = shortcuts::palette_label(existing_source);
                        ui.label(format!(
                            "{} is already assigned to {} ({}).",
                            dialog.attempted_chord, existing_label, existing_source
                        ));
                        if let Some(previous_chord) = dialog.target_previous {
                            ui.add_space(theme::SPACE_XS);
                            ui.label(
                                egui::RichText::new(format!(
                                    "Replace clears {} from {}. Swap moves {} from {} to {}.",
                                    existing_label,
                                    dialog.attempted_chord,
                                    previous_chord,
                                    target_label,
                                    existing_label
                                ))
                                .small()
                                .color(tokens.text_muted),
                            );
                        }
                    }
                }

                ui.add_space(theme::SPACE_LG);
                let available_width = ui.available_width();
                modal::button_row(
                    ui,
                    available_width,
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| match dialog.conflict {
                        ShortcutConflict::System(_) => {
                            let cancel_resp = widgets::primary_button(ui, tokens, true, "Cancel");
                            let cancel_id = cancel_resp.id;
                            tab_ids.push(cancel_id);
                            button_ids.push(cancel_id);
                            default_id = Some(cancel_id);
                            if cancel_resp.clicked() {
                                chosen_action = Some(ConflictAction::Cancel);
                            }
                        }
                        ShortcutConflict::Palette(existing_source) => {
                            let cancel_resp = widgets::standard_button(ui, "Cancel");
                            let replace_resp = widgets::primary_button(ui, tokens, true, "Replace");
                            let swap_id = if dialog.target_previous.is_some() {
                                let swap_resp = widgets::standard_button(ui, "Swap");
                                if swap_resp.clicked() {
                                    chosen_action = Some(ConflictAction::Swap(existing_source));
                                }
                                Some(swap_resp.id)
                            } else {
                                None
                            };
                            if replace_resp.clicked() {
                                chosen_action = Some(ConflictAction::Replace);
                            } else if cancel_resp.clicked() {
                                chosen_action = Some(ConflictAction::Cancel);
                            }
                            if let Some(id) = swap_id {
                                tab_ids.push(id);
                                button_ids.push(id);
                            }
                            tab_ids.extend([replace_resp.id, cancel_resp.id]);
                            button_ids.extend([replace_resp.id, cancel_resp.id]);
                            default_id = Some(replace_resp.id);
                        }
                    },
                );
            });

        match chosen_action {
            Some(ConflictAction::Cancel) => {
                modal::clear_focus_state(ctx, modal_id);
                ctx.request_repaint();
            }
            Some(ConflictAction::Replace) => {
                self.shortcuts
                    .replace_binding(dialog.target_source, dialog.attempted_chord);
                self.sync_shortcut_overrides();
                modal::clear_focus_state(ctx, modal_id);
                ctx.request_repaint();
            }
            Some(ConflictAction::Swap(existing_source)) => {
                self.shortcuts
                    .swap_bindings(dialog.target_source, existing_source);
                self.sync_shortcut_overrides();
                modal::clear_focus_state(ctx, modal_id);
                ctx.request_repaint();
            }
            None => {
                let next_state = modal::ModalFocusState {
                    layer_id: Some(response.response.layer_id),
                    tab_ids,
                    button_ids,
                    default_id,
                };
                modal::store_focus_state(ctx, modal_id, previous_state.is_none(), next_state);
                self.shortcut_conflict = Some(dialog);
            }
        }
    }

    /// Renders unsaved-changes / active-batch confirmation modals or auto-cancels untouched sessions.
    pub(super) fn draw_dialogs(&mut self, ctx: &egui::Context) {
        let modal_id = egui::Id::unique("finish-equation-dialog");
        // An Office session alone is not an unsaved change. Cancel an
        // untouched session before closing/switching so its add-in wait ends
        // without replacing the equation in the document.
        if self.pending.is_some() && !self.is_dirty() && !self.batch_running {
            modal::clear_focus_state(ctx, modal_id);
            if !self.busy {
                self.finish_session(false);
            }
            return;
        }
        if self.pending.is_none() {
            modal::clear_focus_state(ctx, modal_id);
            return;
        }

        let previous_state = modal::read_focus_state(ctx, modal_id);
        if modal::handle_escape(ctx, previous_state.as_ref(), !self.busy) {
            self.pending = None;
            modal::clear_focus_state(ctx, modal_id);
            self.pending_focus_restore = Some(self.active_editor_focus);
            return;
        }
        modal::apply_focus_navigation(ctx, previous_state.as_ref());

        let tokens = ThemeTokens::for_mode_with_background_and_accent(
            ctx.theme() == egui::Theme::Dark,
            self.is_image_background_active(),
            self.background_opacity,
            self.active_accent_seed(),
        );
        let frame = modal::modal_frame(&ctx.style_of(ctx.theme()));
        let mut action = None;
        let mut tab_ids = Vec::new();
        let mut button_ids = Vec::new();
        let mut default_id = None;

        let response = egui::Modal::new(modal_id)
            .frame(frame)
            .area(egui::Modal::default_area(modal_id).default_width(240.0))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Finish current equation");
                });
                if self.batch_running {
                    ui.add_space(theme::SPACE_SM);
                    ui.label(
                        "A document is being converted. Stop the batch and wait before closing.",
                    );
                    ui.scope(|ui| {
                        ui.spacing_mut().button_padding = modal::MODAL_BUTTON_PADDING;
                        let stop_resp = widgets::standard_button(ui, "Stop batch");
                        tab_ids.push(stop_resp.id);
                        button_ids.push(stop_resp.id);
                        default_id = Some(stop_resp.id);
                        if stop_resp.clicked() {
                            self.stop.store(true, Ordering::Relaxed);
                        }
                    });
                }
                ui.add_space(theme::SPACE_LG);
                let actions_enabled = !self.busy && !self.batch_running;
                ui.add_enabled_ui(actions_enabled, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().button_padding = modal::MODAL_BUTTON_PADDING;
                        let save_resp = widgets::primary_button(ui, tokens, true, "Save");
                        let discard_resp = widgets::standard_button(ui, "Discard / cancel session");
                        let continue_resp = widgets::standard_button(ui, "Continue editing");

                        if actions_enabled {
                            tab_ids.extend([save_resp.id, discard_resp.id, continue_resp.id]);
                            button_ids.extend([save_resp.id, discard_resp.id, continue_resp.id]);
                            default_id.get_or_insert(save_resp.id);
                        }

                        if save_resp.clicked() {
                            action = Some(FinishAction::Save);
                        } else if discard_resp.clicked() {
                            action = Some(FinishAction::Discard);
                        } else if continue_resp.clicked() {
                            action = Some(FinishAction::Continue);
                        }
                    });
                });
            });

        match action {
            Some(FinishAction::Save) => self.finish_session(true),
            Some(FinishAction::Discard) => self.finish_session(false),
            Some(FinishAction::Continue) => {
                self.pending = None;
                self.pending_focus_restore = Some(self.active_editor_focus);
            }
            None => {}
        }

        if self.pending.is_none() {
            modal::clear_focus_state(ctx, modal_id);
        } else {
            let next_state = modal::ModalFocusState {
                layer_id: Some(response.response.layer_id),
                tab_ids,
                button_ids,
                default_id,
            };
            modal::store_focus_state(ctx, modal_id, previous_state.is_none(), next_state);
        }
    }
}

/// User-selected outcome from the `Shortcut conflict` modal.
enum ConflictAction {
    Cancel,
    Replace,
    Swap(&'static str),
}

/// User-selected outcome from the `Finish current equation` confirmation modal.
enum FinishAction {
    Save,
    Discard,
    Continue,
}
