//! Balanced, bounded native extensions around latex2mathml's ordinary grammar.
//! Native fragments are substituted as complete atoms, so nested fractions,
//! scripts, matrices and escaped braces keep their normal TeX grouping.
use crate::{Error, MathNode, Result};

const MAX_DEPTH: usize = 64;

fn error(message: impl Into<String>) -> Error {
    Error::Xml(format!("LaTeX parsing error: {}", message.into()))
}

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
            let ch = scanner.next().unwrap();
            output.push(ch);
            continue;
        }
        let start = scanner.pos;
        let name = scanner.command();
        if matches!(name.as_str(), "left" | "right" | "middle") {
            scanner.whitespace();
            if source[scanner.pos..].starts_with('\\') {
                let delimiter_start = scanner.pos;
                let delimiter = scanner.command();
                // latex2mathml 0.2 accepts literal bars but lacks the common names.
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
        // Text arguments are opaque: their backslashes are not math commands.
        if matches!(name.as_str(), "text" | "operatorname") {
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
    let mut math = super::parse_standard(&output)?;
    fn replace(node: &mut MathNode, marker: &str, nodes: &[MathNode]) {
        if node.tag == "mtext" {
            if let Some(index) = node.text.strip_prefix(marker).and_then(decode_marker_index) {
                if let Some(native) = nodes.get(index) {
                    *node = native.clone();
                    return;
                }
            }
        }
        for child in &mut node.children {
            replace(child, marker, nodes);
        }
    }
    replace(&mut math, &marker, &scanner.nodes);
    Ok(math)
}

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
    fn command(&mut self) -> String {
        debug_assert!(self.source[self.pos..].starts_with('\\'));
        self.next();
        let start = self.pos;
        while self.source[self.pos..]
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic())
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
                (Some(a), Some(b)) => MathNode::with_children(
                    if side { "msubsup" } else { "munderover" },
                    vec![node, a, b],
                ),
                (Some(a), None) => {
                    MathNode::with_children(if side { "msub" } else { "munder" }, vec![node, a])
                }
                (None, Some(b)) => {
                    MathNode::with_children(if side { "msup" } else { "mover" }, vec![node, b])
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
            "frac" | "dfrac" | "tfrac" | "sfrac" | "tsfrac" | "atopfrac" => {
                let a = self.math_argument()?;
                let b = self.math_argument()?;
                let mut n = MathNode::with_children("mfrac", vec![a, b]);
                match name {
                    "dfrac" => n.set_attr("displaystyle", "true"),
                    "tfrac" => n.set_attr("displaystyle", "false"),
                    "sfrac" => n.set_attr("bevelled", "true"),
                    "tsfrac" => {
                        n.set_attr("bevelled", "true");
                        n.set_attr("displaystyle", "false");
                    }
                    "atopfrac" => n.set_attr("linethickness", "0"),
                    _ => {}
                }
                n
            }
            "textstyle" | "displaystyle" => {
                let body = self.math_argument()?;
                let mut n = MathNode::with_children("mstyle", vec![body]);
                n.set_attr(
                    "displaystyle",
                    if name == "textstyle" { "false" } else { "true" },
                );
                n
            }
            "prescript" => {
                let sup = self.math_argument()?;
                let sub = self.math_argument()?;
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
                let mut n = MathNode::with_children("menclose", vec![body]);
                n.set_attr("notation", &notation);
                n
            }
            "mathbb" | "mathfrak" | "mathcal" | "mathscr" | "mathrm" | "mathbf" | "mathit"
            | "boldsymbol" => {
                let mut n = self.math_argument()?;
                let style = match name {
                    "mathbb" => "double-struck",
                    "mathfrak" => "fraktur",
                    "mathcal" | "mathscr" => "script",
                    "mathrm" => "normal",
                    "mathbf" => "bold",
                    "boldsymbol" => "bold-italic",
                    _ => "italic",
                };
                fn apply(n: &mut MathNode, value: &str) {
                    if matches!(n.tag.as_str(), "mi" | "mo" | "mn") {
                        n.set_attr("mathvariant", value);
                    }
                    for c in &mut n.children {
                        apply(c, value);
                    }
                }
                apply(&mut n, style);
                n
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
                let mut n = MathNode::with_children(
                    if name == "overaccent" {
                        "mover"
                    } else {
                        "munder"
                    },
                    vec![base, mark],
                );
                n.set_attr(
                    if name == "overaccent" {
                        "accent"
                    } else {
                        "accentunder"
                    },
                    "true",
                );
                n
            }
            "doubleoverline" | "doubleunderline" => {
                let body = self.math_argument()?;
                let tag = if name == "doubleoverline" {
                    "mover"
                } else {
                    "munder"
                };
                let mut n = accent(tag, body, "¯");
                n = accent(tag, n, "¯");
                n
            }
            "hspace" => {
                let width = self.argument()?.trim();
                let units = ["em", "ex", "mu", "pt"];
                let number = units
                    .iter()
                    .find_map(|u| width.strip_suffix(u))
                    .ok_or_else(|| error("hspace requires em, ex, mu or pt units"))?;
                if !number.trim().parse::<f32>().is_ok_and(f32::is_finite) {
                    return Err(error("invalid hspace dimension"));
                }
                let mut n = MathNode::new("mspace");
                n.set_attr("width", width);
                n
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
                let mut n = MathNode::new("mspace");
                n.set_attr("width", width);
                n
            }
            "begin" => {
                let start = self.pos;
                let environment = self.argument()?;
                if matches!(environment, "Bmatrix" | "Vmatrix") {
                    let body = self.environment_body(environment)?;
                    let matrix = format!("\\begin{{matrix}}{body}\\end{{matrix}}");
                    let mut n = parse_at(&matrix, self.depth + 1)?;
                    let table = if n.children.len() == 1 {
                        n.children.remove(0)
                    } else {
                        n.tag = "mrow".into();
                        n.attrs.clear();
                        n
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
                    if columns.is_empty() || columns.chars().any(|c| !matches!(c, 'l' | 'c' | 'r'))
                    {
                        return Err(error("array columns must contain l, c or r"));
                    }
                    let body = self.environment_body("array")?;
                    let matrix = format!("\\begin{{matrix}}{body}\\end{{matrix}}");
                    let mut n = parse_at(&matrix, self.depth + 1)?;
                    let aligns = columns
                        .chars()
                        .map(|c| match c {
                            'l' => "left",
                            'r' => "right",
                            _ => "center",
                        })
                        .collect::<Vec<_>>()
                        .join(" ");
                    fn align(n: &mut MathNode, value: &str) {
                        if n.tag == "mtable" {
                            n.set_attr("columnalign", value);
                        } else {
                            for c in &mut n.children {
                                align(c, value);
                            }
                        }
                    }
                    align(&mut n, &aligns);
                    if n.children.len() == 1 {
                        n.children.remove(0)
                    } else {
                        n.tag = "mrow".into();
                        n.attrs.clear();
                        n
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

fn encode_marker_index(index: usize) -> String {
    format!("{index:x}")
        .bytes()
        .map(|b| match b {
            b'0'..=b'9' => (b'A' + (b - b'0')) as char,
            _ => (b'A' + (b - b'a' + 10)) as char,
        })
        .collect()
}

fn decode_marker_index(s: &str) -> Option<usize> {
    if s.is_empty() {
        return None;
    }
    let mut val = 0usize;
    for b in s.bytes() {
        if !(b'A'..=b'P').contains(&b) {
            return None;
        }
        val = val.checked_mul(16)?.checked_add((b - b'A') as usize)?;
    }
    Some(val)
}

fn command_end(source: &str, pos: usize) -> bool {
    !source[pos..]
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic())
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
