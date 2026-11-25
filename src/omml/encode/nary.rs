//! OMML N-ary operator (`<m:nary>`) detection and operand grouping.

use crate::error::Result;
use crate::mathml::MathNode;
use crate::xml::Element;

use super::{is_empty_node, m_el, m_el_children, m_val_el, MathmlEncoder};

impl MathmlEncoder {
    /// Process a slice of `MathNode`s without stretchy fences, grouping N-ary operators
    /// with their operand body `<m:e>`.
    pub(super) fn encode_nary_and_items(&mut self, nodes: &[MathNode]) -> Result<Vec<Element>> {
        let mut out = Vec::new();
        let mut idx = 0;
        while idx < nodes.len() {
            if let Some((chr, lim_loc, sub_opt, sup_opt)) = as_nary_op(&nodes[idx]) {
                let (body_els, next_idx) = if idx + 1 >= nodes.len() {
                    (Vec::new(), idx + 1)
                } else if (idx == 0
                    && nodes.len() > 2
                    && !nodes[1..]
                        .iter()
                        .any(|node| node.tag == "mo" && node.text.trim() == "="))
                    || as_nary_op(&nodes[idx + 1]).is_some()
                {
                    (self.encode_nary_and_items(&nodes[idx + 1..])?, nodes.len())
                } else {
                    (self.encode_node(&nodes[idx + 1])?, idx + 2)
                };

                let nary_el = self.build_nary(chr, lim_loc, sub_opt, sup_opt, body_els)?;
                out.push(nary_el);
                idx = next_idx;
            } else {
                out.extend(self.encode_node(&nodes[idx])?);
                idx += 1;
            }
        }
        Ok(out)
    }

    /// Build an `<m:nary>` OMML element with optional sub/sup limits and integrand/summand `body_els`.
    pub(super) fn build_nary(
        &mut self,
        chr: &str,
        lim_loc: &str,
        sub_opt: Option<&MathNode>,
        sup_opt: Option<&MathNode>,
        body_els: Vec<Element>,
    ) -> Result<Element> {
        let mut nary_pr = m_el("naryPr");
        nary_pr.push(m_val_el("chr", chr));
        nary_pr.push(m_val_el("limLoc", lim_loc));

        let sub_els = match sub_opt {
            Some(sub_node) if !is_empty_node(sub_node) => self.encode_node(sub_node)?,
            _ => Vec::new(),
        };
        let sup_els = match sup_opt {
            Some(sup_node) if !is_empty_node(sup_node) => self.encode_node(sup_node)?,
            _ => Vec::new(),
        };

        if sub_els.is_empty() {
            nary_pr.push(m_val_el("subHide", "1"));
        }
        if sup_els.is_empty() {
            nary_pr.push(m_val_el("supHide", "1"));
        }

        let mut nary = m_el("nary");
        nary.push(nary_pr);
        nary.push(m_el_children("sub", sub_els));
        nary.push(m_el_children("sup", sup_els));
        nary.push(m_el_children("e", body_els));
        Ok(nary)
    }
}

/// Extract `(chr, lim_loc, sub_node, sup_node)` if `node` represents an N-ary operator
/// (either a bare `<mo>` with a large operator character, or a script/underover node
/// whose base is such an `<mo>`).
pub(super) fn as_nary_op(
    node: &MathNode,
) -> Option<(&str, &'static str, Option<&MathNode>, Option<&MathNode>)> {
    if let Some((mo, big_style)) = unwrap_single_mo(node) {
        let text = mo.text.trim();
        if mo.attr("stretchy") != Some("false") {
            if let Some(chr) = normalize_nary_chr(text, big_style) {
                return Some((chr, "subSup", None, None));
            }
        }
        return None;
    }

    let explicit_loc = match node.attr("data-limloc") {
        Some("subSup") => Some("subSup"),
        Some("undOvr") => Some("undOvr"),
        _ => None,
    };

    match node.tag.as_str() {
        "msub" if node.children.len() == 2 => {
            let (mo, _) = unwrap_single_mo(&node.children[0])?;
            let chr = normalize_nary_chr(mo.text.trim(), true)?;
            Some((
                chr,
                explicit_loc.unwrap_or("subSup"),
                Some(&node.children[1]),
                None,
            ))
        }
        "msup" if node.children.len() == 2 => {
            let (mo, _) = unwrap_single_mo(&node.children[0])?;
            let chr = normalize_nary_chr(mo.text.trim(), true)?;
            Some((
                chr,
                explicit_loc.unwrap_or("subSup"),
                None,
                Some(&node.children[1]),
            ))
        }
        "msubsup" if node.children.len() == 3 => {
            let (mo, _) = unwrap_single_mo(&node.children[0])?;
            let chr = normalize_nary_chr(mo.text.trim(), true)?;
            Some((
                chr,
                explicit_loc.unwrap_or("subSup"),
                Some(&node.children[1]),
                Some(&node.children[2]),
            ))
        }
        "munder" if node.children.len() == 2 => {
            let (mo, _) = unwrap_single_mo(&node.children[0])?;
            let chr = normalize_nary_chr(mo.text.trim(), true)?;
            Some((
                chr,
                explicit_loc.unwrap_or("undOvr"),
                Some(&node.children[1]),
                None,
            ))
        }
        "mover" if node.children.len() == 2 => {
            let (mo, _) = unwrap_single_mo(&node.children[0])?;
            let chr = normalize_nary_chr(mo.text.trim(), true)?;
            Some((
                chr,
                explicit_loc.unwrap_or("undOvr"),
                None,
                Some(&node.children[1]),
            ))
        }
        "munderover" if node.children.len() == 3 => {
            let (mo, _) = unwrap_single_mo(&node.children[0])?;
            let chr = normalize_nary_chr(mo.text.trim(), true)?;
            Some((
                chr,
                explicit_loc.unwrap_or("undOvr"),
                Some(&node.children[1]),
                Some(&node.children[2]),
            ))
        }
        _ => None,
    }
}

fn unwrap_single_mo(mut node: &MathNode) -> Option<(&MathNode, bool)> {
    let mut had_displaystyle = false;
    loop {
        if node.tag == "mo" {
            return Some((node, had_displaystyle));
        }
        if matches!(node.tag.as_str(), "mstyle" | "mrow" | "mpadded") && node.children.len() == 1 {
            if node.attr("displaystyle") == Some("true") || node.attr("mathsize").is_some() {
                had_displaystyle = true;
            }
            node = &node.children[0];
        } else {
            return None;
        }
    }
}

fn normalize_nary_chr(op_text: &str, has_scripts_or_big_style: bool) -> Option<&str> {
    if is_nary_char(op_text) {
        Some(op_text)
    } else if has_scripts_or_big_style {
        match op_text {
            "\u{222A}" => Some("⋃"),
            "\u{2229}" => Some("⋂"),
            _ => None,
        }
    } else {
        None
    }
}

fn is_nary_char(op_text: &str) -> bool {
    matches!(
        op_text,
        "∑" | "∏"
            | "∐"
            | "∫"
            | "∬"
            | "∭"
            | "∮"
            | "∯"
            | "∰"
            | "∱"
            | "∲"
            | "∳"
            | "⋃"
            | "⋂"
            | "⋀"
            | "⋁"
            | "⨁"
            | "⨂"
            | "⨀"
            | "⨃"
            | "⨄"
            | "⨅"
            | "⨆"
    )
}
