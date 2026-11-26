//! Keyboard shortcut handling, canvas text/IME input, and inline LaTeX command popup.

use super::{App, Intent};
use crate::{commands, palette, theme};
use eframe::egui::{self, Color32, Pos2, Stroke};
use eqntide_editor::{Caret, Move};

impl App {
    /// Processes keyboard, clipboard, IME, and text events while the equation canvas is focused.
    pub(super) fn handle_canvas_keyboard(&mut self, ui: &mut egui::Ui) {
        let events = ui.input(|input| input.events.clone());
        let before = self.editor.revision;
        for event in events {
            if let egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } = &event
            {
                if self.handle_command_key(*key, *modifiers) {
                    continue;
                }
            }
            match event {
                egui::Event::Copy => self.copy_selection_latex(ui.ctx()),
                egui::Event::Cut => {
                    self.command_input = None;
                    self.latex_groups.clear();
                    self.expects_latex_group = false;
                    if self.editor.selected_math().is_some() {
                        self.copy_selection_latex(ui.ctx());
                        self.editor.delete(true);
                    }
                }
                egui::Event::Paste(text) => {
                    self.command_input = None;
                    self.latex_groups.clear();
                    self.expects_latex_group = false;
                    if self.is_caret_in_text() {
                        self.editor.insert_text(&text);
                    } else if text.trim_start().starts_with('<') {
                        match eqntide::service::decode_input_to_mathml(&text, "auto") {
                            Ok(math) => self.editor.insert_math(&math),
                            Err(err) => self.status = err.to_string(),
                        }
                    } else if text.contains('\\') {
                        match eqntide::service::parse_latex_to_mathml(&text) {
                            Ok(math) => self.editor.insert_math(&math),
                            Err(err) => self.status = err.to_string(),
                        }
                    } else {
                        self.editor.insert_text(&text);
                    }
                }
                egui::Event::Text(text) => {
                    if self.ime.is_empty() {
                        self.handle_canvas_text(&text);
                    }
                }
                egui::Event::Ime(egui::ImeEvent::Preedit { text, .. }) => self.ime = text,
                egui::Event::Ime(egui::ImeEvent::Commit(text)) => {
                    self.ime.clear();
                    self.handle_canvas_text(&text);
                }
                egui::Event::Ime(egui::ImeEvent::DeleteSurrounding {
                    before_chars,
                    after_chars,
                }) => {
                    for _ in 0..before_chars {
                        self.editor.delete(true);
                    }
                    for _ in 0..after_chars {
                        self.editor.delete(false);
                    }
                }
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    use egui::Key;
                    if modifiers.command {
                        match key {
                            Key::Z => {
                                if modifiers.shift {
                                    self.editor.redo()
                                } else {
                                    self.editor.undo()
                                }
                            }
                            Key::Y => self.editor.redo(),
                            Key::A => self.editor.select_all(),
                            Key::F => self.insert_template("\\frac{#?}{#?}"),
                            Key::R => self.insert_template(if modifiers.shift {
                                "\\sqrt[#?]{#?}"
                            } else {
                                "\\sqrt{#?}"
                            }),
                            Key::H => self.insert_template("^{#?}"),
                            Key::L => self.insert_template(if modifiers.shift {
                                "\\lim_{#? \\to #?}"
                            } else {
                                "_{#?}"
                            }),
                            Key::J => self.insert_template("_{#?}^{#?}"),
                            Key::I => self.insert_template(if modifiers.shift {
                                "\\int_{#?}^{#?} #? \\, d#?"
                            } else {
                                "\\int #? \\, d#?"
                            }),
                            Key::S if modifiers.shift => self.insert_template("\\sum_{#?}^{#?} #?"),
                            Key::P if modifiers.shift => {
                                self.insert_template("\\prod_{#?}^{#?} #?")
                            }
                            Key::M if modifiers.shift => self.insert_template(
                                "\\begin{pmatrix} #? & #? \\\\ #? & #? \\end{pmatrix}",
                            ),
                            Key::C if modifiers.shift => self.insert_template(
                                "\\begin{cases} #? & #? \\\\ #? & #? \\end{cases}",
                            ),
                            Key::Num9 => self.insert_template("\\left( #? \\right)"),
                            Key::OpenBracket | Key::CloseBracket => {
                                self.insert_template("\\left[ #? \\right]")
                            }
                            _ => {}
                        }
                    } else if self.ime.is_empty() {
                        match key {
                            Key::ArrowLeft => self.editor.move_caret(Move::Left, modifiers.shift),
                            Key::ArrowRight => self.editor.move_caret(Move::Right, modifiers.shift),
                            Key::ArrowUp => self.move_caret_vertically(false, modifiers.shift),
                            Key::ArrowDown => self.move_caret_vertically(true, modifiers.shift),
                            Key::Home => self.editor.move_caret(Move::Home, modifiers.shift),
                            Key::End => self.editor.move_caret(Move::End, modifiers.shift),
                            Key::Tab => self.editor.move_caret(
                                if modifiers.shift {
                                    Move::PreviousPlaceholder
                                } else {
                                    Move::NextPlaceholder
                                },
                                false,
                            ),
                            Key::Enter if !modifiers.alt => {
                                if !self.exit_text_run(self.editor.caret().node)
                                    && self.editor.insert_row_or_promote_fence()
                                {
                                    self.latex_groups.clear();
                                    self.expects_latex_group = false;
                                }
                            }
                            Key::Backspace => self.editor.delete(true),
                            Key::Delete => self.editor.delete(false),
                            Key::Escape => {
                                self.pending = Some(Intent::Close);
                                ui.ctx().input_mut(|input| {
                                    let _ = input.consume_key(modifiers, Key::Escape);
                                });
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        if self.editor.revision != before {
            self.mark_canvas_changed();
        }
    }

    /// Routes key presses for the inline `\command` popup and resets brace scope on navigation.
    pub(super) fn handle_command_key(
        &mut self,
        key: egui::Key,
        modifiers: egui::Modifiers,
    ) -> bool {
        use egui::Key;
        if modifiers.command {
            if key == Key::Z && self.command_input.is_some() {
                self.dismiss_command_input();
                self.latex_groups.clear();
                self.expects_latex_group = false;
                return true;
            }
            if !matches!(key, Key::S | Key::C) {
                self.dismiss_command_input();
                self.latex_groups.clear();
                self.expects_latex_group = false;
            }
            return false;
        }
        let Some(input) = &mut self.command_input else {
            if matches!(
                key,
                Key::ArrowLeft
                    | Key::ArrowRight
                    | Key::ArrowUp
                    | Key::ArrowDown
                    | Key::Home
                    | Key::End
            ) {
                self.latex_groups.clear();
                self.expects_latex_group = false;
            }
            return false;
        };
        match key {
            Key::ArrowDown | Key::ArrowUp => {
                if input.is_command_prefix() {
                    let count = commands::suggestions(&input.name).len();
                    if count > 0 {
                        input.selected = if key == Key::ArrowDown {
                            (input.selected + 1) % count
                        } else {
                            (input.selected + count - 1) % count
                        };
                        self.clear_command_error();
                    }
                }
                true
            }
            Key::Tab => {
                if input.is_command_prefix() {
                    if let Some(command) = commands::suggestions(&input.name).get(input.selected) {
                        input.set_name_end(command.name);
                        self.clear_command_error();
                    }
                }
                true
            }
            Key::Enter => {
                self.commit_command(true);
                true
            }
            Key::Escape => {
                self.dismiss_command_input();
                true
            }
            Key::Backspace => {
                if !input.backspace() {
                    self.command_input = None;
                }
                self.clear_command_error();
                true
            }
            Key::Delete => {
                input.delete();
                self.clear_command_error();
                true
            }
            Key::ArrowLeft => {
                input.move_left();
                true
            }
            Key::ArrowRight => {
                input.move_right();
                true
            }
            Key::Home => {
                input.move_home();
                true
            }
            Key::End => {
                input.move_end();
                true
            }
            _ => false,
        }
    }

    /// Renders the floating inline `\command` entry and autocomplete list near the caret.
    pub(super) fn draw_command_popup(&mut self, ui: &mut egui::Ui, anchor: Pos2) {
        if self.command_input.is_none() {
            return;
        }
        let cmd_id = egui::Id::unique("canvas-command-input");
        let canvas_id = egui::Id::unique("equation-canvas");
        let has_input_focus = ui.memory(|mem| mem.has_focus(cmd_id));
        let escape_blurred_new_input = ui.memory(|mem| mem.focused().is_none())
            && self
                .command_input
                .as_ref()
                .is_some_and(|input| !input.needs_focus);
        if (has_input_focus || escape_blurred_new_input)
            && ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            self.dismiss_command_input();
            ui.memory_mut(|mem| mem.request_focus(canvas_id));
            return;
        }
        if has_input_focus {
            ui.memory_mut(|mem| {
                mem.set_focus_lock_filter(
                    cmd_id,
                    egui::EventFilter {
                        tab: true,
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        escape: true,
                    },
                );
                // Clear any focus_direction queued by begin_pass before frame-1 filter took effect.
                mem.move_focus(egui::FocusDirection::None);
            });
            let enter_pressed = ui.input_mut(|input| {
                input.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
                    || input.consume_key(egui::Modifiers::SHIFT, egui::Key::Enter)
            });
            if enter_pressed {
                if self.commit_command(true) {
                    ui.memory_mut(|mem| mem.request_focus(canvas_id));
                    ui.ctx().request_repaint();
                    return;
                }
                ui.memory_mut(|mem| mem.request_focus(cmd_id));
                ui.ctx().request_repaint();
            }
            let buffer_empty = self
                .command_input
                .as_ref()
                .is_some_and(|input| input.name.is_empty());
            if buffer_empty
                && ui.input_mut(|input| {
                    input.consume_key(egui::Modifiers::NONE, egui::Key::Backspace)
                })
            {
                self.dismiss_command_input();
                ui.memory_mut(|mem| mem.request_focus(canvas_id));
                return;
            }
            if ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Tab)) {
                if let Some(input) = &mut self.command_input {
                    if input.is_command_prefix() {
                        if let Some(command) =
                            commands::suggestions(&input.name).get(input.selected)
                        {
                            input.set_name_end(command.name);
                            self.clear_command_error();
                        }
                    }
                }
            }
            let has_suggestions = self.command_input.as_ref().is_some_and(|input| {
                input.is_command_prefix() && !commands::suggestions(&input.name).is_empty()
            });
            if has_suggestions {
                let down = ui.input_mut(|input| {
                    input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)
                });
                let up = ui.input_mut(|input| {
                    input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)
                });
                if down || up {
                    if let Some(input) = &mut self.command_input {
                        let count = commands::suggestions(&input.name).len();
                        if count > 0 {
                            input.selected = if down {
                                (input.selected + 1) % count
                            } else {
                                (input.selected + count - 1) % count
                            };
                            self.clear_command_error();
                        }
                    }
                }
            }
            if self
                .command_input
                .as_ref()
                .is_some_and(|input| input.is_command_prefix())
            {
                ui.input_mut(|input| {
                    input.events.retain(|event| match event {
                        egui::Event::Text(text) => !text.chars().all(char::is_whitespace),
                        egui::Event::Key {
                            key: egui::Key::Space,
                            ..
                        } => false,
                        _ => true,
                    });
                });
            }
        }

        let tokens = self.theme_tokens(ui);
        let mut accept = None;
        let mut popup_edited = false;
        egui::Area::new(egui::Id::unique("canvas-command-popup"))
            .order(egui::Order::Foreground)
            .fixed_pos(anchor)
            .constrain(true)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_min_width(240.0);
                    let Some(input) = &mut self.command_input else {
                        return;
                    };
                    if input.sync_cursor {
                        let mut state =
                            egui::TextEdit::load_state(ui.ctx(), cmd_id).unwrap_or_default();
                        let ccursor = egui::text::CCursor::new(input.cursor.min(input.char_len()));
                        state
                            .cursor
                            .set_char_range(Some(egui::text::CCursorRange::one(ccursor)));
                        state.store(ui.ctx(), cmd_id);
                        input.sync_cursor = false;
                    }
                    let box_focused = has_input_focus || input.needs_focus;
                    let box_resp = egui::Frame::new()
                        .fill(Color32::TRANSPARENT)
                        .corner_radius(theme::CONTROL_CORNER_RADIUS)
                        .stroke(Stroke::new(
                            1.0,
                            if box_focused {
                                tokens.accent
                            } else {
                                tokens.border
                            },
                        ))
                        .inner_margin(egui::Margin::symmetric(8, 5))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 2.0;
                                let prefix_resp = ui.add(
                                    egui::Label::new(
                                        egui::RichText::new("\\").monospace().color(tokens.accent),
                                    )
                                    .sense(egui::Sense::click()),
                                );
                                let text_resp = ui.add(
                                    egui::TextEdit::singleline(&mut input.name)
                                        .id(cmd_id)
                                        .frame(egui::Frame::NONE)
                                        .background_color(Color32::TRANSPARENT)
                                        .font(egui::TextStyle::Monospace)
                                        .text_color(tokens.text_primary)
                                        .align(egui::Align2::LEFT_CENTER)
                                        .desired_width(f32::INFINITY)
                                        .lock_focus(true),
                                );
                                if prefix_resp.clicked() {
                                    let mut state = egui::TextEdit::load_state(ui.ctx(), cmd_id)
                                        .unwrap_or_default();
                                    state.cursor.set_char_range(Some(
                                        egui::text::CCursorRange::one(egui::text::CCursor::new(0)),
                                    ));
                                    state.store(ui.ctx(), cmd_id);
                                    input.cursor = 0;
                                    text_resp.request_focus();
                                }
                                text_resp
                            })
                            .inner
                        });
                    let text_resp = box_resp.inner;
                    if input.needs_focus {
                        text_resp.request_focus();
                        input.needs_focus = false;
                    }
                    popup_edited = text_resp.changed();
                    if popup_edited {
                        input.selected = 0;
                    }
                    if let Some(state) = egui::TextEdit::load_state(ui.ctx(), cmd_id) {
                        if let Some(range) = state.cursor.char_range() {
                            input.cursor = usize::from(range.primary.index).min(input.char_len());
                        }
                    }

                    let selected = input.selected;
                    let is_prefix = input.is_command_prefix();
                    let candidates = if is_prefix {
                        commands::suggestions(&input.name)
                    } else {
                        Vec::new()
                    };
                    if is_prefix && candidates.is_empty() {
                        ui.label("No matching command — Esc to cancel");
                    }
                    for (index, command) in candidates.iter().enumerate() {
                        let is_selected = index == selected;
                        let item_text = format!("\\{}   {}", command.name, command.label);
                        if Self::with_selectable_style(ui, tokens, is_selected, |ui| {
                            ui.selectable_label(is_selected, item_text)
                        })
                        .clicked()
                        {
                            accept = Some(index);
                        }
                    }
                    if is_prefix {
                        ui.weak("↑↓ select · Tab complete · Enter insert · Esc cancel");
                    } else {
                        ui.weak("Enter insert · Esc cancel");
                    }
                });
            });
        if popup_edited {
            self.clear_command_error();
        }
        if let Some(index) = accept {
            if let Some(input) = &mut self.command_input {
                input.selected = index;
            }
            if self.commit_command(true) {
                ui.memory_mut(|mem| mem.request_focus(canvas_id));
                ui.ctx().request_repaint();
            } else {
                ui.memory_mut(|mem| mem.request_focus(cmd_id));
                ui.ctx().request_repaint();
            }
        }
    }

    /// Inserts the highlighted suggestion or parses the raw command buffer into `editor`.
    pub(super) fn commit_command(&mut self, choose_suggestion: bool) -> bool {
        let Some(input) = &self.command_input else {
            return true;
        };
        if input.name.trim().is_empty() {
            self.dismiss_command_input();
            return true;
        }
        let command = if !input.is_command_prefix() {
            None
        } else if choose_suggestion {
            commands::suggestions(&input.name)
                .get(input.selected)
                .copied()
        } else {
            commands::exact(&input.name)
        };
        if let Some(command) =
            command.filter(|cmd| commands::is_argument_requiring_command(cmd.name))
        {
            let msg = commands::bare_command_error(command.name);
            if let Some(cmd_input) = self.command_input.as_mut() {
                cmd_input.set_name_end(command.name);
            }
            self.command_error = Some(msg.clone());
            self.status = msg;
            return false;
        }
        let result = if let Some(command) = command {
            palette::template(command.source).map(|math| (math, command.source.contains("#?")))
        } else {
            Self::parse_desktop_latex(&format!("\\{}", input.name)).map(|math| (math, false))
        };
        match result {
            Ok((math, template)) => {
                self.command_input = None;
                self.command_error = None;
                self.expects_latex_group = template;
                if template {
                    if command.is_some_and(|cmd| palette::attaches_to_previous(cmd.source)) {
                        self.editor.insert_template_wrapping_previous(&math);
                    } else {
                        self.editor.insert_template(&math);
                    }
                } else {
                    self.editor.insert_math(&math);
                }
                self.mark_canvas_changed();
                true
            }
            Err(msg) => {
                self.command_error = Some(msg.clone());
                self.status = msg;
                false
            }
        }
    }

    /// Closes the inline `\command` popup and clears any associated command error.
    pub(super) fn dismiss_command_input(&mut self) {
        self.command_input = None;
        self.clear_command_error();
    }

    /// Clears `command_error` and removes it from the status bar if it is still displayed.
    pub(super) fn clear_command_error(&mut self) {
        if let Some(error) = self.command_error.take() {
            if self.status == error {
                self.status.clear();
            }
        }
    }

    /// Returns `true` when the caret is currently positioned inside an `<mtext>` run.
    pub(super) fn is_caret_in_text(&self) -> bool {
        self.editor
            .node(self.editor.caret().node)
            .is_some_and(|node| node.tag == "mtext")
    }

    /// Applies typed canvas characters, opening `\command` popups, scripts, fractions, or braces.
    pub(super) fn handle_canvas_text(&mut self, text: &str) {
        for ch in text.chars() {
            if let Some(input) = &mut self.command_input {
                if ch.is_whitespace() && input.is_command_prefix() {
                    continue;
                }
                input.insert_char(ch);
                self.clear_command_error();
                continue;
            }
            if self.is_caret_in_text() {
                if !ch.is_control() {
                    self.editor.insert_text(&ch.to_string());
                }
                continue;
            }
            match ch {
                '\\' => {
                    self.expects_latex_group = false;
                    self.status.clear();
                    self.command_input = Some(commands::Input::opened());
                }
                '{' if self.expects_latex_group || !self.latex_groups.is_empty() => {
                    self.latex_groups.push(self.editor.caret().node);
                    self.expects_latex_group = false;
                }
                '}' if !self.latex_groups.is_empty() => self.close_latex_group(),
                '^' | '_' => {
                    self.insert_template(if ch == '^' { "^{#?}" } else { "_{#?}" });
                    self.expects_latex_group = true;
                }
                '/' => {
                    if let Ok(template) = palette::template("\\frac{#?}{#?}") {
                        self.editor.insert_template_wrapping_previous(&template);
                    }
                }
                ch if ch.is_whitespace() => {}
                _ => {
                    self.expects_latex_group = false;
                    self.editor.insert_text(&ch.to_string());
                }
            }
        }
    }

    /// Moves the caret outside the `<mtext>` node `id` if it points to a text run.
    pub(super) fn exit_text_run(&mut self, id: u64) -> bool {
        if !self.editor.node(id).is_some_and(|node| node.tag == "mtext") {
            return false;
        }
        let Some(mut path) = self.editor.path(id) else {
            return false;
        };
        while let Some(index) = path.pop() {
            let Some(parent_id) = self.editor.id_at_path(&path) else {
                return false;
            };
            if self
                .editor
                .node(parent_id)
                .is_some_and(|node| node.is_row())
            {
                self.editor.set_caret(
                    Caret {
                        node: parent_id,
                        offset: index + 1,
                    },
                    false,
                );
                if self.latex_groups.last() == Some(&id) {
                    self.latex_groups.pop();
                }
                self.expects_latex_group = false;
                return true;
            }
        }
        false
    }

    /// Advances or exits the enclosing `{...}` scope captured on the canvas.
    pub(super) fn close_latex_group(&mut self) {
        let Some(start) = self.latex_groups.pop() else {
            return;
        };
        if self.exit_text_run(start) {
            return;
        }
        let Some(mut path) = self.editor.path(start) else {
            return;
        };
        while let Some(index) = path.pop() {
            let Some(parent_id) = self.editor.id_at_path(&path) else {
                return;
            };
            let Some(parent) = self.editor.node(parent_id) else {
                return;
            };
            if !matches!(
                parent.tag.as_str(),
                "mfrac"
                    | "mroot"
                    | "msqrt"
                    | "msub"
                    | "msup"
                    | "msubsup"
                    | "munder"
                    | "mover"
                    | "munderover"
            ) {
                continue;
            }
            if parent.tag == "mfrac" && index == 0 {
                if let Some(denom) = parent.children.get(1) {
                    self.expects_latex_group = true;
                    self.editor.set_caret(
                        Caret {
                            node: denom.id,
                            offset: 0,
                        },
                        false,
                    );
                }
                return;
            }
            while let Some(index) = path.pop() {
                let Some(id) = self.editor.id_at_path(&path) else {
                    return;
                };
                if self.editor.node(id).is_some_and(|node| node.is_row()) {
                    self.editor.set_caret(
                        Caret {
                            node: id,
                            offset: index + 1,
                        },
                        false,
                    );
                    return;
                }
            }
            return;
        }
    }
}
