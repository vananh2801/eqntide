//! Balanced, bounded native extensions around `latex2mathml`'s grammar.
//!
//! ```text
//! raw TeX ──▶ Scanner (extract native macros / braces / environments into nodes[])
//!                │
//!                ▼  placeholder tokens `\text{EQNTIDENATIVE<hex>}`
//!         ScriptRewriter (pair adjacent `_` / `^` and primes into `<msubsup>`)
//!                │
//!                ▼
//!     super::parse_standard (`latex2mathml` + XML fixups)
//!                │
//!                ▼
//!         replace_markers ──▶ flatten_prime_superscripts ──▶ MathNode
//! ```
//!
//! Native fragments are substituted as complete atoms so nested fractions,
//! scripts, matrices, and escaped braces keep their normal TeX grouping.

use crate::{Error, MathNode, Result};

const MAX_DEPTH: usize = 64;

/// Parse a trimmed LaTeX math body (without outer delimiters) into a [`MathNode`].
pub(super) fn parse(source: &str) -> Result<MathNode> {
    parse_at(source, 0)
}

fn parse_at(source: &str, depth: usize) -> Result<MathNode> {
    if depth > MAX_DEPTH {
        return Err(error("expression nesting exceeds 64 levels"));
    }
    let mut nesting = 0usize;
    let mut escaped = false;
    for ch in source.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '{' => {
                nesting += 1;
                if nesting + depth > MAX_DEPTH {
                    return Err(error("expression nesting exceeds 64 levels"));
                }
            }
            '}' => {
                nesting = nesting
                    .checked_sub(1)
                    .ok_or_else(|| error("unexpected closing brace"))?
            }
            _ => {}
        }
    }
    if nesting != 0 {
        return Err(error("unclosed group"));
    }

    let mut scanner = Scanner {
        source,
        pos: 0,
        depth,
        nodes: Vec::new(),
    };
    let mut output = String::new();
    // Pick a marker absent from user text rather than reserving a user-visible name.
    let mut marker = "EQNTIDENATIVE".to_string();
    while source.contains(&marker) {
        marker.push('X');
    }
    while scanner.pos < source.len() {
        if !source[scanner.pos..].starts_with('\\') {
            if source[scanner.pos..].starts_with('{') {
                if let Some(node) = scanner.try_braced_extension()? {
                    output.push_str(&format!(
                        "\\text{{{marker}{}}}",
                        encode_marker_index(scanner.nodes.len())
                    ));
                    scanner.nodes.push(node);
                    continue;
                }
            }
            if source[scanner.pos..].starts_with('^') && scanner.consume_degree_superscript() {
                output.push_str(&format!(
                    "\\text{{{marker}{}}}",
                    encode_marker_index(scanner.nodes.len())
                ));
                scanner.nodes.push(MathNode::leaf("mo", "°"));
                continue;
            }
            let Some(ch) = scanner.next() else {
                break;
            };
            if ch == '°' {
                output.push_str(&format!(
                    "\\text{{{marker}{}}}",
                    encode_marker_index(scanner.nodes.len())
                ));
                scanner.nodes.push(MathNode::leaf("mo", "°"));
            } else {
                output.push(ch);
            }
            continue;
        }
        let start = scanner.pos;
        let name = scanner.command();
        if matches!(name.as_str(), "left" | "right" | "middle") {
            scanner.whitespace();
            if source[scanner.pos..].starts_with('\\') {
                let delimiter_start = scanner.pos;
                let delimiter = scanner.command();
                // latex2mathml 0.2 accepts literal bars but lacks the common command names.
                if matches!(
                    delimiter.as_str(),
                    "Vert" | "lVert" | "rVert" | "vert" | "lvert" | "rvert"
                ) {
                    output.push_str(&source[start..delimiter_start]);
                    output.push_str(if delimiter.contains("Vert") {
                        r"\|"
                    } else {
                        "|"
                    });
                    continue;
                }
            } else if scanner.next().is_none() {
                return Err(error("missing delimiter"));
            }
            output.push_str(&source[start..scanner.pos]);
            continue;
        }
        if name == "operatorname" {
            scanner.argument()?;
            output.push_str(&source[start..scanner.pos]);
            continue;
        }
        if let Some(node) = scanner.native(&name)? {
            output.push_str(&format!(
                "\\text{{{marker}{}}}",
                encode_marker_index(scanner.nodes.len())
            ));
            scanner.nodes.push(node);
        } else {
            output.push_str(&source[start..scanner.pos]);
        }
    }
    let rewritten = rewrite_script_pairs(&output, &marker, &mut scanner.nodes)?;
    let mut math = super::parse_standard(&rewritten)?;
    replace_markers(&mut math, &marker, &scanner.nodes);
    flatten_prime_superscripts(&mut math);
    Ok(math)
}

// ── Script-Pair Preprocessor ────────────────────────────────────────────────

fn rewrite_script_pairs(src: &str, marker: &str, nodes: &mut Vec<MathNode>) -> Result<String> {
    ScriptRewriter {
        src,
        pos: 0,
        marker,
        nodes,
    }
    .rewrite()
}

struct ScriptRewriter<'a> {
    src: &'a str,
    pos: usize,
    marker: &'a str,
    nodes: &'a mut Vec<MathNode>,
}

impl<'a> ScriptRewriter<'a> {
    fn peek_char(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn next(&mut self) -> Option<char> {
        let ch = self.peek_char()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }

    fn skip_ws(&mut self) {
        while self.peek_char().is_some_and(char::is_whitespace) {
            self.next();
        }
    }

    fn read_command_name(&mut self) -> &'a str {
        debug_assert!(self.src[self.pos..].starts_with('\\'));
        self.next();
        let start = self.pos;
        while self.peek_char().is_some_and(|ch| ch.is_ascii_alphabetic()) {
            self.next();
        }
        if start == self.pos {
            self.next();
        }
        &self.src[start..self.pos]
    }

    fn read_braced(&mut self) -> Result<&'a str> {
        if self.next() != Some('{') {
            return Err(error("expected '{'"));
        }
        let begin = self.pos;
        let mut level = 1usize;
        while let Some(ch) = self.next() {
            match ch {
                '\\' => {
                    self.next();
                }
                '{' => level += 1,
                '}' => {
                    level -= 1;
                    if level == 0 {
                        return Ok(&self.src[begin..self.pos - 1]);
                    }
                }
                _ => {}
            }
        }
        Err(error("unclosed group"))
    }

    fn read_delim(&mut self) -> Result<&'a str> {
        let start = self.pos;
        self.skip_ws();
        if self.src[self.pos..].starts_with('\\') {
            self.read_command_name();
        } else if self.next().is_none() {
            return Err(error("missing delimiter"));
        }
        Ok(&self.src[start..self.pos])
    }

    fn read_tex_arg(&mut self) -> Result<String> {
        let ws_start = self.pos;
        self.skip_ws();
        let ws = &self.src[ws_start..self.pos];
        let atom = self
            .read_atom()?
            .ok_or_else(|| error("missing command argument"))?;
        Ok(format!("{ws}{atom}"))
    }

    fn read_left_right(&mut self, delim1: &str) -> Result<String> {
        let body_start = self.pos;
        let mut depth = 1usize;
        while self.pos < self.src.len() {
            if self.src[self.pos..].starts_with('\\') {
                let cmd_start = self.pos;
                let cmd = self.read_command_name();
                if matches!(cmd, "text" | "operatorname") {
                    self.skip_ws();
                    if self.peek_char() == Some('{') {
                        self.read_braced()?;
                    }
                } else if cmd == "left" {
                    depth += 1;
                } else if cmd == "right" {
                    depth -= 1;
                    if depth == 0 {
                        let body = &self.src[body_start..cmd_start];
                        let delim2 = self.read_delim()?;
                        let rewritten_body = rewrite_script_pairs(body, self.marker, self.nodes)?;
                        return Ok(format!("\\left{delim1}{rewritten_body}\\right{delim2}"));
                    }
                }
            } else {
                self.next();
            }
        }
        Err(error("unclosed \\left delimiter"))
    }

    fn read_begin_end(&mut self, env: &str) -> Result<String> {
        let body_start = self.pos;
        let mut level = 1usize;
        while self.pos < self.src.len() {
            if self.src[self.pos..].starts_with('\\') {
                let cmd_start = self.pos;
                let cmd = self.read_command_name();
                if matches!(cmd, "text" | "operatorname") {
                    self.skip_ws();
                    if self.peek_char() == Some('{') {
                        self.read_braced()?;
                    }
                } else if matches!(cmd, "begin" | "end") {
                    self.skip_ws();
                    if self.peek_char() == Some('{') {
                        let name = self.read_braced()?;
                        if name == env {
                            if cmd == "begin" {
                                level += 1;
                            } else {
                                level -= 1;
                                if level == 0 {
                                    let body = &self.src[body_start..cmd_start];
                                    let rewritten_body =
                                        rewrite_script_pairs(body, self.marker, self.nodes)?;
                                    return Ok(format!(
                                        "\\begin{{{env}}}{rewritten_body}\\end{{{env}}}"
                                    ));
                                }
                            }
                        }
                    }
                }
            } else {
                self.next();
            }
        }
        Err(error(format!("unclosed {env} environment")))
    }

    fn read_atom(&mut self) -> Result<Option<String>> {
        let Some(ch) = self.peek_char() else {
            return Ok(None);
        };
        if matches!(ch, '_' | '^' | '}' | '&') {
            return Ok(None);
        }
        if ch == '{' {
            let inner = self.read_braced()?;
            let rewritten = rewrite_script_pairs(inner, self.marker, self.nodes)?;
            return Ok(Some(format!("{{{rewritten}}}")));
        }
        if ch == '\\' {
            let start = self.pos;
            let cmd = self.read_command_name();
            let raw = &self.src[start..self.pos];
            return match cmd {
                "text" | "operatorname" => {
                    let ws_start = self.pos;
                    self.skip_ws();
                    let ws = &self.src[ws_start..self.pos];
                    if self.peek_char() == Some('{') {
                        let inner = self.read_braced()?;
                        Ok(Some(format!("{raw}{ws}{{{inner}}}")))
                    } else {
                        Ok(Some(format!("{raw}{ws}")))
                    }
                }
                "left" => {
                    let delim1 = self.read_delim()?;
                    Ok(Some(self.read_left_right(delim1)?))
                }
                "right" => {
                    let delim = self.read_delim()?;
                    Ok(Some(format!("{raw}{delim}")))
                }
                "begin" => {
                    let ws_start = self.pos;
                    self.skip_ws();
                    let ws = &self.src[ws_start..self.pos];
                    if self.peek_char() == Some('{') {
                        let env = self.read_braced()?;
                        let mut out = String::from(raw);
                        out.push_str(ws);
                        let body_and_end = self.read_begin_end(env)?;
                        if let Some(rest) = body_and_end.strip_prefix("\\begin") {
                            out.push_str(rest);
                            Ok(Some(out))
                        } else {
                            Ok(Some(body_and_end))
                        }
                    } else {
                        Ok(Some(format!("{raw}{ws}")))
                    }
                }
                "sqrt" => {
                    let mut out = String::from(raw);
                    let ws_pos = self.pos;
                    self.skip_ws();
                    if self.peek_char() == Some('[') {
                        out.push_str(&self.src[ws_pos..self.pos]);
                        self.next();
                        let deg_start = self.pos;
                        let mut brace_level = 0usize;
                        let mut found = false;
                        while let Some(next_ch) = self.next() {
                            match next_ch {
                                '\\' => {
                                    self.next();
                                }
                                '{' => brace_level += 1,
                                '}' => brace_level = brace_level.saturating_sub(1),
                                ']' if brace_level == 0 => {
                                    let deg = &self.src[deg_start..self.pos - 1];
                                    let rewritten_deg =
                                        rewrite_script_pairs(deg, self.marker, self.nodes)?;
                                    out.push('[');
                                    out.push_str(&rewritten_deg);
                                    out.push(']');
                                    found = true;
                                    break;
                                }
                                _ => {}
                            }
                        }
                        if !found {
                            return Err(error("unclosed root degree"));
                        }
                    } else {
                        self.pos = ws_pos;
                    }
                    out.push_str(&self.read_tex_arg()?);
                    Ok(Some(out))
                }
                "frac" | "binom" | "dbinom" | "tbinom" | "overset" | "underset" => {
                    let first = self.read_tex_arg()?;
                    let second = self.read_tex_arg()?;
                    Ok(Some(format!("{raw}{first}{second}")))
                }
                "mathrm" | "mathit" | "mathbf" | "mathbb" | "mathfrak" | "mathscr" | "mathsf"
                | "texttt" | "textit" | "textbf" | "bm" | "symbf" | "boldsymbol" | "underline"
                | "overline" | "slashed" => {
                    let arg = self.read_tex_arg()?;
                    Ok(Some(format!("{raw}{arg}")))
                }
                "bigl" | "bigr" | "Bigl" | "Bigr" | "biggl" | "biggr" | "Biggl" | "Biggr" => {
                    let delim = self.read_delim()?;
                    Ok(Some(format!("{raw}{delim}")))
                }
                _ => Ok(Some(raw.to_string())),
            };
        }
        if ch.is_ascii_digit() {
            let start = self.pos;
            let mut has_period = false;
            while let Some(next_ch) = self.peek_char() {
                if next_ch.is_ascii_digit() {
                    self.next();
                } else if next_ch == '.' && !has_period {
                    has_period = true;
                    self.next();
                } else {
                    break;
                }
            }
            return Ok(Some(self.src[start..self.pos].to_string()));
        }
        if self.src[self.pos..].starts_with(":=") {
            self.pos += 2;
            return Ok(Some(":=".to_string()));
        }
        self.next();
        Ok(Some(ch.to_string()))
    }

    fn rewrite(&mut self) -> Result<String> {
        let mut out = String::with_capacity(self.src.len());
        while self.pos < self.src.len() {
            let ws_start = self.pos;
            self.skip_ws();
            out.push_str(&self.src[ws_start..self.pos]);
            if self.pos >= self.src.len() {
                break;
            }
            let Some(mut atom) = self.read_atom()? else {
                let Some(ch) = self.next() else {
                    break;
                };
                out.push(ch);
                continue;
            };
            if atom != "'" {
                let save_prime = self.pos;
                self.skip_ws();
                let mut primes = 0usize;
                while self.peek_char() == Some('\'') {
                    self.next();
                    primes += 1;
                }
                if primes > 0 {
                    let after_prime = self.pos;
                    self.skip_ws();
                    if matches!(self.peek_char(), Some('_') | Some('^')) {
                        for _ in 0..primes {
                            atom.push('\'');
                        }
                        self.pos = after_prime;
                    } else {
                        self.pos = save_prime;
                    }
                } else {
                    self.pos = save_prime;
                }
            }
            loop {
                let save = self.pos;
                self.skip_ws();
                if let Some(op1) = self.peek_char().filter(|&ch| ch == '_' || ch == '^') {
                    self.next();
                    self.skip_ws();
                    let Some(arg1) = self.read_atom()? else {
                        self.pos = save;
                        break;
                    };
                    let after_arg1 = self.pos;
                    self.skip_ws();
                    if self
                        .peek_char()
                        .is_some_and(|ch| (op1 == '_' && ch == '^') || (op1 == '^' && ch == '_'))
                    {
                        self.next();
                        self.skip_ws();
                        if let Some(arg2) = self.read_atom()? {
                            let (sub_src, sup_src) = if op1 == '_' {
                                (arg1, arg2)
                            } else {
                                (arg2, arg1)
                            };
                            let base_node = parse_fragment(&atom, self.marker, self.nodes)?;
                            let sub_node = parse_fragment(&sub_src, self.marker, self.nodes)?;
                            let sup_node = parse_fragment(&sup_src, self.marker, self.nodes)?;
                            let msubsup = MathNode::with_children(
                                "msubsup",
                                vec![base_node, sub_node, sup_node],
                            );
                            let idx = self.nodes.len();
                            self.nodes.push(msubsup);
                            atom = format!("\\text{{{}{}}}", self.marker, encode_marker_index(idx));
                            continue;
                        }
                    }
                    self.pos = after_arg1;
                    atom = format!("{atom}{op1}{arg1}");
                    break;
                } else {
                    self.pos = save;
                    break;
                }
            }
            out.push_str(&atom);
        }
        Ok(out)
    }
}

// ── Native Macro Scanner ────────────────────────────────────────────────────

struct Scanner<'a> {
    source: &'a str,
    pos: usize,
    depth: usize,
    nodes: Vec<MathNode>,
}

impl<'a> Scanner<'a> {
    fn next(&mut self) -> Option<char> {
        let ch = self.source[self.pos..].chars().next()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }

    fn whitespace(&mut self) {
        while self.source[self.pos..]
            .chars()
            .next()
            .is_some_and(char::is_whitespace)
        {
            self.next();
        }
    }

    fn consume_degree_superscript(&mut self) -> bool {
        let save = self.pos;
        if self.next() != Some('^') {
            self.pos = save;
            return false;
        }
        self.whitespace();
        if self.source[self.pos..].starts_with('{') {
            self.next();
            self.whitespace();
            if self.source[self.pos..].starts_with("\\circ")
                && command_end(self.source, self.pos + 5)
            {
                self.pos += 5;
                self.whitespace();
                if self.next() == Some('}') {
                    return true;
                }
            }
        } else if self.source[self.pos..].starts_with("\\circ")
            && command_end(self.source, self.pos + 5)
        {
            self.pos += 5;
            return true;
        }
        self.pos = save;
        false
    }

    fn try_braced_extension(&mut self) -> Result<Option<MathNode>> {
        let save = self.pos;
        let inner = self.argument()?;
        let trimmed = inner.trim_start();
        for (cmd, display) in [("\\displaystyle", "true"), ("\\textstyle", "false")] {
            if let Some(rest) = trimmed.strip_prefix(cmd) {
                if rest.is_empty()
                    || !rest
                        .chars()
                        .next()
                        .is_some_and(|ch| ch.is_ascii_alphabetic())
                {
                    let mut body = parse_at(rest, self.depth + 1)?;
                    let body = if body.children.len() == 1 {
                        body.children.remove(0)
                    } else {
                        body.tag = "mrow".into();
                        body.attrs.clear();
                        body
                    };
                    let mut style = MathNode::with_children("mstyle", vec![body]);
                    style.set_attr("displaystyle", display);
                    return Ok(Some(style));
                }
            }
        }
        if let Some((num_src, den_src)) = split_top_level_atop(inner) {
            let num = parse_fragment_at(num_src, self.depth + 1)?;
            let den = parse_fragment_at(den_src, self.depth + 1)?;
            let mut frac = MathNode::with_children("mfrac", vec![num, den]);
            frac.set_attr("linethickness", "0");
            return Ok(Some(frac));
        }
        self.pos = save;
        Ok(None)
    }

    fn command(&mut self) -> String {
        debug_assert!(self.source[self.pos..].starts_with('\\'));
        self.next();
        let start = self.pos;
        while self.source[self.pos..]
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_alphabetic())
        {
            self.next();
        }
        if start == self.pos {
            self.next();
        }
        self.source[start..self.pos].to_string()
    }

    /// Return one argument without outer braces, preserving nested/escaped groups.
    fn argument(&mut self) -> Result<&'a str> {
        self.whitespace();
        let start = self.pos;
        if self.next() == Some('{') {
            let begin = self.pos;
            let mut level = 1;
            while let Some(ch) = self.next() {
                match ch {
                    '\\' => {
                        self.next();
                    }
                    '{' => level += 1,
                    '}' => {
                        level -= 1;
                        if level == 0 {
                            return Ok(&self.source[begin..self.pos - 1]);
                        }
                    }
                    _ => {}
                }
            }
            Err(error("unclosed command argument"))
        } else {
            self.pos = start;
            if self.source[self.pos..].starts_with('\\') {
                self.command();
            } else if self.next().is_none() {
                return Err(error("missing command argument"));
            }
            Ok(&self.source[start..self.pos])
        }
    }

    fn math_argument(&mut self) -> Result<MathNode> {
        let arg = self.argument()?;
        let mut math = parse_at(arg, self.depth + 1)?;
        if math.children.len() == 1 {
            Ok(math.children.remove(0))
        } else {
            math.tag = "mrow".into();
            math.attrs.clear();
            Ok(math)
        }
    }

    fn prescript_argument(&mut self) -> Result<MathNode> {
        let arg = self.argument()?;
        if arg.trim().is_empty() {
            return Ok(MathNode::new("none"));
        }
        let mut math = parse_at(arg, self.depth + 1)?;
        if math.children.len() == 1 {
            Ok(math.children.remove(0))
        } else {
            math.tag = "mrow".into();
            math.attrs.clear();
            Ok(math)
        }
    }

    fn native(&mut self, name: &str) -> Result<Option<MathNode>> {
        if let Some((glyph, tag)) = big_operator(name) {
            let mut node = MathNode::leaf(tag, glyph);
            let before = self.pos;
            self.whitespace();
            let mut side = false;
            if self.source[self.pos..].starts_with("\\nolimits")
                && command_end(self.source, self.pos + 9)
            {
                self.pos += 9;
                side = true;
            } else if self.source[self.pos..].starts_with("\\limits")
                && command_end(self.source, self.pos + 7)
            {
                self.pos += 7;
            }
            let (mut sub, mut sup) = (None, None);
            loop {
                self.whitespace();
                if self.source[self.pos..].starts_with('_') && sub.is_none() {
                    self.next();
                    sub = Some(self.math_argument()?);
                } else if self.source[self.pos..].starts_with('^') && sup.is_none() {
                    self.next();
                    sup = Some(self.math_argument()?);
                } else {
                    break;
                }
            }
            node = match (sub, sup) {
                (Some(lower), Some(upper)) => MathNode::with_children(
                    if side { "msubsup" } else { "munderover" },
                    vec![node, lower, upper],
                ),
                (Some(lower), None) => {
                    MathNode::with_children(if side { "msub" } else { "munder" }, vec![node, lower])
                }
                (None, Some(upper)) => {
                    MathNode::with_children(if side { "msup" } else { "mover" }, vec![node, upper])
                }
                _ => {
                    self.pos = if side { self.pos } else { before };
                    node
                }
            };
            if side {
                node.set_attr("data-limloc", "subSup");
            }
            return Ok(Some(node));
        }
        let node = match name {
            "text" => MathNode::leaf("mtext", &unescape_text(self.argument()?)),
            "frac" | "dfrac" | "tfrac" | "sfrac" | "tsfrac" | "atopfrac" => {
                let num = self.math_argument()?;
                let den = self.math_argument()?;
                let mut frac = MathNode::with_children("mfrac", vec![num, den]);
                match name {
                    "dfrac" => frac.set_attr("displaystyle", "true"),
                    "tfrac" => frac.set_attr("displaystyle", "false"),
                    "sfrac" => frac.set_attr("bevelled", "true"),
                    "tsfrac" => {
                        frac.set_attr("bevelled", "true");
                        frac.set_attr("displaystyle", "false");
                    }
                    "atopfrac" => frac.set_attr("linethickness", "0"),
                    _ => {}
                }
                frac
            }
            "binom" | "dbinom" | "tbinom" => {
                let num = self.math_argument()?;
                let den = self.math_argument()?;
                let mut frac = MathNode::with_children("mfrac", vec![num, den]);
                frac.set_attr("linethickness", "0");
                match name {
                    "dbinom" => frac.set_attr("displaystyle", "true"),
                    "tbinom" => frac.set_attr("displaystyle", "false"),
                    _ => {}
                }
                wrap_fenced_frac("(", ")", frac)
            }
            "genfrac" => {
                let left = self.argument()?.trim();
                let right = self.argument()?.trim();
                let thickness = self.argument()?.trim();
                let style = self.argument()?.trim();
                let num = self.math_argument()?;
                let den = self.math_argument()?;
                let mut frac = MathNode::with_children("mfrac", vec![num, den]);
                if matches!(thickness, "0" | "0pt" | "0px" | "none") {
                    frac.set_attr("linethickness", "0");
                }
                match style {
                    "0" => frac.set_attr("displaystyle", "true"),
                    "1" => frac.set_attr("displaystyle", "false"),
                    _ => {}
                }
                if left.is_empty() && right.is_empty() {
                    frac
                } else {
                    wrap_fenced_frac(
                        normalize_genfrac_delim(left),
                        normalize_genfrac_delim(right),
                        frac,
                    )
                }
            }
            "textstyle" | "displaystyle" => {
                self.whitespace();
                let body = if self.source[self.pos..].starts_with('{') {
                    let save = self.pos;
                    let arg = self.argument()?;
                    if self.source[self.pos..].trim().is_empty() {
                        parse_fragment_at(arg, self.depth + 1)?
                    } else {
                        let tail = &self.source[save..];
                        self.pos = self.source.len();
                        parse_fragment_at(tail, self.depth + 1)?
                    }
                } else {
                    let tail = &self.source[self.pos..];
                    self.pos = self.source.len();
                    parse_fragment_at(tail, self.depth + 1)?
                };
                let mut style = MathNode::with_children("mstyle", vec![body]);
                style.set_attr(
                    "displaystyle",
                    if name == "textstyle" { "false" } else { "true" },
                );
                style
            }
            "prescript" => {
                let sup = self.prescript_argument()?;
                let sub = self.prescript_argument()?;
                let base = self.math_argument()?;
                MathNode::with_children(
                    "mmultiscripts",
                    vec![base, MathNode::new("mprescripts"), sub, sup],
                )
            }
            "enclose" | "boxed" | "cancel" | "bcancel" | "xcancel" => {
                let notation = match name {
                    "enclose" => self.argument()?.to_string(),
                    "boxed" => "box".into(),
                    "cancel" => "updiagonalstrike".into(),
                    "bcancel" => "downdiagonalstrike".into(),
                    _ => "updiagonalstrike downdiagonalstrike".into(),
                };
                if notation.split_whitespace().any(|part| {
                    !matches!(
                        part,
                        "box"
                            | "roundedbox"
                            | "circle"
                            | "phasorangle"
                            | "top"
                            | "bottom"
                            | "left"
                            | "right"
                            | "longdiv"
                            | "actuarial"
                            | "horizontalstrike"
                            | "verticalstrike"
                            | "updiagonalstrike"
                            | "downdiagonalstrike"
                            | "radical"
                    )
                }) {
                    return Err(error("unsupported enclosure notation"));
                }
                let body = self.math_argument()?;
                let mut enclose = MathNode::with_children("menclose", vec![body]);
                enclose.set_attr("notation", &notation);
                enclose
            }
            "mathbb" | "mathfrak" | "mathcal" | "mathscr" | "mathrm" | "mathbf" | "mathit"
            | "boldsymbol" => {
                let mut node = self.math_argument()?;
                let style = match name {
                    "mathbb" => "double-struck",
                    "mathfrak" => "fraktur",
                    "mathcal" | "mathscr" => "script",
                    "mathrm" => "normal",
                    "mathbf" => "bold",
                    "boldsymbol" => "bold-italic",
                    _ => "italic",
                };
                fn apply(target: &mut MathNode, value: &str) {
                    if matches!(target.tag.as_str(), "mi" | "mo" | "mn") {
                        target.set_attr("mathvariant", value);
                    }
                    for child in &mut target.children {
                        apply(child, value);
                    }
                }
                apply(&mut node, style);
                node
            }
            "overset" | "underset" => {
                let mark = self.math_argument()?;
                let base = self.math_argument()?;
                if name == "overset" && base.tag == "munder" {
                    let mut children = base.children;
                    children.push(mark);
                    MathNode::with_children("munderover", children)
                } else if name == "underset" && base.tag == "mover" {
                    MathNode::with_children(
                        "munderover",
                        vec![base.children[0].clone(), mark, base.children[1].clone()],
                    )
                } else {
                    MathNode::with_children(
                        if name == "overset" { "mover" } else { "munder" },
                        vec![base, mark],
                    )
                }
            }
            "overaccent" | "underaccent" => {
                let raw_mark = self.argument()?;
                let mut mark = if raw_mark.chars().count() == 1 {
                    MathNode::leaf("mo", raw_mark)
                } else {
                    let mut parsed = parse_at(raw_mark, self.depth + 1)?;
                    if parsed.children.len() == 1 {
                        parsed.children.remove(0)
                    } else {
                        parsed.tag = "mrow".into();
                        parsed.attrs.clear();
                        parsed
                    }
                };
                mark.set_attr("stretchy", "true");
                let base = self.math_argument()?;
                let mut scripted = MathNode::with_children(
                    if name == "overaccent" {
                        "mover"
                    } else {
                        "munder"
                    },
                    vec![base, mark],
                );
                scripted.set_attr(
                    if name == "overaccent" {
                        "accent"
                    } else {
                        "accentunder"
                    },
                    "true",
                );
                scripted
            }
            "doubleoverline" | "doubleunderline" => {
                let body = self.math_argument()?;
                let tag = if name == "doubleoverline" {
                    "mover"
                } else {
                    "munder"
                };
                let inner = accent(tag, body, "¯");
                accent(tag, inner, "¯")
            }
            "hspace" => {
                let width = self.argument()?.trim();
                let units = ["em", "ex", "mu", "pt"];
                let number = units
                    .iter()
                    .find_map(|unit| width.strip_suffix(unit))
                    .ok_or_else(|| error("hspace requires em, ex, mu or pt units"))?;
                if !number.trim().parse::<f32>().is_ok_and(f32::is_finite) {
                    return Err(error("invalid hspace dimension"));
                }
                let mut space = MathNode::new("mspace");
                space.set_attr("width", width);
                space
            }
            "," | ":" | ";" | "!" | " " | "quad" | "qquad" | "enspace" | "thinspace"
            | "medspace" | "thickspace" | "negthinspace" => {
                let width = match name {
                    "," | "thinspace" => "0.166667em",
                    ":" | "medspace" => "0.222222em",
                    ";" | "thickspace" => "0.277778em",
                    "!" | "negthinspace" => "-0.166667em",
                    "enspace" => "0.5em",
                    "qquad" => "2em",
                    _ => "1em",
                };
                let mut space = MathNode::new("mspace");
                space.set_attr("width", width);
                space
            }
            "begin" => {
                let start = self.pos;
                let environment = self.argument()?;
                if matches!(environment, "Bmatrix" | "Vmatrix") {
                    let body = self.environment_body(environment)?;
                    let matrix = format!("\\begin{{matrix}}{body}\\end{{matrix}}");
                    let mut parsed = parse_at(&matrix, self.depth + 1)?;
                    let table = if parsed.children.len() == 1 {
                        parsed.children.remove(0)
                    } else {
                        parsed.tag = "mrow".into();
                        parsed.attrs.clear();
                        parsed
                    };
                    let (open, close) = if environment == "Bmatrix" {
                        ("{", "}")
                    } else {
                        ("‖", "‖")
                    };
                    let mut left = MathNode::leaf("mo", open);
                    left.set_attr("stretchy", "true");
                    let mut right = MathNode::leaf("mo", close);
                    right.set_attr("stretchy", "true");
                    MathNode::with_children("mrow", vec![left, table, right])
                } else if environment != "array" {
                    self.pos = start;
                    return Ok(None);
                } else {
                    let columns = self.argument()?.trim();
                    if columns.is_empty()
                        || columns.chars().any(|ch| !matches!(ch, 'l' | 'c' | 'r'))
                    {
                        return Err(error("array columns must contain l, c or r"));
                    }
                    let body = self.environment_body("array")?;
                    let matrix = format!("\\begin{{matrix}}{body}\\end{{matrix}}");
                    let mut parsed = parse_at(&matrix, self.depth + 1)?;
                    let aligns = columns
                        .chars()
                        .map(|ch| match ch {
                            'l' => "left",
                            'r' => "right",
                            _ => "center",
                        })
                        .collect::<Vec<_>>()
                        .join(" ");
                    fn align(target: &mut MathNode, value: &str) {
                        if target.tag == "mtable" {
                            target.set_attr("columnalign", value);
                        } else {
                            for child in &mut target.children {
                                align(child, value);
                            }
                        }
                    }
                    align(&mut parsed, &aligns);
                    if parsed.children.len() == 1 {
                        parsed.children.remove(0)
                    } else {
                        parsed.tag = "mrow".into();
                        parsed.attrs.clear();
                        parsed
                    }
                }
            }
            "limits" | "nolimits" => return Err(error("limits must follow a large operator")),
            _ => {
                if let Some((tag, glyph)) = accent_spec(name) {
                    let body = self.math_argument()?;
                    let mut result = accent(tag, body, glyph);
                    // Brace labels are stacked above/below the brace, not side scripts.
                    if matches!(
                        name,
                        "overbrace"
                            | "underbrace"
                            | "overbracket"
                            | "underbracket"
                            | "overgroup"
                            | "undergroup"
                            | "overparen"
                            | "underparen"
                    ) {
                        let before = self.pos;
                        self.whitespace();
                        let script = if tag == "mover" { '^' } else { '_' };
                        if self.source[self.pos..].starts_with(script) {
                            self.next();
                            let label = self.math_argument()?;
                            result = MathNode::with_children(tag, vec![result, label]);
                        } else {
                            self.pos = before;
                        }
                    }
                    result
                } else if name == "dots" {
                    MathNode::leaf("mo", "…")
                } else if let Some(symbol) = symbol_or_alias(name) {
                    symbol
                } else if let Some(symbol) = crate::catalog::symbol(name) {
                    symbol
                } else {
                    return Ok(None);
                }
            }
        };
        Ok(Some(node))
    }

    fn environment_body(&mut self, environment: &str) -> Result<&'a str> {
        let start = self.pos;
        let mut level = 1usize;
        while self.pos < self.source.len() {
            if self.source[self.pos..].starts_with('\\') {
                let command_start = self.pos;
                let command = self.command();
                if matches!(command.as_str(), "begin" | "end") {
                    let name = self.argument()?;
                    if name == environment {
                        if command == "begin" {
                            level += 1;
                        } else {
                            level -= 1;
                            if level == 0 {
                                return Ok(&self.source[start..command_start]);
                            }
                        }
                    }
                }
            } else {
                self.next();
            }
        }
        Err(error(format!("unclosed {environment} environment")))
    }
}

// ── Marker Substitution & Prime Normalization ───────────────────────────────

fn parse_fragment(fragment: &str, marker: &str, nodes: &[MathNode]) -> Result<MathNode> {
    let mut math = super::parse_standard(fragment)?;
    replace_markers(&mut math, marker, nodes);
    flatten_prime_superscripts(&mut math);
    Ok(if math.children.len() == 1 {
        math.children.remove(0)
    } else {
        math.tag = "mrow".into();
        math.attrs.clear();
        math
    })
}

fn parse_fragment_at(arg: &str, depth: usize) -> Result<MathNode> {
    let mut math = parse_at(arg, depth)?;
    if math.children.len() == 1 {
        Ok(math.children.remove(0))
    } else {
        math.tag = "mrow".into();
        math.attrs.clear();
        Ok(math)
    }
}

fn replace_markers(node: &mut MathNode, marker: &str, nodes: &[MathNode]) {
    if node.tag == "mtext" {
        if let Some(index) = node.text.strip_prefix(marker).and_then(decode_marker_index) {
            if let Some(native) = nodes.get(index) {
                *node = native.clone();
                return;
            }
        }
    }
    for child in &mut node.children {
        replace_markers(child, marker, nodes);
    }
}

fn flatten_prime_superscripts(node: &mut MathNode) {
    for child in &mut node.children {
        flatten_prime_superscripts(child);
    }
    let mut idx = 0;
    while idx + 1 < node.children.len() {
        let can_merge = node.children[idx].tag == "msup" && node.children[idx].children.len() == 2;
        if can_merge {
            if let (Some(left_count), Some(right_count)) = (
                prime_token_count(&node.children[idx].children[1]),
                prime_token_count(&node.children[idx + 1]),
            ) {
                let sum = left_count.saturating_add(right_count);
                if sum <= 3 {
                    node.children.remove(idx + 1);
                    node.children[idx].children[1] =
                        MathNode::leaf("mo", prime_glyph_for_count(sum));
                    continue;
                }
            }
        }
        idx += 1;
    }
    if node.tag == "msup" && node.children.len() == 2 {
        if let Some(outer_count) = prime_token_count(&node.children[1]) {
            let combined = if node.children[0].tag == "msup" && node.children[0].children.len() == 2
            {
                if let Some(inner_count) = prime_token_count(&node.children[0].children[1]) {
                    let sum = inner_count.saturating_add(outer_count);
                    if sum <= 3 {
                        let inner_base = node.children[0].children.remove(0);
                        node.children[0] = inner_base;
                        Some(sum)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else if outer_count <= 3 {
                Some(outer_count)
            } else {
                None
            };
            if let Some(total) = combined {
                node.children[1] = MathNode::leaf("mo", prime_glyph_for_count(total));
            }
        }
    }
}

fn prime_token_count(node: &MathNode) -> Option<u8> {
    if node.tag == "mo" && node.children.is_empty() {
        let mut total = 0u8;
        let text = node.text.trim();
        if text.is_empty() {
            return None;
        }
        for ch in text.chars() {
            let add = match ch {
                '\'' | '\u{2032}' => 1,
                '\u{2033}' => 2,
                '\u{2034}' => 3,
                _ => return None,
            };
            total = total.saturating_add(add);
        }
        return (total > 0).then_some(total);
    }
    if node.tag == "mrow" && !node.children.is_empty() {
        let mut total = 0u8;
        for child in &node.children {
            total = total.saturating_add(prime_token_count(child)?);
        }
        return (total > 0).then_some(total);
    }
    if node.tag == "msup" && node.children.len() == 2 {
        let base_count = prime_token_count(&node.children[0])?;
        let sup_count = prime_token_count(&node.children[1])?;
        return base_count.checked_add(sup_count);
    }
    None
}

fn prime_glyph_for_count(count: u8) -> &'static str {
    match count {
        1 => "\u{2032}",
        2 => "\u{2033}",
        _ => "\u{2034}",
    }
}

fn encode_marker_index(index: usize) -> String {
    format!("{index:x}")
        .bytes()
        .map(|byte| match byte {
            b'0'..=b'9' => (b'A' + (byte - b'0')) as char,
            _ => (b'A' + (byte - b'a' + 10)) as char,
        })
        .collect()
}

fn decode_marker_index(encoded: &str) -> Option<usize> {
    if encoded.is_empty() {
        return None;
    }
    let mut val = 0usize;
    for byte in encoded.bytes() {
        if !(b'A'..=b'P').contains(&byte) {
            return None;
        }
        val = val.checked_mul(16)?.checked_add((byte - b'A') as usize)?;
    }
    Some(val)
}

// ── TeX & Catalog Helpers ───────────────────────────────────────────────────

fn unescape_text(mut source: &str) -> String {
    let mut text = String::with_capacity(source.len());
    while let Some(ch) = source.chars().next() {
        source = &source[ch.len_utf8()..];
        if ch == '\\' {
            let command = [
                ("textbackslash", '\\'),
                ("textasciitilde", '~'),
                ("textasciicircum", '^'),
            ]
            .into_iter()
            .find(|(name, _)| source.starts_with(name) && command_end(source, name.len()));
            if let Some((name, value)) = command {
                let rest = &source[name.len()..];
                source = rest.strip_prefix("{}").unwrap_or(rest);
                text.push(value);
                continue;
            }
            if let Some(escaped @ ('{' | '}' | '_' | '&' | '%' | '$' | '#' | '\\' | ' ')) =
                source.chars().next()
            {
                source = &source[escaped.len_utf8()..];
                text.push(escaped);
                continue;
            }
        }
        text.push(if ch == '~' { '\u{00A0}' } else { ch });
    }
    text
}

fn command_end(source: &str, pos: usize) -> bool {
    !source[pos..]
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_alphabetic())
}

fn split_top_level_atop(src: &str) -> Option<(&str, &str)> {
    let bytes = src.as_bytes();
    let mut idx = 0usize;
    let mut level = 0usize;
    while idx < bytes.len() {
        if bytes[idx] == b'\\' {
            if level == 0 && src[idx..].starts_with("\\atop") && command_end(src, idx + 5) {
                return Some((&src[..idx], &src[idx + 5..]));
            }
            idx += 1;
            if idx < bytes.len() && bytes[idx].is_ascii_alphabetic() {
                while idx < bytes.len() && bytes[idx].is_ascii_alphabetic() {
                    idx += 1;
                }
            } else if idx < bytes.len() {
                let ch = src[idx..].chars().next()?;
                idx += ch.len_utf8();
            }
            continue;
        }
        match bytes[idx] {
            b'{' => level += 1,
            b'}' => level = level.saturating_sub(1),
            _ => {}
        }
        idx += 1;
    }
    None
}

fn normalize_genfrac_delim(delim: &str) -> &str {
    match delim {
        "" | "." => "",
        "\\{" => "{",
        "\\}" => "}",
        "\\|" | "\\Vert" | "\\lVert" | "\\rVert" => "‖",
        "\\langle" => "⟨",
        "\\rangle" => "⟩",
        "\\lfloor" => "⌊",
        "\\rfloor" => "⌋",
        "\\lceil" => "⌈",
        "\\rceil" => "⌉",
        "\\llbracket" => "⟦",
        "\\rrbracket" => "⟧",
        other => other,
    }
}

fn wrap_fenced_frac(open_delim: &str, close_delim: &str, frac: MathNode) -> MathNode {
    let mut open = MathNode::leaf("mo", open_delim);
    open.set_attr("stretchy", "true");
    open.set_attr("form", "prefix");
    let mut close = MathNode::leaf("mo", close_delim);
    close.set_attr("stretchy", "true");
    close.set_attr("form", "postfix");
    MathNode::with_children("mrow", vec![open, frac, close])
}

fn accent(tag: &str, base: MathNode, glyph: &str) -> MathNode {
    let mut mark = MathNode::leaf("mo", glyph);
    mark.set_attr("stretchy", "true");
    let mut node = MathNode::with_children(tag, vec![base, mark]);
    node.set_attr(
        if tag == "mover" {
            "accent"
        } else {
            "accentunder"
        },
        "true",
    );
    node
}

fn accent_spec(name: &str) -> Option<(&'static str, &'static str)> {
    Some(match name {
        "dot" => ("mover", "˙"),
        "ddot" => ("mover", "¨"),
        "dddot" => ("mover", "⃛"),
        "ddddot" => ("mover", "⃜"),
        "underdot" => ("munder", "˙"),
        "underddot" => ("munder", "¨"),
        "underdddot" => ("munder", "⃛"),
        "underddddot" => ("munder", "⃜"),
        "hat" | "widehat" => ("mover", "^"),
        "check" => ("mover", "ˇ"),
        "breve" => ("mover", "˘"),
        "acute" => ("mover", "´"),
        "grave" => ("mover", "`"),
        "mathring" => ("mover", "°"),
        "tilde" | "widetilde" => ("mover", "~"),
        "undertilde" => ("munder", "~"),
        "underhat" => ("munder", "^"),
        "undercheck" => ("munder", "ˇ"),
        "underbreve" => ("munder", "˘"),
        "underring" => ("munder", "°"),
        "bar" | "overline" => ("mover", "¯"),
        "underline" => ("munder", "¯"),
        "vec" | "overrightarrow" => ("mover", "→"),
        "overleftarrow" => ("mover", "←"),
        "overleftrightarrow" => ("mover", "↔"),
        "underrightarrow" => ("munder", "→"),
        "underleftarrow" => ("munder", "←"),
        "underleftrightarrow" => ("munder", "↔"),
        "overleftharpoon" => ("mover", "↼"),
        "overrightharpoon" => ("mover", "⇀"),
        "underleftharpoon" => ("munder", "↼"),
        "underrightharpoon" => ("munder", "⇀"),
        "wideparen" | "overparen" | "overgroup" => ("mover", "⏜"),
        "underparen" | "undergroup" => ("munder", "⏝"),
        "overbrace" => ("mover", "⏞"),
        "underbrace" => ("munder", "⏟"),
        "overbracket" => ("mover", "⎴"),
        "underbracket" => ("munder", "⎵"),
        _ => return None,
    })
}

fn big_operator(name: &str) -> Option<(&'static str, &'static str)> {
    Some(match name {
        "sum" => ("∑", "mo"),
        "prod" => ("∏", "mo"),
        "coprod" => ("∐", "mo"),
        "bigcup" => ("⋃", "mo"),
        "bigcap" => ("⋂", "mo"),
        "bigvee" => ("⋁", "mo"),
        "bigwedge" => ("⋀", "mo"),
        "bigodot" => ("⨀", "mo"),
        "bigotimes" => ("⨂", "mo"),
        "bigoplus" => ("⨁", "mo"),
        "bigsqcup" => ("⨆", "mo"),
        "biguplus" => ("⨄", "mo"),
        "int" => ("∫", "mo"),
        "iint" => ("∬", "mo"),
        "iiint" => ("∭", "mo"),
        "oint" => ("∮", "mo"),
        "oiint" => ("∯", "mo"),
        "oiiint" => ("∰", "mo"),
        "varointclockwise" | "ointclockwise" => ("∲", "mo"),
        "ointctrclockwise" => ("∳", "mo"),
        "lim" => ("lim", "mi"),
        "liminf" => ("lim inf", "mi"),
        "limsup" => ("lim sup", "mi"),
        "min" => ("min", "mi"),
        "max" => ("max", "mi"),
        "inf" => ("inf", "mi"),
        "sup" => ("sup", "mi"),
        _ => return None,
    })
}

fn symbol_or_alias(name: &str) -> Option<MathNode> {
    let (tag, glyph) = match name {
        "le" | "leq" => ("mo", "≤"),
        "ge" | "geq" => ("mo", "≥"),
        "ne" | "neq" => ("mo", "≠"),
        "to" | "rightarrow" => ("mo", "→"),
        "gets" | "leftarrow" => ("mo", "←"),
        "implies" | "Rightarrow" => ("mo", "⇒"),
        "iff" | "Leftrightarrow" => ("mo", "⇔"),
        "land" | "wedge" => ("mo", "∧"),
        "lor" | "vee" => ("mo", "∨"),
        "lnot" | "neg" => ("mo", "¬"),
        "dots" | "ldots" => ("mo", "…"),
        "smallsetminus" | "backslash" | "setminus" => ("mo", "∖"),
        "varnothing" | "emptyset" => ("mo", "∅"),
        "vert" | "lvert" | "rvert" => ("mo", "|"),
        "Vert" | "lVert" | "rVert" => ("mo", "‖"),
        "dprime" => ("mo", "″"),
        "trprime" => ("mo", "‴"),
        "lg" => ("mi", "lg"),
        "hom" => ("mi", "hom"),
        "deg" => ("mi", "deg"),
        "arg" => ("mi", "arg"),
        "gcd" => ("mi", "gcd"),
        "Pr" => ("mi", "Pr"),
        _ => return None,
    };
    Some(MathNode::leaf(tag, glyph))
}

fn error(message: impl Into<String>) -> Error {
    Error::Xml(format!("LaTeX parsing error: {}", message.into()))
}
