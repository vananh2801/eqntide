//! Shared modal frame styling, action-row padding, Escape handling, and keyboard focus trapping.

use crate::theme;
use eframe::egui::{self, Vec2};

/// Four-side inner padding (DIP) shared by all centered desktop modals.
pub(crate) const MODAL_INNER_MARGIN: i8 = 20;
/// Internal padding applied to action buttons inside modal button rows.
pub(crate) const MODAL_BUTTON_PADDING: Vec2 = Vec2::new(14.0, 6.0);

/// Cached keyboard focus targets and layer identity for an open modal dialog.
#[derive(Clone, Default)]
pub(crate) struct ModalFocusState {
    /// Egui layer ID occupied by the open modal dialog.
    pub(crate) layer_id: Option<egui::LayerId>,
    /// Ordered focus cycle targets traversed by `Tab` and `Shift+Tab`.
    pub(crate) tab_ids: Vec<egui::Id>,
    /// Action button IDs navigated horizontally via `ArrowLeft` and `ArrowRight`.
    pub(crate) button_ids: Vec<egui::Id>,
    /// Initial fallback focus target when opening or reclaiming focus.
    pub(crate) default_id: Option<egui::Id>,
}

/// Builds the uniform modal popup frame with [`MODAL_INNER_MARGIN`] inner spacing.
pub(crate) fn modal_frame(style: &egui::Style) -> egui::Frame {
    egui::Frame::popup(style).inner_margin(egui::Margin::same(MODAL_INNER_MARGIN))
}

/// Renders a fixed-height modal action button row with [`MODAL_BUTTON_PADDING`].
pub(crate) fn button_row(
    ui: &mut egui::Ui,
    width: f32,
    layout: egui::Layout,
    add_buttons: impl FnOnce(&mut egui::Ui),
) {
    ui.allocate_ui_with_layout(Vec2::new(width, theme::BUTTON_HEIGHT), layout, |ui| {
        ui.spacing_mut().button_padding = MODAL_BUTTON_PADDING;
        add_buttons(ui);
    });
}

/// Reads the temporary [`ModalFocusState`] stored for `modal_id`, if any.
pub(crate) fn read_focus_state(ctx: &egui::Context, modal_id: egui::Id) -> Option<ModalFocusState> {
    ctx.data(|data| data.get_temp::<ModalFocusState>(modal_id))
}

/// Clears any stored [`ModalFocusState`] for `modal_id` when the dialog closes.
pub(crate) fn clear_focus_state(ctx: &egui::Context, modal_id: egui::Id) {
    ctx.data_mut(|data| data.remove::<ModalFocusState>(modal_id));
}

/// Stores `state` for `modal_id` and focuses `default_id` on open or when focus escapes.
pub(crate) fn store_focus_state(
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

/// Consumes `Escape` when `previous_state` is the active top modal and `allow_escape` is true.
pub(crate) fn handle_escape(
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

/// Traps `Tab` / `Shift+Tab` inside `tab_ids` and routes `ArrowLeft` / `ArrowRight` across `button_ids`.
pub(crate) fn apply_focus_navigation(
    ctx: &egui::Context,
    previous_state: Option<&ModalFocusState>,
) {
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

fn is_active_top_modal(ctx: &egui::Context, state: &ModalFocusState) -> bool {
    !egui::Popup::is_any_open(ctx)
        && state
            .layer_id
            .is_some_and(|layer| ctx.memory(|mem| mem.top_modal_layer() == Some(layer)))
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
