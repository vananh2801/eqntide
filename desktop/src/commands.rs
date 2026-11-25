//! Local LaTeX command completion and validation for the structural canvas and desktop editor.

#[cfg(test)]
use crate::palette;

const COMMANDS: &[(&str, &str, &str)] = eqntide::catalog::COMMANDS;

/// Autocomplete entry shown in the canvas backslash popup.
#[derive(Clone, Copy)]
pub struct Command {
    pub name: &'static str,
    pub label: &'static str,
    pub source: &'static str,
}

/// Interactive state for the inline backslash command buffer on the Equation Canvas.
#[derive(Default)]
pub struct Input {
    pub name: String,
    pub selected: usize,
    pub cursor: usize,
    pub needs_focus: bool,
    pub sync_cursor: bool,
}

impl Input {
    /// Opens a fresh command buffer with initial focus and cursor sync requested.
    pub fn opened() -> Self {
        Self {
            needs_focus: true,
            sync_cursor: true,
            ..Self::default()
        }
    }

    /// Returns the number of Unicode scalar values currently in the command buffer.
    pub fn char_len(&self) -> usize {
        self.name.chars().count()
    }

    /// Clamps `cursor` to the valid character boundary range of `name`.
    pub fn clamp_cursor(&mut self) {
        self.cursor = self.cursor.min(self.char_len());
    }

    /// Inserts `ch` at the current character cursor position and advances it.
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

    /// Deletes the character at the cursor without moving the cursor position.
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

    /// Moves the cursor one character to the left.
    pub fn move_left(&mut self) {
        self.clamp_cursor();
        if self.cursor > 0 {
            self.cursor -= 1;
            self.sync_cursor = true;
        }
    }

    /// Moves the cursor one character to the right.
    pub fn move_right(&mut self) {
        self.clamp_cursor();
        if self.cursor < self.char_len() {
            self.cursor += 1;
            self.sync_cursor = true;
        }
    }

    /// Moves the cursor to the start of the command buffer.
    pub fn move_home(&mut self) {
        if self.cursor != 0 {
            self.cursor = 0;
            self.sync_cursor = true;
        }
    }

    /// Moves the cursor to the end of the command buffer.
    pub fn move_end(&mut self) {
        let end = self.char_len();
        if self.cursor != end {
            self.cursor = end;
            self.sync_cursor = true;
        }
    }

    /// Replaces the command text and places the cursor at the end.
    pub fn set_name_end(&mut self, name: impl Into<String>) {
        self.name = name.into();
        self.cursor = self.char_len();
        self.selected = 0;
        self.sync_cursor = true;
    }

    /// Returns `true` when the buffer contains only ASCII letters (a command prefix).
    pub fn is_command_prefix(&self) -> bool {
        self.name.bytes().all(|byte| byte.is_ascii_alphabetic())
    }

    fn byte_index(&self, char_idx: usize) -> usize {
        self.name
            .char_indices()
            .nth(char_idx)
            .map_or(self.name.len(), |(idx, _)| idx)
    }
}

/// Returns `true` for desktop LaTeX commands that require non-empty braced arguments.
pub fn is_argument_requiring_command(name: &str) -> bool {
    matches!(
        name,
        "text"
            | "mathbb"
            | "mathfrak"
            | "mathcal"
            | "mathscr"
            | "mathrm"
            | "mathbf"
            | "mathit"
            | "boldsymbol"
            | "operatorname"
            | "hspace"
            | "textstyle"
            | "displaystyle"
            | "enclose"
    )
}

/// Formats the user-facing error message when an argument-requiring command is missing its body.
pub fn bare_command_error(name: &str) -> String {
    match name {
        "text" => r"Invalid LaTeX command. Use \text{...} to insert text.".into(),
        "enclose" => r"Invalid LaTeX command. Use \enclose{notation}{...} with non-empty notation and body arguments.".into(),
        "textstyle" | "displaystyle" => format!(
            r"Invalid LaTeX command. Use \{name}{{...}} or {{\{name} ...}} with a non-empty body."
        ),
        _ => format!(
            r"Invalid LaTeX command. Use \{name}{{...}} with a non-empty braced argument."
        ),
    }
}

/// Validates desktop LaTeX input (both Equation Canvas command buffers and `LaTeX source`)
/// for argument completeness and specialized font-alphabet constraints.
pub fn validate_desktop_latex(source: &str) -> Result<(), String> {
    let mut scanner = Scanner {
        src: source,
        pos: 0,
    };
    while let Some(ch) = scanner.peek_char() {
        if ch == '{' {
            let saved = scanner.pos;
            if let Some(inner) = scanner.read_braced() {
                if let Some((cmd, body)) = match_style_group_prefix(inner) {
                    if body.trim().is_empty() {
                        return Err(bare_command_error(cmd));
                    }
                    validate_desktop_latex(body)?;
                } else {
                    validate_desktop_latex(inner)?;
                }
            } else {
                if let Some((cmd, _)) = match_style_group_prefix(&scanner.src[saved + 1..]) {
                    return Err(bare_command_error(cmd));
                }
                scanner.pos = saved + 1;
            }
            continue;
        }
        if ch == '\\' {
            scanner.pos += 1;
            let Some(next) = scanner.peek_char() else {
                break;
            };
            if !next.is_ascii_alphabetic() {
                scanner.pos += next.len_utf8();
                continue;
            }
            let cmd_start = scanner.pos;
            while scanner
                .peek_char()
                .is_some_and(|next_ch| next_ch.is_ascii_alphabetic())
            {
                scanner.pos += 1;
            }
            let cmd = &scanner.src[cmd_start..scanner.pos];
            match cmd {
                "text" => {
                    scanner.skip_ws();
                    let Some(inner) = scanner.read_braced() else {
                        return Err(bare_command_error("text"));
                    };
                    if inner.is_empty() {
                        return Err(bare_command_error("text"));
                    }
                }
                "mathbb" | "mathcal" | "mathscr" | "mathfrak" => {
                    scanner.skip_ws();
                    let Some(inner) = scanner.read_braced() else {
                        return Err(bare_command_error(cmd));
                    };
                    let trimmed = inner.trim();
                    if trimmed.is_empty() {
                        return Err(bare_command_error(cmd));
                    }
                    validate_specialized_alphabet(cmd, trimmed)?;
                }
                "mathrm" | "mathbf" | "mathit" | "boldsymbol" | "operatorname" | "textstyle"
                | "displaystyle" => {
                    scanner.skip_ws();
                    let Some(inner) = scanner.read_braced() else {
                        return Err(bare_command_error(cmd));
                    };
                    if inner.trim().is_empty() {
                        return Err(bare_command_error(cmd));
                    }
                    validate_desktop_latex(inner)?;
                }
                "hspace" => {
                    scanner.skip_ws();
                    let Some(inner) = scanner.read_braced() else {
                        return Err(bare_command_error("hspace"));
                    };
                    if inner.trim().is_empty() {
                        return Err(bare_command_error("hspace"));
                    }
                }
                "enclose" => {
                    scanner.skip_ws();
                    let Some(notation) = scanner.read_braced() else {
                        return Err(bare_command_error("enclose"));
                    };
                    if notation.trim().is_empty() {
                        return Err(bare_command_error("enclose"));
                    }
                    scanner.skip_ws();
                    let Some(body) = scanner.read_braced() else {
                        return Err(bare_command_error("enclose"));
                    };
                    if body.trim().is_empty() {
                        return Err(bare_command_error("enclose"));
                    }
                    validate_desktop_latex(body)?;
                }
                _ => {}
            }
            continue;
        }
        scanner.pos += ch.len_utf8();
    }
    Ok(())
}

/// Returns up to 8 deduplicated autocomplete suggestions matching `prefix`.
pub fn suggestions(prefix: &str) -> Vec<Command> {
    let mut commands: Vec<_> = COMMANDS
        .iter()
        .map(|&(name, label, source)| Command {
            name,
            label,
            source,
        })
        .filter(|cmd| cmd.name.starts_with(prefix))
        .collect();
    commands.sort_by_key(|cmd| (cmd.name != prefix, cmd.name));
    commands.dedup_by_key(|cmd| cmd.name);
    commands.truncate(8);
    commands
}

/// Looks up a command whose name matches `name` exactly.
pub fn exact(name: &str) -> Option<Command> {
    suggestions(name).into_iter().find(|cmd| cmd.name == name)
}

struct Scanner<'a> {
    src: &'a str,
    pos: usize,
}

impl<'a> Scanner<'a> {
    fn rest(&self) -> &'a str {
        &self.src[self.pos..]
    }

    fn peek_char(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn skip_ws(&mut self) {
        while let Some(ch) = self.peek_char() {
            if ch.is_whitespace() {
                self.pos += ch.len_utf8();
            } else {
                break;
            }
        }
    }

    fn read_braced(&mut self) -> Option<&'a str> {
        if !self.rest().starts_with('{') {
            return None;
        }
        let start = self.pos + 1;
        let mut depth = 1usize;
        let mut escaped = false;
        for (offset, ch) in self.src[start..].char_indices() {
            if !escaped {
                match ch {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            let end = start + offset;
                            self.pos = end + 1;
                            return Some(&self.src[start..end]);
                        }
                    }
                    _ => {}
                }
            }
            escaped = !escaped && ch == '\\';
        }
        None
    }
}

fn validate_specialized_alphabet(cmd: &str, trimmed: &str) -> Result<(), String> {
    let valid = match cmd {
        "mathbb" => trimmed
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch == 'k'),
        "mathcal" => trimmed
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch == 'l'),
        "mathscr" | "mathfrak" => trimmed.chars().all(|ch| ch.is_ascii_alphabetic()),
        _ => true,
    };
    if valid {
        return Ok(());
    }
    Err(match cmd {
        "mathbb" => r"Invalid LaTeX command: \mathbb{...} supports only uppercase A-Z and k (no digits, operators, or nested scripts; place exponents outside like \mathbb{R}^2).".into(),
        "mathcal" => r"Invalid LaTeX command: \mathcal{...} supports only uppercase A-Z and l (no digits, operators, or nested scripts; place exponents outside like \mathcal{A}^2).".into(),
        "mathscr" => r"Invalid LaTeX command: \mathscr{...} supports only letters A-Z and a-z (no digits, operators, or nested scripts; place exponents outside like \mathscr{A}^2).".into(),
        "mathfrak" => r"Invalid LaTeX command: \mathfrak{...} supports only letters A-Z and a-z (no digits, operators, or nested scripts; place exponents outside like \mathfrak{g}^2).".into(),
        _ => bare_command_error(cmd),
    })
}

fn match_style_group_prefix(inner: &str) -> Option<(&'static str, &str)> {
    let trimmed = inner.trim_start();
    for cmd in ["textstyle", "displaystyle"] {
        if let Some(after_cmd) = trimmed
            .strip_prefix('\\')
            .and_then(|rest| rest.strip_prefix(cmd))
        {
            if after_cmd.is_empty() || !after_cmd.as_bytes()[0].is_ascii_alphabetic() {
                return Some((cmd, after_cmd));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_completion_is_an_insertable_template() {
        for &(name, _, source) in COMMANDS {
            assert!(palette::template(source).is_ok(), "{name}");
        }
        for prefix in [
            "lim",
            "infty",
            "rightarrow",
            "alpha",
            "frac",
            "dfrac",
            "mathscr",
            "mathrm",
            "mathbf",
            "mathit",
            "boldsymbol",
            "operatorname",
            "textstyle",
            "displaystyle",
        ] {
            assert_eq!(suggestions(prefix)[0].name, prefix);
            assert!(exact(prefix).is_some());
        }
        assert_eq!(suggestions("math")[0].name, "mathbb");
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
        assert_eq!(editor.root().to_math_node().to_latex(), r"\text{5 cm}");
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

    #[test]
    fn desktop_validation_rejects_bare_incomplete_and_empty_argument_commands() {
        for invalid in [
            r"\text",
            r"\text ",
            r"\text{",
            r"\text{}",
            r"\text+",
            r"\mathbb",
            r"\mathbb{}",
            r"\mathbb{ }",
            r"\mathbb{R",
            r"\mathfrak",
            r"\mathfrak{}",
            r"\mathfrak{ }",
            r"\mathcal",
            r"\mathcal{}",
            r"\mathcal{ }",
            r"\mathscr",
            r"\mathscr{}",
            r"\mathscr{ }",
            r"\mathrm",
            r"\mathrm{}",
            r"\mathrm{ }",
            r"\mathbf",
            r"\mathbf{}",
            r"\mathbf{ }",
            r"\mathit",
            r"\mathit{}",
            r"\mathit{ }",
            r"\boldsymbol",
            r"\boldsymbol{}",
            r"\boldsymbol{ }",
            r"\operatorname",
            r"\operatorname{}",
            r"\operatorname{ }",
            r"\hspace",
            r"\hspace{}",
            r"\hspace{ }",
            r"\textstyle",
            r"\textstyle{}",
            r"\textstyle{ }",
            r"{\textstyle}",
            r"{\textstyle }",
            r"{\textstyle",
            r"\textstyle x",
            r"\displaystyle",
            r"\displaystyle{}",
            r"\displaystyle{ }",
            r"{\displaystyle}",
            r"{\displaystyle }",
            r"\enclose",
            r"\enclose{}",
            r"\enclose{box}",
            r"\enclose{box}{",
            r"\enclose{box}{}",
            r"\enclose{box}{ }",
            r"\enclose{}{x}",
            r"\frac{\mathbb{}}{2}",
        ] {
            let err = validate_desktop_latex(invalid)
                .expect_err(&format!("expected error for `{invalid}`"));
            assert!(
                err.contains("Invalid LaTeX command"),
                "unexpected error for `{invalid}`: {err}"
            );
        }
    }

    #[test]
    fn desktop_validation_allows_space_only_text_style_groups_and_complete_arguments() {
        for valid in [
            r"\text{ }",
            r"\text{5 cm }",
            r"\text{\alpha \{x\} }",
            r"\mathbb{R}",
            r"\mathbb{Z}",
            r"\mathbb{k}",
            r"\mathbb{R}^2",
            r"\mathcal{A}",
            r"\mathcal{L}",
            r"\mathcal{l}",
            r"\mathscr{A}",
            r"\mathscr{a}",
            r"\mathfrak{Ag}",
            r"\mathrm{H_2O}",
            r"\mathrm{m/s^2}",
            r"\mathbf{x^2}",
            r"\mathit{AB}",
            r"\boldsymbol{\alpha}",
            r"\operatorname{sgn}",
            r"\hspace{1em}",
            r"\textstyle{x}",
            r"{\textstyle \frac{1}{2}}",
            r"{\displaystyle \sum_{i=1}^n i}",
            r"\enclose{box}{x+y}",
            r"\frac{1}{\sqrt{2}}",
        ] {
            assert!(
                validate_desktop_latex(valid).is_ok(),
                "expected `{valid}` to be valid, got {:?}",
                validate_desktop_latex(valid)
            );
        }
    }

    #[test]
    fn desktop_validation_checks_specialized_math_font_alphabets() {
        for invalid in [
            r"\mathbb{1^2}",
            r"\mathbb{1}",
            r"\mathbb{a}",
            r"\mathbb{R^2}",
            r"\mathbb{R_0}",
            r"\mathbb{R Z}",
            r"\mathbb{R+1}",
            r"\mathbb{\alpha}",
            r"\mathcal{1}",
            r"\mathcal{a}",
            r"\mathcal{A^2}",
            r"\mathscr{1}",
            r"\mathscr{A^2}",
            r"\mathfrak{1+2}",
            r"\mathfrak{1}",
            r"\mathfrak{g_1}",
            r"\frac{\mathbb{1^2}}{2}",
            r"\mathbf{\mathbb{1^2}}",
        ] {
            let err = validate_desktop_latex(invalid)
                .expect_err(&format!("expected alphabet error for `{invalid}`"));
            assert!(
                err.contains("Invalid LaTeX command"),
                "unexpected message for `{invalid}`: {err}"
            );
        }
    }
}
