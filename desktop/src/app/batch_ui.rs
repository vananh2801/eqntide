//! Batch DOCX conversion tab, folder-tree/flat file list rendering, and progress tracking.

use super::{App, UI_ICON_SIZE};
use crate::{
    batch, runtime,
    theme::{self, ThemeTokens},
    Event,
};
use eframe::egui::{self, Pos2, Rect, Stroke, Vec2};
use std::{
    path::{Path, PathBuf},
    sync::{atomic::Ordering, Arc},
};

impl App {
    /// Renders the Batch DOCX tab for adding files/folders, configuring options, and tracking runs.
    pub(super) fn draw_batch_tab(&mut self, ui: &mut egui::Ui) {
        self.sync_batch_row_state();
        let tokens = self.theme_tokens(ui);
        self.draw_back_to_editor_button(ui);
        Self::full_width_card(ui, tokens, |ui| {
            ui.label(
                egui::RichText::new("Input")
                    .small()
                    .color(tokens.text_muted),
            );
            ui.horizontal_wrapped(|ui| {
                ui.add_enabled_ui(!self.batch_running, |ui| {
                    if self
                        .icon_standard_button(ui, "file", "Add DOCX files…")
                        .clicked()
                    {
                        let tx = self.tx.clone();
                        std::thread::spawn(move || {
                            if let Some(paths) = rfd::FileDialog::new()
                                .add_filter("Word documents", &["docx"])
                                .pick_files()
                            {
                                let _ = tx.send(Event::Paths(paths));
                            }
                        });
                    }
                    if self
                        .icon_standard_button(ui, "folder", "Add folder…")
                        .clicked()
                    {
                        let tx = self.tx.clone();
                        std::thread::spawn(move || {
                            if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                let _ = tx.send(Event::Paths(vec![folder]));
                            }
                        });
                    }
                });
                if ui
                    .add_enabled(
                        !self.batch_running && !self.files.is_empty(),
                        egui::Button::new("Clear list")
                            .min_size(Vec2::new(0.0, theme::BUTTON_HEIGHT)),
                    )
                    .clicked()
                {
                    self.files.clear();
                    self.batch_rows.clear();
                    self.batch_collapsed_dirs.clear();
                    self.batch_logs.clear();
                    self.batch_run_total = 0;
                    self.batch_completed = 0;
                    self.batch_totals = [0; 4];
                }
            });
            ui.add_space(theme::SPACE_XS);
            Self::draw_section_separator(ui, tokens);
            ui.add_space(theme::SPACE_XS);
            ui.label(
                egui::RichText::new("Options & Actions")
                    .small()
                    .color(tokens.text_muted),
            );
            ui.add_enabled_ui(!self.batch_running, |ui| {
                ui.horizontal_wrapped(|ui| {
                    let before_batch_target = self.batch_target;
                    egui::ComboBox::from_id_salt("batch-target")
                        .selected_text(batch::TARGETS[self.batch_target].1)
                        .popup_style(tokens.combo_popup_style())
                        .show_ui(ui, |ui| {
                            for (index, (_, label)) in batch::TARGETS.iter().enumerate() {
                                let selected = self.batch_target == index;
                                Self::with_selectable_style(ui, tokens, selected, |ui| {
                                    ui.selectable_value(&mut self.batch_target, index, *label)
                                });
                            }
                        });
                    if self.batch_target != before_batch_target {
                        self.persisted.batch_target = self.batch_target;
                        self.settings_dirty = true;
                    }
                    if Self::accent_checkbox(
                        ui,
                        tokens,
                        &mut self.batch_overwrite,
                        false,
                        "Overwrite original documents (replace each selected input)",
                    )
                    .changed()
                    {
                        self.persisted.batch_overwrite = self.batch_overwrite;
                        self.settings_dirty = true;
                    }
                });
            });
            ui.horizontal_wrapped(|ui| {
                let selected_count = self.selected_batch_count();
                if self
                    .icon_primary_button(
                        ui,
                        !self.batch_running && selected_count > 0,
                        "arrows-left-right",
                        "Convert",
                    )
                    .clicked()
                {
                    let target = batch::TARGETS[self.batch_target].0;
                    let timestamp = batch::timestamp_now();
                    let mode = self.batch_selection_mode;
                    let selected_items: Vec<(usize, batch::BatchItem)> = self
                        .files
                        .iter()
                        .zip(self.batch_rows.iter())
                        .enumerate()
                        .filter(|(_, (_, row))| row.is_selected(mode))
                        .map(|(idx, (item, _))| (idx, item.clone()))
                        .collect();
                    self.batch_running = true;
                    self.batch_totals = [0; 4];
                    self.batch_completed = 0;
                    self.batch_run_total = selected_items.len();
                    self.batch_run_target = target.into();
                    self.batch_run_overwrite = self.batch_overwrite;
                    self.batch_run_timestamp = timestamp.clone();
                    self.stop.store(false, Ordering::Relaxed);
                    for (idx, _) in &selected_items {
                        if let Some(log) = self.batch_logs.get_mut(*idx) {
                            *log = "Waiting".into();
                        }
                        if let Some(row_state) = self.batch_rows.get_mut(*idx) {
                            row_state.last_output = None;
                        }
                    }
                    batch::run_items(
                        selected_items,
                        target.into(),
                        self.font_pt,
                        self.batch_overwrite,
                        timestamp,
                        Arc::clone(&self.stop),
                        self.tx.clone(),
                    );
                }
                if ui
                    .add_enabled(
                        self.batch_running,
                        egui::Button::new("Stop").min_size(Vec2::new(0.0, theme::BUTTON_HEIGHT)),
                    )
                    .clicked()
                {
                    self.stop.store(true, Ordering::Relaxed);
                }
            });
        });
        ui.add_space(theme::SPACE_MD);
        if self.files.is_empty() {
            return;
        }
        let selected_count = self.selected_batch_count();
        let (display_completed, display_total) = if self.batch_run_total > 0 {
            (self.batch_completed, self.batch_run_total)
        } else {
            (0, selected_count)
        };
        let progress_fraction = if display_total > 0 && self.batch_run_total > 0 {
            display_completed as f32 / display_total as f32
        } else {
            0.0
        };
        ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width(), theme::COMPACT_BUTTON_HEIGHT),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                ui.add_enabled_ui(!self.batch_running, |ui| {
                    if Self::compact_button(ui, "Deselect all").clicked() {
                        let mode = self.batch_selection_mode;
                        for row in &mut self.batch_rows {
                            row.set_selected(mode, false);
                        }
                        self.batch_run_total = 0;
                        self.batch_completed = 0;
                    }
                    if Self::compact_button(ui, "Select all").clicked() {
                        let mode = self.batch_selection_mode;
                        for row in &mut self.batch_rows {
                            row.set_selected(mode, true);
                        }
                        self.batch_run_total = 0;
                        self.batch_completed = 0;
                    }
                });
                let bar_width = ui.available_width().max(80.0);
                Self::draw_batch_progress_bar(
                    ui,
                    tokens,
                    bar_width,
                    theme::COMPACT_BUTTON_HEIGHT,
                    progress_fraction,
                    &format!("{display_completed}/{display_total} files"),
                );
            },
        );
        ui.add_space(theme::SPACE_XS);
        ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width(), theme::COMPACT_BUTTON_HEIGHT),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                for (mode, label) in [
                    (batch::BatchSelectionMode::FlatList, "File tree"),
                    (batch::BatchSelectionMode::FolderTree, "Folder tree"),
                ] {
                    let active = self.batch_selection_mode == mode;
                    if Self::with_selectable_style(ui, tokens, active, |ui| {
                        Self::compact_button(ui, label)
                    })
                    .clicked()
                    {
                        self.batch_selection_mode = mode;
                        if !self.batch_running {
                            self.batch_run_total = 0;
                            self.batch_completed = 0;
                        }
                    }
                }
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.label(format!(
                        "Found: {}   Converted: {}   Skipped: {}   Errors: {}",
                        self.batch_totals[0],
                        self.batch_totals[1],
                        self.batch_totals[2],
                        self.batch_totals[3]
                    ));
                });
            },
        );
        ui.add_space(theme::SPACE_XS);
        let text_h = ui.text_style_height(&egui::TextStyle::Body);
        let header_h = text_h.max(theme::COMPACT_BUTTON_HEIGHT);
        let row_height = header_h + theme::SPACE_MD;
        let outer_h = ui.available_height().max(140.0);
        const TREE_INDENT: f32 = 24.0;
        tokens.group_frame().show(ui, |ui| {
            let inner_h = (outer_h - 2.0 * theme::SPACE_LG).max(116.0);
            ui.set_min_size(Vec2::new(ui.available_width(), inner_h));
            ui.set_max_height(inner_h);
            ui.spacing_mut().item_spacing.y = 0.0;
            let scroll = egui::ScrollArea::vertical().auto_shrink([false, false]);
            match self.batch_selection_mode {
                batch::BatchSelectionMode::FlatList => {
                    scroll.show_rows(ui, row_height, self.files.len(), |ui, row_range| {
                        for idx in row_range {
                            let display_str = self.files[idx].display_label();
                            self.draw_batch_file_row(
                                ui,
                                tokens,
                                idx,
                                &display_str,
                                0,
                                TREE_INDENT,
                                row_height,
                                None,
                            );
                        }
                    });
                }
                batch::BatchSelectionMode::FolderTree => {
                    let tree_rows = batch::build_tree_rows(&self.files, &self.batch_collapsed_dirs);
                    let guides = batch::compute_tree_guides(&tree_rows);
                    scroll.show_rows(ui, row_height, tree_rows.len(), |ui, row_range| {
                        for row_idx in row_range {
                            let guide = Some(&guides[row_idx]);
                            match &tree_rows[row_idx] {
                                batch::TreeRow::Group {
                                    key,
                                    label,
                                    hover_path,
                                    depth,
                                    collapsed,
                                    descendant_indices,
                                } => {
                                    self.draw_batch_tree_group_row(
                                        ui,
                                        tokens,
                                        key,
                                        label,
                                        hover_path.as_deref(),
                                        *depth,
                                        *collapsed,
                                        descendant_indices,
                                        TREE_INDENT,
                                        row_height,
                                        guide,
                                    );
                                }
                                batch::TreeRow::File {
                                    item_index,
                                    label,
                                    depth,
                                } => {
                                    self.draw_batch_file_row(
                                        ui,
                                        tokens,
                                        *item_index,
                                        label,
                                        *depth,
                                        TREE_INDENT,
                                        row_height,
                                        guide,
                                    );
                                }
                            }
                        }
                    });
                }
            }
        });
    }

    /// Renders the batch conversion progress bar with `primary_bg` fill and split-contrast status text.
    pub(super) fn draw_batch_progress_bar(
        ui: &mut egui::Ui,
        tokens: ThemeTokens,
        width: f32,
        height: f32,
        fraction: f32,
        label: &str,
    ) {
        let clamped = if fraction.is_finite() {
            fraction.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::hover());
        response.widget_info(|| {
            let mut info =
                egui::WidgetInfo::labeled(egui::Role::ProgressIndicator, ui.is_enabled(), label);
            info.value = Some((f64::from(clamped) * 100.0).floor());
            info
        });
        if !ui.is_rect_visible(rect) {
            return;
        }
        let painter = ui.painter();
        painter.rect_filled(rect, theme::CONTROL_CORNER_RADIUS, tokens.extreme_bg);
        let mut fill_rect = None;
        if clamped > 0.0 {
            let min_fill = (2.0 * f32::from(theme::RADIUS_CONTROL)).min(rect.width());
            let fill_width = (rect.width() * clamped).max(min_fill).min(rect.width());
            let bar_rect = Rect::from_min_size(rect.min, Vec2::new(fill_width, rect.height()));
            painter.rect_filled(bar_rect, theme::CONTROL_CORNER_RADIUS, tokens.primary_bg);
            fill_rect = Some(bar_rect);
        }
        let font_id = egui::TextStyle::Button.resolve(ui.style());
        let galley = ui.fonts_mut(|fonts| {
            fonts.layout_no_wrap(label.to_owned(), font_id, egui::Color32::PLACEHOLDER)
        });
        let text_pos = Pos2::new(
            rect.left() + theme::SPACE_MD,
            rect.center().y - galley.size().y * 0.5,
        );
        if let Some(bar_rect) = fill_rect {
            painter.with_clip_rect(bar_rect.intersect(rect)).galley(
                text_pos,
                Arc::clone(&galley),
                tokens.primary_fg,
            );
            if bar_rect.right() < rect.right() {
                let remaining_rect =
                    Rect::from_min_max(Pos2::new(bar_rect.right(), rect.top()), rect.max);
                painter
                    .with_clip_rect(remaining_rect.intersect(rect))
                    .galley(text_pos, galley, tokens.text_primary);
            }
        } else {
            painter
                .with_clip_rect(rect)
                .galley(text_pos, galley, tokens.text_primary);
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn paint_batch_row_chrome(
        ui: &egui::Ui,
        tokens: ThemeTokens,
        rect: Rect,
        depth: usize,
        indent_step: f32,
        guide: Option<&batch::TreeGuide>,
        branch_end_offset: f32,
        is_group: bool,
        has_expanded_children: bool,
    ) {
        let painter = ui.painter();
        let tree_stroke = Stroke::new(1.25, tokens.border_hover);
        let divider_stroke = Stroke::new(1.0, tokens.border.gamma_multiply(0.85));

        let divider_left = if guide.is_some() {
            let content_depth = if is_group { depth + 1 } else { depth };
            rect.left() + content_depth as f32 * indent_step + theme::COMPACT_BUTTON_HEIGHT
        } else {
            rect.left()
        };
        painter.hline(divider_left..=rect.right(), rect.bottom(), divider_stroke);

        if let Some(guide) = guide {
            for (ancestor_depth, &continues) in guide.ancestor_continues.iter().enumerate() {
                if continues {
                    let x = rect.left() + ancestor_depth as f32 * indent_step + 10.0;
                    painter.vline(x, rect.y_range(), tree_stroke);
                }
            }
            if depth > 0 {
                let parent_x = rect.left() + (depth - 1) as f32 * indent_step + 10.0;
                let mid_y = rect.center().y;
                let v_end = if guide.is_last_sibling {
                    mid_y
                } else {
                    rect.bottom()
                };
                painter.vline(parent_x, rect.top()..=v_end, tree_stroke);
                painter.hline(
                    parent_x..=(parent_x + branch_end_offset),
                    mid_y,
                    tree_stroke,
                );
            }
        }

        if has_expanded_children {
            let trunk_x = rect.left() + depth as f32 * indent_step + 10.0;
            let trunk_start =
                (rect.center().y + theme::COMPACT_BUTTON_HEIGHT * 0.5).min(rect.bottom());
            painter.vline(trunk_x, trunk_start..=rect.bottom(), tree_stroke);
        }
    }

    /// Renders a collapsible directory row in the batch folder tree with aggregate selection.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_batch_tree_group_row(
        &mut self,
        ui: &mut egui::Ui,
        tokens: ThemeTokens,
        key: &str,
        label: &str,
        hover_path: Option<&str>,
        depth: usize,
        collapsed: bool,
        descendant_indices: &[usize],
        indent_step: f32,
        row_height: f32,
        guide: Option<&batch::TreeGuide>,
    ) {
        let total_desc = descendant_indices.len();
        let selected_desc = descendant_indices
            .iter()
            .filter(|&&idx| self.batch_rows[idx].tree_selected)
            .count();
        let mut all_checked = total_desc > 0 && selected_desc == total_desc;
        let partial = selected_desc > 0 && selected_desc < total_desc;
        let folder_icon = self.ui_icon(ui.ctx(), "folder", UI_ICON_SIZE, tokens.text_muted);

        let resp = ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width(), row_height),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                let indent = depth as f32 * indent_step;
                if indent > 0.0 {
                    ui.add_space(indent);
                }
                let toggle_label = if collapsed { "+" } else { "-" };
                if Self::compact_button(ui, toggle_label).clicked() {
                    if collapsed {
                        self.batch_collapsed_dirs.remove(key);
                    } else {
                        self.batch_collapsed_dirs.insert(key.to_owned());
                    }
                }
                ui.add_enabled_ui(!self.batch_running, |ui| {
                    if Self::accent_checkbox(ui, tokens, &mut all_checked, partial, "").changed() {
                        let target = all_checked;
                        for &idx in descendant_indices {
                            self.batch_rows[idx].tree_selected = target;
                        }
                        self.batch_run_total = 0;
                        self.batch_completed = 0;
                    }
                });
                ui.add(folder_icon);
                let group_output_dir = hover_path.and_then(|source_dir| {
                    descendant_indices.iter().find_map(|&idx| {
                        let out = self.batch_rows[idx].last_output.as_ref()?;
                        let root_out = out.root_output_dir.as_deref()?;
                        let source_root = self.files[idx].source_root.as_deref()?;
                        let rel = Path::new(source_dir)
                            .strip_prefix(source_root)
                            .unwrap_or(Path::new(""));
                        Some(if rel.as_os_str().is_empty() {
                            root_out.to_path_buf()
                        } else {
                            root_out.join(rel)
                        })
                    })
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(dir_path) = group_output_dir.as_deref() {
                        if Self::compact_button(ui, "Open folder")
                            .on_hover_text(dir_path.display().to_string())
                            .clicked()
                        {
                            if let Err(err) = runtime::open_folder(dir_path) {
                                self.status = err;
                            }
                        }
                    }
                    ui.label(
                        egui::RichText::new(format!("{selected_desc}/{total_desc} selected"))
                            .small()
                            .color(tokens.text_muted),
                    );
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        let lbl = ui
                            .add(egui::Label::new(egui::RichText::new(label).strong()).truncate());
                        if let Some(hover) = hover_path {
                            lbl.on_hover_text(hover);
                        }
                    });
                });
            },
        );
        Self::paint_batch_row_chrome(
            ui,
            tokens,
            resp.response.rect,
            depth,
            indent_step,
            guide,
            11.0,
            true,
            !collapsed && !descendant_indices.is_empty(),
        );
    }

    /// Renders a single `.docx` item row in either the flat list or folder tree view.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_batch_file_row(
        &mut self,
        ui: &mut egui::Ui,
        tokens: ThemeTokens,
        idx: usize,
        display_str: &str,
        depth: usize,
        indent_step: f32,
        row_height: f32,
        guide: Option<&batch::TreeGuide>,
    ) {
        let full_path = self.files[idx].input.display().to_string();
        let log = self.batch_logs[idx].clone();
        let mode = self.batch_selection_mode;
        let mut checked = self.batch_rows[idx].is_selected(mode);
        let output_meta = self.batch_rows[idx].last_output.clone();

        let resp = ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width(), row_height),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                let indent = depth as f32 * indent_step;
                if indent > 0.0 {
                    ui.add_space(indent);
                }
                if depth > 0 {
                    // Align file checkboxes and labels past the parent folder's collapse toggle.
                    ui.add_space(theme::COMPACT_BUTTON_HEIGHT);
                }
                ui.add_enabled_ui(!self.batch_running, |ui| {
                    if Self::accent_checkbox(ui, tokens, &mut checked, false, "").changed() {
                        self.batch_rows[idx].set_selected(mode, checked);
                        self.batch_run_total = 0;
                        self.batch_completed = 0;
                    }
                });
                if depth > 0 {
                    let file_icon = self.ui_icon(ui.ctx(), "file", UI_ICON_SIZE, tokens.text_muted);
                    ui.add(file_icon);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(out) = output_meta.as_ref() {
                        let show_open_folder = match mode {
                            batch::BatchSelectionMode::FlatList => true,
                            batch::BatchSelectionMode::FolderTree => out.root_output_dir.is_none(),
                        };
                        if show_open_folder
                            && Self::compact_button(ui, "Open folder")
                                .on_hover_text(out.file_dir.display().to_string())
                                .clicked()
                        {
                            if let Err(err) = runtime::open_folder(&out.file_dir) {
                                self.status = err;
                            }
                        }
                    }
                    if log != "Ready" && log != "Waiting" {
                        let (status_text, status_color) = if log.starts_with("Error:") {
                            (log.as_str(), tokens.error_fg)
                        } else if log == "Stopped" {
                            ("Stopped", tokens.text_muted)
                        } else {
                            (
                                log.split(" → ").next().unwrap_or(log.as_str()),
                                tokens.text_muted,
                            )
                        };
                        ui.label(egui::RichText::new(status_text).small().color(status_color))
                            .on_hover_text(&log);
                    }
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        let name_color = if checked {
                            tokens.text_primary
                        } else {
                            tokens.text_disabled
                        };
                        ui.add(
                            egui::Label::new(egui::RichText::new(display_str).color(name_color))
                                .truncate(),
                        )
                        .on_hover_text(&full_path);
                    });
                });
            },
        );
        Self::paint_batch_row_chrome(
            ui,
            tokens,
            resp.response.rect,
            depth,
            indent_step,
            guide,
            30.0,
            false,
            false,
        );
    }

    pub(super) fn sync_batch_row_state(&mut self) {
        let len = self.files.len();
        if self.batch_rows.len() < len {
            self.batch_rows
                .resize_with(len, batch::BatchRowState::default);
        } else if self.batch_rows.len() > len {
            self.batch_rows.truncate(len);
        }
        if self.batch_logs.len() < len {
            self.batch_logs.resize_with(len, || "Ready".into());
        } else if self.batch_logs.len() > len {
            self.batch_logs.truncate(len);
        }
    }

    pub(super) fn selected_batch_count(&self) -> usize {
        self.batch_rows
            .iter()
            .take(self.files.len())
            .filter(|row| row.is_selected(self.batch_selection_mode))
            .count()
    }

    pub(super) fn push_batch_item(&mut self, item: batch::BatchItem) -> bool {
        if let Some(idx) = self
            .files
            .iter()
            .position(|existing| existing.input == item.input)
        {
            if self.files[idx].source_root.is_none() && item.source_root.is_some() {
                self.files[idx].source_root = item.source_root;
            }
            return false;
        }
        self.files.push(item);
        self.batch_rows.push(batch::BatchRowState::default());
        self.batch_logs.push("Ready".into());
        true
    }

    pub(super) fn add_files(&mut self, paths: Vec<PathBuf>) {
        if self.batch_running {
            return;
        }
        let mut added = 0usize;
        let mut saw_empty_dir = false;
        for path in paths {
            if path.is_dir() {
                let mut found = Vec::new();
                match eqntide::docx::collect_docx_files_recursive(&path, None, &mut found) {
                    Ok(()) => {
                        if found.is_empty() {
                            saw_empty_dir = true;
                        }
                        for file in found {
                            if self
                                .push_batch_item(batch::BatchItem::from_folder(file, path.clone()))
                            {
                                added += 1;
                            }
                        }
                    }
                    Err(err) => {
                        self.status = format!("Failed to scan folder {}: {err}", path.display());
                    }
                }
            } else if eqntide::docx::is_docx_file(&path)
                && self.push_batch_item(batch::BatchItem::file(path))
            {
                added += 1;
            }
        }
        if added > 0 {
            self.batch_run_total = 0;
            self.batch_completed = 0;
        } else if saw_empty_dir {
            self.status = "No .docx files found in selected folder.".into();
        }
    }

    /// Renders the Back button returning from Batch, Settings, or About tabs to the Editor.
    pub(super) fn draw_back_to_editor_button(&mut self, ui: &mut egui::Ui) {
        if self
            .icon_standard_button(ui, "chevron-left", "Back")
            .clicked()
        {
            self.tab = 0;
            self.active_editor_focus = self.startup_focus;
            self.pending_focus_restore = Some(self.startup_focus);
        }
        ui.add_space(theme::SPACE_MD);
    }
}
