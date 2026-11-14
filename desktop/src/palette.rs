//! Native palette data ported from the existing desktop's complete button inventory.
use eqntide::MathNode;
pub const PALETTE: &[(&str, &str, &str)] = &[
    (
        "Fractions",
        "Standard Fraction (Ctrl+F or /)",
        "\\frac{#?}{#?}",
    ),
    ("Fractions", "Small Inline Fraction", "\\tfrac{#?}{#?}"),
    ("Fractions", "Diagonal / Linear Fraction", "{#?}/{#?}"),
    ("Fractions", "Square Root (Ctrl+R)", "\\sqrt{#?}"),
    ("Fractions", "N-th Root (Ctrl+Shift+R)", "\\sqrt[#?]{#?}"),
    ("Fractions", "Root with Fraction", "\\sqrt{\\frac{#?}{#?}}"),
    ("Scripts", "Superscript / Power (Ctrl+H or ^)", "^{#?}"),
    ("Scripts", "Subscript (Ctrl+L or _)", "_{#?}"),
    ("Scripts", "Super + Subscript (Ctrl+J)", "_{#?}^{#?}"),
    ("Scripts", "Prescript (Tensor)", "{}^{#?}_{#?}#?"),
    (
        "Scripts",
        "Limit with Approach (Ctrl+Shift+L)",
        "\\lim_{#? \\to #?}",
    ),
    ("Scripts", "Maximum with Subscript", "\\max_{#?}"),
    ("Scripts", "Minimum with Subscript", "\\min_{#?}"),
    ("Scripts", "Vector Arrow (Ctrl+Alt+V)", "\\vec{#?}"),
    ("Scripts", "Hat Accent", "\\hat{#?}"),
    ("Scripts", "Overline / Bar Accent", "\\bar{#?}"),
    ("Scripts", "Single Dot (Derivative)", "\\dot{#?}"),
    ("Scripts", "Double Dot", "\\ddot{#?}"),
    ("Scripts", "Underbrace", "\\underbrace{#?}_{#?}"),
    ("Scripts", "Overbrace", "\\overbrace{#?}^{#?}"),
    (
        "Integrals",
        "Indefinite Integral (Ctrl+I)",
        "\\int #? \\, d#?",
    ),
    (
        "Integrals",
        "Definite Integral (Ctrl+Shift+I)",
        "\\int_{#?}^{#?} #? \\, d#?",
    ),
    (
        "Integrals",
        "Double Integral",
        "\\iint_{#?}^{#?} #? \\, d#?",
    ),
    (
        "Integrals",
        "Triple Integral",
        "\\iiint_{#?}^{#?} #? \\, d#?",
    ),
    (
        "Integrals",
        "Contour / Closed Integral",
        "\\oint_{#?}^{#?} #? \\, d#?",
    ),
    (
        "Integrals",
        "Summation with Limits (Ctrl+Shift+S)",
        "\\sum_{#?}^{#?} #?",
    ),
    (
        "Integrals",
        "Product with Limits (Ctrl+Shift+P)",
        "\\prod_{#?}^{#?} #?",
    ),
    ("Integrals", "Union of Sets", "\\bigcup_{#?}^{#?} #?"),
    ("Integrals", "Intersection of Sets", "\\bigcap_{#?}^{#?} #?"),
    (
        "Fences",
        "Parentheses (Ctrl+9 or Ctrl+()",
        "\\left( #? \\right)",
    ),
    ("Fences", "Square Brackets (Ctrl+[)", "\\left[ #? \\right]"),
    ("Fences", "Curly Braces (Ctrl+{)", "\\left\\{ #? \\right\\}"),
    (
        "Fences",
        "Absolute Value / Modulus (Ctrl+|)",
        "\\left| #? \\right|",
    ),
    ("Fences", "Norm / Magnitude", "\\left\\| #? \\right\\|"),
    (
        "Fences",
        "Floor Function",
        "\\left\\lfloor #? \\right\\rfloor",
    ),
    (
        "Fences",
        "Ceiling Function",
        "\\left\\lceil #? \\right\\rceil",
    ),
    (
        "Fences",
        "Angle Brackets (Dirac Bra-Ket)",
        "\\left\\langle #? \\right\\rangle",
    ),
    (
        "Fences",
        "2-Piece Cases (Ctrl+Shift+C)",
        "\\begin{cases} #? & #? \\\\ #? & #? \\end{cases}",
    ),
    (
        "Fences",
        "3-Piece Cases",
        "\\begin{cases} #? & #? \\\\ #? & #? \\\\ #? & #? \\end{cases}",
    ),
    ("Fences", "Evaluation Bar", "\\left. #? \\right|_{#?}^{#?}"),
    (
        "Matrices",
        "2x2 Matrix (Ctrl+Shift+M)",
        "\\begin{pmatrix} #? & #? \\\\ #? & #? \\end{pmatrix}",
    ),
    (
        "Matrices",
        "3x3 Matrix",
        "\\begin{pmatrix} #? & #? & #? \\\\ #? & #? & #? \\\\ #? & #? & #? \\end{pmatrix}",
    ),
    (
        "Matrices",
        "2x2 Bracket Matrix",
        "\\begin{bmatrix} #? & #? \\\\ #? & #? \\end{bmatrix}",
    ),
    (
        "Matrices",
        "2x2 Determinant",
        "\\begin{vmatrix} #? & #? \\\\ #? & #? \\end{vmatrix}",
    ),
    (
        "Matrices",
        "3x3 Determinant",
        "\\begin{vmatrix} #? & #? & #? \\\\ #? & #? & #? \\\\ #? & #? & #? \\end{vmatrix}",
    ),
    (
        "Matrices",
        "2x1 Column Vector",
        "\\begin{pmatrix} #? \\\\ #? \\end{pmatrix}",
    ),
    (
        "Matrices",
        "1x2 Row Vector",
        "\\begin{pmatrix} #? & #? \\end{pmatrix}",
    ),
    (
        "Matrices",
        "Aligned Multiline Equations",
        "\\begin{aligned} #? &= #? \\\\ #? &= #? \\end{aligned}",
    ),
    (
        "Matrices",
        "Gathered Centered Multiline",
        "\\begin{gathered} #? \\\\ #? \\end{gathered}",
    ),
    ("Symbols", "Alpha (\\alpha)", "\\alpha"),
    ("Symbols", "Beta (\\beta)", "\\beta"),
    ("Symbols", "Gamma (\\gamma)", "\\gamma"),
    ("Symbols", "Delta (\\delta)", "\\delta"),
    ("Symbols", "Epsilon (\\epsilon)", "\\epsilon"),
    ("Symbols", "Theta (\\theta)", "\\theta"),
    ("Symbols", "Lambda (\\lambda)", "\\lambda"),
    ("Symbols", "Mu (\\mu)", "\\mu"),
    ("Symbols", "Pi (\\pi)", "\\pi"),
    ("Symbols", "Rho (\\rho)", "\\rho"),
    ("Symbols", "Sigma (\\sigma)", "\\sigma"),
    ("Symbols", "Tau (\\tau)", "\\tau"),
    ("Symbols", "Phi (\\phi)", "\\phi"),
    ("Symbols", "Omega (\\omega)", "\\omega"),
    ("Symbols", "Delta (\\Delta)", "\\Delta"),
    ("Symbols", "Theta (\\Theta)", "\\Theta"),
    ("Symbols", "Lambda (\\Lambda)", "\\Lambda"),
    ("Symbols", "Sigma (\\Sigma)", "\\Sigma"),
    ("Symbols", "Phi (\\Phi)", "\\Phi"),
    ("Symbols", "Omega (\\Omega)", "\\Omega"),
    ("Symbols", "Right Arrow (\\to)", "\\to"),
    ("Symbols", "Left Arrow (\\leftarrow)", "\\leftarrow"),
    ("Symbols", "Implies (\\Rightarrow)", "\\Rightarrow"),
    (
        "Symbols",
        "Equivalent (\\Leftrightarrow)",
        "\\Leftrightarrow",
    ),
    ("Symbols", "If and only if (\\iff)", "\\iff"),
    ("Symbols", "Less than or equal (\\le)", "\\le"),
    ("Symbols", "Greater than or equal (\\ge)", "\\ge"),
    ("Symbols", "Not equal (\\neq)", "\\neq"),
    ("Symbols", "Approximately (\\approx)", "\\approx"),
    ("Symbols", "Identical (\\equiv)", "\\equiv"),
    ("Symbols", "Much less than (\\ll)", "\\ll"),
    ("Symbols", "Much greater than (\\gg)", "\\gg"),
    ("Symbols", "Plus-minus (\\pm)", "\\pm"),
    ("Symbols", "Minus-plus (\\mp)", "\\mp"),
    ("Symbols", "Multiplication cross (\\times)", "\\times"),
    ("Symbols", "Division (\\div)", "\\div"),
    ("Symbols", "Dot product (\\cdot)", "\\cdot"),
    ("Symbols", "Element of (\\in)", "\\in"),
    ("Symbols", "Not element of (\\notin)", "\\notin"),
    ("Symbols", "Subset (\\subset)", "\\subset"),
    ("Symbols", "Subset or equal (\\subseteq)", "\\subseteq"),
    ("Symbols", "Union (\\cup)", "\\cup"),
    ("Symbols", "Intersection (\\cap)", "\\cap"),
    ("Symbols", "Empty set (\\emptyset)", "\\emptyset"),
    ("Symbols", "Infinity (\\infty)", "\\infty"),
    ("Symbols", "For all (\\forall)", "\\forall"),
    ("Symbols", "There exists (\\exists)", "\\exists"),
    ("Symbols", "Nabla / Del (\\nabla)", "\\nabla"),
    ("Symbols", "Partial derivative (\\partial)", "\\partial"),
];

pub fn template(source: &str) -> Result<MathNode, String> {
    let mut source = source
        .replace("\\tfrac", "\\frac")
        .replace("\\iff", "\\Leftrightarrow");
    if source.starts_with('^') || source.starts_with('_') {
        source = format!("{{#?}}{source}");
    }
    if source == "{}^{#?}_{#?}#?" {
        return Ok(MathNode::with_children(
            "mmultiscripts",
            vec![
                MathNode::new("mrow"),
                MathNode::new("mprescripts"),
                MathNode::new("mrow"),
                MathNode::new("mrow"),
            ],
        ));
    }
    let source = source.replace("#?", "\\square");
    let mut node = eqntide::service::parse_latex_to_mathml(&source).map_err(|e| e.to_string())?;
    fn placeholders(n: &mut MathNode) {
        if n.text == "□" {
            n.tag = "mrow".into();
            n.text.clear();
            n.attrs.clear();
        }
        for child in &mut n.children {
            placeholders(child);
        }
    }
    placeholders(&mut node);
    if node.children.len() == 1 {
        Ok(node.children.remove(0))
    } else {
        node.tag = "mrow".into();
        Ok(node)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_existing_palette_button_has_a_native_template() {
        let errors: Vec<_> = PALETTE
            .iter()
            .filter_map(|(_, label, source)| {
                template(source).err().map(|e| format!("{label}: {e}"))
            })
            .collect();
        assert!(errors.is_empty(), "{}", errors.join("\n"));
    }
    #[test]
    fn filled_palette_templates_render_and_export_to_office() {
        fn fill(n: &mut MathNode) {
            if matches!(n.tag.as_str(), "mrow" | "mtd")
                && n.children.is_empty()
                && n.text.is_empty()
            {
                n.children.push(MathNode::leaf("mi", "x"));
            }
            for child in &mut n.children {
                fill(child);
            }
        }
        for (_, label, source) in PALETTE {
            let mut node = template(source).unwrap();
            fill(&mut node);
            let math = MathNode::with_children("math", vec![node]);
            let preview = eqntide::render::render_interactive(&math, 12.0)
                .unwrap_or_else(|e| panic!("{label}: {e}"));
            assert!(!preview.preview.png.is_empty(), "{label}");
            assert!(
                eqntide::mathml_to_omml_xml(&math)
                    .unwrap()
                    .contains("oMath"),
                "{label}"
            );
            assert!(
                !eqntide::render_emf(&math, 12.0).unwrap().emf.is_empty(),
                "{label}"
            );
        }
    }
}
