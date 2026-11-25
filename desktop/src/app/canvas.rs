//! Visual equation canvas painting, placeholder hit-testing, caret geometry, and LaTeX source box.

use super::{
    App, GpuProfile, CONTENT_PADDING, LATEX_INPUT_HEIGHT, LATEX_PANEL_HEIGHT, PALETTE_HEIGHT,
};
use crate::{
    palette,
    settings::StartupFocus,
    theme::{self, CanvasPalette},
};
use eframe::egui::{self, Pos2, Rect, Stroke, Vec2};
use eqntide_editor::Caret;
use std::time::{Duration, Instant};

impl App {
    /// Renders the main editor workspace: top palette ribbon, equation canvas, and bottom LaTeX box.
    pub(super) fn draw_editor(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("templates-panel")
            .exact_size(PALETTE_HEIGHT)
            .frame(egui::Frame::NONE.inner_margin(egui::Margin {
                bottom: CONTENT_PADDING,
                ..Default::default()
            }))
            .show_separator_line(false)
            .show(ui, |ui| {
                ui.add_enabled_ui(!self.draft_dirty, |ui| self.draw_palette(ui));
            });
        egui::Panel::bottom("latex-panel")
            .exact_size(LATEX_PANEL_HEIGHT)
            .show_separator_line(false)
            .frame(egui::Frame::NONE.inner_margin(egui::Margin {
                top: CONTENT_PADDING,
                ..Default::default()
            }))
            .show(ui, |ui| self.draw_latex_panel(ui));
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                ui.set_clip_rect(ui.clip_rect().intersect(ui.max_rect()));
                ui.add_enabled_ui(!self.draft_dirty, |ui| {
                    if palette::group_contains(&self.category, "Matrices") {
                        ui.horizontal(|ui| {
                            for (label, column, grow) in [
                                ("+ row", false, true),
                                ("− row", false, false),
                                ("+ col", true, true),
                                ("− col", true, false),
                            ] {
                                if Self::compact_button(ui, label).clicked() {
                                    let changed = if grow {
                                        self.editor.grow_matrix(column)
                                    } else {
                                        self.editor.shrink_matrix(column)
                                    };
                                    if changed {
                                        self.mark_canvas_changed();
                                    } else {
                                        self.status =
                                            "Place the caret inside a matrix first.".into();
                                    }
                                    self.pending_focus_restore = Some(self.active_editor_focus);
                                }
                            }
                        });
                    }
                    self.draw_canvas(ui);
                });
            });
        self.sync_editor_focus(ui);
    }

    /// Renders the scrollable interactive equation preview, placeholders, selection, and blinking caret.
    pub(super) fn draw_canvas(
        &mut self,
        ui: &mut egui::Ui,
    ) -> egui::scroll_area::ScrollAreaOutput<()> {
        self.sync_render(ui.ctx());
        // A pending/invalid source disables editing, but keeps the last preview legible.
        ui.set_opacity(1.0);
        let scale = self.zoom * 96.0 / 72.0;
        let height = (ui.available_height() - 26.0).max(12.0);
        let pal = self.active_canvas_palette(ui.visuals().dark_mode);
        egui::Frame::new()
            .fill(pal.bg)
            .corner_radius(theme::SURFACE_CORNER_RADIUS)
            .stroke(Stroke::new(1.0, pal.border))
            .inner_margin(theme::SPACE_LG)
            .show(ui, |ui| {
                self.configure_canvas_scroll_style(ui, pal);

                egui::ScrollArea::both()
                    .id_salt("formula-scroll")
                    .max_height(height)
                    .min_scrolled_height(height)
                    .auto_shrink([false, false])
                    .show_viewport(ui, |ui, viewport| {
                        let size = self
                            .rendered
                            .as_ref()
                            .map(|rendered| {
                                Vec2::new(
                                    rendered.preview.preview.width_pt * scale,
                                    rendered.preview.preview.height_pt * scale,
                                )
                            })
                            .unwrap_or(Vec2::new(60.0, 40.0));
                        // Only the formula contributes to scrollable content. Filling the
                        // viewport via allocate_space can round past its width at fractional DPI.
                        let content_size = size + Vec2::new(12.0, 20.0);
                        let (_, content_rect) = ui.allocate_space(content_size);
                        // Empty canvas space remains clickable without creating overflow.
                        let rect = Rect::from_min_size(
                            content_rect.min,
                            viewport.size().max(content_size),
                        );
                        let response = ui.interact(
                            rect,
                            egui::Id::unique("equation-canvas"),
                            egui::Sense::click_and_drag(),
                        );
                        response.widget_info(|| {
                            egui::WidgetInfo::labeled(
                                egui::Role::TextInput,
                                true,
                                "Visual equation editor",
                            )
                        });
                        let origin = rect.min + Vec2::new(4.0, 8.0);
                        if let Some(texture) = &self.texture {
                            ui.painter().image(
                                texture.id(),
                                Rect::from_min_size(origin, size),
                                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                pal.ink,
                            );
                        }
                        if let Some(rendered) = &self.rendered {
                            for slot in &rendered.placeholders {
                                let slot_rect = Rect::from_min_size(
                                    origin + Vec2::new(slot.x, slot.y) * scale,
                                    Vec2::new(slot.width, slot.height) * scale,
                                );
                                Self::paint_placeholder(ui.painter(), slot_rect, pal);
                            }
                        }
                        if response.clicked() || response.drag_started() {
                            response.request_focus();
                        }
                        // Enter/Space also set clicked() for a focused egui widget.
                        // Only a real pointer click may cancel command entry or reposition the caret.
                        let pointer_clicked = response.clicked_by(egui::PointerButton::Primary);
                        if pointer_clicked || response.drag_started() {
                            self.dismiss_command_input();
                            self.latex_groups.clear();
                            self.expects_latex_group = false;
                        }
                        if response.drag_started() {
                            if let Some(pos) = ui.input(|input| input.pointer.press_origin()) {
                                if let Some(caret) = self.hit_test_caret(pos, origin, scale) {
                                    self.editor.set_caret(caret, false);
                                }
                            }
                        }
                        if (pointer_clicked || response.dragged()) && !self.render_busy {
                            if let Some(pos) = response.interact_pointer_pos() {
                                if let Some(caret) = self.hit_test_caret(pos, origin, scale) {
                                    let extend = ui.input(|input| input.modifiers.shift)
                                        || response.dragged();
                                    if self
                                        .editor
                                        .node(caret.node)
                                        .is_some_and(|node| node.is_opaque())
                                    {
                                        self.editor.select_node(caret.node);
                                    } else {
                                        self.editor.set_caret(caret, extend);
                                    }
                                    if response.double_clicked() {
                                        self.editor.select_node(caret.node);
                                    }
                                }
                            }
                        }
                        let selected = self.editor.selected_ids();
                        for id in selected {
                            if let Some(node_rect) = self.node_rect(id, origin, scale) {
                                ui.painter().rect_filled(
                                    node_rect,
                                    theme::CONTROL_CORNER_RADIUS,
                                    pal.selection_bg,
                                );
                            }
                        }
                        let has_canvas_focus =
                            response.has_focus() || ui.memory(|mem| mem.has_focus(response.id));
                        if has_canvas_focus && ui.is_enabled() {
                            ui.memory_mut(|mem| {
                                mem.set_focus_lock_filter(
                                    response.id,
                                    egui::EventFilter {
                                        tab: true,
                                        horizontal_arrows: true,
                                        vertical_arrows: true,
                                        escape: true,
                                    },
                                )
                            });
                            if !self.keyboard_handled {
                                self.handle_canvas_keyboard(ui);
                                self.sync_render(ui.ctx());
                            }
                            let cursor = self
                                .caret_rect(origin, scale)
                                .unwrap_or(Rect::from_min_size(origin, Vec2::new(1.0, 24.0)));
                            if self.is_canvas_caret_visible(ui) {
                                let ppp = ui.ctx().pixels_per_point();
                                let snap = |coord: f32| (coord * ppp).round() / ppp;
                                let x = snap(cursor.left());
                                ui.painter().line_segment(
                                    [
                                        Pos2::new(x, snap(cursor.top())),
                                        Pos2::new(x, snap(cursor.bottom())),
                                    ],
                                    Stroke::new(
                                        1.5,
                                        if self.is_caret_in_text() {
                                            pal.text_caret
                                        } else {
                                            pal.caret
                                        },
                                    ),
                                );
                            }
                            ui.output_mut(|out| {
                                out.ime = Some(egui::output::IMEOutput {
                                    purpose: egui::IMEPurpose::Normal,
                                    rect,
                                    cursor_rect: cursor,
                                    should_interrupt_composition: false,
                                })
                            });
                            if !self.ime.is_empty() {
                                ui.painter().text(
                                    cursor.right_bottom(),
                                    egui::Align2::LEFT_TOP,
                                    &self.ime,
                                    egui::FontId::proportional(18.0),
                                    pal.ime_fg,
                                );
                            }
                        } else {
                            self.caret_blink = None;
                        }
                        self.keyboard_handled = false;
                        if self.command_input.is_some() && ui.is_enabled() {
                            let cursor = self
                                .caret_rect(origin, scale)
                                .unwrap_or(Rect::from_min_size(origin, Vec2::new(1.0, 24.0)));
                            self.draw_command_popup(ui, cursor.left_bottom() + Vec2::new(0.0, 6.0));
                        }
                    })
            })
            .inner
    }

    /// Renders the synchronized multiline LaTeX source editor panel beneath the canvas.
    pub(super) fn draw_latex_panel(&mut self, ui: &mut egui::Ui) {
        let pal = self.active_canvas_palette(ui.visuals().dark_mode);
        let latex_id = Self::editor_focus_id(StartupFocus::Latex);
        let box_height = ui.available_height().clamp(24.0, LATEX_INPUT_HEIGHT);
        let frame_resp = egui::Frame::new()
            .fill(pal.bg)
            .corner_radius(theme::CONTROL_CORNER_RADIUS)
            .show(ui, |ui| {
                self.configure_canvas_scroll_style(ui, pal);
                ui.spacing_mut().scroll.floating_allocated_width = 0.0;
                ui.spacing_mut().scroll.bar_outer_margin = 4.0;
                ui.visuals_mut().text_cursor.stroke = Stroke::new(1.5, pal.caret);
                ui.visuals_mut().selection.bg_fill = pal.selection_bg;
                ui.visuals_mut().selection.stroke = Stroke::new(1.0, pal.caret);
                egui::ScrollArea::vertical()
                    .id_salt("latex-scroll")
                    .max_height(box_height)
                    .min_scrolled_height(box_height)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let width = ui.available_width();
                        ui.add(
                            egui::TextEdit::multiline(&mut self.latex)
                                .id(latex_id)
                                .frame(egui::Frame::NONE.inner_margin(egui::Margin {
                                    left: 8,
                                    right: 12,
                                    top: 6,
                                    bottom: 6,
                                }))
                                .desired_rows(1)
                                .desired_width(f32::INFINITY)
                                .min_size(Vec2::new(width, box_height))
                                .font(egui::TextStyle::Monospace)
                                .text_color(pal.ink),
                        )
                    })
                    .inner
            });
        ui.painter().rect_stroke(
            frame_resp.response.rect,
            theme::CONTROL_CORNER_RADIUS,
            Stroke::new(1.0, pal.border),
            egui::StrokeKind::Inside,
        );
        if frame_resp.inner.changed() {
            self.mark_source_changed(Instant::now());
        }
    }

    pub(super) fn configure_canvas_scroll_style(&self, ui: &mut egui::Ui, pal: CanvasPalette) {
        ui.visuals_mut().widgets.inactive.bg_fill = pal.scroll_inactive;
        ui.visuals_mut().widgets.inactive.fg_stroke.color = pal.scroll_inactive;
        ui.visuals_mut().widgets.hovered.bg_fill = pal.scroll_hover;
        ui.visuals_mut().widgets.hovered.fg_stroke.color = pal.scroll_hover;
        ui.visuals_mut().widgets.active.bg_fill = pal.scroll_active;
        ui.visuals_mut().widgets.active.fg_stroke.color = pal.scroll_active;
        ui.visuals_mut().extreme_bg_color = pal.scroll_track;

        let scroll = &mut ui.spacing_mut().scroll;
        if matches!(
            self.gpu_profile,
            GpuProfile::VirtualGpu | GpuProfile::Software
        ) {
            scroll.dormant_handle_opacity = 0.85;
            scroll.active_handle_opacity = 0.85;
            scroll.interact_handle_opacity = 0.85;
            scroll.dormant_background_opacity = 0.6;
            scroll.active_background_opacity = 0.6;
            scroll.interact_background_opacity = 0.6;
            scroll.floating_allocated_width = scroll.bar_width;
        } else {
            scroll.dormant_handle_opacity = 0.5;
            scroll.active_handle_opacity = 0.85;
            scroll.interact_handle_opacity = 1.0;
            scroll.dormant_background_opacity = 0.3;
            scroll.active_background_opacity = 0.6;
            scroll.interact_background_opacity = 0.85;
        }
    }

    pub(super) fn paint_placeholder(painter: &egui::Painter, rect: Rect, pal: CanvasPalette) {
        // Cover only the synthetic square; the formula texture and hit regions stay unchanged.
        painter.rect_filled(rect.expand(0.5), 0.0, pal.bg);
        let rect = rect.shrink(1.0);
        let radius = theme::CONTROL_CORNER_RADIUS;
        painter.rect_filled(rect, radius, pal.placeholder_bg);
        let mut path = vec![];
        egui::epaint::tessellator::path::rounded_rectangle(
            &mut path,
            egui::epaint::RoundedRect::new(rect, radius),
        );
        if let Some(&first) = path.first() {
            path.push(first);
        }
        painter.extend(egui::Shape::dashed_line(
            &path,
            Stroke::new(1.2, pal.placeholder_stroke),
            4.0,
            3.0,
        ));
    }

    /// Resolves the merged layout bounding box for node `id` in rendered equation coordinates.
    pub(super) fn hit_region(&self, id: u64) -> Option<eqntide::render::HitRegion> {
        let path = self.editor.path(id)?;
        let rendered = self.rendered.as_ref()?;
        if let Some(region) = rendered
            .preview
            .regions
            .iter()
            .find(|region| region.path == path)
        {
            return Some(region.clone());
        }
        let mut result: Option<eqntide::render::HitRegion> = None;
        for region in rendered
            .preview
            .regions
            .iter()
            .filter(|region| region.path.starts_with(&path))
        {
            if let Some(out) = &mut result {
                let right = (out.x + out.width).max(region.x + region.width);
                let bottom = (out.y + out.height).max(region.y + region.height);
                out.x = out.x.min(region.x);
                out.y = out.y.min(region.y);
                out.width = right - out.x;
                out.height = bottom - out.y;
            } else {
                result = Some(region.clone());
            }
        }
        result
    }

    pub(super) fn node_rect(&self, id: u64, origin: Pos2, scale: f32) -> Option<Rect> {
        let region = self.hit_region(id)?;
        Some(Rect::from_min_size(
            origin + Vec2::new(region.x * scale, region.y * scale),
            Vec2::new(
                region.width.max(2.0) * scale,
                region.height.max(6.0) * scale,
            ),
        ))
    }

    /// Computes the effective MathML script/displaystyle font size (pt) at node `id`.
    pub(super) fn caret_font_pt(&self, id: u64) -> f32 {
        let Some(path) = self.editor.path(id) else {
            return self.font_pt;
        };
        let mut node = self.editor.root();
        let mut em = self.font_pt;
        let mut display = true;
        for &child_idx in &path {
            let ds = node
                .attrs
                .iter()
                .find(|(attr_name, _)| attr_name == "displaystyle")
                .map(|(_, attr_val)| attr_val.as_str());
            match node.tag.as_str() {
                "mstyle" => {
                    if ds == Some("false") {
                        display = false;
                    } else if ds == Some("true") {
                        display = true;
                    }
                }
                "mfrac" => {
                    if ds == Some("false") || !display {
                        em *= 0.71;
                    }
                    display = false;
                }
                "msub" | "msup" | "msubsup" | "munder" | "mover" | "munderover"
                    if child_idx >= 1 =>
                {
                    em *= 0.71;
                    display = false;
                }
                "mroot" if child_idx == 1 => {
                    em *= 0.71;
                    display = false;
                }
                "msqrt" => {
                    display = false;
                }
                _ => {}
            }
            let Some(next) = node.children.get(child_idx) else {
                break;
            };
            node = next;
        }
        em.max(self.font_pt * 0.45)
    }

    /// Computes the screen-space caret bar rectangle for the active editor selection/cursor.
    pub(super) fn caret_rect(&self, origin: Pos2, scale: f32) -> Option<Rect> {
        let caret = self.editor.caret();
        let node = self.editor.node(caret.node)?;
        let region = self.hit_region(caret.node)?;
        let is_placeholder =
            caret.node != self.editor.root().id && node.text.is_empty() && node.children.is_empty();
        if is_placeholder {
            let inset_x = (region.width * 0.5).min(2.5 / scale);
            let inset_y = (2.0 / scale).min(region.height * 0.25);
            let top = region.y + inset_y;
            let height = (region.height - 2.0 * inset_y).max(4.0 / scale);
            return Some(Rect::from_min_size(
                origin + Vec2::new((region.x + inset_x) * scale, top * scale),
                Vec2::new(1.0, height * scale),
            ));
        }
        let x = if node.is_token() {
            if region.carets.len() == node.text.chars().count() + 1 {
                region.carets[caret.offset.min(region.carets.len() - 1)]
            } else {
                region.x
                    + region.width * caret.offset as f32 / node.text.chars().count().max(1) as f32
            }
        } else if let Some(previous_text) = caret
            .offset
            .checked_sub(1)
            .and_then(|index| node.children.get(index))
            .filter(|child| child.tag == "mtext")
            .and_then(|child| self.hit_region(child.id))
        {
            previous_text.x + previous_text.width
        } else if let Some(child) = node.children.get(caret.offset) {
            self.hit_region(child.id)
                .map_or(region.x, |child_region| child_region.x)
        } else if let Some(child) = node.children.last() {
            self.hit_region(child.id)
                .map_or(region.x + region.width, |child_region| {
                    child_region.x + child_region.width
                })
        } else {
            region.x
        };
        let em_pt = self.caret_font_pt(caret.node);
        let top = region.baseline - 0.72 * em_pt;
        let height = (0.72 + 0.22) * em_pt;
        Some(Rect::from_min_size(
            origin + Vec2::new(x * scale, top * scale),
            Vec2::new(1.0, (height * scale).max(6.0)),
        ))
    }

    /// Maps a screen-space pointer coordinate to the deepest matching structural [`Caret`].
    pub(super) fn hit_test_caret(&self, pos: Pos2, origin: Pos2, scale: f32) -> Option<Caret> {
        let rendered = self.rendered.as_ref()?;
        let point = (pos - origin) / scale;
        let matched = rendered
            .preview
            .regions
            .iter()
            .filter(|region| {
                point.x >= region.x - 1.0
                    && point.x <= region.x + region.width + 1.0
                    && point.y >= region.y - 1.0
                    && point.y <= region.y + region.height + 1.0
            })
            .max_by_key(|region| region.path.len());
        let Some(matched) = matched else {
            return Some(Caret {
                node: self.editor.root().id,
                offset: self.editor.root().children.len(),
            });
        };
        let mut path = matched.path.clone();
        let id = loop {
            if let Some(id) = self.editor.id_at_path(&path) {
                break id;
            }
            path.pop()?;
        };
        let mut id = id;
        for depth in 0..=path.len() {
            if let Some(ancestor) = self.editor.id_at_path(&path[..depth]) {
                if self
                    .editor
                    .node(ancestor)
                    .is_some_and(|node| node.is_opaque())
                {
                    id = ancestor;
                    break;
                }
            }
        }
        let node = self.editor.node(id)?;
        // Unknown elements are selected atomically, never flattened into text.
        let offset = if node.is_token() {
            if matched.carets.len() == node.text.chars().count() + 1 {
                matched
                    .carets
                    .iter()
                    .enumerate()
                    .min_by(|(_, left_x), (_, right_x)| {
                        (**left_x - point.x)
                            .abs()
                            .total_cmp(&(**right_x - point.x).abs())
                    })
                    .map_or(0, |(idx, _)| idx)
            } else {
                ((point.x - matched.x) / matched.width.max(1.0) * node.text.chars().count() as f32)
                    .round()
                    .clamp(0.0, node.text.chars().count() as f32) as usize
            }
        } else if node.is_row() {
            node.children
                .iter()
                .position(|child| {
                    self.hit_region(child.id)
                        .is_some_and(|region| point.x < region.x + region.width / 2.0)
                })
                .unwrap_or(node.children.len())
        } else {
            0
        };
        Some(Caret { node: id, offset })
    }

    /// Moves the caret up or down across stacked fractions, scripts, roots, matrix rows, or baselines.
    pub(super) fn move_caret_vertically(&mut self, down: bool, extend: bool) {
        let Some(current) = self.hit_region(self.editor.caret().node) else {
            return;
        };
        let x = current.x + current.width / 2.0;
        if let Some(mut path) = self.editor.path(self.editor.caret().node) {
            while let Some(index) = path.pop() {
                let Some(parent_id) = self.editor.id_at_path(&path) else {
                    break;
                };
                let Some(parent) = self.editor.node(parent_id) else {
                    break;
                };
                let target_index = match (parent.tag.as_str(), index, down) {
                    ("mfrac", 0, true) => Some(1),
                    ("mfrac", 1, false) => Some(0),
                    ("mroot", 0, false) => Some(1),
                    ("mroot", 1, true) => Some(0),
                    ("msup" | "mover", 0, false) => Some(1),
                    ("msup" | "mover", 1, true) => Some(0),
                    ("msub" | "munder", 0, true) => Some(1),
                    ("msub" | "munder", 1, false) => Some(0),
                    ("msubsup" | "munderover", 0, true) => Some(1),
                    ("msubsup" | "munderover", 0, false) => Some(2),
                    ("msubsup" | "munderover", 1, false) | ("msubsup" | "munderover", 2, true) => {
                        Some(0)
                    }
                    _ => None,
                };
                if let Some(target) = target_index
                    .and_then(|child_idx| parent.children.get(child_idx))
                    .map(|child| child.id)
                {
                    self.enter_subtree(target, x, extend);
                    return;
                }
                if parent.tag == "mtr" {
                    let column = index;
                    if let Some(row) = path.pop() {
                        if let Some(table) = self
                            .editor
                            .id_at_path(&path)
                            .and_then(|id| self.editor.node(id))
                        {
                            if table.tag == "mtable" {
                                let other = if down {
                                    row.checked_add(1)
                                } else {
                                    row.checked_sub(1)
                                };
                                if let Some(cell) = other
                                    .and_then(|row_idx| table.children.get(row_idx))
                                    .and_then(|row_node| row_node.children.get(column))
                                {
                                    let id = cell.id;
                                    self.enter_subtree(id, x, extend);
                                    return;
                                }
                            }
                        }
                        path.push(row);
                    }
                }
            }
        }
        let Some(rendered) = &self.rendered else {
            return;
        };
        let candidate = rendered
            .preview
            .regions
            .iter()
            .filter(|region| {
                if down {
                    region.baseline > current.baseline + 1.0
                } else {
                    region.baseline < current.baseline - 1.0
                }
            })
            .filter_map(|region| {
                let id = self.editor.id_at_path(&region.path)?;
                let node = self.editor.node(id)?;
                if !node.is_token() && !(node.is_row() && node.children.is_empty()) {
                    return None;
                }
                let score = (region.baseline - current.baseline).abs() * 3.0
                    + (region.x + region.width / 2.0 - x).abs();
                Some((
                    score,
                    Caret {
                        node: id,
                        offset: 0,
                    },
                ))
            })
            .min_by(|first, second| first.0.total_cmp(&second.0));
        if let Some((_, caret)) = candidate {
            self.editor.set_caret(caret, extend);
        }
    }

    pub(super) fn enter_subtree(&mut self, id: u64, x: f32, extend: bool) {
        let Some(path) = self.editor.path(id) else {
            return;
        };
        let Some(rendered) = &self.rendered else {
            return;
        };
        let target = rendered
            .preview
            .regions
            .iter()
            .filter(|region| region.path.starts_with(&path))
            .filter_map(|region| {
                let node_id = self.editor.id_at_path(&region.path)?;
                let node = self.editor.node(node_id)?;
                if !node.is_token() && !(node.is_row() && node.children.is_empty()) {
                    return None;
                }
                let offset = if node.is_token() {
                    region
                        .carets
                        .iter()
                        .enumerate()
                        .min_by(|(_, left_x), (_, right_x)| {
                            (**left_x - x).abs().total_cmp(&(**right_x - x).abs())
                        })
                        .map_or(0, |(idx, _)| idx.min(node.text.chars().count()))
                } else {
                    0
                };
                Some((
                    (region.x + region.width / 2.0 - x).abs(),
                    Caret {
                        node: node_id,
                        offset,
                    },
                ))
            })
            .min_by(|first, second| first.0.total_cmp(&second.0));
        if let Some((_, caret)) = target {
            self.editor.set_caret(caret, extend);
        }
    }

    /// Returns whether the canvas caret should be painted on this frame and schedules blink wakes.
    pub(super) fn is_canvas_caret_visible(&mut self, ui: &egui::Ui) -> bool {
        let style = ui.visuals().text_cursor.clone();
        if !style.blink {
            self.caret_blink = None;
            return true;
        }
        let now = ui.input(|input| input.time);
        let caret = self.editor.caret();
        let revision = self.editor.revision;
        let command_len = self
            .command_input
            .as_ref()
            .map_or(0, |input| input.name.len() + 1);
        let start = match self.caret_blink {
            Some((prev_caret, prev_rev, prev_cmd, start))
                if prev_caret == caret && prev_rev == revision && prev_cmd == command_len =>
            {
                start
            }
            _ => {
                self.caret_blink = Some((caret, revision, command_len, now));
                now
            }
        };
        let on = f64::from(style.on_duration.max(0.05));
        let off = f64::from(style.off_duration.max(0.05));
        let period = on + off;
        let elapsed = (now - start).max(0.0);
        let phase = elapsed % period;
        let visible = phase < on;
        let next_toggle = if visible { on - phase } else { period - phase };
        ui.ctx()
            .request_repaint_after(Duration::from_secs_f64(next_toggle.max(0.01)));
        visible
    }

    pub(super) fn sync_editor_focus(&mut self, ui: &mut egui::Ui) {
        if self.tab != 0 || !ui.is_enabled() {
            return;
        }
        let canvas_id = Self::editor_focus_id(StartupFocus::Canvas);
        let latex_id = Self::editor_focus_id(StartupFocus::Latex);
        let (has_canvas, has_latex) =
            ui.memory(|mem| (mem.has_focus(canvas_id), mem.has_focus(latex_id)));
        if let Some(target) = self.pending_focus_restore.take() {
            if has_canvas {
                self.active_editor_focus = StartupFocus::Canvas;
            } else if has_latex {
                self.active_editor_focus = StartupFocus::Latex;
                self.dismiss_command_input();
            } else {
                ui.memory_mut(|mem| mem.request_focus(Self::editor_focus_id(target)));
                self.active_editor_focus = target;
            }
        } else if has_canvas {
            self.active_editor_focus = StartupFocus::Canvas;
        } else if has_latex {
            self.active_editor_focus = StartupFocus::Latex;
            self.dismiss_command_input();
        }
    }

    pub(super) fn editor_focus_id(target: StartupFocus) -> egui::Id {
        match target {
            StartupFocus::Canvas => egui::Id::unique("equation-canvas"),
            StartupFocus::Latex => egui::Id::unique("latex-source"),
        }
    }
}
