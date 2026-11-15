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
}
