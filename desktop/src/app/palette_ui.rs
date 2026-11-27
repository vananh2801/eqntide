//! Palette ribbon UI, pre-rasterized SVG sprite atlas loader, and shared button/card helpers.

use super::{
    tabler_icon_bytes, App, MatrixDialog, PaletteAtlas, PaletteAtlasEntry, PALETTE_BUTTON_SIZE,
    PALETTE_GAP, UI_ICON_SIZE,
};
use crate::{
    palette,
    settings::MAX_RECENT_PALETTE,
    theme::{self, ThemeTokens},
};
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use std::collections::HashMap;

const PALETTE_ATLAS_PNG: &[u8] =
    include_bytes!("../../../assets/icons/generated/palette-atlas/atlas.png");

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
                            self.ensure_palette_atlas(ui.ctx());
                            let icon_color =
                                self.theme_tokens(ui).icon_tint(false, ui.is_enabled());
                            let mut clicked_source = None;
                            {
                                let atlas = self.palette_atlas.as_ref();
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
                            if has_matrix_tools {
                                let dots_image =
                                    self.ui_icon(ui.ctx(), "dots", UI_ICON_SIZE, icon_color);
                                let custom_btn = ui
                                    .add_sized(PALETTE_BUTTON_SIZE, egui::Button::new(dots_image));
                                custom_btn.widget_info(|| {
                                    egui::WidgetInfo::labeled(
                                        egui::Role::Button,
                                        ui.is_enabled(),
                                        "Custom matrix",
                                    )
                                });
                                if custom_btn.on_hover_text("Custom matrix").clicked() {
                                    self.dismiss_command_input();
                                    self.matrix_dialog = Some(MatrixDialog::default());
                                    ui.memory_mut(|mem| {
                                        mem.surrender_focus(Self::editor_focus_id(
                                            self.active_editor_focus,
                                        ))
                                    });
                                }
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
        self.persisted.recent_palette = self
            .recent_palette
            .iter()
            .map(|&item_src| item_src.to_owned())
            .collect();
        self.settings_dirty = true;
    }

    /// Lazily decodes the bundled pre-rasterized palette sprite atlas and uploads it once.
    pub(super) fn ensure_palette_atlas(&mut self, ctx: &egui::Context) -> &PaletteAtlas {
        self.palette_atlas.get_or_insert_with(|| {
            let meta = palette::AtlasMetadata::load_embedded();
            let pixels = image::load_from_memory(PALETTE_ATLAS_PNG)
                .expect("bundled palette atlas PNG is valid")
                .to_rgba8();
            let width = pixels.width() as usize;
            let height = pixels.height() as usize;
            assert_eq!(
                (width as u32, height as u32),
                (meta.atlas_width, meta.atlas_height),
                "bundled palette atlas PNG dimensions must match atlas.json"
            );
            let image = egui::ColorImage::from_rgba_unmultiplied([width, height], pixels.as_raw());
            let texture = ctx.load_texture("palette-atlas", image, egui::TextureOptions::LINEAR);
            let atlas_width = meta.atlas_width as f32;
            let atlas_height = meta.atlas_height as f32;
            let display_size = Vec2::new(meta.icon_width, meta.icon_height);
            let mut entries = HashMap::with_capacity(meta.entries.len());
            for item in meta.entries {
                if let Some(canonical) = palette::canonical_source(&item.source) {
                    let uv = Rect::from_min_max(
                        Pos2::new(item.x as f32 / atlas_width, item.y as f32 / atlas_height),
                        Pos2::new(
                            (item.x + item.width) as f32 / atlas_width,
                            (item.y + item.height) as f32 / atlas_height,
                        ),
                    );
                    entries.insert(canonical, PaletteAtlasEntry { uv, display_size });
                }
            }
            PaletteAtlas { texture, entries }
        })
    }

    /// Returns the active GPU texture IDs used for palette icons.
    #[cfg(test)]
    pub(super) fn palette_texture_ids(&self) -> std::collections::HashSet<egui::TextureId> {
        self.palette_atlas
            .iter()
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

    /// Styles and enables a primary action widget using the supplied [`ThemeTokens`].
    pub(super) fn primary_button_with<R>(
        ui: &mut egui::Ui,
        tokens: ThemeTokens,
        enabled: bool,
        add_button: impl FnOnce(&mut egui::Ui) -> R,
    ) -> R {
        ui.scope(|ui| {
            let widgets = &mut ui.visuals_mut().widgets;
            if enabled {
                widgets.inactive.bg_fill = tokens.primary_bg;
                widgets.inactive.weak_bg_fill = tokens.primary_bg;
                widgets.inactive.bg_stroke = Stroke::NONE;
                widgets.inactive.fg_stroke = Stroke::new(1.0, tokens.primary_fg);

                widgets.hovered.bg_fill = tokens.primary_hover_bg;
                widgets.hovered.weak_bg_fill = tokens.primary_hover_bg;
                widgets.hovered.bg_stroke = Stroke::NONE;
                widgets.hovered.fg_stroke = Stroke::new(1.0, tokens.primary_fg);

                let focus_border = if tokens.dark_mode {
                    tokens.text_primary
                } else {
                    tokens.primary_fg
                };
                widgets.active.bg_fill = tokens.primary_active_bg;
                widgets.active.weak_bg_fill = tokens.primary_active_bg;
                widgets.active.bg_stroke = Stroke::new(1.0, focus_border);
                widgets.active.fg_stroke = Stroke::new(1.0, tokens.primary_fg);
            } else {
                widgets.noninteractive.bg_fill = tokens.control_bg;
                widgets.noninteractive.weak_bg_fill = tokens.control_bg;
                widgets.noninteractive.bg_stroke = Stroke::NONE;
                widgets.noninteractive.fg_stroke = Stroke::new(1.0, tokens.text_primary);
                widgets.inactive.bg_fill = tokens.control_bg;
                widgets.inactive.weak_bg_fill = tokens.control_bg;
                widgets.inactive.bg_stroke = Stroke::NONE;
                widgets.inactive.fg_stroke = Stroke::new(1.0, tokens.text_primary);
            }
            ui.add_enabled_ui(enabled, add_button).inner
        })
        .inner
    }

    /// Renders a standard-height primary action button tinted with the active accent.
    pub(super) fn primary_button(
        &self,
        ui: &mut egui::Ui,
        enabled: bool,
        label: &str,
    ) -> egui::Response {
        let tokens = self.theme_tokens(ui);
        Self::primary_button_with(ui, tokens, enabled, |ui| {
            ui.add(egui::Button::new(label).min_size(Vec2::new(0.0, theme::BUTTON_HEIGHT)))
        })
    }

    /// Renders a standard-height primary action button with a theme-contrasted Tabler icon.
    pub(super) fn icon_primary_button(
        &mut self,
        ui: &mut egui::Ui,
        enabled: bool,
        icon: &'static str,
        label: &str,
    ) -> egui::Response {
        let tokens = self.theme_tokens(ui);
        let tint = if enabled {
            tokens.primary_fg
        } else {
            tokens.text_primary
        };
        let image = self.ui_icon(ui.ctx(), icon, UI_ICON_SIZE, tint);
        Self::primary_button_with(ui, tokens, enabled, |ui| {
            ui.add(
                egui::Button::image_and_text(image, label)
                    .min_size(Vec2::new(0.0, theme::BUTTON_HEIGHT)),
            )
        })
    }

    pub(super) fn standard_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
        ui.add(egui::Button::new(label).min_size(Vec2::new(0.0, theme::BUTTON_HEIGHT)))
    }

    /// Renders a standard-height secondary button with a theme-tinted Tabler icon.
    pub(super) fn icon_standard_button(
        &mut self,
        ui: &mut egui::Ui,
        icon: &'static str,
        label: &str,
    ) -> egui::Response {
        let tint = self.theme_tokens(ui).icon_tint(false, ui.is_enabled());
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

    /// Applies tinted-accent selection styling around a selectable option widget.
    pub(super) fn with_selectable_style<R>(
        ui: &mut egui::Ui,
        tokens: ThemeTokens,
        selected: bool,
        add_contents: impl FnOnce(&mut egui::Ui) -> R,
    ) -> R {
        ui.scope(|ui| {
            if selected {
                let visuals = ui.visuals_mut();
                visuals.selection.bg_fill = tokens.selected_bg;
                visuals.selection.stroke = Stroke::new(1.0, tokens.selected_fg);

                let widgets = &mut visuals.widgets;
                widgets.inactive.bg_fill = tokens.selected_bg;
                widgets.inactive.weak_bg_fill = tokens.selected_bg;
                widgets.inactive.bg_stroke = Stroke::NONE;
                widgets.inactive.fg_stroke = Stroke::new(1.0, tokens.selected_fg);

                widgets.hovered.bg_fill = tokens.selected_hover_bg;
                widgets.hovered.weak_bg_fill = tokens.selected_hover_bg;
                widgets.hovered.bg_stroke = Stroke::NONE;
                widgets.hovered.fg_stroke = Stroke::new(1.0, tokens.selected_fg);

                widgets.active.bg_fill = tokens.selected_hover_bg;
                widgets.active.weak_bg_fill = tokens.selected_hover_bg;
                widgets.active.bg_stroke = Stroke::NONE;
                widgets.active.fg_stroke = Stroke::new(1.0, tokens.selected_fg);
            }
            add_contents(ui)
        })
        .inner
    }

    /// Renders a checkbox whose checked or indeterminate box uses the active theme accent while keeping the label neutral.
    pub(super) fn accent_checkbox(
        ui: &mut egui::Ui,
        tokens: ThemeTokens,
        checked: &mut bool,
        indeterminate: bool,
        label: &str,
    ) -> egui::Response {
        let enabled = ui.is_enabled();
        let label_color = if enabled {
            tokens.text_primary
        } else {
            tokens.text_disabled
        };
        let is_marked = *checked || indeterminate;
        ui.scope(|ui| {
            if !enabled {
                // Keep disabled checkbox fills and text crisp instead of double-dimming via painter opacity.
                ui.set_opacity(1.0);
                let widgets = &mut ui.visuals_mut().widgets;
                let border_color = if is_marked {
                    tokens.disabled_accent()
                } else {
                    tokens.border
                };
                for widget_visuals in [
                    &mut widgets.noninteractive,
                    &mut widgets.inactive,
                    &mut widgets.hovered,
                    &mut widgets.active,
                ] {
                    widget_visuals.bg_fill = tokens.control_disabled_bg;
                    widget_visuals.weak_bg_fill = tokens.control_disabled_bg;
                    widget_visuals.bg_stroke = Stroke::new(1.0, border_color);
                    widget_visuals.fg_stroke = Stroke::new(1.5, border_color);
                }
            } else if is_marked {
                let widgets = &mut ui.visuals_mut().widgets;
                widgets.inactive.bg_fill = tokens.selected_bg;
                widgets.inactive.bg_stroke = Stroke::new(1.0, tokens.selected_border);
                widgets.inactive.fg_stroke = Stroke::new(1.5, tokens.selected_fg);

                widgets.hovered.bg_fill = tokens.selected_hover_bg;
                widgets.hovered.bg_stroke = Stroke::new(1.0, tokens.selected_border);
                widgets.hovered.fg_stroke = Stroke::new(1.5, tokens.selected_fg);

                widgets.active.bg_fill = tokens.selected_hover_bg;
                widgets.active.bg_stroke = Stroke::new(1.0, tokens.selected_border);
                widgets.active.fg_stroke = Stroke::new(1.5, tokens.selected_fg);
            } else {
                let widgets = &mut ui.visuals_mut().widgets;
                widgets.inactive.bg_stroke = Stroke::new(1.0, tokens.border);
                widgets.hovered.bg_stroke = Stroke::new(1.0, tokens.border_hover);
                widgets.active.bg_stroke = Stroke::new(1.0, tokens.accent);
            }
            let checkbox = if label.is_empty() {
                egui::Checkbox::without_text(checked)
            } else {
                egui::Checkbox::new(checked, egui::RichText::new(label).color(label_color))
            };
            ui.add(checkbox.indeterminate(indeterminate))
        })
        .inner
    }

    /// Renders a radio option whose selected indicator uses the active theme accent while keeping the caption neutral.
    pub(super) fn accent_radio_value<Value: PartialEq>(
        ui: &mut egui::Ui,
        tokens: ThemeTokens,
        current_value: &mut Value,
        alternative: Value,
        label: &str,
    ) -> egui::Response {
        let enabled = ui.is_enabled();
        let selected = *current_value == alternative;
        let font = egui::TextStyle::Body.resolve(ui.style());
        let caption_color = if enabled {
            tokens.text_primary
        } else {
            tokens.text_disabled
        };
        let caption = egui::RichText::new(label).font(font).color(caption_color);
        Self::with_selectable_style(ui, tokens, selected && enabled, |ui| {
            if !enabled {
                // Keep disabled radio indicators and captions crisp instead of double-dimming via painter opacity.
                ui.set_opacity(1.0);
                let widgets = &mut ui.visuals_mut().widgets;
                let border_color = if selected {
                    tokens.disabled_accent()
                } else {
                    tokens.border
                };
                for widget_visuals in [
                    &mut widgets.noninteractive,
                    &mut widgets.inactive,
                    &mut widgets.hovered,
                    &mut widgets.active,
                ] {
                    widget_visuals.bg_fill = tokens.control_disabled_bg;
                    widget_visuals.weak_bg_fill = tokens.control_disabled_bg;
                    widget_visuals.bg_stroke = Stroke::new(1.0, border_color);
                    widget_visuals.fg_stroke = Stroke::new(1.0, border_color);
                }
            } else if selected {
                let widgets = &mut ui.visuals_mut().widgets;
                widgets.inactive.bg_stroke = Stroke::new(1.0, tokens.selected_border);
                widgets.hovered.bg_stroke = Stroke::new(1.0, tokens.selected_border);
                widgets.active.bg_stroke = Stroke::new(1.0, tokens.selected_border);
            } else {
                let widgets = &mut ui.visuals_mut().widgets;
                widgets.inactive.bg_stroke = Stroke::new(1.0, tokens.border);
                widgets.hovered.bg_stroke = Stroke::new(1.0, tokens.border_hover);
                widgets.active.bg_stroke = Stroke::new(1.0, tokens.accent);
            }
            ui.radio_value(current_value, alternative, caption)
        })
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
