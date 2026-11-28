//! Visual equation canvas painting, placeholder hit-testing, caret geometry, and LaTeX source box.

use super::{App, CONTENT_PADDING, LATEX_INPUT_HEIGHT, LATEX_PANEL_HEIGHT};
use crate::{
    settings::StartupFocus,
    theme::{self, CanvasPalette},
};
use eframe::egui::{self, Pos2, Rect, Stroke, Vec2};
use eqntide_editor::Caret;
use std::time::{Duration, Instant};

impl App {
    /// Renders the main editor workspace: top palette ribbon, equation canvas, and bottom LaTeX box.
    pub(super) fn draw_editor(&mut self, ui: &mut egui::Ui) {
        self.draw_palette(ui);
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
        // Keep the preview legible while editing an invalid/pending LaTeX draft,
        // but dim uniformly when disabled by a modal or blocking operation.
        if self.busy
            || self.pending.is_some()
            || self.matrix_dialog.is_some()
            || self.shortcut_conflict.is_some()
        {
            ui.set_opacity(ui.visuals().disabled_alpha());
        } else {
            ui.set_opacity(1.0);
        }
        let height = (ui.available_height() - 26.0).max(12.0);
        let pal = self.active_canvas_palette(ui.visuals().dark_mode);
        egui::Frame::new()
            .fill(pal.bg)
            .corner_radius(theme::SURFACE_CORNER_RADIUS)
            .stroke(Stroke::NONE)
            .inner_margin(theme::SPACE_LG)
            .show(ui, |ui| {
                Self::configure_canvas_scroll_style(ui, pal);

                let viewport_size =
                    Vec2::new(ui.available_width(), ui.available_height().min(height));
                let viewport_min = ui.next_widget_position();
                let canvas_hit_rect =
                    Rect::from_min_size(viewport_min, viewport_size).expand(theme::SPACE_LG);
                let scroll_id = ui.make_persistent_id(egui::IdSalt::new("formula-scroll"));
                let mut anchored_zoom = None;
                if ui.is_enabled() && ui.rect_contains_pointer(canvas_hit_rect) {
                    let steps = ctrl_wheel_steps(ui, scroll_id.with("ctrl-wheel-accum"));
                    if steps != 0 {
                        let old_zoom = self.zoom;
                        let new_zoom =
                            step_snapped_value(old_zoom, steps, 10.0, crate::settings::ZOOM_RANGE);
                        if (new_zoom - old_zoom).abs() > f32::EPSILON {
                            let old_offset = egui::scroll_area::State::load(ui.ctx(), scroll_id)
                                .map_or(Vec2::ZERO, |state| state.clamped_offset());
                            let pointer_pos = ui
                                .ctx()
                                .input(|input| input.pointer.interact_pos())
                                .unwrap_or(viewport_min);
                            let cursor_in_viewport =
                                (pointer_pos - viewport_min).clamp(Vec2::ZERO, viewport_size);
                            anchored_zoom = Some((old_zoom, old_offset, cursor_in_viewport));
                            self.zoom = new_zoom;
                            self.persisted.zoom = new_zoom;
                            self.settings_dirty = true;
                            ui.ctx().request_repaint();
                        }
                    }
                }

                let scale = self.zoom * 96.0 / 72.0;
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
                let content_size = size + Vec2::new(12.0, 20.0);
                let gutter = ui.spacing().scroll.allocated_width();
                // Start with the full viewport so two existing gutters cannot keep each other open.
                let overflows_x = content_size.x > viewport_size.x.ceil();
                let overflows_y = content_size.y > viewport_size.y.ceil();
                let scroll_x = overflows_x
                    || (overflows_y && content_size.x > (viewport_size.x - gutter).ceil());
                let scroll_y = overflows_y
                    || (overflows_x && content_size.y > (viewport_size.y - gutter).ceil());
                let mut scroll_area = egui::ScrollArea::new([scroll_x, scroll_y])
                    .id_salt("formula-scroll")
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                    .max_height(height)
                    .min_scrolled_height(0.0)
                    .auto_shrink([false, false]);
                if let Some((old_zoom, old_offset, cursor_in_viewport)) = anchored_zoom {
                    let effective_viewport = Vec2::new(
                        if scroll_y {
                            (viewport_size.x - gutter).max(1.0)
                        } else {
                            viewport_size.x
                        },
                        if scroll_x {
                            (viewport_size.y - gutter).max(1.0)
                        } else {
                            viewport_size.y
                        },
                    );
                    let next_offset = anchored_canvas_scroll_offset(
                        old_zoom,
                        self.zoom,
                        old_offset,
                        cursor_in_viewport.min(effective_viewport),
                        content_size,
                        effective_viewport,
                    );
                    scroll_area = scroll_area.scroll_offset(next_offset);
                }
                scroll_area.show_viewport(ui, |ui, viewport| {
                    // Only the formula contributes to scrollable content. Filling the
                    // viewport via allocate_space can round past its width at fractional DPI.
                    let (_, content_rect) = ui.allocate_space(content_size);
                    // Empty canvas space remains clickable without creating overflow.
                    let rect =
                        Rect::from_min_size(content_rect.min, viewport.size().max(content_size));
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
                                let extend =
                                    ui.input(|input| input.modifiers.shift) || response.dragged();
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
                    if let Some(selection_rect) = self.selection_rect(origin, scale) {
                        ui.painter().rect_filled(
                            selection_rect,
                            theme::CANVAS_OVERLAY_CORNER_RADIUS,
                            pal.selection_bg,
                        );
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
        let latex_hit_rect = Rect::from_min_size(
            ui.next_widget_position(),
            Vec2::new(ui.available_width(), box_height),
        );
        if ui.is_enabled() && ui.rect_contains_pointer(latex_hit_rect) {
            let steps = ctrl_wheel_steps(ui, latex_id.with("ctrl-wheel-accum"));
            if steps != 0 {
                let next_font = step_snapped_value(
                    self.latex_font_pt,
                    steps,
                    2.0,
                    crate::settings::LATEX_FONT_PT_RANGE,
                );
                if (next_font - self.latex_font_pt).abs() > f32::EPSILON {
                    self.latex_font_pt = next_font;
                    self.persisted.latex_font_pt = next_font;
                    self.settings_dirty = true;
                }
            }
        }
        let frame_resp = egui::Frame::new()
            .fill(pal.bg)
            .corner_radius(theme::CONTROL_CORNER_RADIUS)
            .show(ui, |ui| {
                Self::configure_canvas_scroll_style(ui, pal);
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
                                .font(egui::FontId::monospace(self.latex_font_pt))
                                .text_color(pal.ink),
                        )
                    })
                    .inner
            });
        if frame_resp.inner.changed() {
            self.mark_source_changed(Instant::now());
        }
    }

    fn configure_canvas_scroll_style(ui: &mut egui::Ui, pal: CanvasPalette) {
        let scroll = &mut ui.spacing_mut().scroll;
        scroll.bar_width = theme::EDITOR_SCROLLBAR_WIDTH;
        scroll.bar_inner_margin = theme::CONTENT_SCROLLBAR_GAP;
        pal.apply_scrollbar_colors(ui.visuals_mut());
    }

    /// Paints an empty structural slot as an unfilled dashed box with light corner rounding.
    pub(super) fn paint_placeholder(painter: &egui::Painter, rect: Rect, pal: CanvasPalette) {
        let rect = rect.shrink(1.0);
        let radius = theme::CANVAS_OVERLAY_CORNER_RADIUS;
        if pal.placeholder_bg.a() > 0 {
            painter.rect_filled(rect, radius, pal.placeholder_bg);
        }
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

    /// Computes a single contiguous screen-space rectangle covering the active selection span.
    pub(super) fn selection_rect(&self, origin: Pos2, scale: f32) -> Option<Rect> {
        let (container_id, start, end) = self.editor.selection_span()?;
        let node = self.editor.node(container_id)?;
        let span_pt = if node.is_token() {
            let region = self.hit_region(container_id)?;
            let char_count = node.text.chars().count();
            let start_idx = start.min(char_count);
            let end_idx = end.min(char_count);
            let (left_x, right_x) = if region.carets.len() == char_count + 1 {
                let first_x = region.carets[start_idx];
                let second_x = region.carets[end_idx];
                (first_x.min(second_x), first_x.max(second_x))
            } else {
                let total = char_count.max(1) as f32;
                let first_x = region.x + region.width * start_idx as f32 / total;
                let second_x = region.x + region.width * end_idx as f32 / total;
                (first_x.min(second_x), first_x.max(second_x))
            };
            let em_pt = self.caret_font_pt(container_id);
            let top_y = region.y.min(region.baseline - 0.72 * em_pt);
            let bottom_y = (region.y + region.height).max(region.baseline + 0.22 * em_pt);
            Some((left_x, top_y, right_x.max(left_x + 2.0), bottom_y))
        } else {
            let start_idx = start.min(node.children.len());
            let end_idx = end.min(node.children.len());
            if start_idx >= end_idx {
                return None;
            }
            let mut bounds: Option<(f32, f32, f32, f32)> = None;
            for child in &node.children[start_idx..end_idx] {
                let Some(region) = self.hit_region(child.id) else {
                    continue;
                };
                let em_pt = self.caret_font_pt(child.id);
                let child_top = region.y.min(region.baseline - 0.72 * em_pt);
                let child_bottom = (region.y + region.height).max(region.baseline + 0.22 * em_pt);
                let child_right = region.x + region.width.max(2.0);
                bounds = Some(match bounds {
                    Some((min_x, min_y, max_x, max_y)) => (
                        min_x.min(region.x),
                        min_y.min(child_top),
                        max_x.max(child_right),
                        max_y.max(child_bottom),
                    ),
                    None => (region.x, child_top, child_right, child_bottom),
                });
            }
            bounds
        }?;
        let (min_x, min_y, max_x, max_y) = span_pt;
        Some(Rect::from_min_max(
            origin + Vec2::new(min_x * scale, min_y * scale),
            origin + Vec2::new(max_x * scale, max_y * scale),
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
        let raw_x = if node.is_token() {
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
        let x = self.selection_caret_x(origin, scale).unwrap_or(raw_x);
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
            let offset = node
                .children
                .iter()
                .position(|child| {
                    self.hit_region(child.id)
                        .is_some_and(|region| point.x < region.x + region.width / 2.0)
                })
                .unwrap_or(node.children.len());
            if let (Some(prev_child), Some(next_child)) = (
                offset.checked_sub(1).and_then(|idx| node.children.get(idx)),
                node.children.get(offset),
            ) {
                if let (Some(prev_region), Some(next_region)) = (
                    self.hit_region(prev_child.id),
                    self.hit_region(next_child.id),
                ) {
                    let gap_midpoint = (prev_region.x + prev_region.width + next_region.x) * 0.5;
                    if point.x < gap_midpoint
                        && prev_child.is_token()
                        && !prev_child.is_fence_delimiter()
                    {
                        return Some(Caret {
                            node: prev_child.id,
                            offset: prev_child.text.chars().count(),
                        });
                    }
                }
            }
            offset
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

    /// Places the caret at the closest stop inside subtree `id` horizontally aligned with `x`.
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

    /// Synchronizes keyboard focus tracking between the visual canvas and LaTeX source box.
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

    /// Returns the deterministic `egui::Id` associated with `target`'s editor widget.
    pub(super) fn editor_focus_id(target: StartupFocus) -> egui::Id {
        match target {
            StartupFocus::Canvas => egui::Id::unique("equation-canvas"),
            StartupFocus::Latex => egui::Id::unique("latex-source"),
        }
    }

    /// Snaps the caret bar to the moving edge of an active selection when the caret lies on
    /// that outer boundary, preventing inter-element spacing jumps while dragging.
    fn selection_caret_x(&self, origin: Pos2, scale: f32) -> Option<f32> {
        let anchor = self.editor.anchor()?;
        let head = self.editor.caret();
        if anchor == head {
            return None;
        }
        let (container_id, start, end) = self.editor.selection_span()?;
        let selection_rect = self.selection_rect(origin, scale)?;
        let anchor_path = self.editor.path(anchor.node)?;
        let head_path = self.editor.path(head.node)?;
        let container_depth = self.editor.path(container_id)?.len();
        let is_forward = self.is_caret_at_or_before(anchor, &anchor_path, head, &head_path);
        let is_on_outer_boundary = if head.node == container_id {
            true
        } else if let Some(&child_idx) = head_path.get(container_depth) {
            if is_forward {
                child_idx == end || child_idx + 1 == end
            } else {
                child_idx == start || child_idx + 1 == start
            }
        } else {
            false
        };
        if !is_on_outer_boundary {
            return None;
        }
        if head.node != container_id {
            let container = self.editor.node(container_id)?;
            if let Some(child_idx) = head_path.get(container_depth).copied() {
                if let Some(child) = container.children.get(child_idx) {
                    if matches!(child.tag.as_str(), "mfrac" | "mroot" | "msqrt")
                        && head_path.len() > container_depth + 1
                    {
                        return None;
                    }
                }
            }
        }
        let target_screen_x = if is_forward {
            selection_rect.right()
        } else {
            selection_rect.left()
        };
        Some((target_screen_x - origin.x) / scale)
    }

    /// Returns `true` if `left` precedes or equals `right` in structural pre-order.
    fn is_caret_at_or_before(
        &self,
        left: Caret,
        left_path: &[usize],
        right: Caret,
        right_path: &[usize],
    ) -> bool {
        if left.node == right.node {
            return left.offset <= right.offset;
        }
        let common = left_path
            .iter()
            .zip(right_path)
            .take_while(|(first, second)| first == second)
            .count();
        match (left_path.get(common), right_path.get(common)) {
            (Some(&left_idx), Some(&right_idx)) => {
                let is_mroot = self
                    .editor
                    .id_at_path(&left_path[..common])
                    .and_then(|id| self.editor.node(id))
                    .is_some_and(|node| node.tag == "mroot");
                if is_mroot {
                    left_idx >= right_idx
                } else {
                    left_idx <= right_idx
                }
            }
            (None, Some(&right_idx)) => left.offset <= right_idx,
            (Some(&left_idx), None) => left_idx < right.offset,
            (None, None) => left.offset <= right.offset,
        }
    }
}

fn ctrl_wheel_steps(ui: &egui::Ui, accum_id: egui::Id) -> i32 {
    const POINT_STEP_THRESHOLD: f32 = 16.0;
    let (has_ctrl_wheel, discrete_steps, point_delta) = ui.ctx().input(|input| {
        let frame_ctrl = input.modifiers.ctrl || input.modifiers.command;
        let mut has_ctrl_wheel = false;
        let mut discrete_steps = 0_i32;
        let mut point_delta = 0.0_f32;
        for event in &input.raw.events {
            if let egui::Event::MouseWheel {
                unit,
                delta,
                modifiers,
                ..
            } = event
            {
                let ctrl_held = frame_ctrl || modifiers.ctrl || modifiers.command;
                if ctrl_held && delta.y.abs() > f32::EPSILON {
                    has_ctrl_wheel = true;
                    match unit {
                        egui::MouseWheelUnit::Line | egui::MouseWheelUnit::Page => {
                            discrete_steps += delta.y.signum() as i32;
                        }
                        egui::MouseWheelUnit::Point => {
                            point_delta += delta.y;
                        }
                    }
                }
            }
        }
        (has_ctrl_wheel, discrete_steps, point_delta)
    });
    if !has_ctrl_wheel {
        return 0;
    }
    ui.ctx().input_mut(|input| {
        input.smooth_scroll_delta = Vec2::ZERO;
    });
    let mut accum = ui
        .ctx()
        .data_mut(|data| data.get_temp::<f32>(accum_id))
        .unwrap_or(0.0);
    if discrete_steps != 0 {
        accum = 0.0;
    }
    if point_delta.abs() > f32::EPSILON {
        if accum * point_delta < 0.0 {
            accum = 0.0;
        }
        accum += point_delta;
    }
    let point_steps = (accum / POINT_STEP_THRESHOLD).trunc() as i32;
    if point_steps != 0 {
        accum -= point_steps as f32 * POINT_STEP_THRESHOLD;
    }
    ui.ctx().data_mut(|data| data.insert_temp(accum_id, accum));
    discrete_steps + point_steps
}

fn step_snapped_value(
    current: f32,
    steps: i32,
    steps_per_unit: f64,
    range: std::ops::RangeInclusive<f32>,
) -> f32 {
    let step = 1.0 / steps_per_unit;
    let next = f64::from(current) + f64::from(steps) * step;
    let snapped = ((next * steps_per_unit).round() / steps_per_unit) as f32;
    snapped.clamp(*range.start(), *range.end())
}

fn anchored_canvas_scroll_offset(
    old_zoom: f32,
    new_zoom: f32,
    old_offset: Vec2,
    cursor_in_viewport: Vec2,
    content_size: Vec2,
    viewport_size: Vec2,
) -> Vec2 {
    let pad = Vec2::new(4.0, 8.0);
    let ratio = new_zoom / old_zoom.max(f32::EPSILON);
    let raw_offset = pad + (old_offset + cursor_in_viewport - pad) * ratio - cursor_in_viewport;
    let max_offset = (content_size - viewport_size).max(Vec2::ZERO);
    raw_offset.clamp(Vec2::ZERO, max_offset)
}
