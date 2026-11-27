//! Desktop matrix configuration and session confirmation dialogs.

use super::{App, MatrixDialog};
use crate::{
    shortcuts::{self, ShortcutConflict},
    theme::{self, ThemeTokens},
};
use eframe::egui::{self, Vec2};
use std::sync::atomic::Ordering;

const MODAL_INNER_MARGIN: i8 = 20;
const MODAL_BUTTON_PADDING: Vec2 = Vec2::new(14.0, 6.0);
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
            clear_modal_focus_state(ctx, modal_id);
            return;
        };
        if self.pending.is_some() || self.tab != 0 {
            clear_modal_focus_state(ctx, modal_id);
            return;
        }
        let previous_state = read_modal_focus_state(ctx, modal_id);
        if handle_modal_escape(ctx, previous_state.as_ref(), true) {
            clear_modal_focus_state(ctx, modal_id);
            self.pending_focus_restore = Some(self.active_editor_focus);
            return;
        }
        apply_modal_focus_navigation(ctx, previous_state.as_ref());

        let tokens = ThemeTokens::for_mode_with_background_and_accent(
            ctx.theme() == egui::Theme::Dark,
            self.is_image_background_active(),
            self.background_opacity,
            self.active_accent_seed(),
        );
        let frame = modal_frame(&ctx.style_of(ctx.theme()));
        let default_outer_width = MATRIX_MIN_INNER_WIDTH + f32::from(MODAL_INNER_MARGIN) * 2.0;
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
                                    Self::with_selectable_style(ui, tokens, selected, |ui| {
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
                                    Self::with_selectable_style(ui, tokens, selected, |ui| {
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
                ui.allocate_ui_with_layout(
                    Vec2::new(button_row_width, theme::BUTTON_HEIGHT),
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        ui.spacing_mut().button_padding = MODAL_BUTTON_PADDING;
                        let cancel_resp = Self::standard_button(ui, "Cancel");
                        cancel_id = cancel_resp.id;
                        cancel = cancel_resp.clicked();

                        let insert_resp = Self::primary_button_with(ui, tokens, true, |ui| {
                            ui.add(
                                egui::Button::new("Insert matrix")
                                    .min_size(Vec2::new(0.0, theme::BUTTON_HEIGHT)),
                            )
                        });
                        insert_id = insert_resp.id;
                        insert = insert_resp.clicked();
                    },
                );
            });

        if insert {
            self.insert_custom_matrix(&settings);
            clear_modal_focus_state(ctx, modal_id);
            self.pending_focus_restore = Some(self.active_editor_focus);
        } else if cancel {
            clear_modal_focus_state(ctx, modal_id);
            self.pending_focus_restore = Some(self.active_editor_focus);
        } else {
            let next_state = ModalFocusState {
                layer_id: Some(response.response.layer_id),
                tab_ids: vec![rows_id, cols_id, align_id, fence_id, insert_id, cancel_id],
                button_ids: vec![insert_id, cancel_id],
                default_id: Some(insert_id),
            };
            store_modal_focus_state(ctx, modal_id, previous_state.is_none(), next_state);
            self.matrix_dialog = Some(settings);
        }
    }

    /// Renders the modal dialog for resolving a shortcut collision with a system or palette binding.
    pub(super) fn draw_shortcut_conflict_dialog(&mut self, ctx: &egui::Context) {
        let modal_id = egui::Id::unique("shortcut-conflict-dialog");
        let Some(dialog) = self.shortcut_conflict.take() else {
            clear_modal_focus_state(ctx, modal_id);
            return;
        };
        if self.pending.is_some() || self.tab != 2 {
            clear_modal_focus_state(ctx, modal_id);
            return;
        }

        let previous_state = read_modal_focus_state(ctx, modal_id);
        if handle_modal_escape(ctx, previous_state.as_ref(), true) {
            clear_modal_focus_state(ctx, modal_id);
            ctx.request_repaint();
            return;
        }
        apply_modal_focus_navigation(ctx, previous_state.as_ref());

        let tokens = ThemeTokens::for_mode_with_background_and_accent(
            ctx.theme() == egui::Theme::Dark,
            self.is_image_background_active(),
            self.background_opacity,
            self.active_accent_seed(),
        );
        let frame = modal_frame(&ctx.style_of(ctx.theme()));
        let default_outer_width = CONFLICT_MIN_INNER_WIDTH + f32::from(MODAL_INNER_MARGIN) * 2.0;
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
                ui.allocate_ui_with_layout(
                    Vec2::new(ui.available_width(), theme::BUTTON_HEIGHT),
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        ui.spacing_mut().button_padding = MODAL_BUTTON_PADDING;
                        match dialog.conflict {
                            ShortcutConflict::System(_) => {
                                let cancel_resp =
                                    Self::primary_button_with(ui, tokens, true, |ui| {
                                        ui.add(
                                            egui::Button::new("Cancel")
                                                .min_size(Vec2::new(0.0, theme::BUTTON_HEIGHT)),
                                        )
                                    });
                                let cancel_id = cancel_resp.id;
                                tab_ids.push(cancel_id);
                                button_ids.push(cancel_id);
                                default_id = Some(cancel_id);
                                if cancel_resp.clicked() {
                                    chosen_action = Some(ConflictAction::Cancel);
                                }
                            }
                            ShortcutConflict::Palette(existing_source) => {
                                let cancel_resp = Self::standard_button(ui, "Cancel");
                                let replace_resp =
                                    Self::primary_button_with(ui, tokens, true, |ui| {
                                        ui.add(
                                            egui::Button::new("Replace")
                                                .min_size(Vec2::new(0.0, theme::BUTTON_HEIGHT)),
                                        )
                                    });
                                let swap_id = if dialog.target_previous.is_some() {
                                    let swap_resp = Self::standard_button(ui, "Swap");
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
                        }
                    },
                );
            });

        match chosen_action {
            Some(ConflictAction::Cancel) => {
                clear_modal_focus_state(ctx, modal_id);
                ctx.request_repaint();
            }
            Some(ConflictAction::Replace) => {
                self.shortcuts
                    .replace_binding(dialog.target_source, dialog.attempted_chord);
                self.sync_shortcut_overrides();
                clear_modal_focus_state(ctx, modal_id);
                ctx.request_repaint();
            }
            Some(ConflictAction::Swap(existing_source)) => {
                self.shortcuts
                    .swap_bindings(dialog.target_source, existing_source);
                self.sync_shortcut_overrides();
                clear_modal_focus_state(ctx, modal_id);
                ctx.request_repaint();
            }
            None => {
                let next_state = ModalFocusState {
                    layer_id: Some(response.response.layer_id),
                    tab_ids,
                    button_ids,
                    default_id,
                };
                store_modal_focus_state(ctx, modal_id, previous_state.is_none(), next_state);
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
            clear_modal_focus_state(ctx, modal_id);
            if !self.busy {
                self.finish_session(false);
            }
            return;
        }
        if self.pending.is_none() {
            clear_modal_focus_state(ctx, modal_id);
            return;
        }

        let previous_state = read_modal_focus_state(ctx, modal_id);
        if handle_modal_escape(ctx, previous_state.as_ref(), !self.busy) {
            self.pending = None;
            clear_modal_focus_state(ctx, modal_id);
            self.pending_focus_restore = Some(self.active_editor_focus);
            return;
        }
        apply_modal_focus_navigation(ctx, previous_state.as_ref());

        let tokens = ThemeTokens::for_mode_with_background_and_accent(
            ctx.theme() == egui::Theme::Dark,
            self.is_image_background_active(),
            self.background_opacity,
            self.active_accent_seed(),
        );
        let frame = modal_frame(&ctx.style_of(ctx.theme()));
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
                        ui.spacing_mut().button_padding = MODAL_BUTTON_PADDING;
                        let stop_resp = Self::standard_button(ui, "Stop batch");
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
                        ui.spacing_mut().button_padding = MODAL_BUTTON_PADDING;
                        let save_resp = Self::primary_button_with(ui, tokens, true, |ui| {
                            ui.add(
                                egui::Button::new("Save")
                                    .min_size(Vec2::new(0.0, theme::BUTTON_HEIGHT)),
                            )
                        });
                        let discard_resp = Self::standard_button(ui, "Discard / cancel session");
                        let continue_resp = Self::standard_button(ui, "Continue editing");

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
            clear_modal_focus_state(ctx, modal_id);
        } else {
            let next_state = ModalFocusState {
                layer_id: Some(response.response.layer_id),
                tab_ids,
                button_ids,
                default_id,
            };
            store_modal_focus_state(ctx, modal_id, previous_state.is_none(), next_state);
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

/// Cached keyboard focus targets and layer identity for an open modal dialog.
#[derive(Clone, Default)]
struct ModalFocusState {
    layer_id: Option<egui::LayerId>,
    tab_ids: Vec<egui::Id>,
    button_ids: Vec<egui::Id>,
    default_id: Option<egui::Id>,
}

fn modal_frame(style: &egui::Style) -> egui::Frame {
    egui::Frame::popup(style).inner_margin(egui::Margin::same(MODAL_INNER_MARGIN))
}

fn read_modal_focus_state(ctx: &egui::Context, modal_id: egui::Id) -> Option<ModalFocusState> {
    ctx.data(|data| data.get_temp::<ModalFocusState>(modal_id))
}

fn clear_modal_focus_state(ctx: &egui::Context, modal_id: egui::Id) {
    ctx.data_mut(|data| data.remove::<ModalFocusState>(modal_id));
}

fn store_modal_focus_state(
    ctx: &egui::Context,
    modal_id: egui::Id,
    just_opened: bool,
    state: ModalFocusState,
) {
    let focused_inside = ctx.memory(|mem| {
        mem.focused()
            .is_some_and(|active| state.tab_ids.contains(&active))
    });
    if just_opened || (!focused_inside && !egui::Popup::is_any_open(ctx)) {
        if let Some(default_id) = state.default_id {
            ctx.memory_mut(|mem| mem.request_focus(default_id));
            ctx.request_repaint();
        }
    }
    ctx.data_mut(|data| data.insert_temp(modal_id, state));
}

fn is_active_top_modal(ctx: &egui::Context, state: &ModalFocusState) -> bool {
    !egui::Popup::is_any_open(ctx)
        && state
            .layer_id
            .is_some_and(|layer| ctx.memory(|mem| mem.top_modal_layer() == Some(layer)))
}

fn handle_modal_escape(
    ctx: &egui::Context,
    previous_state: Option<&ModalFocusState>,
    allow_escape: bool,
) -> bool {
    let Some(state) = previous_state else {
        return false;
    };
    if !allow_escape || !is_active_top_modal(ctx, state) {
        return false;
    }
    ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
}

fn apply_modal_focus_navigation(ctx: &egui::Context, previous_state: Option<&ModalFocusState>) {
    let Some(state) = previous_state else {
        return;
    };
    if !is_active_top_modal(ctx, state) || state.tab_ids.is_empty() {
        return;
    }
    let current = ctx.memory(|mem| mem.focused());
    // Match Shift+Tab before Tab because egui's consume_key ignores extra Shift in Modifiers::NONE.
    let backward_tab =
        ctx.input_mut(|input| input.consume_key(egui::Modifiers::SHIFT, egui::Key::Tab));
    let forward_tab =
        ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Tab));

    if forward_tab {
        let next = cycle_focus_id(&state.tab_ids, current, true);
        ctx.memory_mut(|mem| {
            mem.move_focus(egui::FocusDirection::None);
            mem.request_focus(next);
        });
        return;
    }
    if backward_tab {
        let prev = cycle_focus_id(&state.tab_ids, current, false);
        ctx.memory_mut(|mem| {
            mem.move_focus(egui::FocusDirection::None);
            mem.request_focus(prev);
        });
        return;
    }

    if current.is_some_and(|active| state.button_ids.contains(&active)) {
        let right =
            ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight));
        let left =
            ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft));
        if right && !state.button_ids.is_empty() {
            let next = cycle_focus_id(&state.button_ids, current, true);
            ctx.memory_mut(|mem| {
                mem.move_focus(egui::FocusDirection::None);
                mem.request_focus(next);
            });
            return;
        }
        if left && !state.button_ids.is_empty() {
            let prev = cycle_focus_id(&state.button_ids, current, false);
            ctx.memory_mut(|mem| {
                mem.move_focus(egui::FocusDirection::None);
                mem.request_focus(prev);
            });
            return;
        }
        // Prevent Up/Down focus traversal from escaping modal action buttons to background widgets.
        ctx.memory_mut(|mem| mem.move_focus(egui::FocusDirection::None));
    } else if current.is_none_or(|active| !state.tab_ids.contains(&active)) {
        if let Some(default_id) = state.default_id {
            ctx.memory_mut(|mem| mem.request_focus(default_id));
        }
    }
}

fn cycle_focus_id(ids: &[egui::Id], current: Option<egui::Id>, forward: bool) -> egui::Id {
    let Some(pos) =
        current.and_then(|active| ids.iter().position(|candidate| *candidate == active))
    else {
        return ids[0];
    };
    if forward {
        ids[(pos + 1) % ids.len()]
    } else {
        ids[(pos + ids.len() - 1) % ids.len()]
    }
}
