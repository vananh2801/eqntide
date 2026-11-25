//! WordprocessingML font-property (`<w:rFonts>`) injection across OMML elements.

use crate::omml::decode::NS_M;
use crate::xml::{Element, Node};

use super::m_el;

pub(super) const NS_W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

const CTRL_PR_PARENTS: &[&str] = &[
    "fPr",
    "dPr",
    "sSupPr",
    "sSubPr",
    "sSubSupPr",
    "sPrePr",
    "radPr",
    "accPr",
    "barPr",
    "naryPr",
    "funcPr",
    "limLowPr",
    "limUppPr",
    "mPr",
    "eqArrPr",
    "borderBoxPr",
    "boxPr",
    "groupChrPr",
    "phantPr",
];

/// Inject `<w:rPr><w:rFonts w:ascii="font" w:hAnsi="font" w:cs="font"/></w:rPr>`
/// into every `<m:r>` element and `<m:ctrlPr><w:rPr><w:rFonts .../></w:rPr></m:ctrlPr>`
/// into every structural `*Pr` element inside `omml`.
pub fn apply_math_font(omml: &mut Element, font: &str) {
    fn walk(element: &mut Element, font: &str) {
        if element.is(NS_M, "r") {
            if let Some(w_rpr) = element
                .children
                .iter_mut()
                .filter_map(|node| match node {
                    Node::Element(child) if child.is(NS_W, "rPr") => Some(child),
                    _ => None,
                })
                .next()
            {
                if !w_rpr.elements().any(|child| child.is(NS_W, "rFonts")) {
                    w_rpr.push(make_w_rfonts(font));
                }
            } else {
                let insert_pos = element
                    .children
                    .iter()
                    .position(|node| matches!(node, Node::Element(child) if child.is(NS_M, "t")))
                    .unwrap_or(element.children.len());
                element
                    .children
                    .insert(insert_pos, Node::Element(make_w_rpr(font)));
            }
        } else if element.ns.as_deref() == Some(NS_M)
            && CTRL_PR_PARENTS.contains(&element.local.as_str())
        {
            if let Some(ctrl_pr) = element
                .children
                .iter_mut()
                .filter_map(|node| match node {
                    Node::Element(child) if child.is(NS_M, "ctrlPr") => Some(child),
                    _ => None,
                })
                .next()
            {
                if !ctrl_pr.elements().any(|child| child.is(NS_W, "rPr")) {
                    ctrl_pr.push(make_w_rpr(font));
                }
            } else {
                let mut ctrl_pr = m_el("ctrlPr");
                ctrl_pr.push(make_w_rpr(font));
                element.push(ctrl_pr);
            }
        }
        for child in &mut element.children {
            if let Node::Element(child_el) = child {
                walk(child_el, font);
            }
        }
    }
    walk(omml, font);
}

fn make_w_rfonts(font: &str) -> Element {
    let mut r_fonts = Element::new(Some(NS_W), Some("w"), "rFonts");
    r_fonts.set_attr(Some(NS_W), Some("w"), "ascii", font);
    r_fonts.set_attr(Some(NS_W), Some("w"), "hAnsi", font);
    r_fonts.set_attr(Some(NS_W), Some("w"), "cs", font);
    r_fonts
}

fn make_w_rpr(font: &str) -> Element {
    let mut w_rpr = Element::new(Some(NS_W), Some("w"), "rPr");
    w_rpr.push(make_w_rfonts(font));
    w_rpr
}
