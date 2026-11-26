//! Launch/URI dispatch, Office add-in session lifecycle, save/cancel submission, and close draining.

use super::{App, Intent};
use crate::{runtime, Event};
use eframe::egui;
use eqntide::{bridge::DesktopWindowRequest, MathNode};
use eqntide_editor::Editor;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

impl App {
    /// Handles CLI file arguments or `eqntide:` custom URIs once the bridge is ready.
    pub(super) fn handle_launch_args(&mut self, args: Vec<String>, ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        for arg in args {
            if arg.starts_with("eqntide:") {
                match eqntide::bridge::parse_custom_uri(&arg) {
                    Ok(action) if action.command == "open" => {
                        if action.tab.as_deref() == Some("batch") {
                            self.tab = 1;
                        }
                        if let Some(id) = action.session_id {
                            if self.session.as_ref() != Some(&id)
                                && !self.queue.iter().any(|queued| queued.session_id == id)
                            {
                                self.handle_window_request(DesktopWindowRequest {
                                    session_id: id,
                                    mathml: None,
                                    latex: None,
                                    font_pt: None,
                                    tab: None,
                                    target_mode: action.target_mode.clone(),
                                });
                            }
                        } else if let Some(data) = action.data {
                            match eqntide::service::decode_input_to_mathml(
                                &data,
                                action.format.as_deref().unwrap_or("auto"),
                            ) {
                                Ok(math) => {
                                    self.handle_window_request(DesktopWindowRequest {
                                        session_id: String::new(),
                                        mathml: Some(math.to_xml()),
                                        latex: None,
                                        font_pt: None,
                                        tab: None,
                                        target_mode: action.target_mode.clone(),
                                    });
                                }
                                Err(err) => self.status = err.to_string(),
                            }
                        }
                    }
                    Ok(_) => self.status = "Unsupported EqnTide URI command".into(),
                    Err(err) => self.status = err.into(),
                }
            } else {
                let path = PathBuf::from(arg);
                if path.exists() {
                    self.add_files(vec![path]);
                    self.tab = 1;
                }
            }
        }
    }

    /// Enqueues an incoming Office bridge window request and opens it or prompts if dirty.
    pub(super) fn handle_window_request(&mut self, request: DesktopWindowRequest) {
        if request.tab.as_deref() == Some("batch") {
            self.tab = 1;
        }
        if request.session_id.is_empty() && request.mathml.is_none() {
            return;
        }
        if !request.session_id.is_empty()
            && (self.session.as_ref() == Some(&request.session_id)
                || self.loading_session.as_ref() == Some(&request.session_id)
                || self
                    .queue
                    .iter()
                    .any(|queued| queued.session_id == request.session_id))
        {
            return;
        }
        self.queue.push_back(request);
        if self.busy {
            return;
        }
        if self.is_dirty() || self.session.is_some() {
            if self.pending.is_none() {
                self.pending = Some(Intent::Next);
            }
        } else {
            self.load_next();
        }
    }

    /// Pops the next queued window/session request and loads it into the editor.
    pub(super) fn load_next(&mut self) {
        self.pending = None;
        if let Some(request) = self.queue.pop_front() {
            self.tab = 0;
            self.active_editor_focus = self.startup_focus;
            self.pending_focus_restore = Some(self.startup_focus);
            if request.session_id.is_empty() {
                if self.load_math(
                    request.mathml.as_deref().unwrap_or(""),
                    request.latex.as_deref(),
                    request.font_pt,
                ) {
                    if let Some(mode) = request
                        .target_mode
                        .filter(|mode| mode == "omml" || mode == "mathtype")
                    {
                        self.target = mode;
                        self.saved_mode = self.target.clone();
                    }
                }
            } else {
                self.busy = true;
                let id = request.session_id;
                self.loading_session = Some(id.clone());
                let tx = self.tx.clone();
                std::thread::spawn(move || {
                    let result = runtime::get(&format!("/session/{id}"));
                    let _ = tx.send(Event::Session(id, result));
                });
            }
        }
    }

    /// Loads MathML (or fallback LaTeX) and font size into `editor`, resetting dirty baselines.
    pub(super) fn load_math(&mut self, xml: &str, latex: Option<&str>, font: Option<f32>) -> bool {
        let result = if !xml.trim().is_empty() {
            MathNode::parse(xml)
        } else if let Some(latex_src) = latex.filter(|src| !src.trim().is_empty()) {
            eqntide::service::parse_latex_to_mathml(latex_src)
        } else {
            Ok(MathNode::new("math"))
        };
        match result {
            Ok(math) => {
                match Editor::try_new(&math) {
                    Ok(editor) => {
                        self.editor = editor;
                        self.matrix_dialog = None;
                        self.command_input = None;
                        self.latex_groups.clear();
                        self.expects_latex_group = false;
                        self.active_editor_focus = self.startup_focus;
                        self.pending_focus_restore = Some(self.startup_focus);
                    }
                    Err(err) => {
                        self.status = err.into();
                        return false;
                    }
                }
                self.font_pt = font
                    .filter(|pt| pt.is_finite() && *pt > 0.0)
                    .unwrap_or(self.persisted.font_pt);
                self.saved_font = self.font_pt;
                self.mark_canvas_changed();
                let loaded_math = self.editor.math();
                self.saved_xml = loaded_math.to_xml();
                self.saved_empty = loaded_math.children.is_empty();
                self.saved_mode = self.target.clone();
                true
            }
            Err(err) => {
                self.status = format!("Could not load equation; existing document retained: {err}");
                false
            }
        }
    }

    /// Saves or cancels the active Office session (or writes standalone MathML to disk).
    pub(super) fn finish_session(&mut self, save: bool) {
        if self.busy {
            return;
        }
        if save && self.command_input.is_some() {
            self.status =
                "Complete the LaTeX command with Tab/Enter or cancel with Esc before saving."
                    .into();
            return;
        }
        if save && self.draft_dirty {
            self.status = "Wait for valid LaTeX before saving.".into();
            return;
        }
        if save && self.session.is_some() {
            let math = self.editor.math();
            if self.target == "omml" {
                if let Some(unsupported) = eqntide::check_omml_unsupported(&math) {
                    self.status = format!(
                        "{unsupported} is not supported in OMML. Switch target to MathType to insert."
                    );
                    return;
                }
            } else if self.target == "mathtype" {
                if let Some(unsupported) = eqntide::check_mathtype_unsupported(&math) {
                    self.status = format!("{unsupported} is not supported in MathType.");
                    return;
                }
            }
        }
        if self.session.is_none() && save {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("MathML", &["mathml"])
                .set_file_name("equation.mathml")
                .save_file()
            {
                self.busy = true;
                let xml = self.editor.math().to_xml();
                let tx = self.tx.clone();
                std::thread::spawn(move || {
                    let _ = tx.send(Event::Finished(
                        std::fs::write(path, xml).map_err(|err| err.to_string()),
                    ));
                });
            }
            return;
        }
        self.busy = true;
        let session = self.session.clone();
        let mathml = self.editor.math().to_xml();
        let font = self.font_pt;
        let mode = self.target.clone();
        let tx = self.tx.clone();
        let queued: Vec<_> = if matches!(self.pending, Some(Intent::Close)) {
            self.queue
                .iter()
                .filter(|req| !req.session_id.is_empty())
                .map(|req| req.session_id.clone())
                .collect()
        } else {
            vec![]
        };
        std::thread::spawn(move || {
            let result = (|| {
                if let Some(id) = session {
                    let path = format!("/session/{id}/{}", if save { "update" } else { "cancel" });
                    let body =
                        serde_json::json!({"mathml":mathml,"target_mode":mode,"font_pt":font});
                    runtime::post(&path, &body)?;
                }
                for id in queued {
                    runtime::post(&format!("/session/{id}/cancel"), &serde_json::json!({}))?;
                }
                Ok(())
            })();
            let _ = tx.send(Event::Finished(result));
        });
    }

    pub(super) fn begin_close(&mut self, ctx: &egui::Context) {
        self.flush_settings();
        self.close_deadline = Some(Instant::now() + Duration::from_secs(3));
        self.busy = true;
        self.status = "Returning the result to Office…".into();
        ctx.request_repaint();
    }

    pub(super) fn drain_before_close(&mut self, ctx: &egui::Context) {
        if let Some(deadline) = self.close_deadline {
            let now = Instant::now();
            // Give the client's next long-poll a brief chance to arrive, then
            // wait for active response writes, with a bounded shutdown timeout.
            let grace_elapsed = now >= deadline - Duration::from_millis(2500);
            if grace_elapsed
                && (self
                    .bridge
                    .as_ref()
                    .is_none_or(|server| server.pending_requests() == 0)
                    || now >= deadline)
            {
                self.close_deadline = None;
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else {
                ctx.request_repaint_after(Duration::from_millis(20));
            }
        }
    }

    /// Renders the Save action button in the footer and records clicks for post-widget handling.
    pub(super) fn draw_save_button(&mut self, ui: &mut egui::Ui) {
        let label = if self.session.is_some() {
            "Save & Return to Document (Ctrl+S)"
        } else {
            "Save MathML (Ctrl+S)"
        };
        let enabled = self.can_save() && !self.draft_dirty && self.command_input.is_none();
        if self.primary_button(ui, enabled, label).clicked() {
            self.save_clicked = true;
        }
    }

    /// Returns `true` when saving is permitted (no in-flight session post, batch job, or modal).
    pub(super) fn can_save(&self) -> bool {
        !self.busy
            && !self.batch_running
            && self.pending.is_none()
            && !self.allow_close
            && self.matrix_dialog.is_none()
    }

    /// Extracts `Ctrl/Cmd+S` before child widgets see it so saving runs after frame edits settle.
    pub(super) fn take_save_shortcut(ctx: &egui::Context) -> bool {
        // Consume before widgets see the events, but save after they apply this
        // frame's edits. Ctrl+Shift+S remains the canvas's sum shortcut.
        ctx.input_mut(|input| {
            let mut save = false;
            input.events.retain(|event| {
                let matches = matches!(event, egui::Event::Key {
                    key: egui::Key::S,
                    pressed: true,
                    modifiers,
                    ..
                } if modifiers.command && !modifiers.shift && !modifiers.alt);
                save |= matches;
                !matches
            });
            save
        })
    }

    pub(super) fn selected_equation(&self) -> MathNode {
        let math = self
            .editor
            .selected_math()
            .unwrap_or_else(|| self.editor.math());
        if math.tag == "math" {
            math
        } else {
            MathNode::with_children("math", vec![math])
        }
    }

    /// Copies the selected equation (or full equation if empty selection) to the clipboard as LaTeX.
    pub(super) fn copy_selection_latex(&self, ctx: &egui::Context) {
        ctx.copy_text(self.selected_equation().to_latex());
    }
}
