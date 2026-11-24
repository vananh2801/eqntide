//! Serializer from [`MathNode`] (MathML AST) to standard LaTeX markup.
//!
//! Used by the bridge server and desktop editor to populate the LaTeX source
//! editors when opening sessions or importing equations from OMML / MathType.

use crate::mathml::MathNode;

/// Convert a [`MathNode`] tree into a LaTeX mathematical expression string.
pub fn mathml_to_latex(node: &MathNode) -> String {
    let mut out = String::new();
    format_node(node, None, &mut out);
    clean_latex_output(&out)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum NodeStyle<'a> {
    Neutral,
    Uniform { variant: &'a str, triggers: bool },
    Mixed,
}

fn style_command(variant: &str) -> Option<&'static str> {
    match variant {
        "double-struck" => Some("\\mathbb"),
        "fraktur" => Some("\\mathfrak"),
        "script" => Some("\\mathcal"),
        "bold" => Some("\\mathbf"),
        "bold-italic" => Some("\\boldsymbol"),
        "normal" => Some("\\mathrm"),
        "italic" => Some("\\mathit"),
        _ => None,
    }
}

fn contains_non_l_lowercase_script(nodes: &[&MathNode]) -> bool {
    fn visit(node: &MathNode, saw_non_l_lower: &mut bool, saw_unsupported: &mut bool) {
        if matches!(node.tag.as_str(), "mi" | "mn" | "mo")
            && !is_invisible_operator(node.text.trim())
        {
            for ch in node.text.trim().chars() {
                if ch.is_ascii_lowercase() && ch != 'l' {
                    *saw_non_l_lower = true;
                } else if !ch.is_ascii_alphabetic() {
                    *saw_unsupported = true;
                }
            }
        }
        for child in &node.children {
            visit(child, saw_non_l_lower, saw_unsupported);
        }
    }
    let mut saw_non_l_lower = false;
    let mut saw_unsupported = false;
    for node in nodes {
        visit(node, &mut saw_non_l_lower, &mut saw_unsupported);
    }
    saw_non_l_lower && !saw_unsupported
}

fn style_command_for_nodes(variant: &str, nodes: &[&MathNode]) -> &'static str {
    if variant == "script" && contains_non_l_lowercase_script(nodes) {
        "\\mathscr"
    } else {
        style_command(variant).unwrap_or("\\mathrm")
    }
}

fn combine_styles<'a>(a: NodeStyle<'a>, b: NodeStyle<'a>) -> NodeStyle<'a> {
    match (a, b) {
        (NodeStyle::Neutral, other) | (other, NodeStyle::Neutral) => other,
        (
            NodeStyle::Uniform {
                variant: va,
                triggers: ta,
            },
            NodeStyle::Uniform {
                variant: vb,
                triggers: tb,
            },
        ) if va == vb => NodeStyle::Uniform {
            variant: va,
            triggers: ta || tb,
        },
        _ => NodeStyle::Mixed,
    }
}

fn node_style(node: &MathNode) -> NodeStyle<'_> {
    match node.tag.as_str() {
        "mi" | "mn" | "mo" => {
            if node.tag == "mo" && is_invisible_operator(node.text.trim()) {
                return NodeStyle::Neutral;
            }
            if let Some(variant) = node.attr("mathvariant") {
                if style_command(variant).is_some() {
                    if variant == "normal" && node.tag == "mi" && is_named_function(&node.text) {
                        return NodeStyle::Uniform {
                            variant: "normal",
                            triggers: false,
                        };
                    }
                    let triggers = match variant {
                        "normal" => node.tag == "mi",
                        "italic" => node.tag != "mi",
                        _ => true,
                    };
                    return NodeStyle::Uniform { variant, triggers };
                }
            }
            NodeStyle::Mixed
        }
        "mprescripts" | "none" => NodeStyle::Neutral,
        "mrow" | "mfrac" | "msqrt" | "mroot" | "msup" | "msub" | "msubsup" | "mmultiscripts"
        | "munder" | "mover" | "munderover" | "mfenced" | "mtable" | "mtr" | "mtd" => {
            let mut acc = NodeStyle::Neutral;
            for child in &node.children {
                acc = combine_styles(acc, node_style(child));
                if acc == NodeStyle::Mixed {
                    return NodeStyle::Mixed;
                }
            }
            acc
        }
        _ => NodeStyle::Mixed,
    }
}

fn format_node(node: &MathNode, active_variant: Option<&str>, out: &mut String) {
    if active_variant.is_none() && !matches!(node.tag.as_str(), "math" | "mrow" | "mtr" | "mtd") {
        if let NodeStyle::Uniform {
            variant,
            triggers: true,
        } = node_style(node)
        {
            let command = style_command_for_nodes(variant, &[node]);
            out.push_str(command);
            out.push('{');
            format_node(node, Some(variant), out);
            out.push('}');
            return;
        }
    }
    if matches!(node.tag.as_str(), "mi" | "mn" | "mo") && node.attr("mathvariant") != active_variant
    {
        if let NodeStyle::Uniform {
            variant,
            triggers: true,
        } = node_style(node)
        {
            let command = style_command_for_nodes(variant, &[node]);
            out.push_str(command);
            out.push('{');
            format_node(node, Some(variant), out);
            out.push('}');
            return;
        }
    }
    match node.tag.as_str() {
        "math" | "mrow" => format_children(node, active_variant, out),
        "mstyle" => {
            if let Some(value) = node.attr("displaystyle") {
                out.push_str(if value == "false" {
                    "{\\textstyle "
                } else {
                    "{\\displaystyle "
                });
                format_children(node, active_variant, out);
                out.push('}');
            } else {
                format_children(node, active_variant, out);
            }
        }
        "mi" => format_identifier(node, active_variant, out),
        "mn" => out.push_str(node.text.trim()),
        "mo" => format_operator(node, out),
        "mtext" => format_text(&node.text, out),
        "mfrac" => format_fraction(node, active_variant, out),
        "msqrt" => format_square_root(node, active_variant, out),
        "mroot" => format_indexed_root(node, active_variant, out),
        "msup" => format_superscript(node, active_variant, out),
        "msub" => format_subscript(node, active_variant, out),
        "msubsup" => format_subsuperscript(node, active_variant, out),
        "mmultiscripts" => format_multiscripts(node, active_variant, out),
        "munder" => format_underscript(node, active_variant, out),
        "mover" => format_overscript(node, active_variant, out),
        "munderover" => format_underoverscript(node, active_variant, out),
        "mfenced" => format_fenced(node, active_variant, out),
        "mtable" => format_matrix(node, active_variant, out),
        "mtr" | "mtd" => format_children(node, active_variant, out),
        "menclose" => format_enclosure(node, active_variant, out),
        "mspace" => match node.attr("width").unwrap_or("0em").trim() {
            "-0.166667em" => out.push_str("\\!"),
            "0.166667em" => out.push_str("\\,"),
            "0.222222em" => out.push_str("\\:"),
            "0.277778em" => out.push_str("\\;"),
            "0.5em" | "0.500000em" => out.push_str("\\enspace "),
            "1em" | "1.000000em" => out.push_str("\\quad "),
            "2em" | "2.000000em" => out.push_str("\\qquad "),
            width => {
                out.push_str("\\hspace{");
                out.push_str(width);
                out.push('}');
            }
        },
        "mprescripts" | "none" => {}
        _ => {
            if !node.text.is_empty() {
                out.push_str(&node.text);
            }
            for child in &node.children {
                format_node(child, active_variant, out);
            }
        }
    }
}

fn ends_with_control_word(s: &str) -> bool {
    let bytes = s.as_bytes();
    let mut i = bytes.len();
    while i > 0 && bytes[i - 1].is_ascii_alphabetic() {
        i -= 1;
    }
    if i == bytes.len() || i == 0 || bytes[i - 1] != b'\\' {
        return false;
    }
    let mut slashes = 0usize;
    while i > 0 && bytes[i - 1] == b'\\' {
        slashes += 1;
        i -= 1;
    }
    slashes % 2 == 1
}

fn format_children(node: &MathNode, active_variant: Option<&str>, out: &mut String) {
    format_child_slice(&node.children, active_variant, out);
}

fn format_child_slice(children: &[MathNode], active_variant: Option<&str>, out: &mut String) {
    let mut i = 0;
    while i < children.len() {
        let needs_space = ends_with_control_word(out);
        let mut piece = String::new();
        if active_variant.is_none() {
            if let NodeStyle::Uniform {
                variant,
                triggers: initial_trigger,
            } = node_style(&children[i])
            {
                let mut end = i + 1;
                let mut triggers = initial_trigger;
                let mut last_non_neutral = end;
                while end < children.len() {
                    if children[end].tag == "mtext" {
                        break;
                    }
                    match node_style(&children[end]) {
                        NodeStyle::Neutral => {
                            end += 1;
                        }
                        NodeStyle::Uniform {
                            variant: v,
                            triggers: t,
                        } if v == variant => {
                            triggers |= t;
                            end += 1;
                            last_non_neutral = end;
                        }
                        _ => break,
                    }
                }
                end = last_non_neutral;
                if triggers {
                    let refs: Vec<&MathNode> = children[i..end].iter().collect();
                    let command = style_command_for_nodes(variant, &refs);
                    piece.push_str(command);
                    piece.push('{');
                    format_child_slice(&children[i..end], Some(variant), &mut piece);
                    piece.push('}');
                    i = end;
                    out.push_str(&piece);
                    continue;
                }
            }
        }
        if children[i].tag == "mtext" {
            // MathType decodes text as individual character nodes.
            let mut text = String::new();
            while let Some(child) = children.get(i).filter(|child| child.tag == "mtext") {
                text.push_str(&child.text);
                i += 1;
            }
            format_text(&text, &mut piece);
        } else if i + 2 < children.len()
            && is_binom_triple(&children[i], &children[i + 1], &children[i + 2])
        {
            format_binom(&children[i + 1], active_variant, &mut piece);
            i += 3;
        } else {
            format_node(&children[i], active_variant, &mut piece);
            i += 1;
        }
        if needs_space
            && piece
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic())
        {
            out.push(' ');
        }
        out.push_str(&piece);
    }
}

fn is_binom_triple(open: &MathNode, middle: &MathNode, close: &MathNode) -> bool {
    open.tag == "mo"
        && open.text.trim() == "("
        && middle.tag == "mfrac"
        && is_binomial(middle)
        && middle.attr("bevelled") != Some("true")
        && close.tag == "mo"
        && close.text.trim() == ")"
}

fn format_binom(node: &MathNode, active_variant: Option<&str>, out: &mut String) {
    let command = match node.attr("displaystyle") {
        Some("true") => "\\dbinom",
        Some("false") => "\\tbinom",
        _ => "\\binom",
    };
    out.push_str(command);
    format_braced_child(node.children.first(), active_variant, out);
    format_braced_child(node.children.get(1), active_variant, out);
}

fn format_identifier(node: &MathNode, active_variant: Option<&str>, out: &mut String) {
    let text = node.text.trim();
    if let Some(greek) = map_greek_or_symbol(text) {
        out.push_str(greek);
    } else if let Some(symbol) = crate::catalog::SYMBOLS
        .iter()
        .find(|s| s.tag == "mi" && s.glyph == text)
    {
        out.push('\\');
        out.push_str(symbol.name);
    } else if let Some(fn_latex) = format_named_function(text) {
        out.push_str(fn_latex);
    } else if is_operatorname_function(text) {
        out.push_str("\\operatorname{");
        out.push_str(text);
        out.push_str("} ");
    } else if text.len() > 1
        && text.chars().all(char::is_alphabetic)
        && active_variant != Some("normal")
    {
        out.push_str("\\mathrm{");
        out.push_str(text);
        out.push('}');
    } else {
        out.push_str(text);
    }
}

fn format_operator(node: &MathNode, out: &mut String) {
    let text = node.text.trim();
    if node.attr("stretchy") == Some("true")
        && matches!(node.attr("form"), Some("prefix" | "postfix"))
    {
        out.push_str(&map_fence_delimiter(
            text,
            node.attr("form") == Some("prefix"),
        ));
        return;
    }
    if is_invisible_operator(text) {
        // Invisible times/comma/plus/apply are structural MathML markers.
        return;
    }
    if let Some(operator) = map_operator(text) {
        out.push_str(operator);
    } else if let Some(symbol) = crate::catalog::SYMBOLS
        .iter()
        .find(|s| s.tag == "mo" && s.glyph == text)
    {
        let padded = matches!(
            symbol.group,
            "Relations" | "Operators" | "Arrows" | "Logic" | "Sets"
        ) && !matches!(
            symbol.name,
            "emptyset"
                | "neg"
                | "forall"
                | "exists"
                | "nexists"
                | "dagger"
                | "ddagger"
                | "langle"
                | "rangle"
                | "llbracket"
                | "rrbracket"
                | "Vert"
                | "smallcirc"
        );
        if padded {
            out.push(' ');
        }
        out.push('\\');
        out.push_str(symbol.name);
        out.push(' ');
    } else {
        out.push_str(text);
    }
}

fn format_text(text: &str, out: &mut String) {
    if text.is_empty() {
        return;
    }
    out.push_str("\\text{");
    out.push_str(&escape_latex_text(text));
    out.push('}');
}

fn format_fraction(node: &MathNode, active_variant: Option<&str>, out: &mut String) {
    let command = if node.attr("bevelled") == Some("true") {
        if node.attr("displaystyle") == Some("false") {
            "\\tsfrac"
        } else {
            "\\sfrac"
        }
    } else if node.attr("displaystyle") == Some("false") {
        "\\tfrac"
    } else if node.attr("displaystyle") == Some("true") {
        "\\dfrac"
    } else if is_binomial(node) {
        out.push('{');
        format_optional_node(node.children.first(), active_variant, out);
        out.push_str(" \\atop ");
        format_optional_node(node.children.get(1), active_variant, out);
        out.push('}');
        return;
    } else {
        "\\frac"
    };
    out.push_str(command);
    format_braced_child(node.children.first(), active_variant, out);
    format_braced_child(node.children.get(1), active_variant, out);
}

fn is_binomial(node: &MathNode) -> bool {
    matches!(node.attr("linethickness"), Some("0" | "0pt" | "none"))
}

fn format_square_root(node: &MathNode, active_variant: Option<&str>, out: &mut String) {
    out.push_str("\\sqrt{");
    format_children(node, active_variant, out);
    out.push('}');
}

fn format_indexed_root(node: &MathNode, active_variant: Option<&str>, out: &mut String) {
    out.push_str("\\sqrt[");
    format_optional_node(node.children.get(1), active_variant, out);
    out.push_str("]{");
    format_optional_node(node.children.first(), active_variant, out);
    out.push('}');
}

fn format_braced_child(node: Option<&MathNode>, active_variant: Option<&str>, out: &mut String) {
    out.push('{');
    format_optional_node(node, active_variant, out);
    out.push('}');
}

fn format_optional_node(node: Option<&MathNode>, active_variant: Option<&str>, out: &mut String) {
    if let Some(node) = node {
        format_node(node, active_variant, out);
    }
}

fn format_superscript(node: &MathNode, active_variant: Option<&str>, out: &mut String) {
    let base = node.children.first();
    let superscript = node.children.get(1);
    format_optional_wrapped_node(base, active_variant, out);
    if is_large_operator(base) {
        out.push_str("\\nolimits");
    }

    if let Some(prime) = prime_shortcut(superscript) {
        out.push_str(prime);
        return;
    }

    format_script(superscript, "^{", active_variant, out);
}

fn prime_shortcut(node: Option<&MathNode>) -> Option<&str> {
    let node = node?;
    let text = node.text.trim();
    if node.tag != "mo" {
        return None;
    }
    match text {
        "'" | "\u{2032}" => Some("'"),
        "''" | "\u{2033}" => Some("''"),
        "'''" | "\u{2034}" => Some("'''"),
        _ => None,
    }
}

fn format_subscript(node: &MathNode, active_variant: Option<&str>, out: &mut String) {
    format_optional_wrapped_node(node.children.first(), active_variant, out);
    if is_large_operator(node.children.first()) {
        out.push_str("\\nolimits");
    }
    format_script(node.children.get(1), "_{", active_variant, out);
}

fn format_subsuperscript(node: &MathNode, active_variant: Option<&str>, out: &mut String) {
    format_optional_wrapped_node(node.children.first(), active_variant, out);
    if is_large_operator(node.children.first()) {
        out.push_str("\\nolimits");
    }
    format_script(node.children.get(1), "_{", active_variant, out);
    format_script(node.children.get(2), "^{", active_variant, out);
}

fn format_multiscripts(node: &MathNode, active_variant: Option<&str>, out: &mut String) {
    if node.children.is_empty() {
        return;
    }
    let split = node
        .children
        .iter()
        .position(|n| n.tag == "mprescripts")
        .unwrap_or(node.children.len());
    let mut body = String::new();
    format_optional_wrapped_node(node.children.first(), active_variant, &mut body);
    for pair in node.children[1..split].chunks(2) {
        format_script(pair.first(), "_{", active_variant, &mut body);
        format_script(pair.get(1), "^{", active_variant, &mut body);
    }
    if split < node.children.len() {
        for pair in node.children[split + 1..].chunks(2) {
            let mut wrapped = String::from("\\prescript");
            format_braced_child(pair.get(1), active_variant, &mut wrapped);
            format_braced_child(pair.first(), active_variant, &mut wrapped);
            wrapped.push('{');
            wrapped.push_str(&body);
            wrapped.push('}');
            body = wrapped;
        }
    }
    out.push_str(&body);
}

fn format_optional_wrapped_node(
    node: Option<&MathNode>,
    active_variant: Option<&str>,
    out: &mut String,
) {
    if let Some(node) = node {
        format_wrapped(node, active_variant, out);
    }
}

fn format_script(
    node: Option<&MathNode>,
    prefix: &str,
    active_variant: Option<&str>,
    out: &mut String,
) {
    trim_trailing_whitespace(out);
    out.push_str(prefix);
    format_optional_node(node, active_variant, out);
    out.push('}');
}

fn format_underscript(node: &MathNode, active_variant: Option<&str>, out: &mut String) {
    let base = node.children.first();
    let underscript = node.children.get(1);
    if let Some(command) = underscript
        .filter(|n| n.tag == "mo" && !is_large_operator(base))
        .and_then(|n| map_under_accent(&n.text))
    {
        out.push_str(command);
        format_braced_child(base, active_variant, out);
    } else if node.attr("accentunder") == Some("true") {
        format_over_or_under_set("\\underaccent", underscript, base, active_variant, out);
    } else if is_large_operator(base) {
        format_optional_node(base, active_variant, out);
        if node.attr("data-limloc") == Some("subSup") {
            out.push_str("\\nolimits");
        }
        format_script(underscript, "_{", active_variant, out);
    } else {
        format_over_or_under_set("\\underset", underscript, base, active_variant, out);
    }
}

fn format_overscript(node: &MathNode, active_variant: Option<&str>, out: &mut String) {
    let base = node.children.first();
    let overscript = node.children.get(1);
    if let Some(accent) = overscript
        .filter(|n| n.tag == "mo" && !is_large_operator(base))
        .and_then(|node| map_accent(&node.text))
    {
        out.push_str(accent);
        format_braced_child(base, active_variant, out);
    } else if node.attr("accent") == Some("true") {
        format_over_or_under_set("\\overaccent", overscript, base, active_variant, out);
    } else if is_large_operator(base) {
        format_optional_node(base, active_variant, out);
        if node.attr("data-limloc") == Some("subSup") {
            out.push_str("\\nolimits");
        }
        format_script(overscript, "^{", active_variant, out);
    } else {
        format_over_or_under_set("\\overset", overscript, base, active_variant, out);
    }
}

fn format_underoverscript(node: &MathNode, active_variant: Option<&str>, out: &mut String) {
    if !is_large_operator(node.children.first()) {
        out.push_str("\\overset");
        format_braced_child(node.children.get(2), active_variant, out);
        out.push_str("{\\underset");
        format_braced_child(node.children.get(1), active_variant, out);
        format_braced_child(node.children.first(), active_variant, out);
        out.push('}');
        return;
    }
    format_optional_node(node.children.first(), active_variant, out);
    if node.attr("data-limloc") == Some("subSup") {
        out.push_str("\\nolimits");
    }
    format_script(node.children.get(1), "_{", active_variant, out);
    format_script(node.children.get(2), "^{", active_variant, out);
}

fn format_over_or_under_set(
    command: &str,
    annotation: Option<&MathNode>,
    base: Option<&MathNode>,
    active_variant: Option<&str>,
    out: &mut String,
) {
    out.push_str(command);
    format_braced_child(annotation, active_variant, out);
    format_braced_child(base, active_variant, out);
}

fn format_fenced(node: &MathNode, active_variant: Option<&str>, out: &mut String) {
    let open_raw = node.attr("open").unwrap_or("(").trim();
    let close_raw = node.attr("close").unwrap_or(")").trim();
    if open_raw == "("
        && close_raw == ")"
        && node.children.len() == 1
        && node.children[0].tag == "mfrac"
        && is_binomial(&node.children[0])
        && node.children[0].attr("bevelled") != Some("true")
    {
        format_binom(&node.children[0], active_variant, out);
        return;
    }
    let open = map_fence_delimiter(open_raw, true);
    let close = map_fence_delimiter(close_raw, false);
    out.push_str(&open);
    for (index, child) in node.children.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        format_node(child, active_variant, out);
    }
    out.push_str(&close);
}

fn format_matrix(node: &MathNode, active_variant: Option<&str>, out: &mut String) {
    let align = node.attr("columnalign");
    if let Some(align) = align {
        let cols = node
            .children
            .iter()
            .map(|r| r.children.len())
            .max()
            .unwrap_or(1);
        let values: Vec<_> = align.split_whitespace().collect();
        out.push_str("\\begin{array}{");
        for i in 0..cols {
            out.push(
                match values
                    .get(i)
                    .or_else(|| values.last())
                    .copied()
                    .unwrap_or("center")
                {
                    "left" => 'l',
                    "right" => 'r',
                    _ => 'c',
                },
            );
        }
        out.push_str("} ");
    } else {
        out.push_str("\\begin{matrix} ");
    }
    for (row_index, row) in node.children.iter().enumerate() {
        if row_index > 0 {
            out.push_str(" \\\\ ");
        }
        format_matrix_row(row, active_variant, out);
    }
    out.push_str(if align.is_some() {
        " \\end{array}"
    } else {
        " \\end{matrix}"
    });
}

fn format_matrix_row(row: &MathNode, active_variant: Option<&str>, out: &mut String) {
    for (cell_index, cell) in row.children.iter().enumerate() {
        if cell_index > 0 {
            out.push_str(" & ");
        }
        format_node(cell, active_variant, out);
    }
}

fn format_enclosure(node: &MathNode, active_variant: Option<&str>, out: &mut String) {
    let notation = node.attr("notation").unwrap_or("box");
    match notation {
        "box" => out.push_str("\\boxed{"),
        "updiagonalstrike" => out.push_str("\\cancel{"),
        "downdiagonalstrike" => out.push_str("\\bcancel{"),
        "updiagonalstrike downdiagonalstrike" => out.push_str("\\xcancel{"),
        _ => {
            out.push_str("\\enclose{");
            out.push_str(notation);
            out.push_str("}{");
        }
    }
    format_children(node, active_variant, out);
    out.push('}');
}

fn trim_trailing_whitespace(s: &mut String) {
    while s.ends_with(' ') {
        s.pop();
    }
}

fn format_wrapped(node: &MathNode, active_variant: Option<&str>, out: &mut String) {
    if node.children.len() > 1 || node.tag == "mrow" {
        out.push('{');
        format_node(node, active_variant, out);
        out.push('}');
    } else {
        format_node(node, active_variant, out);
    }
}

fn is_invisible_operator(s: &str) -> bool {
    matches!(s, "\u{2061}" | "\u{2062}" | "\u{2063}" | "\u{2064}")
}

fn is_large_operator(node: Option<&MathNode>) -> bool {
    let Some(mut n) = node else { return false };
    while matches!(n.tag.as_str(), "mstyle" | "mrow") && n.children.len() == 1 {
        n = &n.children[0];
    }
    let t = n.text.trim();
    matches!(
        t,
        "\\sum"
            | "\\prod"
            | "\\coprod"
            | "\\int"
            | "\\iint"
            | "\\iiint"
            | "\\oint"
            | "\\lim"
            | "\\min"
            | "\\max"
            | "\\inf"
            | "\\sup"
            | "lim"
            | "sum"
            | "∑"
            | "∏"
            | "∫"
            | "∮"
    ) || (n.tag == "mo"
        && t.chars()
            .next()
            .is_some_and(|c| crate::symbols::is_big_op(c) || crate::symbols::is_integral(c)))
        || (n.tag == "mi" && matches!(t, "lim" | "min" | "max" | "inf" | "sup"))
}

fn map_accent(s: &str) -> Option<&'static str> {
    match s.trim() {
        "^" | "\u{02C6}" => Some("\\hat"),
        "~" | "\u{02DC}" => Some("\\tilde"),
        "¯" | "-" | "_" => Some("\\overline"),
        "→" => Some("\\vec"),
        "." | "\u{02D9}" => Some("\\dot"),
        ".." | "\u{00A8}" => Some("\\ddot"),
        "°" | "\u{02DA}" | "\u{030A}" => Some("\\mathring"),
        "⃛" => Some("\\dddot"),
        "⃜" => Some("\\ddddot"),
        "←" => Some("\\overleftarrow"),
        "↔" => Some("\\overleftrightarrow"),
        "⇀" => Some("\\overrightharpoon"),
        "↼" => Some("\\overleftharpoon"),
        "ˇ" | "\u{030C}" => Some("\\check"),
        "˘" => Some("\\breve"),
        "´" => Some("\\acute"),
        "`" => Some("\\grave"),
        "⏜" | "⌒" => Some("\\overparen"),
        "⏞" => Some("\\overbrace"),
        "⎴" => Some("\\overbracket"),
        _ => None,
    }
}

fn map_under_accent(s: &str) -> Option<&'static str> {
    Some(match s.trim() {
        "¯" | "_" | "―" | "̲" => "\\underline",
        "." | "˙" => "\\underdot",
        "¨" => "\\underddot",
        "⃛" => "\\underdddot",
        "⃜" => "\\underddddot",
        "^" | "ˆ" => "\\underhat",
        "~" | "˜" => "\\undertilde",
        "ˇ" | "\u{030C}" => "\\undercheck",
        "˘" => "\\underbreve",
        "°" | "\u{02DA}" | "\u{030A}" => "\\underring",
        "→" => "\\underrightarrow",
        "←" => "\\underleftarrow",
        "↔" => "\\underleftrightarrow",
        "⇀" | "⇁" => "\\underrightharpoon",
        "↼" | "↽" => "\\underleftharpoon",
        "⏝" | "⌣" => "\\underparen",
        "⏟" => "\\underbrace",
        "⎵" => "\\underbracket",
        _ => return None,
    })
}

fn map_fence_delimiter(delim: &str, is_open: bool) -> String {
    let d = delim.trim();
    if d.is_empty() {
        return if is_open {
            "\\left.".into()
        } else {
            "\\right.".into()
        };
    }
    let p = match d {
        "{" => "\\{",
        "}" => "\\}",
        "|" => "|",
        "[" => "[",
        "]" => "]",
        "(" => "(",
        ")" => ")",
        "‖" | "∥" => "\\Vert",
        "⟨" | "〈" => "\\langle",
        "⟩" | "〉" => "\\rangle",
        "⌊" => "\\lfloor",
        "⌋" => "\\rfloor",
        "⌈" => "\\lceil",
        "⌉" => "\\rceil",
        "⟦" | "〚" => "\\llbracket",
        "⟧" | "〛" => "\\rrbracket",
        "⦗" => "\\lgroup",
        "⦘" => "\\rgroup",
        _ => d,
    };
    if is_open {
        format!("\\left{p} ")
    } else {
        format!(" \\right{p}")
    }
}

fn map_operator(op: &str) -> Option<&'static str> {
    match op {
        "±" => Some(" \\pm "),
        "∓" => Some(" \\mp "),
        "×" => Some(" \\times "),
        "÷" => Some(" \\div "),
        "⋅" => Some(" \\cdot "),
        "≤" => Some(" \\leq "),
        "≥" => Some(" \\geq "),
        "≠" => Some(" \\neq "),
        "≈" => Some(" \\approx "),
        "≡" => Some(" \\equiv "),
        "∈" => Some(" \\in "),
        "∉" => Some(" \\notin "),
        "⊂" => Some(" \\subset "),
        "⊃" => Some(" \\supset "),
        "⊆" => Some(" \\subseteq "),
        "⊇" => Some(" \\supseteq "),
        "∪" => Some(" \\cup "),
        "∩" => Some(" \\cap "),
        "∖" | "\\" => Some(" \\setminus "),
        "∞" => Some("\\infty "),
        "∂" => Some("\\partial "),
        "∇" => Some("\\nabla "),
        "→" => Some(" \\rightarrow "),
        "←" => Some(" \\leftarrow "),
        "↔" => Some(" \\leftrightarrow "),
        "⇒" => Some(" \\Rightarrow "),
        "⇐" => Some(" \\Leftarrow "),
        "⇔" => Some(" \\Leftrightarrow "),
        "∑" => Some("\\sum "),
        "∏" => Some("\\prod "),
        "∫" => Some("\\int "),
        "∬" => Some("\\iint "),
        "∭" => Some("\\iiint "),
        "∮" => Some("\\oint "),
        "∴" => Some(" \\therefore "),
        "∵" => Some(" \\because "),
        "∼" => Some(" \\sim "),
        "≅" => Some(" \\cong "),
        "√" => Some("\\surd "),
        "∀" => Some("\\forall "),
        "∃" => Some("\\exists "),
        "∄" => Some("\\nexists "),
        "¬" => Some("\\neg "),
        "…" => Some("\\ldots "),
        "⋯" => Some("\\cdots "),
        "⋮" => Some("\\vdots "),
        "⋱" => Some("\\ddots "),
        "′" => Some("'"),
        "″" => Some("''"),
        "‴" => Some("'''"),
        "⁗" => Some("^{(4)}"),
        "°" => Some("^\\circ"),
        "∠" => Some(" \\angle "),
        "⊥" => Some(" \\perp "),
        "∥" => Some(" \\parallel "),
        "{" => Some("\\{"),
        "}" => Some("\\}"),
        "%" => Some("\\%"),
        "&" => Some("\\&"),
        "_" => Some("\\_"),
        _ => None,
    }
}

fn map_greek_or_symbol(s: &str) -> Option<&'static str> {
    match s {
        "α" | "alpha" => Some("\\alpha"),
        "β" | "beta" => Some("\\beta"),
        "γ" | "gamma" => Some("\\gamma"),
        "δ" | "delta" => Some("\\delta"),
        "ε" | "varepsilon" => Some("\\varepsilon"),
        "ϵ" | "epsilon" => Some("\\epsilon"),
        "ζ" | "zeta" => Some("\\zeta"),
        "η" | "eta" => Some("\\eta"),
        "θ" | "theta" => Some("\\theta"),
        "ϑ" | "vartheta" => Some("\\vartheta"),
        "ι" | "iota" => Some("\\iota"),
        "κ" | "kappa" => Some("\\kappa"),
        "ϰ" | "varkappa" => Some("\\varkappa"),
        "λ" | "lambda" => Some("\\lambda"),
        "μ" | "mu" => Some("\\mu"),
        "ν" | "nu" => Some("\\nu"),
        "ξ" | "xi" => Some("\\xi"),
        "ο" | "omicron" => Some("o"),
        "π" | "pi" => Some("\\pi"),
        "ϖ" | "varpi" => Some("\\varpi"),
        "ρ" | "rho" => Some("\\rho"),
        "ϱ" | "varrho" => Some("\\varrho"),
        "σ" | "sigma" => Some("\\sigma"),
        "ς" | "varsigma" => Some("\\varsigma"),
        "τ" | "tau" => Some("\\tau"),
        "υ" | "upsilon" => Some("\\upsilon"),
        "φ" | "varphi" => Some("\\varphi"),
        "ϕ" | "phi" => Some("\\phi"),
        "χ" | "chi" => Some("\\chi"),
        "ψ" | "psi" => Some("\\psi"),
        "ω" | "omega" => Some("\\omega"),

        "Α" | "Alpha" => Some("A"),
        "Β" | "Beta" => Some("B"),
        "Γ" | "Gamma" => Some("\\Gamma"),
        "Δ" | "Delta" => Some("\\Delta"),
        "Ε" | "Epsilon" => Some("E"),
        "Ζ" | "Zeta" => Some("Z"),
        "Η" | "Eta" => Some("H"),
        "Θ" | "Theta" => Some("\\Theta"),
        "Ι" | "Iota" => Some("I"),
        "Κ" | "Kappa" => Some("K"),
        "Λ" | "Lambda" => Some("\\Lambda"),
        "Μ" | "Mu" => Some("M"),
        "Ν" | "Nu" => Some("N"),
        "Ξ" | "Xi" => Some("\\Xi"),
        "Ο" | "Omicron" => Some("O"),
        "Π" | "Pi" => Some("\\Pi"),
        "Ρ" | "Rho" => Some("P"),
        "Σ" | "Sigma" => Some("\\Sigma"),
        "Τ" | "Tau" => Some("T"),
        "Υ" | "Upsilon" => Some("\\Upsilon"),
        "Φ" | "Phi" => Some("\\Phi"),
        "Χ" | "Chi" => Some("X"),
        "Ψ" | "Psi" => Some("\\Psi"),
        "Ω" | "Omega" => Some("\\Omega"),

        "∞" | "infty" => Some("\\infty"),
        _ => None,
    }
}

fn format_named_function(s: &str) -> Option<&'static str> {
    Some(match s {
        "sin" => "\\sin ",
        "cos" => "\\cos ",
        "tan" => "\\tan ",
        "cot" => "\\cot ",
        "sec" => "\\sec ",
        "csc" => "\\csc ",
        "arcsin" => "\\arcsin ",
        "arccos" => "\\arccos ",
        "arctan" => "\\arctan ",
        "sinh" => "\\sinh ",
        "cosh" => "\\cosh ",
        "tanh" => "\\tanh ",
        "coth" => "\\coth ",
        "ln" => "\\ln ",
        "log" => "\\log ",
        "lg" => "\\lg ",
        "exp" => "\\exp ",
        "lim" => "\\lim ",
        "liminf" | "lim inf" => "\\liminf ",
        "limsup" | "lim sup" => "\\limsup ",
        "min" => "\\min ",
        "max" => "\\max ",
        "inf" => "\\inf ",
        "sup" => "\\sup ",
        "det" => "\\det ",
        "dim" => "\\dim ",
        "gcd" => "\\gcd ",
        "hom" => "\\hom ",
        "ker" => "\\ker ",
        "deg" => "\\deg ",
        "arg" => "\\arg ",
        "Pr" => "\\Pr ",
        _ => return None,
    })
}

fn is_operatorname_function(s: &str) -> bool {
    matches!(
        s,
        "arccot"
            | "arcsec"
            | "arccsc"
            | "sech"
            | "csch"
            | "arsinh"
            | "arcosh"
            | "artanh"
            | "arcoth"
            | "tg"
            | "ctg"
            | "cotg"
            | "cosec"
            | "lcm"
            | "mod"
            | "sgn"
    )
}

fn is_named_function(s: &str) -> bool {
    format_named_function(s).is_some() || is_operatorname_function(s)
}

fn escape_latex_text(s: &str) -> String {
    let mut res = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '{' => res.push_str("\\{"),
            '}' => res.push_str("\\}"),
            '%' => res.push_str("\\%"),
            '&' => res.push_str("\\&"),
            '$' => res.push_str("\\$"),
            '#' => res.push_str("\\#"),
            '_' => res.push_str("\\_"),
            '\\' => res.push_str("\\textbackslash{}"),
            '~' => res.push_str("\\textasciitilde{}"),
            '^' => res.push_str("\\textasciicircum{}"),
            '\u{00A0}' => res.push(' '),
            _ => res.push(c),
        }
    }
    res
}

fn clean_latex_output(s: &str) -> String {
    // Collapse math-mode whitespace while copying text arguments verbatim.
    const TEXT_START: &str = r"\text{";
    let mut result = String::with_capacity(s.len());
    let mut prev_space = false;
    let mut chars = s.char_indices();
    let mut text_depth = 0;
    let mut escaped = false;
    while let Some((offset, c)) = chars.next() {
        if text_depth > 0 {
            result.push(c);
            if !escaped {
                match c {
                    '{' => text_depth += 1,
                    '}' => text_depth -= 1,
                    _ => {}
                }
            }
        } else if !escaped && c == '\\' && s[offset..].starts_with(TEXT_START) {
            result.push_str(TEXT_START);
            for _ in 1..TEXT_START.len() {
                chars.next();
            }
            text_depth = 1;
            prev_space = false;
            continue;
        } else if c.is_whitespace() {
            if !prev_space {
                result.push(' ');
                prev_space = true;
            }
        } else {
            result.push(c);
            prev_space = false;
        }
        escaped = !escaped && c == '\\';
    }
    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn serialize(mathml: &str) -> String {
        let node = MathNode::parse(mathml).expect("test MathML is valid");
        mathml_to_latex(&node)
    }

    #[test]
    fn test_fraction_to_latex() {
        let node = MathNode::with_children(
            "mfrac",
            vec![MathNode::leaf("mi", "x"), MathNode::leaf("mn", "2")],
        );
        assert_eq!(mathml_to_latex(&node), "\\frac{x}{2}");
    }

    #[test]
    fn test_quadratic_formula_to_latex() {
        // x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}
        let sqrt_content = MathNode::with_children(
            "mrow",
            vec![
                MathNode::with_children(
                    "msup",
                    vec![MathNode::leaf("mi", "b"), MathNode::leaf("mn", "2")],
                ),
                MathNode::leaf("mo", "-"),
                MathNode::leaf("mn", "4"),
                MathNode::leaf("mi", "a"),
                MathNode::leaf("mi", "c"),
            ],
        );

        let num = MathNode::with_children(
            "mrow",
            vec![
                MathNode::leaf("mo", "-"),
                MathNode::leaf("mi", "b"),
                MathNode::leaf("mo", "±"),
                MathNode::with_children("msqrt", vec![sqrt_content]),
            ],
        );

        let den = MathNode::with_children(
            "mrow",
            vec![MathNode::leaf("mn", "2"), MathNode::leaf("mi", "a")],
        );

        let frac = MathNode::with_children("mfrac", vec![num, den]);
        let eq = MathNode::with_children(
            "mrow",
            vec![MathNode::leaf("mi", "x"), MathNode::leaf("mo", "="), frac],
        );

        let tex = mathml_to_latex(&eq);
        assert_eq!(tex, "x=\\frac{-b \\pm \\sqrt{b^{2}-4ac}}{2a}");
    }

    #[test]
    fn test_greek_and_sub_sup() {
        let node = MathNode::with_children(
            "msubsup",
            vec![
                MathNode::leaf("mi", "α"),
                MathNode::leaf("mn", "1"),
                MathNode::leaf("mn", "2"),
            ],
        );
        let tex = mathml_to_latex(&node);
        assert_eq!(tex, "\\alpha_{1}^{2}");
    }

    #[test]
    fn serializes_named_functions_text_and_invisible_operators() {
        let latex = serialize(
            "<math><mi>sin</mi><mo>\u{2062}</mo><mi>x</mi><mtext>price_&amp;{tax}</mtext></math>",
        );

        assert_eq!(latex, "\\sin x\\text{price\\_\\&\\{tax\\}}");
    }

    #[test]
    fn text_serialization_roundtrips_spaces_and_escapes() {
        for (text, expected_latex, expected_text) in [
            (" ", r"\text{ }", " "),
            ("\u{00A0}\u{00A0}", r"\text{  }", "  "),
            (" với  m\u{00A0}", r"\text{ với  m }", " với  m "),
            (
                r" \ {x}_$&%#~^ ",
                r"\text{ \textbackslash{} \{x\}\_\$\&\%\#\textasciitilde{}\textasciicircum{} }",
                r" \ {x}_$&%#~^ ",
            ),
        ] {
            let latex = MathNode::leaf("mtext", text).to_latex();
            assert_eq!(latex, expected_latex);
            let math = crate::service::parse_latex_to_mathml(&latex).unwrap();
            assert_eq!(math.children, [MathNode::leaf("mtext", expected_text)]);
        }
    }

    #[test]
    fn serializes_binomials_and_indexed_roots() {
        let bare_atop = serialize("<mfrac linethickness=\"0\"><mi>n</mi><mi>k</mi></mfrac>");
        let binom_mrow = serialize(
            "<math><mrow><mo stretchy=\"true\">(</mo><mfrac linethickness=\"0\"><mi>n</mi><mi>k</mi></mfrac><mo stretchy=\"true\">)</mo></mrow></math>",
        );
        let binom_fenced = serialize(
            "<mfenced open=\"(\" close=\")\"><mfrac linethickness=\"0\"><mi>n</mi><mi>k</mi></mfrac></mfenced>",
        );
        let indexed_root = serialize("<mroot><mi>x</mi><mn>3</mn></mroot>");

        assert_eq!(bare_atop, "{n \\atop k}");
        assert_eq!(binom_mrow, "\\binom{n}{k}");
        assert_eq!(binom_fenced, "\\binom{n}{k}");
        assert_eq!(indexed_root, "\\sqrt[3]{x}");
    }

    #[test]
    fn serializes_fences_and_matrices_with_current_spacing() {
        let fenced = serialize("<mfenced open=\"{\" close=\"}\"><mi>x</mi><mi>y</mi></mfenced>");
        let matrix = serialize(
            "<mtable><mtr><mtd><mi>a</mi></mtd><mtd><mi>b</mi></mtd></mtr><mtr><mtd><mi>c</mi></mtd><mtd><mi>d</mi></mtd></mtr></mtable>",
        );

        assert_eq!(fenced, "\\left\\{ x, y \\right\\}");
        assert_eq!(matrix, "\\begin{matrix} a & b \\\\ c & d \\end{matrix}");
    }

    #[test]
    fn serializes_accents_large_operator_limits_and_enclosures() {
        let accent = serialize("<mover><mi>x</mi><mo>^</mo></mover>");
        let limits = serialize("<munderover><mo>∑</mo><mi>i</mi><mi>n</mi></munderover>");
        let enclosure =
            serialize("<menclose notation=\"box\"><mi>x</mi><mo>+</mo><mi>y</mi></menclose>");

        assert_eq!(accent, "\\hat{x}");
        assert_eq!(limits, "\\sum_{i}^{n}");
        assert_eq!(enclosure, "\\boxed{x+y}");
    }

    #[test]
    fn serializes_explicit_operators_setminus_degree_and_primes() {
        assert_eq!(
            serialize("<math><mi>A</mi><mo>∖</mo><mi>B</mi></math>"),
            "A \\setminus B"
        );
        assert_eq!(
            serialize("<math><mi mathvariant=\"double-struck\">R</mi><mo>\\</mo><mrow><mo>{</mo><mo>-</mo><mn>1</mn><mo>}</mo></mrow></math>"),
            "\\mathbb{R} \\setminus \\{-1\\}"
        );
        assert_eq!(
            serialize("<math><mi>x</mi><mo>≤</mo><mi>y</mi><mo>≥</mo><mi>z</mi><mo>≠</mo><mn>0</mn></math>"),
            "x \\leq y \\geq z \\neq 0"
        );
        assert_eq!(
            serialize("<math><mi>A</mi><mo>→</mo><mi>B</mi><mo>←</mo><mi>C</mi><mo>⇒</mo><mi>D</mi><mo>⇔</mo><mi>E</mi></math>"),
            "A \\rightarrow B \\leftarrow C \\Rightarrow D \\Leftrightarrow E"
        );
        assert_eq!(
            serialize("<math><mi>a</mi><mo>,</mo><mo>…</mo><mo>,</mo><mi>b</mi></math>"),
            "a,\\ldots ,b"
        );
        assert_eq!(serialize("<math><mn>90</mn><mo>°</mo></math>"), "90^\\circ");
        assert_eq!(serialize("<math><mi>f</mi><mo>⁗</mo></math>"), "f^{(4)}");
        assert_eq!(
            serialize("<math><mi>A</mi><mo>⊕</mo><mi>B</mi><mo>∉</mo><mi>S</mi></math>"),
            "A \\oplus B \\notin S"
        );
    }

    #[test]
    fn serializes_spaces_styles_greek_variants_and_functions() {
        assert_eq!(
            serialize("<math><mi>x</mi><mspace width=\"0.166667em\"/><mi>y</mi><mspace width=\"0.222222em\"/><mi>z</mi><mspace width=\"0.277778em\"/><mi>w</mi><mspace width=\"-0.166667em\"/><mi>u</mi><mspace width=\"0.5em\"/><mi>v</mi><mspace width=\"1em\"/><mi>t</mi><mspace width=\"2em\"/><mi>s</mi><mspace width=\"0.083333em\"/><mi>q</mi></math>"),
            "x\\,y\\:z\\;w\\!u\\enspace v\\quad t\\qquad s\\hspace{0.083333em}q"
        );
        assert_eq!(
            serialize("<math><mstyle displaystyle=\"true\"><mfrac><mi>a</mi><mi>b</mi></mfrac></mstyle><mstyle displaystyle=\"false\"><mi>x</mi></mstyle></math>"),
            "{\\displaystyle \\frac{a}{b}}{\\textstyle x}"
        );
        assert_eq!(
            serialize("<math><mi>ϵ</mi><mi>ε</mi><mi>ϕ</mi><mi>φ</mi><mi>θ</mi><mi>ϑ</mi><mi>π</mi><mi>ϖ</mi><mi>ρ</mi><mi>ϱ</mi><mi>σ</mi><mi>ς</mi><mi>ϰ</mi><mi>x</mi></math>"),
            "\\epsilon\\varepsilon\\phi\\varphi\\theta\\vartheta\\pi\\varpi\\rho\\varrho\\sigma\\varsigma\\varkappa x"
        );
        assert_eq!(
            serialize("<math><mi>Α</mi><mi>Β</mi><mi>ο</mi></math>"),
            "ABo"
        );
        assert_eq!(
            serialize("<math><mi>limsup</mi><mi>x</mi><mi>lim inf</mi><mi>y</mi><mi>sgn</mi><mi>z</mi><mi mathvariant=\"normal\">lcm</mi><mo>(</mo><mi>a</mi><mo>,</mo><mi>b</mi><mo>)</mo></math>"),
            "\\limsup x\\liminf y\\operatorname{sgn} z\\operatorname{lcm} (a,b)"
        );
    }

    #[test]
    fn groups_consecutive_and_subtree_style_runs_without_repeating_commands() {
        for (input, expected) in [
            (r"\mathbf{x^2 + 2x - 1 = 0}", r"\mathbf{x^{2}+2x-1=0}"),
            (
                r"\boldsymbol{x^2 + 2x - 1 = 0}",
                r"\boldsymbol{x^{2}+2x-1=0}",
            ),
            (r"\mathrm{H_2O + m/s^2}", r"\mathrm{H_{2}O+m/s^{2}}"),
            (r"\mathit{x^2 + 2x}", r"\mathit{x^{2}+2x}"),
            (r"\mathbb{RZ}", r"\mathbb{RZ}"),
            (r"\mathfrak{Ag}", r"\mathfrak{Ag}"),
            (r"\mathcal{AL}", r"\mathcal{AL}"),
            (r"\mathscr{Aa}", r"\mathscr{Aa}"),
            (r"\mathbf{\frac{a}{b}}^{2}", r"{\mathbf{\frac{a}{b}}}^{2}"),
            (r"\mathbf{\frac{a}{b}^{2}}", r"\mathbf{{\frac{a}{b}}^{2}}"),
            (r"\mathbf{x} + \mathbf{y}", r"\mathbf{x}+\mathbf{y}"),
            (r"\mathbb{R}^2", r"\mathbb{R}^{2}"),
            (
                r"\mathbf{x}\text{ and }\mathbf{y}",
                r"\mathbf{x}\text{ and }\mathbf{y}",
            ),
            (r"\operatorname{sgn} x", r"\operatorname{sgn} x"),
            (r"\operatorname{argmax} x", r"\mathrm{argmax}x"),
            (r"\mathbf{\alpha x + \sin y}", r"\mathbf{\alpha x+\sin y}"),
            (
                r"\mathbf{\sum_{i=1}^n x_i}",
                r"\mathbf{\sum_{i=1}^{n}x_{i}}",
            ),
            (r"\mathbf{\boxed{x+y}}", r"\boxed{\mathbf{x+y}}"),
            (r"\text{hello world}", r"\text{hello world}"),
            (r"\cancel{x+y}", r"\cancel{x+y}"),
            (r"\bcancel{x+y}", r"\bcancel{x+y}"),
            (r"\xcancel{x+y}", r"\xcancel{x+y}"),
            (r"\boxed{x+y}", r"\boxed{x+y}"),
            (r"\underset{a}{b}", r"\underset{a}{b}"),
            (r"\overset{a}{b}", r"\overset{a}{b}"),
            (r"\prescript{a}{b}{c}", r"\prescript{a}{b}{c}"),
            (r"\textstyle{\frac{a}{b}}", r"{\textstyle \frac{a}{b}}"),
            (
                r"{\displaystyle \frac{a}{b}}",
                r"{\displaystyle \frac{a}{b}}",
            ),
            (r"\enclose{roundedbox}{x+y}", r"\enclose{roundedbox}{x+y}"),
            (r"\hspace{12pt}", r"\hspace{12pt}"),
        ] {
            let math = crate::service::parse_latex_to_mathml(input)
                .unwrap_or_else(|e| panic!("{input}: {e}"));
            let actual = math.to_latex();
            assert_eq!(actual, expected, "input: {input}");
        }
    }
}
