//! Serializer from [`MathNode`] (MathML AST) to standard LaTeX markup.
//!
//! Used by the bridge server and desktop editor to populate the LaTeX source
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
        "math" | "mrow" | "mstyle" => format_children(node, out),
        "mi" => format_identifier(node, out),
        "mn" => out.push_str(node.text.trim()),
        "mo" => format_operator(node, out),
        "mtext" => format_text(node, out),
        "mfrac" => format_fraction(node, out),
        "msqrt" => format_square_root(node, out),
        "mroot" => format_indexed_root(node, out),
        "msup" => format_superscript(node, out),
        "msub" => format_subscript(node, out),
        "msubsup" => format_subsuperscript(node, out),
        "munder" => format_underscript(node, out),
        "mover" => format_overscript(node, out),
        "munderover" => format_underoverscript(node, out),
        "mfenced" => format_fenced(node, out),
        "mtable" => format_matrix(node, out),
        "mtr" | "mtd" => format_children(node, out),
        "menclose" => format_enclosure(node, out),
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

fn format_children(node: &MathNode, out: &mut String) {
    for child in &node.children {
        format_node(child, out);
    }
}

fn format_identifier(node: &MathNode, out: &mut String) {
    let text = node.text.trim();
    if let Some(greek) = map_greek_or_symbol(text) {
        out.push_str(greek);
    } else if is_named_function(text) {
        out.push('\\');
        out.push_str(text);
        out.push(' ');
    } else if text.len() > 1 && text.chars().all(char::is_alphabetic) {
        out.push_str("\\mathrm{");
        out.push_str(text);
        out.push('}');
    } else {
        out.push_str(text);
    }
}

fn format_operator(node: &MathNode, out: &mut String) {
    let text = node.text.trim();
    if is_invisible_operator(text) {
        // Invisible times/comma/plus/apply are structural MathML markers.
        return;
    }
    if let Some(operator) = map_operator(text) {
        out.push_str(operator);
    } else {
        out.push_str(text);
    }
}

fn format_text(node: &MathNode, out: &mut String) {
    let text = node.text.trim();
    if text.is_empty() {
        return;
    }
    out.push_str("\\text{");
    out.push_str(&escape_latex_text(text));
    out.push('}');
}

fn format_fraction(node: &MathNode, out: &mut String) {
    let command = if is_binomial(node) {
        "\\binom"
    } else {
        "\\frac"
    };
    out.push_str(command);
    format_braced_child(node.children.first(), out);
    format_braced_child(node.children.get(1), out);
}

fn is_binomial(node: &MathNode) -> bool {
    matches!(node.attr("linethickness"), Some("0" | "0pt" | "none"))
}

fn format_square_root(node: &MathNode, out: &mut String) {
    out.push_str("\\sqrt{");
    format_children(node, out);
    out.push('}');
}

fn format_indexed_root(node: &MathNode, out: &mut String) {
    out.push_str("\\sqrt[");
    format_optional_node(node.children.get(1), out);
    out.push_str("]{");
    format_optional_node(node.children.first(), out);
    out.push('}');
}

fn format_braced_child(node: Option<&MathNode>, out: &mut String) {
    out.push('{');
    format_optional_node(node, out);
    out.push('}');
}

fn format_optional_node(node: Option<&MathNode>, out: &mut String) {
    if let Some(node) = node {
        format_node(node, out);
    }
}

fn format_superscript(node: &MathNode, out: &mut String) {
    let base = node.children.first();
    let superscript = node.children.get(1);
    format_optional_wrapped_node(base, out);

    if let Some(prime) = prime_shortcut(superscript) {
        out.push_str(prime);
        return;
    }

    format_script(superscript, "^{", out);
}

fn prime_shortcut(node: Option<&MathNode>) -> Option<&str> {
    let node = node?;
    let text = node.text.trim();
    (node.tag == "mo" && matches!(text, "'" | "\u{2032}" | "\u{2033}")).then_some(text)
}

fn format_subscript(node: &MathNode, out: &mut String) {
    format_optional_wrapped_node(node.children.first(), out);
    format_script(node.children.get(1), "_{", out);
}

fn format_subsuperscript(node: &MathNode, out: &mut String) {
    format_optional_wrapped_node(node.children.first(), out);
    format_script(node.children.get(1), "_{", out);
    format_script(node.children.get(2), "^{", out);
}

fn format_optional_wrapped_node(node: Option<&MathNode>, out: &mut String) {
    if let Some(node) = node {
        format_wrapped(node, out);
    }
}

fn format_script(node: Option<&MathNode>, prefix: &str, out: &mut String) {
    trim_trailing_whitespace(out);
    out.push_str(prefix);
    format_optional_node(node, out);
    out.push('}');
}

fn format_underscript(node: &MathNode, out: &mut String) {
    let base = node.children.first();
    let underscript = node.children.get(1);
    if is_large_operator(base) {
        format_optional_node(base, out);
        format_script(underscript, "_{", out);
        return;
    }
    format_over_or_under_set("\\underset", underscript, base, out);
}

fn format_overscript(node: &MathNode, out: &mut String) {
    let base = node.children.first();
    let overscript = node.children.get(1);
    if let Some(accent) = overscript.and_then(|node| map_accent(&node.text)) {
        out.push_str(accent);
        format_braced_child(base, out);
    } else if is_large_operator(base) {
        format_optional_node(base, out);
        format_script(overscript, "^{", out);
    } else {
        format_over_or_under_set("\\overset", overscript, base, out);
    }
}

fn format_underoverscript(node: &MathNode, out: &mut String) {
    format_optional_node(node.children.first(), out);
    format_script(node.children.get(1), "_{", out);
    format_script(node.children.get(2), "^{", out);
}

fn format_over_or_under_set(
    command: &str,
    annotation: Option<&MathNode>,
    base: Option<&MathNode>,
    out: &mut String,
) {
    out.push_str(command);
    format_braced_child(annotation, out);
    format_braced_child(base, out);
}

fn format_fenced(node: &MathNode, out: &mut String) {
    let open = map_fence_delimiter(node.attr("open").unwrap_or("("), true);
    let close = map_fence_delimiter(node.attr("close").unwrap_or(")"), false);
    out.push_str(&open);
    for (index, child) in node.children.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        format_node(child, out);
    }
    out.push_str(&close);
}

fn format_matrix(node: &MathNode, out: &mut String) {
    out.push_str("\\begin{matrix} ");
    for (row_index, row) in node.children.iter().enumerate() {
        if row_index > 0 {
            out.push_str(" \\\\ ");
        }
        format_matrix_row(row, out);
    }
    out.push_str(" \\end{matrix}");
}

fn format_matrix_row(row: &MathNode, out: &mut String) {
    for (cell_index, cell) in row.children.iter().enumerate() {
        if cell_index > 0 {
            out.push_str(" & ");
        }
        format_node(cell, out);
    }
}

fn format_enclosure(node: &MathNode, out: &mut String) {
    out.push_str(enclosure_command(node.attr("notation").unwrap_or("box")));
    format_children(node, out);
    out.push('}');
}

fn enclosure_command(notation: &str) -> &'static str {
    if notation.contains("box") {
        "\\boxed{"
    } else if notation.contains("radical") {
        "\\sqrt{"
    } else if notation.contains("top") || notation.contains("overline") {
        "\\overline{"
    } else if notation.contains("bottom") || notation.contains("underline") {
        "\\underline{"
    } else {
        "{"
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
    fn serializes_binomials_and_indexed_roots() {
        let binomial = serialize("<mfrac linethickness=\"0\"><mi>n</mi><mi>k</mi></mfrac>");
        let indexed_root = serialize("<mroot><mi>x</mi><mn>3</mn></mroot>");

        assert_eq!(binomial, "\\binom{n}{k}");
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
}
