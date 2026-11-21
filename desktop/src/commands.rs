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
}

impl Input {
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
}
