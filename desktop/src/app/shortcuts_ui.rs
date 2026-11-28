//! Settings keyboard shortcuts card covering fixed system bindings and customizable palette keys.

use super::{App, ShortcutCategoryFilter, ShortcutConflictDialog};
use crate::{
    palette,
    shortcuts::{self, ShortcutChord, ShortcutKey, ShortcutMap},
    theme::{self, ThemeTokens},
};
use eframe::egui::{self, Stroke, Vec2};

const SHORTCUT_ROW_HEIGHT: f32 = 36.0;
const SHORTCUT_SYSTEM_ROW_HEIGHT: f32 = 32.0;
const SHORTCUT_CAPTURE_BUTTON_WIDTH: f32 = 128.0;
const SHORTCUT_CLEAR_BUTTON_WIDTH: f32 = 28.0;
const SHORTCUT_LEFT_COLUMN_RATIO: f32 = 0.30;
const SHORTCUT_COLUMN_GAP: f32 = theme::SPACE_XL;
const SHORTCUT_LIST_INNER_MARGIN: egui::Margin = egui::Margin::same(16);
const SHORTCUT_SCROLL_HEIGHT: f32 = 360.0;
const SHORTCUT_ICON_SIZE: Vec2 = Vec2::new(44.0, 28.6);
const SHORTCUT_CAPTURE_HINT: &str =
    "Press Ctrl, Alt, or F1–F12 to bind · Backspace clears · Esc cancels";
const SHORTCUT_CAPTURE_RESERVED_HINT: &str =
    "Reserved for text editing, clipboard, or navigation · Backspace clears · Esc cancels";

impl App {
    /// Persists active palette shortcut overrides and triggers an immediate settings flush.
    pub(super) fn sync_shortcut_overrides(&mut self) {
        self.persisted.palette_shortcuts = self.shortcuts.to_overrides();
        self.settings_dirty = true;
        self.flush_settings();
    }

    /// Renders the two-column **Keyboard Shortcuts** card inside the Settings tab.
    pub(super) fn draw_keyboard_shortcuts_card(&mut self, ui: &mut egui::Ui, tokens: ThemeTokens) {
        Self::full_width_card(ui, tokens, |ui| {
            let mut search_has_focus = false;
            let mut search_activated = false;

            ui.allocate_ui_with_layout(
                Vec2::new(ui.available_width(), theme::BUTTON_HEIGHT),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.label(egui::RichText::new("Keyboard Shortcuts").strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        (search_has_focus, search_activated) =
                            self.draw_palette_shortcut_filters(ui, tokens);

                        let can_reset = !self.shortcuts.is_default();
                        if ui
                            .add_enabled(
                                can_reset,
                                egui::Button::new("Reset to defaults")
                                    .min_size(Vec2::new(0.0, theme::BUTTON_HEIGHT)),
                            )
                            .clicked()
                        {
                            self.shortcuts = ShortcutMap::default_map();
                            self.shortcut_capture = None;
                            self.shortcut_capture_error = None;
                            self.shortcut_conflict = None;
                            self.sync_shortcut_overrides();
                        }
                    });
                },
            );

            if search_activated {
                self.shortcut_capture = None;
                self.shortcut_capture_error = None;
            }

            if self.shortcut_capture.is_some()
                && !search_has_focus
                && ui.is_enabled()
                && self.shortcut_conflict.is_none()
                && !egui::Popup::is_any_open(ui.ctx())
            {
                self.process_shortcut_capture(ui.ctx());
            }

            ui.add_space(theme::SPACE_MD);

            let total_width = ui.available_width();
            let left_width =
                ((total_width - SHORTCUT_COLUMN_GAP).max(0.0) * SHORTCUT_LEFT_COLUMN_RATIO).floor();
            let right_width = (total_width - SHORTCUT_COLUMN_GAP - left_width).max(0.0);

            ui.horizontal_top(|ui| {
                let item_spacing_x = ui.spacing().item_spacing.x;
                ui.spacing_mut().item_spacing.x = 0.0;
                draw_system_shortcuts_group(ui, tokens, left_width, item_spacing_x);
                ui.add_space(SHORTCUT_COLUMN_GAP);
                ui.allocate_ui_with_layout(
                    Vec2::new(right_width, 0.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.spacing_mut().item_spacing.x = item_spacing_x;
                        ui.set_width(right_width);
                        if let Some(target_source) = self.shortcut_capture {
                            if let Some(error_msg) = &self.shortcut_capture_error {
                                ui.label(
                                    egui::RichText::new(error_msg)
                                        .small()
                                        .color(tokens.error_fg),
                                );
                            } else {
                                let target_label = shortcuts::palette_label(target_source);
                                ui.label(
                                    egui::RichText::new(format!(
                                        "Recording for {target_label} — {SHORTCUT_CAPTURE_HINT}"
                                    ))
                                    .small()
                                    .color(tokens.text_muted),
                                );
                            }
                            ui.add_space(theme::SPACE_SM);
                        }

                        self.draw_palette_shortcuts_list(ui, tokens);
                    },
                );
            });
        });
    }

    fn draw_palette_shortcut_filters(
        &mut self,
        ui: &mut egui::Ui,
        tokens: ThemeTokens,
    ) -> (bool, bool) {
        let selected_text = match self.shortcut_filter {
            ShortcutCategoryFilter::All => "All categories",
            ShortcutCategoryFilter::AssignedOnly => "Assigned only",
            ShortcutCategoryFilter::Group(group_id) => {
                palette::ui_group(group_id).map_or(group_id, |group| group.label)
            }
        };
        let before_filter = self.shortcut_filter;
        egui::ComboBox::from_id_salt("shortcut-category-filter")
            .width(196.0)
            .selected_text(selected_text)
            .popup_style(tokens.combo_popup_style())
            .show_ui(ui, |ui| {
                let mut render_option = |value: ShortcutCategoryFilter, label: &str| {
                    let is_selected = self.shortcut_filter == value;
                    Self::with_selectable_style(ui, tokens, is_selected, |ui| {
                        ui.selectable_value(&mut self.shortcut_filter, value, label)
                    });
                };
                render_option(ShortcutCategoryFilter::All, "All categories");
                render_option(ShortcutCategoryFilter::AssignedOnly, "Assigned only");
                for group in palette::UI_GROUPS
                    .iter()
                    .filter(|group| group.id != palette::RECENT_GROUP)
                {
                    render_option(ShortcutCategoryFilter::Group(group.id), group.label);
                }
            });
        let mut search_activated = self.shortcut_filter != before_filter;

        let search_resp = ui.add_sized(
            [240.0, theme::BUTTON_HEIGHT],
            egui::TextEdit::singleline(&mut self.shortcut_search)
                .hint_text("Search label, LaTeX, or shortcut…"),
        );
        let search_has_focus = search_resp.has_focus();
        search_activated |= search_resp.gained_focus() || search_resp.changed();
        (search_has_focus, search_activated)
    }

    fn process_shortcut_capture(&mut self, ctx: &egui::Context) {
        let Some(target_source) = self.shortcut_capture else {
            return;
        };

        let outcome = ctx.input_mut(|input| {
            let mut result = CaptureOutcome::None;
            input.events.retain(|event| {
                if !matches!(result, CaptureOutcome::None) {
                    return true;
                }
                match event {
                    egui::Event::Copy => {
                        result = CaptureOutcome::Chord(ShortcutChord::cmd(ShortcutKey::C));
                        false
                    }
                    egui::Event::Cut => {
                        result = CaptureOutcome::Chord(ShortcutChord::cmd(ShortcutKey::X));
                        false
                    }
                    egui::Event::Paste(_) => {
                        result = CaptureOutcome::Chord(ShortcutChord::cmd(ShortcutKey::V));
                        false
                    }
                    egui::Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } => {
                        if *key == egui::Key::Escape {
                            result = CaptureOutcome::Cancel;
                            return false;
                        }
                        if matches!(key, egui::Key::Backspace | egui::Key::Delete)
                            && !modifiers.command
                            && !modifiers.ctrl
                            && !modifiers.alt
                            && !modifiers.shift
                        {
                            result = CaptureOutcome::Clear;
                            return false;
                        }
                        if let Some(chord) = ShortcutChord::from_egui_event(*key, *modifiers) {
                            if chord.is_valid_palette_chord() {
                                result = CaptureOutcome::Chord(chord);
                            } else {
                                result = CaptureOutcome::Invalid;
                            }
                        } else if is_reserved_editing_or_navigation_key(*key, *modifiers) {
                            result = CaptureOutcome::Reserved;
                        } else {
                            result = CaptureOutcome::Invalid;
                        }
                        false
                    }
                    egui::Event::Text(_) => false,
                    _ => true,
                }
            });
            result
        });

        match outcome {
            CaptureOutcome::None => {}
            CaptureOutcome::Cancel => {
                self.shortcut_capture = None;
                self.shortcut_capture_error = None;
            }
            CaptureOutcome::Clear => {
                self.shortcuts.clear_binding(target_source);
                self.sync_shortcut_overrides();
                self.shortcut_capture = None;
                self.shortcut_capture_error = None;
            }
            CaptureOutcome::Invalid => {
                self.shortcut_capture_error = Some(SHORTCUT_CAPTURE_HINT.into());
            }
            CaptureOutcome::Reserved => {
                self.shortcut_capture_error = Some(SHORTCUT_CAPTURE_RESERVED_HINT.into());
            }
            CaptureOutcome::Chord(chord) => {
                self.shortcut_capture = None;
                self.shortcut_capture_error = None;
                match self.shortcuts.check_assignment(target_source, chord) {
                    Ok(()) => {
                        self.shortcuts.replace_binding(target_source, chord);
                        self.sync_shortcut_overrides();
                    }
                    Err(conflict) => {
                        self.shortcut_conflict = Some(ShortcutConflictDialog {
                            target_source,
                            attempted_chord: chord,
                            target_previous: self.shortcuts.binding_for(target_source),
                            conflict,
                        });
                    }
                }
            }
        }
    }

    fn draw_palette_shortcuts_list(&mut self, ui: &mut egui::Ui, tokens: ThemeTokens) {
        let rows = self.filtered_palette_rows();
        if rows.is_empty() {
            ui.weak("No palette items match the current filter.");
            return;
        }

        self.ensure_palette_atlas(ui.ctx());
        let icon_tint = tokens.icon_tint(false, ui.is_enabled());
        let mut clicked_record: Option<(&'static str, bool)> = None;
        let mut clicked_clear: Option<&'static str> = None;

        let frame_resp = egui::Frame::new()
            .fill(tokens.panel_bg)
            .corner_radius(theme::CONTROL_CORNER_RADIUS)
            .inner_margin(SHORTCUT_LIST_INNER_MARGIN)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.scope(|ui| {
                    ui.spacing_mut().scroll.bar_inner_margin = theme::CONTENT_SCROLLBAR_GAP;
                    egui::ScrollArea::vertical()
                        .id_salt("palette-shortcuts-scroll")
                        .min_scrolled_height(SHORTCUT_SCROLL_HEIGHT)
                        .max_height(SHORTCUT_SCROLL_HEIGHT)
                        .auto_shrink([false, true])
                        .show_rows(ui, SHORTCUT_ROW_HEIGHT, rows.len(), |ui, row_range| {
                            ui.spacing_mut().item_spacing.y = theme::SPACE_XS;
                            for row in &rows[row_range] {
                                let is_recording = self.shortcut_capture == Some(row.source);
                                let binding = self.shortcuts.binding_for(row.source);
                                let atlas_uv = self.palette_atlas.as_ref().and_then(|atlas| {
                                    atlas.entries.get(row.source).map(|entry| (atlas, entry))
                                });

                                ui.allocate_ui_with_layout(
                                    Vec2::new(ui.available_width(), SHORTCUT_ROW_HEIGHT),
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        if let Some((atlas, entry)) = atlas_uv {
                                            let image = egui::Image::new((
                                                atlas.texture.id(),
                                                SHORTCUT_ICON_SIZE,
                                            ))
                                            .uv(entry.uv)
                                            .tint(icon_tint);
                                            ui.add_sized(SHORTCUT_ICON_SIZE, image)
                                                .on_hover_text(row.label);
                                        } else {
                                            ui.allocate_space(SHORTCUT_ICON_SIZE);
                                        }

                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                let clear_resp = ui.add_visible(
                                                    binding.is_some(),
                                                    egui::Button::new("×").min_size(Vec2::new(
                                                        SHORTCUT_CLEAR_BUTTON_WIDTH,
                                                        theme::BUTTON_HEIGHT,
                                                    )),
                                                );
                                                if clear_resp
                                                    .on_hover_text("Clear shortcut")
                                                    .clicked()
                                                {
                                                    clicked_clear = Some(row.source);
                                                }

                                                let button_text = if is_recording {
                                                    egui::RichText::new("Press shortcut…")
                                                } else if let Some(chord) = binding {
                                                    egui::RichText::new(chord.to_canonical_string())
                                                        .monospace()
                                                } else {
                                                    egui::RichText::new("Set shortcut")
                                                        .color(tokens.text_muted)
                                                };
                                                let capture_btn = Self::with_selectable_style(
                                                    ui,
                                                    tokens,
                                                    is_recording,
                                                    |ui| {
                                                        ui.scope(|ui| {
                                                            if is_recording {
                                                                ui.visuals_mut()
                                                                    .widgets
                                                                    .inactive
                                                                    .bg_stroke =
                                                                    Stroke::new(1.0, tokens.accent);
                                                                ui.visuals_mut()
                                                                    .widgets
                                                                    .hovered
                                                                    .bg_stroke =
                                                                    Stroke::new(1.0, tokens.accent);
                                                            }
                                                            ui.add(
                                                                egui::Button::new(button_text)
                                                                    .min_size(Vec2::new(
                                                                    SHORTCUT_CAPTURE_BUTTON_WIDTH,
                                                                    theme::BUTTON_HEIGHT,
                                                                )),
                                                            )
                                                        })
                                                        .inner
                                                    },
                                                );
                                                if capture_btn.clicked() {
                                                    clicked_record =
                                                        Some((row.source, is_recording));
                                                }

                                                ui.with_layout(
                                                    egui::Layout::left_to_right(
                                                        egui::Align::Center,
                                                    ),
                                                    |ui| {
                                                        ui.set_max_width(ui.available_width());
                                                        ui.add_space(theme::SPACE_SM);
                                                        ui.add(
                                                            egui::Label::new(
                                                                egui::RichText::new(row.source)
                                                                    .monospace()
                                                                    .color(tokens.text_primary),
                                                            )
                                                            .truncate(),
                                                        )
                                                        .on_hover_text(row.label);
                                                    },
                                                );
                                            },
                                        );
                                    },
                                );
                            }
                        });
                });
            });

        // Prevent scroll wheel events inside the right list from chaining to the outer Settings
        // ScrollArea when the inner list is already at its top or bottom boundary.
        if ui.is_enabled() && ui.rect_contains_pointer(frame_resp.response.rect) {
            ui.ctx().input_mut(|input| {
                input.smooth_scroll_delta.y = 0.0;
            });
        }

        if let Some(clear_source) = clicked_clear {
            self.shortcuts.clear_binding(clear_source);
            self.sync_shortcut_overrides();
            if self.shortcut_capture == Some(clear_source) {
                self.shortcut_capture = None;
                self.shortcut_capture_error = None;
            }
            ui.ctx().request_repaint();
        } else if let Some((record_source, was_recording)) = clicked_record {
            if was_recording {
                self.shortcut_capture = None;
                self.shortcut_capture_error = None;
            } else {
                self.shortcut_capture = Some(record_source);
                self.shortcut_capture_error = None;
                ui.ctx().memory_mut(|mem| {
                    if let Some(focused) = mem.focused() {
                        mem.surrender_focus(focused);
                    }
                });
            }
            ui.ctx().request_repaint();
        }
    }

    fn filtered_palette_rows(&self) -> Vec<PaletteShortcutRow> {
        let query = self.shortcut_search.trim().to_ascii_lowercase();
        let mut rows = Vec::new();
        for group in palette::UI_GROUPS
            .iter()
            .filter(|group| group.id != palette::RECENT_GROUP)
        {
            if let ShortcutCategoryFilter::Group(selected_group) = self.shortcut_filter {
                if group.id != selected_group {
                    continue;
                }
            }
            for &(_, label, source) in palette::items_for_ui_group(group.id, &[]) {
                let binding = self.shortcuts.binding_for(source);
                if self.shortcut_filter == ShortcutCategoryFilter::AssignedOnly && binding.is_none()
                {
                    continue;
                }
                if !query.is_empty() && !matches_query(&query, label, source, binding) {
                    continue;
                }
                rows.push(PaletteShortcutRow { label, source });
            }
        }
        rows
    }
}

struct PaletteShortcutRow {
    label: &'static str,
    source: &'static str,
}

enum CaptureOutcome {
    None,
    Cancel,
    Clear,
    Invalid,
    Reserved,
    Chord(ShortcutChord),
}

fn draw_system_shortcuts_group(
    ui: &mut egui::Ui,
    tokens: ThemeTokens,
    width: f32,
    item_spacing_x: f32,
) {
    ui.allocate_ui_with_layout(
        Vec2::new(width, 0.0),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.spacing_mut().item_spacing.x = item_spacing_x;
            ui.set_width(width);
            egui::Frame::new()
                .fill(tokens.panel_bg)
                .corner_radius(theme::CONTROL_CORNER_RADIUS)
                .inner_margin(SHORTCUT_LIST_INNER_MARGIN)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.set_min_height(SHORTCUT_SCROLL_HEIGHT);
                    ui.spacing_mut().item_spacing.y = theme::SPACE_XS;
                    for entry in shortcuts::SYSTEM_SHORTCUTS {
                        ui.allocate_ui_with_layout(
                            Vec2::new(ui.available_width(), SHORTCUT_SYSTEM_ROW_HEIGHT),
                            egui::Layout::left_to_right(egui::Align::Center),
                            |ui| {
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        egui::Frame::new()
                                            .fill(tokens.control_bg)
                                            .corner_radius(theme::CONTROL_CORNER_RADIUS)
                                            .inner_margin(egui::Margin::symmetric(8, 2))
                                            .show(ui, |ui| {
                                                ui.label(
                                                    egui::RichText::new(entry.display)
                                                        .small()
                                                        .monospace()
                                                        .color(tokens.text_primary),
                                                );
                                            });
                                        ui.with_layout(
                                            egui::Layout::left_to_right(egui::Align::Center),
                                            |ui| {
                                                ui.set_max_width(ui.available_width());
                                                ui.add(
                                                    egui::Label::new(
                                                        egui::RichText::new(entry.label)
                                                            .color(tokens.text_primary),
                                                    )
                                                    .truncate(),
                                                )
                                                .on_hover_text(entry.label);
                                            },
                                        );
                                    },
                                );
                            },
                        );
                    }
                });
        },
    );
}

fn matches_query(query: &str, label: &str, source: &str, binding: Option<ShortcutChord>) -> bool {
    if label.to_ascii_lowercase().contains(query) || source.to_ascii_lowercase().contains(query) {
        return true;
    }
    binding.is_some_and(|chord| {
        chord
            .to_canonical_string()
            .to_ascii_lowercase()
            .contains(query)
    })
}

fn is_reserved_editing_or_navigation_key(key: egui::Key, modifiers: egui::Modifiers) -> bool {
    let has_modifier = modifiers.command || modifiers.ctrl || modifiers.alt || modifiers.shift;
    match key {
        egui::Key::Tab
        | egui::Key::Enter
        | egui::Key::Space
        | egui::Key::Home
        | egui::Key::End
        | egui::Key::Insert => true,
        egui::Key::Backspace | egui::Key::Delete => has_modifier,
        egui::Key::ArrowLeft
        | egui::Key::ArrowRight
        | egui::Key::ArrowUp
        | egui::Key::ArrowDown => modifiers.command || modifiers.ctrl || modifiers.alt,
        _ => false,
    }
}
