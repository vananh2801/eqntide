//! Palette ribbon UI, DPI-aware icon/glyph atlas builder, and shared button/card helpers.

use super::{
    tabler_icon_bytes, App, MatrixDialog, PaletteAtlas, PaletteAtlasEntry, PALETTE_BUTTON_SIZE,
    PALETTE_GAP, PALETTE_ICON_TARGET_EM, UI_ICON_SIZE,
};
use crate::{
    palette,
    settings::MAX_RECENT_PALETTE,
    theme::{self, ThemeTokens},
};
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use std::collections::HashMap;

impl App {
    /// Renders the horizontal template/symbol palette grid for the active sidebar category.
    pub(super) fn draw_palette(&mut self, ui: &mut egui::Ui) {
        let height = ui.available_height();
        let items = palette::items_for_ui_group(&self.category, &self.recent_palette);
        if self.category == palette::RECENT_GROUP && items.is_empty() {
            ui.allocate_ui_with_layout(
                Vec2::new(ui.available_width(), height),
                egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                |ui| {
                    ui.weak("Click any symbol or template in the palette to pin it here for quick reuse.");
                },
            );
            return;
        }
        let has_matrix_tools = palette::group_contains(&self.category, "Matrices");
        let fitting_columns = ((ui.available_width() + PALETTE_GAP)
            / (PALETTE_BUTTON_SIZE.x + PALETTE_GAP))
            .floor() as usize;
        // Native DPI/startup adjustments may briefly give us less than the minimum width.
        // Keep three rows in that frame too, scrolling horizontally if necessary.
        let item_count = items.len() + usize::from(has_matrix_tools);
        let columns = fitting_columns.max(item_count.div_ceil(3)).max(1);
        ui.scope(|ui| {
            ui.style_mut().always_scroll_the_only_direction = true;
            egui::ScrollArea::horizontal()
                .id_salt(("palette", &self.category))
                .vertical_scroll_offset(0.0)
                .max_height(height)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    egui::Grid::new(("palette-grid", &self.category))
                        .num_columns(columns)
                        .spacing(Vec2::splat(PALETTE_GAP))
                        .show(ui, |ui| {
                            ui.spacing_mut().button_padding = Vec2::splat(3.0);
                            ui.spacing_mut().item_spacing = Vec2::splat(PALETTE_GAP);
                            let ppp = ui.ctx().pixels_per_point();
                            self.sync_atlas_dpi(ppp);
                            if !self.palette_atlases.contains_key(&self.category) {
                                if let Some(atlas) =
                                    Self::build_palette_atlas(ui.ctx(), &self.category, &items, ppp)
                                {
                                    self.palette_atlases.insert(self.category.clone(), atlas);
                                }
                            }
                            let icon_color = Self::icon_tint(ui);
                            let mut clicked_source = None;
                            {
                                let atlas = self.palette_atlases.get(&self.category);
                                for (index, (_, label, source)) in items.iter().enumerate() {
                                    let response = if let Some((atlas, entry)) =
                                        atlas.and_then(|cached| {
                                            cached.entries.get(source).map(|entry| (cached, entry))
                                        }) {
                                        let image = egui::Image::new((
                                            atlas.texture.id(),
                                            entry.display_size,
                                        ))
                                        .uv(entry.uv)
                                        .tint(icon_color);
                                        ui.add_sized(PALETTE_BUTTON_SIZE, egui::Button::new(image))
                                    } else {
                                        ui.add_sized(PALETTE_BUTTON_SIZE, egui::Button::new(*label))
                                    };
                                    response.widget_info(|| {
                                        egui::WidgetInfo::labeled(
                                            egui::Role::Button,
                                            ui.is_enabled(),
                                            *label,
                                        )
                                    });
                                    if response
                                        .on_hover_ui(|ui| {
                                            ui.label(*label);
                                            ui.weak(*source);
                                        })
                                        .clicked()
                                    {
                                        clicked_source = Some(*source);
                                    }
                                    if (index + 1) % columns == 0 {
                                        ui.end_row();
                                    }
                                }
                            }
                            if let Some(source) = clicked_source {
                                self.command_input = None;
                                self.latex_groups.clear();
                                self.expects_latex_group = false;
                                self.record_recent_palette(source);
                                self.insert_template(source);
                                self.pending_focus_restore = Some(self.active_editor_focus);
                            }
                            if has_matrix_tools
                                && ui
                                    .add_sized(PALETTE_BUTTON_SIZE, egui::Button::new("⋯"))
                                    .on_hover_text("Custom matrix")
                                    .clicked()
                            {
                                self.dismiss_command_input();
                                self.matrix_dialog = Some(MatrixDialog::default());
                                ui.memory_mut(|mem| {
                                    mem.surrender_focus(Self::editor_focus_id(
                                        self.active_editor_focus,
                                    ))
                                });
                            }
                        });
                });
        });
    }

    /// Parses a LaTeX template snippet and inserts it at the current caret/selection.
    pub(super) fn insert_template(&mut self, source: &str) {
        match palette::template(source) {
            Ok(template) => {
                if palette::attaches_to_previous(source) {
                    self.editor.insert_template_wrapping_previous(&template);
                } else {
                    self.editor.insert_template(&template);
                }
                self.mark_canvas_changed();
            }
            Err(err) => self.status = err,
        }
    }

    pub(super) fn record_recent_palette(&mut self, source: &'static str) {
        self.recent_palette.retain(|&existing| existing != source);
        self.recent_palette.insert(0, source);
        self.recent_palette.truncate(MAX_RECENT_PALETTE);
        self.palette_atlases.remove(palette::RECENT_GROUP);
        self.persisted.recent_palette = self
            .recent_palette
            .iter()
            .map(|&item_src| item_src.to_owned())
            .collect();
        self.settings_dirty = true;
    }

    pub(super) fn sync_atlas_dpi(&mut self, ppp: f32) {
        if (self.palette_ppp - ppp).abs() > f32::EPSILON {
            self.palette_atlases.clear();
            self.palette_ppp = ppp;
        }
    }

    /// Rasterizes and packs all glyph previews in a palette category into a single texture atlas.
    pub(super) fn build_palette_atlas(
        ctx: &egui::Context,
        category: &str,
        items: &[&palette::PaletteItem],
        ppp: f32,
    ) -> Option<PaletteAtlas> {
        let mut rendered = Vec::new();
        for (_, _, source) in items {
            let Ok(math) = palette::preview_template(source) else {
                continue;
            };
            let Ok(mask) = eqntide::render_alpha_at(
                &math,
                PALETTE_ICON_TARGET_EM * ppp,
                40.0 * ppp,
                26.0 * ppp,
            ) else {
                continue;
            };
            if mask.width > 0 && mask.height > 0 {
                rendered.push((*source, mask));
            }
        }
        Self::build_mask_atlas(ctx, &format!("palette-atlas-{category}"), rendered, ppp)
    }

    /// Packs rendered alpha masks row-by-row into a GPU texture atlas with gamma compensation.
    pub(super) fn build_mask_atlas(
        ctx: &egui::Context,
        texture_name: &str,
        rendered: Vec<(&'static str, eqntide::AlphaMask)>,
        ppp: f32,
    ) -> Option<PaletteAtlas> {
        if rendered.is_empty() {
            return None;
        }
        const PAD: usize = 1;
        const MAX_ATLAS_W: usize = 512;
        let mut placements = Vec::with_capacity(rendered.len());
        let mut cur_x = PAD;
        let mut cur_y = PAD;
        let mut row_h = 0usize;
        let mut atlas_w = 0usize;
        for (idx, (_, mask)) in rendered.iter().enumerate() {
            let width = mask.width as usize;
            let height = mask.height as usize;
            if cur_x > PAD && cur_x + width + PAD > MAX_ATLAS_W {
                cur_x = PAD;
                cur_y += row_h + PAD;
                row_h = 0;
            }
            placements.push((idx, cur_x, cur_y, width, height));
            cur_x += width + PAD;
            atlas_w = atlas_w.max(cur_x);
            row_h = row_h.max(height);
        }
        let atlas_h = (cur_y + row_h + PAD).max(1);
        let atlas_w = atlas_w.max(1);
        let mut rgba = vec![0u8; atlas_w * atlas_h * 4];
        let mut entries = HashMap::with_capacity(placements.len());
        let mut gamma_lut = [0u8; 256];
        for (idx, slot) in gamma_lut.iter_mut().enumerate().skip(1) {
            let alpha_norm = idx as f32 / 255.0;
            *slot = (alpha_norm.powf(0.70) * 255.0).round().min(255.0) as u8;
        }
        for (idx, glyph_x, glyph_y, width, height) in placements {
            let (key, mask) = &rendered[idx];
            for y in 0..height {
                let src_row = y * width;
                let dst_row = (glyph_y + y) * atlas_w + glyph_x;
                for x in 0..width {
                    let alpha = mask.alpha[src_row + x];
                    if alpha == 0 {
                        continue;
                    }
                    let dst = (dst_row + x) * 4;
                    rgba[dst] = 255;
                    rgba[dst + 1] = 255;
                    rgba[dst + 2] = 255;
                    rgba[dst + 3] = gamma_lut[alpha as usize];
                }
            }
            let uv = Rect::from_min_max(
                Pos2::new(
                    glyph_x as f32 / atlas_w as f32,
                    glyph_y as f32 / atlas_h as f32,
                ),
                Pos2::new(
                    (glyph_x + width) as f32 / atlas_w as f32,
                    (glyph_y + height) as f32 / atlas_h as f32,
                ),
            );
            entries.insert(
                *key,
                PaletteAtlasEntry {
                    uv,
                    display_size: Vec2::new(width as f32 / ppp, height as f32 / ppp),
                },
            );
        }
        let image = egui::ColorImage::from_rgba_unmultiplied([atlas_w, atlas_h], &rgba);
        let texture = ctx.load_texture(texture_name, image, egui::TextureOptions::NEAREST);
        Some(PaletteAtlas { texture, entries })
    }

    #[cfg(test)]
    pub(super) fn palette_texture_ids(&self) -> std::collections::HashSet<egui::TextureId> {
        self.palette_atlases
            .values()
            .map(|atlas| atlas.texture.id())
            .collect()
    }

    pub(super) fn ui_icon(
        &mut self,
        ctx: &egui::Context,
        name: &'static str,
        size: f32,
        tint: Color32,
    ) -> egui::Image<'static> {
        let texture_id = self
            .ui_icons
            .entry(name)
            .or_insert_with(|| {
                let pixels = image::load_from_memory(tabler_icon_bytes(name))
                    .expect("bundled tabler icon is valid")
                    .to_rgba8();
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [pixels.width() as usize, pixels.height() as usize],
                    pixels.as_raw(),
                );
                ctx.load_texture(
                    format!("tabler-icon-{name}"),
                    image,
                    egui::TextureOptions::LINEAR,
                )
            })
            .id();
        egui::Image::new((texture_id, Vec2::splat(size))).tint(tint)
    }

    #[cfg(test)]
    pub(super) fn ui_icon_texture_id(&self, name: &str) -> Option<egui::TextureId> {
        self.ui_icons.get(name).map(|texture| texture.id())
    }

    pub(super) fn icon_tint(ui: &egui::Ui) -> Color32 {
        ThemeTokens::from_ui(ui).icon_tint(false, ui.is_enabled())
    }

    pub(super) fn primary_button_with<R>(
        ui: &mut egui::Ui,
        enabled: bool,
        add_button: impl FnOnce(&mut egui::Ui) -> R,
    ) -> R {
        let tokens = ThemeTokens::from_ui(ui);
        ui.scope(|ui| {
            let widgets = &mut ui.visuals_mut().widgets;
            if enabled {
                widgets.inactive.bg_fill = tokens.primary_bg;
                widgets.inactive.weak_bg_fill = tokens.primary_bg;
                widgets.inactive.bg_stroke = Stroke::new(1.0, tokens.primary_bg);
                widgets.inactive.fg_stroke = Stroke::new(1.0, tokens.primary_fg);

                widgets.hovered.bg_fill = tokens.primary_hover_bg;
                widgets.hovered.weak_bg_fill = tokens.primary_hover_bg;
                widgets.hovered.bg_stroke = Stroke::new(1.0, tokens.primary_hover_bg);
                widgets.hovered.fg_stroke = Stroke::new(1.0, tokens.primary_fg);

                widgets.active.bg_fill = tokens.primary_active_bg;
                widgets.active.weak_bg_fill = tokens.primary_active_bg;
                widgets.active.bg_stroke = Stroke::new(1.0, tokens.primary_active_bg);
                widgets.active.fg_stroke = Stroke::new(1.0, tokens.primary_fg);
            } else {
                widgets.noninteractive.bg_fill = tokens.control_bg;
                widgets.noninteractive.weak_bg_fill = tokens.control_bg;
                widgets.noninteractive.bg_stroke = Stroke::new(1.0, tokens.border);
                widgets.noninteractive.fg_stroke = Stroke::new(1.0, tokens.text_primary);
                widgets.inactive.bg_fill = tokens.control_bg;
                widgets.inactive.weak_bg_fill = tokens.control_bg;
                widgets.inactive.bg_stroke = Stroke::new(1.0, tokens.border);
                widgets.inactive.fg_stroke = Stroke::new(1.0, tokens.text_primary);
            }
            ui.add_enabled_ui(enabled, add_button).inner
        })
        .inner
    }

    pub(super) fn primary_button(ui: &mut egui::Ui, enabled: bool, label: &str) -> egui::Response {
        Self::primary_button_with(ui, enabled, |ui| {
            ui.add(egui::Button::new(label).min_size(Vec2::new(0.0, theme::BUTTON_HEIGHT)))
        })
    }

    pub(super) fn icon_primary_button(
        &mut self,
        ui: &mut egui::Ui,
        enabled: bool,
        icon: &'static str,
        label: &str,
    ) -> egui::Response {
        let tokens = ThemeTokens::from_ui(ui);
        let tint = if enabled {
            tokens.primary_fg
        } else {
            tokens.text_primary
        };
        let image = self.ui_icon(ui.ctx(), icon, UI_ICON_SIZE, tint);
        Self::primary_button_with(ui, enabled, |ui| {
            ui.add(
                egui::Button::image_and_text(image, label)
                    .min_size(Vec2::new(0.0, theme::BUTTON_HEIGHT)),
            )
        })
    }

    pub(super) fn standard_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
        ui.add(egui::Button::new(label).min_size(Vec2::new(0.0, theme::BUTTON_HEIGHT)))
    }

    pub(super) fn icon_standard_button(
        &mut self,
        ui: &mut egui::Ui,
        icon: &'static str,
        label: &str,
    ) -> egui::Response {
        let tint = ThemeTokens::from_ui(ui).icon_tint(false, ui.is_enabled());
        let image = self.ui_icon(ui.ctx(), icon, UI_ICON_SIZE, tint);
        ui.add(
            egui::Button::image_and_text(image, label)
                .min_size(Vec2::new(0.0, theme::BUTTON_HEIGHT)),
        )
    }

    pub(super) fn compact_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
        ui.scope(|ui| {
            ui.spacing_mut().button_padding = Vec2::new(theme::SPACE_MD, 3.0);
            ui.add(egui::Button::new(label).min_size(Vec2::new(0.0, theme::COMPACT_BUTTON_HEIGHT)))
        })
        .inner
    }

    pub(super) fn with_selectable_style<R>(
        ui: &mut egui::Ui,
        selected: bool,
        add_contents: impl FnOnce(&mut egui::Ui) -> R,
    ) -> R {
        let tokens = ThemeTokens::from_ui(ui);
        ui.scope(|ui| {
            if selected {
                let widgets = &mut ui.visuals_mut().widgets;
                widgets.inactive.bg_fill = tokens.selected_bg;
                widgets.inactive.weak_bg_fill = tokens.selected_bg;
                widgets.inactive.bg_stroke = Stroke::new(1.0, tokens.selected_border);
                widgets.inactive.fg_stroke = Stroke::new(1.0, tokens.selected_fg);

                widgets.hovered.bg_fill = tokens.selected_hover_bg;
                widgets.hovered.weak_bg_fill = tokens.selected_hover_bg;
                widgets.hovered.bg_stroke = Stroke::new(1.0, tokens.selected_border);
                widgets.hovered.fg_stroke = Stroke::new(1.0, tokens.selected_fg);

                widgets.active.bg_fill = tokens.selected_hover_bg;
                widgets.active.weak_bg_fill = tokens.selected_hover_bg;
                widgets.active.bg_stroke = Stroke::new(1.0, tokens.selected_border);
                widgets.active.fg_stroke = Stroke::new(1.0, tokens.selected_fg);
            }
            add_contents(ui)
        })
        .inner
    }

    pub(super) fn full_width_card<R>(
        ui: &mut egui::Ui,
        tokens: ThemeTokens,
        add_contents: impl FnOnce(&mut egui::Ui) -> R,
    ) -> egui::InnerResponse<R> {
        tokens.group_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            add_contents(ui)
        })
    }
}
