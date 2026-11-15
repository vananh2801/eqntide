//! OMML (Office Math Markup Language) support.
//!
//! * [`decode`] – OMML → MathML
//! * [`encode`] – MathML → OMML

pub mod decode;
pub mod encode;

pub use decode::{is_number, is_operator, omml_to_mathml, text_node, NS_M};
pub use encode::{apply_math_font, mathml_to_omml, mathml_to_omml_xml};

/// Inspect a [`crate::MathNode`] tree for mathematical structures that are not natively supported
/// in standard OMML (ECMA-376 / Word Office Math Markup Language).
///
/// Returns a descriptive name of the unsupported construct if found.
pub fn check_omml_unsupported(node: &crate::MathNode) -> Option<&'static str> {
    for n in node.iter() {
        if n.tag == "menclose" {
            let notation = n.attr("notation").unwrap_or("");
            for token in notation.split_whitespace() {
                match token {
                    "longdiv" => return Some("Long division"),
                    "roundedbox" => return Some("Rounded box"),
                    "verticalstrike" => return Some("Vertical strike"),
                    "circle" => return Some("Circle enclosure"),
                    "phasorangle" => return Some("Phasor angle"),
                    _ => {}
                }
            }
        } else if n.tag == "munder" && n.attr("accentunder") == Some("true") {
            if let Some(accent) = n.children.get(1) {
                if accent.text.contains('\u{20DC}') || accent.text.contains('⃜') {
                    return Some("Quadruple underdot");
                }
            }
        }
    }
    None
}

/// Inspect a LaTeX string for mathematical structures that are not natively supported
/// in standard OMML (ECMA-376 / Word Office Math Markup Language).
pub fn check_omml_unsupported_latex(latex: &str) -> Option<&'static str> {
    if latex.contains("longdiv") {
        Some("Long division")
    } else if latex.contains("roundedbox") {
        Some("Rounded box")
    } else if latex.contains("verticalstrike") {
        Some("Vertical strike")
    } else if latex.contains("phasorangle") {
        Some("Phasor angle")
    } else if latex.contains(r"\underddddot") || latex.contains('⃜') || latex.contains('\u{20DC}')
    {
        Some("Quadruple underdot")
    } else if crate::mtef::enclose_has_token(latex, "circle") {
        Some("Circle enclosure")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::parse_latex_to_mathml;

    #[test]
    fn test_unsupported_omml_detection() {
        let cases = [
            (r"\enclose{longdiv}{144}", "Long division"),
            (r"\overset{12}{\enclose{longdiv}{144}}", "Long division"),
            (r"\enclose{roundedbox}{x+y}", "Rounded box"),
            (r"\enclose{verticalstrike}{x}", "Vertical strike"),
            (
                r"\enclose{horizontalstrike verticalstrike}{x}",
                "Vertical strike",
            ),
            (r"\enclose{circle}{x}", "Circle enclosure"),
            (r"\enclose{box circle}{x}", "Circle enclosure"),
            (r"\enclose{phasorangle}{x}", "Phasor angle"),
            (r"\underddddot{x}", "Quadruple underdot"),
        ];

        for (latex, expected) in cases {
            let math = parse_latex_to_mathml(latex).unwrap();
            assert_eq!(
                check_omml_unsupported(&math),
                Some(expected),
                "Failed on MathNode for {latex}"
            );
            assert_eq!(
                check_omml_unsupported_latex(latex),
                Some(expected),
                "Failed on LaTeX for {latex}"
            );
        }

        // Standard OMML-supported formulas should return None
        let supported = [
            r"\frac{a}{b}",
            r"\sqrt{x^2+y^2}",
            r"\sum_{i=1}^n i",
            r"\int_0^\infty e^{-x} dx",
            r"\begin{matrix} 1 & 2 \\ 3 & 4 \end{matrix}",
            r"\cancel{x}",
            r"\bcancel{x}",
            r"\xcancel{x}",
            r"\boxed{x}",
            r"\enclose{top left}{x}",
            r"\enclose{bottom right}{x}",
            r"\enclose{horizontalstrike}{x}",
            r"\enclose{actuarial}{n}",
            r"\overbrace{a+b}",
            r"\underbrace{a+b}",
            r"\vec{v}",
            r"\dot{x}",
            r"\ddot{x}",
            r"\dddot{x}",
            r"\ddddot{x}",
            r"\bigcirc",
        ];

        for latex in supported {
            let math = parse_latex_to_mathml(latex).unwrap();
            assert_eq!(
                check_omml_unsupported(&math),
                None,
                "False positive on MathNode for {latex}"
            );
            assert_eq!(
                check_omml_unsupported_latex(latex),
                None,
                "False positive on LaTeX for {latex}"
            );
        }
    }
}
