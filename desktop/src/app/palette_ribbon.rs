//! Three-row symbol and template palette with horizontal navigation.

use super::{App, MatrixDialog, PALETTE_BUTTON_SIZE, PALETTE_GAP, PALETTE_ROWS};
use crate::{palette, theme, ui::icons::UI_ICON_SIZE};
use eframe::egui::{self, Color32, Rect, Vec2};

const PALETTE_COLUMN_STRIDE: f32 = PALETTE_BUTTON_SIZE.x + PALETTE_GAP;
const PALETTE_SUBGROUP_GAP: f32 = theme::SPACE_LG;
pub(super) const MATRIX_CUSTOM_BUTTON_HEIGHT: f32 = 40.0;
pub(super) const MATRIX_ACTION_BUTTON_HEIGHT: f32 = 28.0;
const MATRIX_ACTION_BUTTON_WIDTH: f32 = 50.0;
pub(super) const MATRIX_TOOL_BLOCK_WIDTH: f32 = MATRIX_ACTION_BUTTON_WIDTH * 2.0 + PALETTE_GAP;

const MATRIX_ACTIONS: [(&str, bool, bool); 4] = [
    ("+ row", false, true),
    ("− row", false, false),
    ("+ col", true, true),
    ("− col", true, false),
];

impl App {
    /// Renders the fixed-height, three-row palette ribbon inside the top chrome panel.
    pub(super) fn draw_palette(&mut self, ui: &mut egui::Ui) {
        let subgroups = palette::subgroups_for_ui_group(&self.category, &self.recent_palette);
        let has_matrix_tools = palette::group_contains(&self.category, "Matrices");
        ui.add_enabled_ui(!self.draft_dirty, |ui| {
            self.draw_palette_grid(ui, &subgroups, has_matrix_tools);
        });
    }

    fn draw_palette_grid(
        &mut self,
        ui: &mut egui::Ui,
        subgroups: &[palette::PaletteSubgroup],
        has_matrix_tools: bool,
    ) {
        let (_, viewport) = ui.allocate_space(ui.available_size());
        if self.category == palette::RECENT_GROUP && subgroups.is_empty() {
            let mut message_ui = ui.new_child(egui::UiBuilder::new().max_rect(viewport).layout(
                egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
            ));
            message_ui.weak(
                "Click any symbol or template in the palette to pin it here for quick reuse.",
            );
            return;
        }

        let layout = build_palette_layout(subgroups, has_matrix_tools);
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
        let max_offset = (layout.grid_width - viewport.width()).max(0.0);
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
                ui.set_min_size(Vec2::new(layout.grid_width, viewport.height()));
                ui.spacing_mut().button_padding = Vec2::splat(3.0);
                self.ensure_palette_atlas(ui.ctx());
                let icon_color = self.theme_tokens(ui).icon_tint(false, ui.is_enabled());
                let origin = ui.max_rect().min;
                for placement in &layout.tiles {
                    let &(_, label, source) = placement.item;
                    let rect = Rect::from_min_size(origin + placement.offset, PALETTE_BUTTON_SIZE);
                    if self.draw_palette_tile(ui, rect, label, source, icon_color) {
                        self.command_input = None;
                        self.latex_groups.clear();
                        self.expects_latex_group = false;
                        self.record_recent_palette(source);
                        self.insert_template(source);
                        self.pending_focus_restore = Some(self.active_editor_focus);
                    }
                }
                if let Some(matrix_tools) = &layout.matrix_tools {
                    self.draw_matrix_tools(ui, origin, matrix_tools, icon_color);
                }
            });
        if initial_offset != offset || output.state.clamped_offset().x != offset {
            ui.ctx().request_repaint();
        }
    }

    fn draw_matrix_tools(
        &mut self,
        ui: &mut egui::Ui,
        origin: egui::Pos2,
        matrix_tools: &MatrixToolsLayout,
        icon_color: Color32,
    ) {
        let custom_rect = matrix_tools.custom_rect.translate(origin.to_vec2());
        let icon = self
            .ui_icons
            .image(ui.ctx(), "dots", UI_ICON_SIZE, icon_color);
        let custom_response = ui
            .scope(|ui| {
                ui.spacing_mut().button_padding = Vec2::new(theme::SPACE_SM, 2.0);
                labeled_palette_button(
                    ui,
                    custom_rect,
                    egui::Button::image_and_text(icon, "Custom"),
                    "Custom matrix",
                )
            })
            .inner;
        if custom_response.on_hover_text("Custom matrix").clicked() {
            self.dismiss_command_input();
            self.matrix_dialog = Some(MatrixDialog::default());
            ui.memory_mut(|mem| {
                mem.surrender_focus(Self::editor_focus_id(self.active_editor_focus))
            });
        }

        ui.scope(|ui| {
            ui.spacing_mut().button_padding = Vec2::new(theme::SPACE_XS, 2.0);
            for (action_rect, (label, column, grow)) in
                matrix_tools.action_rects.iter().zip(MATRIX_ACTIONS)
            {
                let rect = action_rect.translate(origin.to_vec2());
                if labeled_palette_button(ui, rect, egui::Button::new(label), label).clicked() {
                    let changed = if grow {
                        self.editor.grow_matrix(column)
                    } else {
                        self.editor.shrink_matrix(column)
                    };
                    if changed {
                        self.mark_canvas_changed();
                    } else {
                        self.status = "Place the caret inside a matrix first.".into();
                    }
                    self.pending_focus_restore = Some(self.active_editor_focus);
                }
            }
        });
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

struct PaletteTilePlacement {
    item: &'static palette::PaletteItem,
    offset: Vec2,
}

struct MatrixToolsLayout {
    custom_rect: Rect,
    action_rects: [Rect; 4],
}

struct PaletteLayout {
    tiles: Vec<PaletteTilePlacement>,
    matrix_tools: Option<MatrixToolsLayout>,
    grid_width: f32,
}

fn build_palette_layout(
    subgroups: &[palette::PaletteSubgroup],
    has_matrix_tools: bool,
) -> PaletteLayout {
    let row_stride = PALETTE_BUTTON_SIZE.y + PALETTE_GAP;
    let total_items = subgroups.iter().map(palette::PaletteSubgroup::len).sum();
    let mut tiles = Vec::with_capacity(total_items);
    let mut cursor_x = 0.0;
    let mut has_placed_group = false;

    for subgroup in subgroups {
        if subgroup.is_empty() {
            continue;
        }
        if has_placed_group {
            cursor_x += PALETTE_SUBGROUP_GAP;
        }
        has_placed_group = true;
        let mut has_placed_run = false;
        for run in &subgroup.runs {
            if run.is_empty() {
                continue;
            }
            if has_placed_run {
                cursor_x += PALETTE_GAP;
            }
            has_placed_run = true;
            let run_columns = run.len().div_ceil(PALETTE_ROWS);
            for (item_index, &item) in run.iter().enumerate() {
                let column = (item_index / PALETTE_ROWS) as f32;
                let row = (item_index % PALETTE_ROWS) as f32;
                tiles.push(PaletteTilePlacement {
                    item,
                    offset: Vec2::new(cursor_x + column * PALETTE_COLUMN_STRIDE, row * row_stride),
                });
            }
            cursor_x += (run_columns - 1) as f32 * PALETTE_COLUMN_STRIDE + PALETTE_BUTTON_SIZE.x;
        }
    }

    let matrix_tools = if has_matrix_tools {
        if has_placed_group {
            cursor_x += PALETTE_SUBGROUP_GAP;
        }
        let custom_rect = Rect::from_min_size(
            egui::Pos2::new(cursor_x, 0.0),
            Vec2::new(MATRIX_TOOL_BLOCK_WIDTH, MATRIX_CUSTOM_BUTTON_HEIGHT),
        );
        let row2_y = MATRIX_CUSTOM_BUTTON_HEIGHT + PALETTE_GAP;
        let row3_y = row2_y + MATRIX_ACTION_BUTTON_HEIGHT + PALETTE_GAP;
        let col2_x = cursor_x + MATRIX_ACTION_BUTTON_WIDTH + PALETTE_GAP;
        let action_size = Vec2::new(MATRIX_ACTION_BUTTON_WIDTH, MATRIX_ACTION_BUTTON_HEIGHT);
        let action_rects = [
            Rect::from_min_size(egui::Pos2::new(cursor_x, row2_y), action_size),
            Rect::from_min_size(egui::Pos2::new(col2_x, row2_y), action_size),
            Rect::from_min_size(egui::Pos2::new(cursor_x, row3_y), action_size),
            Rect::from_min_size(egui::Pos2::new(col2_x, row3_y), action_size),
        ];
        cursor_x += MATRIX_TOOL_BLOCK_WIDTH;
        Some(MatrixToolsLayout {
            custom_rect,
            action_rects,
        })
    } else {
        None
    };

    PaletteLayout {
        tiles,
        matrix_tools,
        grid_width: cursor_x,
    }
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
