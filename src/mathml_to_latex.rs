//! Serializer from [`MathNode`] (MathML AST) to standard LaTeX markup.
//!
//! Used by the bridge server and desktop editor to populate MathLive / LaTeX
//! editors when opening sessions or importing equations from OMML / MathType.

use crate::mathml::MathNode;

/// Convert a [`MathNode`] tree into a LaTeX mathematical expression string.
pub fn mathml_to_latex(node: &MathNode) -> String {
    let mut out = String::new();
    format_node(node, &mut out);
    clean_latex_output(&out)
}

fn format_node(node: &MathNode, out: &mut String) {
    match node.tag.as_str() {
        "math" | "mrow" | "mstyle" => {
            for child in &node.children {
                format_node(child, out);
            }
        }
        "mi" => {
            let t = node.text.trim();
            if let Some(greek) = map_greek_or_symbol(t) {
                out.push_str(greek);
            } else if is_named_function(t) {
                out.push('\\');
                out.push_str(t);
                out.push(' ');
            } else if t.len() > 1 && t.chars().all(|c| c.is_alphabetic()) {
                out.push_str("\\mathrm{");
                out.push_str(t);
                out.push('}');
            } else {
                out.push_str(t);
            }
        }
        "mn" => {
            out.push_str(node.text.trim());
        }
        "mo" => {
            let t = node.text.trim();
            if is_invisible_operator(t) {
                // Invisible times/comma/plus/apply: omit
                return;
            }
            if let Some(op) = map_operator(t) {
                out.push_str(op);
            } else {
                out.push_str(t);
            }
        }
        "mtext" => {
            let t = node.text.trim();
            if !t.is_empty() {
                out.push_str("\\text{");
                out.push_str(&escape_latex_text(t));
                out.push('}');
            }
        }
        "mfrac" => {
            let num = node.children.first();
            let den = node.children.get(1);

            let is_binom = node
                .attr("linethickness")
                .map(|v| v == "0" || v == "0pt" || v == "none")
                .unwrap_or(false);

            if is_binom {
                out.push_str("\\binom{");
                if let Some(n) = num {
                    format_node(n, out);
                }
                out.push_str("}{");
                if let Some(d) = den {
                    format_node(d, out);
                }
                out.push('}');
            } else {
                out.push_str("\\frac{");
                if let Some(n) = num {
                    format_node(n, out);
                }
                out.push_str("}{");
                if let Some(d) = den {
                    format_node(d, out);
                }
                out.push('}');
            }
        }
        "msqrt" => {
            out.push_str("\\sqrt{");
            for child in &node.children {
                format_node(child, out);
            }
            out.push('}');
        }
        "mroot" => {
            let base = node.children.first();
            let index = node.children.get(1);
            out.push_str("\\sqrt[");
            if let Some(idx) = index {
                format_node(idx, out);
            }
            out.push_str("]{");
            if let Some(b) = base {
                format_node(b, out);
            }
            out.push('}');
        }
        "msup" => {
            let base = node.children.first();
            let sup = node.children.get(1);

            if let Some(b) = base {
                format_wrapped(b, out);
            }

            // Prime shortcut: x' or x'' instead of x^{\prime}
            if let Some(s) = sup {
                let st = s.text.trim();
                if s.tag == "mo" && (st == "'" || st == "\u{2032}" || st == "\u{2033}" || st == "′" || st == "″") {
                    out.push_str(st);
                    return;
                }
            }

            trim_trailing_whitespace(out);
            out.push_str("^{");
            if let Some(s) = sup {
                format_node(s, out);
            }
            out.push('}');
        }
        "msub" => {
            let base = node.children.first();
            let sub = node.children.get(1);

            if let Some(b) = base {
                format_wrapped(b, out);
            }
            trim_trailing_whitespace(out);
            out.push_str("_{");
            if let Some(s) = sub {
                format_node(s, out);
            }
            out.push('}');
        }
        "msubsup" => {
            let base = node.children.first();
            let sub = node.children.get(1);
            let sup = node.children.get(2);

            if let Some(b) = base {
                format_wrapped(b, out);
            }
            trim_trailing_whitespace(out);
            out.push_str("_{");
            if let Some(s) = sub {
                format_node(s, out);
            }
            out.push_str("}^{");
            if let Some(s) = sup {
                format_node(s, out);
            }
            out.push('}');
        }
        "munder" => {
            let base = node.children.first();
            let under = node.children.get(1);

            if is_large_operator(base) {
                if let Some(b) = base {
                    format_node(b, out);
                }
                trim_trailing_whitespace(out);
                out.push_str("_{");
                if let Some(u) = under {
                    format_node(u, out);
                }
                out.push('}');
            } else {
                out.push_str("\\underset{");
                if let Some(u) = under {
                    format_node(u, out);
                }
                out.push_str("}{");
                if let Some(b) = base {
                    format_node(b, out);
                }
                out.push('}');
            }
        }
        "mover" => {
            let base = node.children.first();
            let over = node.children.get(1);

            let accent_cmd = over.and_then(|o| map_accent(&o.text));
            if let Some(cmd) = accent_cmd {
                out.push_str(cmd);
                out.push('{');
                if let Some(b) = base {
                    format_node(b, out);
                }
                out.push('}');
            } else if is_large_operator(base) {
                if let Some(b) = base {
                    format_node(b, out);
                }
                trim_trailing_whitespace(out);
                out.push_str("^{");
                if let Some(o) = over {
                    format_node(o, out);
                }
                out.push('}');
            } else {
                out.push_str("\\overset{");
                if let Some(o) = over {
                    format_node(o, out);
                }
                out.push_str("}{");
                if let Some(b) = base {
                    format_node(b, out);
                }
                out.push('}');
            }
        }
        "munderover" => {
            let base = node.children.first();
            let under = node.children.get(1);
            let over = node.children.get(2);

            if let Some(b) = base {
                format_node(b, out);
            }
            trim_trailing_whitespace(out);
            out.push_str("_{");
            if let Some(u) = under {
                format_node(u, out);
            }
            out.push_str("}^{");
            if let Some(o) = over {
                format_node(o, out);
            }
            out.push('}');
        }
        "mfenced" => {
            let open = node.attr("open").unwrap_or("(");
            let close = node.attr("close").unwrap_or(")");
            let open_tex = map_fence_delimiter(open, true);
            let close_tex = map_fence_delimiter(close, false);

            out.push_str(&open_tex);
            for (i, child) in node.children.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                format_node(child, out);
            }
            out.push_str(&close_tex);
        }
        "mtable" => {
            out.push_str("\\begin{matrix} ");
            for (r_idx, row) in node.children.iter().enumerate() {
                if r_idx > 0 {
                    out.push_str(" \\\\ ");
                }
                for (c_idx, cell) in row.children.iter().enumerate() {
                    if c_idx > 0 {
                        out.push_str(" & ");
                    }
                    format_node(cell, out);
                }
            }
            out.push_str(" \\end{matrix}");
        }
        "mtr" | "mtd" => {
            for child in &node.children {
                format_node(child, out);
            }
        }
        "menclose" => {
            let notation = node.attr("notation").unwrap_or("box");
            if notation.contains("box") {
                out.push_str("\\boxed{");
            } else if notation.contains("radical") {
                out.push_str("\\sqrt{");
            } else if notation.contains("top") || notation.contains("overline") {
                out.push_str("\\overline{");
            } else if notation.contains("bottom") || notation.contains("underline") {
                out.push_str("\\underline{");
            } else {
                out.push('{');
            }
            for child in &node.children {
                format_node(child, out);
            }
            out.push('}');
        }
        "mprescripts" | "none" => {}
        _ => {
            if !node.text.is_empty() {
                out.push_str(&node.text);
            }
            for child in &node.children {
                format_node(child, out);
            }
        }
    }
}

fn trim_trailing_whitespace(s: &mut String) {
    while s.ends_with(' ') {
        s.pop();
    }
}

fn format_wrapped(node: &MathNode, out: &mut String) {
    if node.children.len() > 1 || node.tag == "mrow" {
        out.push('{');
        format_node(node, out);
        out.push('}');
    } else {
        format_node(node, out);
    }
}

fn is_invisible_operator(s: &str) -> bool {
    matches!(s, "\u{2061}" | "\u{2062}" | "\u{2063}" | "\u{2064}")
}

fn is_large_operator(node: Option<&MathNode>) -> bool {
    let Some(n) = node else { return false };
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
    ) || (n.tag == "mo" && (t == "∑" || t == "∏" || t == "∫" || t == "∮"))
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
        "°" => Some("\\mathring"),
        _ => None,
    }
}

fn map_fence_delimiter(delim: &str, is_open: bool) -> String {
    let d = delim.trim();
    if d.is_empty() {
        return if is_open { "\\left.".into() } else { "\\right.".into() };
    }
    let p = match d {
        "{" => "\\{",
        "}" => "\\}",
        "|" => "|",
        "[" => "[",
        "]" => "]",
        "(" => "(",
        ")" => ")",
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
        "≤" => Some(" \\le "),
        "≥" => Some(" \\ge "),
        "≠" => Some(" \\ne "),
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
        "∞" => Some("\\infty "),
        "∂" => Some("\\partial "),
        "∇" => Some("\\nabla "),
        "→" => Some(" \\to "),
        "←" => Some(" \\leftarrow "),
        "↔" => Some(" \\leftrightarrow "),
        "⇒" => Some(" \\implies "),
        "⇔" => Some(" \\iff "),
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
        "…" => Some("\\dots "),
        "⋯" => Some("\\cdots "),
        "⋮" => Some("\\vdots "),
        "⋱" => Some("\\ddots "),
        "′" => Some("'"),
        "″" => Some("''"),
        "‴" => Some("'''"),
        "°" => Some("^{\\circ}"),
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
        "ε" | "ϵ" | "epsilon" => Some("\\epsilon"),
        "ζ" | "zeta" => Some("\\zeta"),
        "η" | "eta" => Some("\\eta"),
        "θ" | "ϑ" | "theta" => Some("\\theta"),
        "ι" | "iota" => Some("\\iota"),
        "κ" | "kappa" => Some("\\kappa"),
        "λ" | "lambda" => Some("\\lambda"),
        "μ" | "mu" => Some("\\mu"),
        "ν" | "nu" => Some("\\nu"),
        "ξ" | "xi" => Some("\\xi"),
        "π" | "ϖ" | "pi" => Some("\\pi"),
        "ρ" | "ϱ" | "rho" => Some("\\rho"),
        "σ" | "ς" | "sigma" => Some("\\sigma"),
        "τ" | "tau" => Some("\\tau"),
        "υ" | "upsilon" => Some("\\upsilon"),
        "φ" | "ϕ" | "phi" => Some("\\phi"),
        "χ" | "chi" => Some("\\chi"),
        "ψ" | "psi" => Some("\\psi"),
        "ω" | "omega" => Some("\\omega"),

        "Γ" | "Gamma" => Some("\\Gamma"),
        "Δ" | "Delta" => Some("\\Delta"),
        "Θ" | "Theta" => Some("\\Theta"),
        "Λ" | "Lambda" => Some("\\Lambda"),
        "Ξ" | "Xi" => Some("\\Xi"),
        "Π" | "Pi" => Some("\\Pi"),
        "Σ" | "Sigma" => Some("\\Sigma"),
        "Υ" | "Upsilon" => Some("\\Upsilon"),
        "Φ" | "Phi" => Some("\\Phi"),
        "Ψ" | "Psi" => Some("\\Psi"),
        "Ω" | "Omega" => Some("\\Omega"),

        "∞" | "infty" => Some("\\infty"),
        _ => None,
    }
}

fn is_named_function(s: &str) -> bool {
    matches!(
        s,
        "sin"
            | "cos"
            | "tan"
            | "cot"
            | "sec"
            | "csc"
            | "arcsin"
            | "arccos"
            | "arctan"
            | "sinh"
            | "cosh"
            | "tanh"
            | "coth"
            | "ln"
            | "log"
            | "exp"
            | "lim"
            | "min"
            | "max"
            | "inf"
            | "sup"
            | "det"
            | "dim"
            | "gcd"
            | "hom"
            | "ker"
            | "deg"
            | "arg"
            | "Pr"
    )
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
            _ => res.push(c),
        }
    }
    res
}

fn clean_latex_output(s: &str) -> String {
    // Normalise multiple spaces and clean up edges
    let mut result = String::with_capacity(s.len());
    let mut prev_space = false;
    for c in s.chars() {
        if c.is_whitespace() {
            if !prev_space {
                result.push(' ');
                prev_space = true;
            }
        } else {
            result.push(c);
            prev_space = false;
        }
    }
    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fraction_to_latex() {
        let node = MathNode::with_children(
            "mfrac",
            vec![
                MathNode::leaf("mi", "x"),
                MathNode::leaf("mn", "2"),
            ],
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
                    vec![
                        MathNode::leaf("mi", "b"),
                        MathNode::leaf("mn", "2"),
                    ],
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
            vec![
                MathNode::leaf("mn", "2"),
                MathNode::leaf("mi", "a"),
            ],
        );

        let frac = MathNode::with_children("mfrac", vec![num, den]);
        let eq = MathNode::with_children(
            "mrow",
            vec![
                MathNode::leaf("mi", "x"),
                MathNode::leaf("mo", "="),
                frac,
            ],
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
}
