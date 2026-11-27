//! Three-row symbol and template palette with horizontal navigation.

use super::{
    App, MatrixDialog, CONTENT_PADDING, PALETTE_BUTTON_SIZE, PALETTE_GAP, PALETTE_HEIGHT,
    PALETTE_ROWS, UI_ICON_SIZE,
};
use crate::palette;
use eframe::egui::{self, Color32, Pos2, Rect, Vec2};

const PALETTE_COLUMN_STRIDE: f32 = PALETTE_BUTTON_SIZE.x + PALETTE_GAP;

impl App {
    /// Renders the fixed-height, three-row palette ribbon.
    pub(super) fn draw_palette(&mut self, ui: &mut egui::Ui) {
        let items = palette::items_for_ui_group(&self.category, &self.recent_palette);
        let has_matrix_tools = palette::group_contains(&self.category, "Matrices");
        egui::Panel::top("templates-panel")
            .exact_size(PALETTE_HEIGHT)
            .frame(egui::Frame::NONE.inner_margin(egui::Margin {
                bottom: CONTENT_PADDING,
                ..Default::default()
            }))
            .show_separator_line(false)
            .show(ui, |ui| {
                ui.add_enabled_ui(!self.draft_dirty, |ui| {
                    self.draw_palette_grid(ui, &items, has_matrix_tools);
                });
            });
    }

    fn draw_palette_grid(
        &mut self,
        ui: &mut egui::Ui,
        items: &[&palette::PaletteItem],
        has_matrix_tools: bool,
    ) {
        let (_, viewport) = ui.allocate_space(ui.available_size());
        let columns = (items.len() + usize::from(has_matrix_tools)).div_ceil(PALETTE_ROWS);
        let grid_width = (columns as f32 * PALETTE_COLUMN_STRIDE - PALETTE_GAP).max(0.0);
        if self.category == palette::RECENT_GROUP && items.is_empty() {
            let mut message_ui = ui.new_child(egui::UiBuilder::new().max_rect(viewport).layout(
                egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
            ));
            message_ui.weak(
                "Click any symbol or template in the palette to pin it here for quick reuse.",
            );
            return;
        }

        let mut grid_ui = ui.new_child(egui::UiBuilder::new().id_salt("tiles").max_rect(viewport));
        grid_ui.shrink_clip_rect(viewport);
        let style = grid_ui.style_mut();
        style.always_scroll_the_only_direction = true;
        style.spacing.scroll.kinetic.rubber_band = false;
        style.spacing.scroll.kinetic.decay_time = 0.0;

        let salt = ("palette", &self.category);
        let scroll_id = grid_ui.make_persistent_id(egui::IdSalt::new(salt));
        let scroll_area = egui::ScrollArea::horizontal()
            .id_salt(salt)
            .max_height(viewport.height())
            .min_scrolled_width(0.0)
            .auto_shrink([false, false])
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .scroll_source(egui::scroll_area::ScrollSource::MOUSE_WHEEL);
        let max_offset = (grid_width - viewport.width()).max(0.0);
        let initial_offset = egui::scroll_area::State::load(ui.ctx(), scroll_id)
            .map_or(0.0, |state| state.clamped_offset().x.min(max_offset));
        let offset = if max_offset > 0.0 {
            let delta = palette_drag_delta(ui, viewport, scroll_id.with("drag"));
            (initial_offset - delta).clamp(0.0, max_offset)
        } else {
            0.0
        };
        let output = scroll_area
            .horizontal_scroll_offset(offset)
            .show(&mut grid_ui, |ui| {
                ui.set_min_size(Vec2::new(grid_width, viewport.height()));
                ui.spacing_mut().button_padding = Vec2::splat(3.0);
                self.ensure_palette_atlas(ui.ctx());
                let icon_color = self.theme_tokens(ui).icon_tint(false, ui.is_enabled());
                let origin = ui.max_rect().min;
                for (index, &&(_, label, source)) in items.iter().enumerate() {
                    if self.draw_palette_tile(
                        ui,
                        palette_tile_rect(origin, index),
                        label,
                        source,
                        icon_color,
                    ) {
                        self.command_input = None;
                        self.latex_groups.clear();
                        self.expects_latex_group = false;
                        self.record_recent_palette(source);
                        self.insert_template(source);
                        self.pending_focus_restore = Some(self.active_editor_focus);
                    }
                }
                if has_matrix_tools {
                    let rect = palette_tile_rect(origin, items.len());
                    let icon = self.ui_icon(ui.ctx(), "dots", UI_ICON_SIZE, icon_color);
                    if labeled_palette_button(ui, rect, egui::Button::new(icon), "Custom matrix")
                        .on_hover_text("Custom matrix")
                        .clicked()
                    {
                        self.dismiss_command_input();
                        self.matrix_dialog = Some(MatrixDialog::default());
                        ui.memory_mut(|mem| {
                            mem.surrender_focus(Self::editor_focus_id(self.active_editor_focus))
                        });
                    }
                }
            });
        if initial_offset != offset || output.state.clamped_offset().x != offset {
            ui.ctx().request_repaint();
        }
    }

    fn draw_palette_tile(
        &self,
        ui: &mut egui::Ui,
        rect: Rect,
        label: &str,
        source: &str,
        icon_color: Color32,
    ) -> bool {
        let button = self
            .palette_atlas
            .as_ref()
            .and_then(|atlas| atlas.entries.get(source).map(|entry| (atlas, entry)))
            .map_or_else(
                || egui::Button::new(label).wrap_mode(egui::TextWrapMode::Truncate),
                |(atlas, entry)| {
                    egui::Button::new(
                        egui::Image::new((atlas.texture.id(), entry.display_size))
                            .uv(entry.uv)
                            .tint(icon_color),
                    )
                },
            );
        labeled_palette_button(ui, rect, button, label)
            .on_hover_ui(|ui| {
                if let Some(chord) = self.shortcuts.binding_for(source) {
                    ui.label(format!("{label} ({chord})"));
                } else {
                    ui.label(label);
                }
                ui.weak(source);
            })
            .clicked()
    }
}

#[derive(Clone, Copy, Default)]
struct PaletteDrag {
    anchor_x: f32,
    active: bool,
}

fn palette_tile_rect(origin: Pos2, index: usize) -> Rect {
    let column = (index / PALETTE_ROWS) as f32;
    let row = (index % PALETTE_ROWS) as f32;
    let position = origin
        + Vec2::new(
            column * PALETTE_COLUMN_STRIDE,
            row * (PALETTE_BUTTON_SIZE.y + PALETTE_GAP),
        );
    Rect::from_min_size(position, PALETTE_BUTTON_SIZE)
}

fn labeled_palette_button(
    ui: &mut egui::Ui,
    rect: Rect,
    button: egui::Button<'_>,
    label: &str,
) -> egui::Response {
    let response = ui.put(rect, button);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), label));
    response
}

fn palette_drag_delta(ui: &egui::Ui, viewport: Rect, id: egui::Id) -> f32 {
    // Register the drag target before the tiles so the buttons still receive clicks.
    let drag = ui.interact(viewport, id, egui::Sense::DRAG);
    let primary = egui::PointerButton::Primary;
    let pending = ui.data_mut(|data| data.remove_temp::<PaletteDrag>(id));
    let dragging = drag.dragged_by(primary);
    if !ui.is_enabled() || !(dragging || drag.drag_stopped_by(primary)) {
        return 0.0;
    }
    let Some(position) = ui.input(|input| input.pointer.interact_pos()) else {
        return 0.0;
    };
    let mut gesture = if drag.drag_started_by(primary) {
        let Some(origin) = ui.input(|input| input.pointer.press_origin()) else {
            return 0.0;
        };
        PaletteDrag {
            anchor_x: origin.x,
            active: false,
        }
    } else if let Some(pending) = pending {
        pending
    } else {
        return 0.0;
    };
    let delta = position.x - gesture.anchor_x;
    let threshold = ui
        .ctx()
        .options(|options| options.input_options.max_click_dist);
    gesture.active |= delta.abs() > threshold;
    if gesture.active {
        gesture.anchor_x = position.x;
    }
    if dragging {
        ui.data_mut(|data| data.insert_temp(id, gesture));
        if gesture.active {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        }
    }
    if gesture.active {
        delta
    } else {
        0.0
    }
}
