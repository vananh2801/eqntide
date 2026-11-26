//! Background event draining, debounced LaTeX parsing, and async raster render sync.

use super::{App, GpuProfile, Intent, PLACEHOLDER_ATTR};
use crate::{
    batch, commands,
    update::{self, UpdateCheckOutcome, UpdateState},
    Event, Rendered,
};
use eframe::egui::{self, Color32, Vec2};
use eqntide::MathNode;
use eqntide_editor::Editor;
use std::{
    path::Path,
    sync::atomic::Ordering,
    time::{Duration, Instant},
};

impl App {
    /// Drains all queued background and IPC [`Event`] messages for the current frame.
    pub(super) fn process_events(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                Event::Launch(args) => {
                    if self.bridge_ready {
                        self.handle_launch_args(args, ctx);
                    } else {
                        self.startup.extend(args);
                    }
                }
                Event::Window(request) => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    self.handle_window_request(request);
                }
                Event::Bridge(result) => match result {
                    Ok(handle) => {
                        self.bridge_status = if handle.is_some() {
                            "Bridge: hosted by desktop"
                        } else {
                            "Bridge: external CLI/service"
                        }
                        .into();
                        self.bridge = handle;
                        self.bridge_ready = true;
                        let args = std::mem::take(&mut self.startup);
                        if !args.is_empty() {
                            self.handle_launch_args(args, ctx);
                        }
                    }
                    Err(err) => {
                        self.bridge_status = err;
                        self.status = "Standalone editing remains available. Free port 17890, then retry the bridge.".into();
                    }
                },
                Event::Latex(generation, result) => {
                    if self.latex_parsing == Some(generation) {
                        self.latex_parsing = None;
                    }
                    if generation != self.latex_generation || !self.draft_dirty {
                        continue;
                    }
                    match result {
                        Ok(math) => {
                            if self.editor.math() != math {
                                self.editor.replace(&math);
                                self.render_key += 1;
                            }
                            // Keep the user's source, spacing and cursor position while typing.
                            self.draft_dirty = false;
                            self.latex_error = None;
                        }
                        Err(error) => self.latex_error = Some(error),
                    }
                }
                Event::Render(key, result) => {
                    self.render_busy = false;
                    if key == self.render_key {
                        match result {
                            Ok(mut rendered) => {
                                let image = std::mem::take(&mut rendered.image);
                                if let Some(texture) = &mut self.texture {
                                    texture.set(image, egui::TextureOptions::LINEAR);
                                } else {
                                    self.texture = Some(ctx.load_texture(
                                        "equation",
                                        image,
                                        egui::TextureOptions::LINEAR,
                                    ));
                                }
                                self.rendered = Some(rendered);
                            }
                            Err(err) => {
                                self.status = format!("Render error: {err}");
                                self.texture = None;
                                self.rendered = None;
                            }
                        }
                    }
                }
                Event::Session(id, result) => {
                    if self.loading_session.as_ref() != Some(&id) {
                        continue;
                    }
                    self.loading_session = None;
                    self.busy = false;
                    match result {
                        Ok(value) => {
                            if value["state"]
                                .as_str()
                                .is_some_and(|state| state != "waiting")
                            {
                                self.status = format!(
                                    "Session {id} has already completed; current work retained."
                                );
                                continue;
                            }
                            if self.load_math(
                                value["mathml"].as_str().unwrap_or(""),
                                value["latex"].as_str(),
                                value["font_pt"].as_f64().map(|font| font as f32),
                            ) {
                                if let Some(mode) = value["target_mode"]
                                    .as_str()
                                    .filter(|mode| *mode == "omml" || *mode == "mathtype")
                                {
                                    self.target = mode.into();
                                    self.saved_mode = self.target.clone();
                                }
                                self.session = Some(id);
                                if !self.queue.is_empty() && self.pending.is_none() {
                                    self.pending = Some(Intent::Next);
                                }
                                self.status = "Office session — Save to return".into();
                            }
                        }
                        Err(err) => {
                            self.status = format!("Session {id} unavailable: {err}");
                        }
                    }
                }
                Event::Finished(result) => {
                    self.busy = false;
                    match result {
                        Ok(()) => {
                            self.flush_settings();
                            let math = self.editor.math();
                            self.saved_xml = math.to_xml();
                            self.saved_empty = math.children.is_empty();
                            self.saved_font = self.font_pt;
                            self.saved_mode = self.target.clone();
                            self.draft_dirty = false;
                            let had_session = self.session.take().is_some();
                            if matches!(self.pending, Some(Intent::Close)) {
                                self.begin_close(ctx);
                            } else if !self.queue.is_empty() {
                                self.load_next();
                            } else if had_session {
                                self.begin_close(ctx);
                            } else {
                                self.pending = None;
                                self.status = "Saved.".into();
                            }
                        }
                        Err(err) => {
                            self.status = err;
                            self.pending = None;
                        }
                    }
                }
                Event::Paths(paths) => self.add_files(paths),
                Event::BackgroundImage(path) => self.set_background_image_from_path(&path, ctx),
                Event::BackgroundImportStarted(seq) => {
                    if seq == self.background_import_seq {
                        self.background_importing = true;
                        self.background_error = None;
                    }
                }
                Event::BackgroundImported(seq, result) => {
                    self.finish_background_import(seq, result, ctx);
                }
                Event::Batch(item_idx, result) => {
                    self.batch_completed += 1;
                    self.sync_batch_row_state();
                    let log = match result {
                        Ok(outcome) => {
                            for (total, value) in self.batch_totals.iter_mut().zip([
                                outcome.found,
                                outcome.converted,
                                outcome.skipped,
                                outcome.errors,
                            ]) {
                                *total += value;
                            }
                            if let (Some(item), Some(row_state)) =
                                (self.files.get(item_idx), self.batch_rows.get_mut(item_idx))
                            {
                                row_state.last_output =
                                    Some(batch::BatchRowOutput::from_conversion(
                                        item,
                                        Path::new(&outcome.output_path),
                                        &self.batch_run_target,
                                        self.batch_run_overwrite,
                                        &self.batch_run_timestamp,
                                    ));
                            }
                            format!(
                                "{} converted, {} skipped, {} errors → {}",
                                outcome.converted,
                                outcome.skipped,
                                outcome.errors,
                                outcome.output_path
                            )
                        }
                        Err(err) => {
                            self.batch_totals[3] += 1;
                            if let Some(row_state) = self.batch_rows.get_mut(item_idx) {
                                row_state.last_output = None;
                            }
                            format!("Error: {err}")
                        }
                    };
                    if let Some(row) = self.batch_logs.get_mut(item_idx) {
                        *row = log;
                    }
                }
                Event::BatchDone => {
                    self.batch_running = false;
                    for row in &mut self.batch_logs {
                        if row == "Waiting" {
                            *row = "Stopped".into();
                        }
                    }
                    self.status = if self.stop.load(Ordering::Relaxed) {
                        "Batch stopped."
                    } else {
                        "Batch finished."
                    }
                    .into();
                }
                Event::UpdateCheck(seq, result) => {
                    if seq == self.update_check_seq {
                        self.update_state = match result {
                            Ok(UpdateCheckOutcome::UpToDate) => UpdateState::UpToDate,
                            Ok(UpdateCheckOutcome::Available(release)) => {
                                UpdateState::Available(release)
                            }
                            Err(err) => UpdateState::Error(err),
                        };
                    }
                }
            }
        }
    }

    /// Starts a manual GitHub Release check off-thread unless one is already in flight.
    pub(super) fn start_update_check(&mut self) {
        if matches!(self.update_state, UpdateState::Checking) {
            return;
        }
        self.update_check_seq = self.update_check_seq.wrapping_add(1);
        self.update_state = UpdateState::Checking;
        update::spawn_update_check(self.tx.clone(), self.update_check_seq);
    }

    /// Pops a ready debounced LaTeX parse job when the typing deadline has elapsed.
    pub(super) fn take_latex_request(&mut self, now: Instant) -> Option<(u64, String)> {
        if !self.draft_dirty || self.latex_parsing.is_some() || now < self.latex_deadline? {
            return None;
        }
        self.latex_deadline = None;
        self.latex_parsing = Some(self.latex_generation);
        Some((self.latex_generation, self.latex.clone()))
    }

    /// Validates and parses desktop LaTeX source into an editable [`MathNode`] tree.
    pub(super) fn parse_desktop_latex(source: &str) -> Result<MathNode, String> {
        if source.trim().is_empty() {
            return Ok(MathNode::new("math"));
        }
        commands::validate_desktop_latex(source)?;
        let math =
            eqntide::service::parse_latex_to_mathml(source).map_err(|err| err.to_string())?;
        Editor::try_new(&math).map(|_| math).map_err(str::to_owned)
    }

    /// Spawns a background LaTeX parse thread once the debounce window expires.
    pub(super) fn request_latex_parse(&mut self, ctx: &egui::Context) {
        let now = Instant::now();
        if let Some((generation, source)) = self.take_latex_request(now) {
            let tx = self.tx.clone();
            std::thread::spawn(move || {
                let result = Self::parse_desktop_latex(&source);
                let _ = tx.send(Event::Latex(generation, result));
            });
        }
        if let Some(deadline) = self.latex_deadline.filter(|_| self.latex_parsing.is_none()) {
            ctx.request_repaint_after(deadline.saturating_duration_since(now));
        }
    }

    /// Marks the LaTeX source box as having unparsed edits and schedules a debounced parse.
    pub(super) fn mark_source_changed(&mut self, now: Instant) {
        self.dismiss_command_input();
        self.latex_groups.clear();
        self.expects_latex_group = false;
        self.draft_dirty = true;
        self.latex_generation += 1;
        self.latex_deadline = Some(now + Duration::from_millis(180));
        self.latex_error = None;
        self.status.clear();
    }

    /// Synchronizes the LaTeX buffer and bumps `render_key` after a visual canvas edit.
    pub(super) fn mark_canvas_changed(&mut self) {
        self.render_key += 1;
        self.latex = self.editor.math().to_latex();
        self.draft_dirty = false;
        self.latex_generation += 1;
        self.latex_deadline = None;
        self.latex_error = None;
        self.command_error = None;
        self.status.clear();
    }

    /// Returns `true` when the active equation differs from the initial/saved baseline.
    ///
    /// Starting with an empty equation (`saved_empty`) and remaining empty (or returning to empty)
    /// does not count as dirty even if font size or target mode changed, whereas clearing a
    /// pre-existing equation does count as dirty.
    pub(super) fn is_dirty(&self) -> bool {
        let current_empty = self.latex.trim().is_empty()
            && self.command_input.is_none()
            && (self.editor.math().children.is_empty() || self.draft_dirty);
        if self.saved_empty && current_empty {
            return false;
        }
        self.draft_dirty
            || self.command_input.is_some()
            || self.editor.math().to_xml() != self.saved_xml
            || self.font_pt != self.saved_font
            || self.target != self.saved_mode
    }

    /// Rasterizes the current equation with placeholder boxes and sends an `Event::Render`.
    pub(super) fn request_render(&mut self, ctx: &egui::Context) {
        if self.render_busy || self.requested_key == self.render_key {
            return;
        }
        self.render_busy = true;
        self.requested_key = self.render_key;
        let key = self.render_key;
        let font = self.font_pt;
        let mut math = self.editor.math();
        fn inject_placeholders(node: &mut MathNode) {
            if eqntide_editor::is_opaque_tag(&node.tag) {
                node.text = format!("[{}]", node.tag);
                node.tag = "mtext".into();
                node.children.clear();
                node.attrs.clear();
                return;
            }
            if node.children.is_empty() && matches!(node.tag.as_str(), "math" | "mtd") {
                let mut slot = MathNode::leaf("mi", "□");
                slot.attrs.push((PLACEHOLDER_ATTR.into(), "true".into()));
                node.children.push(slot);
                return;
            }
            if node.text.is_empty()
                && node.children.is_empty()
                && matches!(
                    node.tag.as_str(),
                    "math" | "mrow" | "mtd" | "mi" | "mn" | "mtext"
                )
            {
                node.tag = "mi".into();
                node.text = "□".into();
                node.attrs.push((PLACEHOLDER_ATTR.into(), "true".into()));
            }
            for child in &mut node.children {
                inject_placeholders(child);
            }
        }
        inject_placeholders(&mut math);
        // The root is always a container, including the initially empty equation.
        if math.tag != "math" {
            math = MathNode::with_children("math", vec![math]);
        }
        let raster_scale = match self.gpu_profile {
            GpuProfile::Software | GpuProfile::VirtualGpu => {
                let ppp = ctx.pixels_per_point().max(1.0);
                (ppp * 2.5 / eqntide::render::PX_PER_PT).clamp(0.5, 1.0)
            }
            GpuProfile::Native => 1.0,
        };
        let result = eqntide::render::render_interactive_alpha_scaled(&math, font, raster_scale)
            .map_err(|err| err.to_string())
            .map(|(mut mask, preview)| {
                let placeholders: Vec<_> = preview
                    .regions
                    .iter()
                    .filter(|region| {
                        region
                            .path
                            .iter()
                            .try_fold(&math, |node, &index| node.children.get(index))
                            .is_some_and(|node| {
                                node.attrs.iter().any(|(key, _)| key == PLACEHOLDER_ATTR)
                            })
                    })
                    .cloned()
                    .collect();
                clear_placeholder_alpha(&mut mask, &preview.preview, &placeholders);
                let pixels = mask
                    .alpha
                    .into_iter()
                    .map(Color32::from_white_alpha)
                    .collect();
                let image = egui::ColorImage {
                    size: [mask.width as usize, mask.height as usize],
                    source_size: Vec2::new(mask.width as f32, mask.height as f32),
                    pixels,
                };
                Rendered {
                    placeholders,
                    image,
                    preview,
                }
            });
        let _ = self.tx.send_local(Event::Render(key, result));
    }

    /// Triggers an immediate render and drains the resulting local texture event if stale.
    pub(super) fn sync_render(&mut self, ctx: &egui::Context) {
        if !self.render_busy && self.requested_key != self.render_key {
            self.request_render(ctx);
            self.process_events(ctx);
        }
    }
}

fn clear_placeholder_alpha(
    mask: &mut eqntide::render::AlphaMask,
    preview: &eqntide::render::Preview,
    placeholders: &[eqntide::render::HitRegion],
) {
    let width = mask.width as usize;
    let height = mask.height as usize;
    if width == 0 || height == 0 || preview.width_pt <= 0.0 || preview.height_pt <= 0.0 {
        return;
    }
    let scale_x = width as f32 / preview.width_pt;
    let scale_y = height as f32 / preview.height_pt;
    for slot in placeholders {
        let min_x = ((slot.x * scale_x).floor().max(0.0) as usize).min(width);
        let max_x = ((slot.x + slot.width) * scale_x)
            .ceil()
            .clamp(0.0, width as f32) as usize;
        let min_y = ((slot.y * scale_y).floor().max(0.0) as usize).min(height);
        let max_y = ((slot.y + slot.height) * scale_y)
            .ceil()
            .clamp(0.0, height as f32) as usize;
        if min_x < max_x && min_y < max_y {
            for y in min_y..max_y {
                let row_start = y * width;
                mask.alpha[row_start + min_x..row_start + max_x].fill(0);
            }
        }
    }
}
