//! Local LaTeX command completion for the structural canvas.
#[cfg(test)]
use crate::palette;

#[derive(Clone, Copy)]
pub struct Command {
    pub name: &'static str,
    pub label: &'static str,
    pub source: &'static str,
}

#[derive(Default)]
pub struct Input {
    pub name: String,
    pub selected: usize,
    pub cursor: usize,
    pub needs_focus: bool,
    pub sync_cursor: bool,
}

impl Input {
    pub fn opened() -> Self {
        Self {
            needs_focus: true,
            sync_cursor: true,
            ..Self::default()
        }
    }

    pub fn char_len(&self) -> usize {
        self.name.chars().count()
    }

    fn byte_index(&self, char_idx: usize) -> usize {
        self.name
            .char_indices()
            .nth(char_idx)
            .map_or(self.name.len(), |(idx, _)| idx)
    }

    pub fn clamp_cursor(&mut self) {
        self.cursor = self.cursor.min(self.char_len());
    }

    pub fn insert_char(&mut self, ch: char) {
        self.clamp_cursor();
        let byte_idx = self.byte_index(self.cursor);
        self.name.insert(byte_idx, ch);
        self.cursor += 1;
        self.selected = 0;
        self.sync_cursor = true;
    }

    /// Deletes the character before the cursor. Returns `false` when the buffer
    /// is already empty so the caller can close the command popup.
    pub fn backspace(&mut self) -> bool {
        if self.name.is_empty() {
            return false;
        }
        self.clamp_cursor();
        if self.cursor > 0 {
            let start = self.byte_index(self.cursor - 1);
            let end = self.byte_index(self.cursor);
            self.name.replace_range(start..end, "");
            self.cursor -= 1;
            self.selected = 0;
            self.sync_cursor = true;
        }
        true
    }

    pub fn delete(&mut self) {
        self.clamp_cursor();
        if self.cursor < self.char_len() {
            let start = self.byte_index(self.cursor);
            let end = self.byte_index(self.cursor + 1);
            self.name.replace_range(start..end, "");
            self.selected = 0;
            self.sync_cursor = true;
        }
    }

    pub fn move_left(&mut self) {
        self.clamp_cursor();
        if self.cursor > 0 {
            self.cursor -= 1;
            self.sync_cursor = true;
        }
    }

    pub fn move_right(&mut self) {
        self.clamp_cursor();
        if self.cursor < self.char_len() {
            self.cursor += 1;
            self.sync_cursor = true;
        }
    }

    pub fn move_home(&mut self) {
        if self.cursor != 0 {
            self.cursor = 0;
            self.sync_cursor = true;
        }
    }

    pub fn move_end(&mut self) {
        let end = self.char_len();
        if self.cursor != end {
            self.cursor = end;
            self.sync_cursor = true;
        }
    }

    pub fn set_name_end(&mut self, name: impl Into<String>) {
        self.name = name.into();
        self.cursor = self.char_len();
        self.selected = 0;
        self.sync_cursor = true;
    }

    pub fn is_command_prefix(&self) -> bool {
        self.name.bytes().all(|b| b.is_ascii_alphabetic())
    }

    pub fn text_argument(&self) -> Option<&str> {
        let argument = self.name.strip_prefix("text")?;
        (!argument.starts_with(|ch: char| ch.is_ascii_alphabetic()))
            .then_some(argument.trim_start())
    }

    pub fn has_complete_text_argument(&self) -> bool {
        let Some(argument) = self
            .text_argument()
            .and_then(|arg| arg.trim_end().strip_prefix('{'))
        else {
            return false;
        };
        let mut depth = 1;
        let mut escaped = false;
        for (offset, ch) in argument.char_indices() {
            if !escaped {
                match ch {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            return offset + 1 == argument.len();
                        }
                    }
                    _ => {}
                }
            }
            escaped = !escaped && ch == '\\';
        }
        false
    }
}

const COMMANDS: &[(&str, &str, &str)] = eqntide::catalog::COMMANDS;

pub fn suggestions(prefix: &str) -> Vec<Command> {
    let mut commands: Vec<_> = COMMANDS
        .iter()
        .map(|&(name, label, source)| Command {
            name,
            label,
            source,
        })
        .filter(|c| c.name.starts_with(prefix))
        .collect();
    commands.sort_by_key(|c| (c.name != prefix, c.name));
    commands.dedup_by_key(|c| c.name);
    commands.truncate(8);
    commands
}

pub fn exact(name: &str) -> Option<Command> {
    suggestions(name).into_iter().find(|c| c.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_completion_is_an_insertable_template() {
        for &(name, _, source) in COMMANDS {
            assert!(palette::template(source).is_ok(), "{name}");
        }
        for prefix in ["lim", "infty", "rightarrow", "alpha", "frac", "dfrac"] {
            assert_eq!(suggestions(prefix)[0].name, prefix);
            assert!(exact(prefix).is_some());
        }
        assert!(suggestions("nosuchcommand").is_empty());
    }

    #[test]
    fn text_completion_inserts_an_editable_text_token() {
        let command = suggestions("text")[0];
        assert_eq!(command.name, "text");
        assert_eq!(command.source, r"\text{#?}");

        let template = palette::template(command.source).unwrap();
        let mut editor = eqntide_editor::Editor::default();
        editor.insert_template(&template);
        editor.insert_text("5 cm");
        assert_eq!(editor.root().math().to_latex(), r"\text{5 cm}");
    }

    #[test]
    fn input_cursor_edits_and_navigates_utf8_safely() {
        let mut input = Input::opened();
        assert!(input.needs_focus);
        assert!(!input.backspace());

        for ch in "text{à}".chars() {
            input.insert_char(ch);
        }
        assert_eq!(input.name, "text{à}");
        assert_eq!(input.cursor, 7);

        input.move_left();
        input.insert_char('b');
        assert_eq!(input.name, "text{àb}");
        assert_eq!(input.cursor, 7);

        input.move_left();
        assert!(input.backspace());
        assert_eq!(input.name, "text{b}");
        assert_eq!(input.cursor, 5);

        input.move_home();
        assert_eq!(input.cursor, 0);
        assert!(input.backspace());
        assert_eq!(input.name, "text{b}");

        input.move_right();
        assert_eq!(input.cursor, 1);
        input.move_home();
        input.delete();
        assert_eq!(input.name, "ext{b}");
        input.move_end();
        assert_eq!(input.cursor, 6);
        input.delete();
        assert_eq!(input.name, "ext{b}");

        input.set_name_end("mathbb");
        assert_eq!(input.name, "mathbb");
        assert_eq!(input.cursor, 6);
        assert!(input.sync_cursor);
    }
}
