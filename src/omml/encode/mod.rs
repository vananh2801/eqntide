//! MathML ([`MathNode`]) → OMML ([`Element`]) converter.
//!
//! ```text
//! MathNode tree ─▶ normalization::normalize_mathml (primes, scripts, function names)
//!                         │
//!                         ▼
//!                MathmlEncoder::encode_root (<m:oMath>)
//!                         │
//!       ┌─────────────────┼────────────────────┐
//!       ▼                 ▼                    ▼
//! encode_row_slice   scripts / accents   encode_mtable /
//! (Dirac, <m:d>,     / bars / groupChr   menclose / mfrac /
//!  n-ary <m:nary>)                       msqrt / mroot
//!       │
//!       ▼
//! font::apply_math_font (<w:rFonts> on <m:r> and <m:ctrlPr>)
//! ```
//!
//! Converts the intermediate [`MathNode`] tree (produced by [`crate::mtef::decode`]
//! or [`crate::omml::decode`]) into an Office Open XML Math `<m:oMath>` element
//! tree suitable for direct insertion into a `.docx` document.

use crate::error::{Error, Result};
use crate::mathml::MathNode;
use crate::omml::decode::NS_M;
use crate::symbols::prime_order;
use crate::xml::{Element, Node, XML_NS};

mod accents;
mod fences;
mod font;
mod nary;
mod normalization;
mod scripts;

#[cfg(test)]
mod tests;

pub use font::apply_math_font;

use fences::{build_delimiter, classify_fence_char, is_stretchy_fence_mo, FenceKind};
use normalization::{collect_text, normalize_mathml};

const MAX_DEPTH: usize = 256;

/// Known multi-letter mathematical function names that MathType stores as
/// individual `<mi>` character records.
pub(super) const KNOWN_FUNCTIONS: &[&str] = &[
    "arcsin", "arccos", "arctan", "arccot", "arcsec", "arccsc", "arsinh", "arcosh", "artanh",
    "arcoth", "sinh", "cosh", "tanh", "coth", "sech", "csch", "cotg", "cosec", "sin", "cos", "tan",
    "cot", "sec", "csc", "tg", "ctg", "log", "ln", "lg", "exp", "lim", "limsup", "liminf",
    "lim sup", "lim inf", "max", "min", "sup", "inf", "det", "dim", "deg", "gcd", "lcm", "arg",
    "ker", "hom", "mod", "sgn", "Pr",
];

/// Convert a MathML [`MathNode`] tree into an OMML `<m:oMath>` [`Element`].
pub fn mathml_to_omml(mathml: &MathNode) -> Result<Element> {
    MathmlEncoder::new().encode_root(mathml)
}

/// Convert a MathML [`MathNode`] tree into an OMML `<m:oMath>` XML string.
pub fn mathml_to_omml_xml(mathml: &MathNode) -> Result<String> {
    let element = mathml_to_omml(mathml)?;
    let bytes = crate::xml::to_bytes(&element);
    String::from_utf8(bytes).map_err(|err| Error::Xml(err.to_string()))
}

pub(super) struct MathmlEncoder {
    depth: usize,
}

impl MathmlEncoder {
    fn new() -> Self {
        Self { depth: 0 }
    }

    fn encode_root(&mut self, mathml: &MathNode) -> Result<Element> {
        let normalized = normalize_mathml(mathml);
        let mut omath = m_el("oMath").with_nsdecl(Some("m"), NS_M);
        let children = if normalized.tag == "math" {
            self.encode_row_slice(&normalized.children)?
        } else {
            self.encode_node(&normalized)?
        };
        for child in children {
            omath.push(child);
        }
        Ok(omath)
    }

    pub(super) fn encode_node(&mut self, node: &MathNode) -> Result<Vec<Element>> {
        if self.depth >= MAX_DEPTH {
            return Err(Error::TooDeep);
        }
        self.depth += 1;
        let result = self.encode_node_inner(node);
        self.depth -= 1;
        result
    }

    fn encode_node_inner(&mut self, node: &MathNode) -> Result<Vec<Element>> {
        match node.tag.as_str() {
            "math" | "mrow" | "mstyle" | "mpadded" => self.encode_row_slice(&node.children),
            "mi" | "mn" | "mo" | "mtext" | "ms" => Ok(self.encode_run(node).into_iter().collect()),
            "mspace" => {
                let width = crate::symbols::misc::space_width_em(node.attr("width").unwrap_or("0"));
                let text = match width {
                    w if (w - 3.0 / 18.0).abs() < 0.00001 => "\u{2009}",
                    w if (w - 4.0 / 18.0).abs() < 0.00001 => "\u{205F}",
                    w if (w - 5.0 / 18.0).abs() < 0.00001 => "\u{2005}",
                    w if (w - 1.0).abs() < 0.00001 => "\u{2003}",
                    w if (w - 2.0).abs() < 0.00001 => "\u{2003}\u{2003}",
                    _ => "\u{200A}",
                };
                Ok(self
                    .encode_run(&MathNode::leaf("mspace", text))
                    .into_iter()
                    .collect())
            }
            "mfrac" => self.encode_mfrac(node).map(|el| vec![el]),
            "msqrt" => self.encode_msqrt(node).map(|el| vec![el]),
            "mroot" => self.encode_mroot(node).map(|el| vec![el]),
            "msub" => self.encode_msub(node).map(|el| vec![el]),
            "msup" => {
                let sup_text = node.children.get(1).map(collect_text).unwrap_or_default();
                if let Some(order) = prime_order(&sup_text) {
                    let prime_str = match order {
                        1 => "'",
                        2 => "''",
                        3.. => "'''",
                        0 => unreachable!(),
                    };
                    let mut row = Vec::new();
                    if let Some(base) = node.children.first() {
                        row.push(base.clone());
                    }
                    row.push(MathNode::leaf("mo", prime_str));
                    self.encode_row_slice(&row)
                } else {
                    self.encode_msup(node).map(|el| vec![el])
                }
            }
            "msubsup" => self.encode_msubsup(node).map(|el| vec![el]),
            "mmultiscripts" => self.encode_mmultiscripts(node),
            "munder" => self.encode_munder(node).map(|el| vec![el]),
            "mover" => self.encode_mover(node).map(|el| vec![el]),
            "munderover" => self.encode_munderover(node).map(|el| vec![el]),
            "mtable" => self.encode_mtable(node).map(|el| vec![el]),
            "menclose" => self.encode_menclose(node).map(|el| vec![el]),
            "mphantom" => self.encode_mphantom(node).map(|el| vec![el]),
            _ => self.encode_row_slice(&node.children),
        }
    }

    /// Encode a slice of sibling `MathNode`s (as found inside `<mrow>`, `<mstyle>`, etc.),
    /// handling Dirac bra-kets, interval/paired/single-sided fences (`<m:d>`), and N-ary
    /// operators (`<m:nary>`).
    fn encode_row_slice(&mut self, nodes: &[MathNode]) -> Result<Vec<Element>> {
        if nodes.is_empty() {
            return Ok(Vec::new());
        }

        // 1. Check for Dirac bra-ket template `<mrow>`:
        //    [optional `<mo>⟨</mo>`, left_slot, `<mo>|</mo>`, right_slot, optional `<mo>⟩</mo>`]
        if let Some(dirac) = self.try_encode_dirac(nodes)? {
            return Ok(vec![dirac]);
        }

        // 2. Dedicated fence `<mrow>` where both ends are stretchy fence `<mo>`
        //    (including interval fences) with no interior stretchy fences.
        if nodes.len() >= 2
            && is_stretchy_fence_mo(&nodes[0])
            && is_stretchy_fence_mo(&nodes[nodes.len() - 1])
            && !nodes[1..nodes.len() - 1].iter().any(is_stretchy_fence_mo)
        {
            let beg = nodes[0].text.trim();
            let end = nodes[nodes.len() - 1].text.trim();
            let is_matched_or_interval = match (classify_fence_char(beg), classify_fence_char(end))
            {
                (Some(FenceKind::Open), Some(FenceKind::Close)) => true,
                (Some(FenceKind::Neutral), Some(FenceKind::Neutral)) => beg == end,
                _ => matches!(beg, "(" | ")" | "[" | "]") && matches!(end, "(" | ")" | "[" | "]"),
            };
            if is_matched_or_interval {
                let inner_els = self.encode_nary_and_items(&nodes[1..nodes.len() - 1])?;
                return Ok(vec![build_delimiter(beg, end, None, vec![inner_els])]);
            }
        }

        // 3. Trailing non-stretchy closer right after `<mtable>` (e.g. `mtable }`).
        if nodes.len() >= 2 {
            let last = &nodes[nodes.len() - 1];
            let prev = &nodes[nodes.len() - 2];
            if last.tag == "mo"
                && matches!(last.text.trim(), "}" | ")" | "]")
                && prev.tag == "mtable"
                && !nodes[..nodes.len() - 1].iter().any(is_stretchy_fence_mo)
            {
                let mut out = self.encode_nary_and_items(&nodes[..nodes.len() - 2])?;
                let mtable_els = self.encode_node(prev)?;
                out.push(build_delimiter(
                    "",
                    last.text.trim(),
                    None,
                    vec![mtable_els],
                ));
                return Ok(out);
            }
        }

        // 4. General stack-based fence matching if any stretchy fence `<mo>` is present.
        if nodes.iter().any(is_stretchy_fence_mo) {
            return self.encode_row_with_fences(nodes);
        }

        // 5. No stretchy fences at this level: process N-ary operators and normal items.
        self.encode_nary_and_items(nodes)
    }

    fn encode_run(&self, node: &MathNode) -> Option<Element> {
        let clean: String = node
            .text
            .chars()
            .filter(|&ch| !('\u{2061}'..='\u{2064}').contains(&ch))
            .collect();
        if clean.is_empty() {
            return None;
        }

        let mut run = m_el("r");
        let mut run_pr = m_el("rPr");

        let variant = node.attr("mathvariant");
        let (script, style) = match variant {
            Some("double-struck") => (Some("double-struck"), Some("p")),
            Some("fraktur") => (Some("fraktur"), Some("p")),
            Some("bold-fraktur") => (Some("fraktur"), Some("b")),
            Some("script") => (Some("script"), None),
            Some("bold-script") => (Some("script"), Some("b")),
            Some("sans-serif") => (Some("sans-serif"), Some("p")),
            Some("bold-sans-serif") => (Some("sans-serif"), Some("b")),
            Some("sans-serif-italic") => (Some("sans-serif"), Some("i")),
            Some("sans-serif-bold-italic") => (Some("sans-serif"), Some("bi")),
            Some("monospace") => (Some("monospace"), Some("p")),
            Some("bold") => (None, Some("b")),
            Some("bold-italic") => (None, Some("bi")),
            Some("normal") => (None, Some("p")),
            Some("italic") => (None, Some("i")),
            _ => {
                // OMML defaults to italic, unlike MathML token defaults. Spell out each
                // token's style so Word/WPS does not italicize digits or operators.
                let inferred = if node.tag == "mi" && clean.chars().count() == 1 {
                    "i"
                } else {
                    "p"
                };
                (None, Some(inferred))
            }
        };

        if node.tag == "mtext" {
            run_pr.push(m_el("nor"));
        }
        if let Some(scr_val) = script {
            run_pr.push(m_val_el("scr", scr_val));
        }
        if let Some(sty_val) = style {
            run_pr.push(m_val_el("sty", sty_val));
        }
        if !run_pr.is_empty() {
            run.push(run_pr);
        }

        let mut text_el = m_el("t");
        if node.tag == "mtext"
            && (clean.starts_with(char::is_whitespace) || clean.ends_with(char::is_whitespace))
        {
            text_el.set_attr(Some(XML_NS), Some("xml"), "space", "preserve");
        }
        text_el.children.push(Node::Text(clean));
        run.push(text_el);
        Some(run)
    }

    fn encode_mfrac(&mut self, node: &MathNode) -> Result<Element> {
        let mut frac = m_el("f");
        let mut frac_pr = m_el("fPr");
        if node.attr("bevelled") == Some("true") {
            frac_pr.push(m_val_el("type", "skw"));
        } else if let Some(linethickness) = node.attr("linethickness") {
            let trimmed = linethickness.trim();
            if trimmed == "0" || trimmed.starts_with("0pt") || trimmed.starts_with("0px") {
                frac_pr.push(m_val_el("type", "noBar"));
            } else {
                frac_pr.push(m_val_el("type", "bar"));
            }
        } else {
            frac_pr.push(m_val_el("type", "bar"));
        }
        frac.push(frac_pr);

        let num_els = match node.children.first() {
            Some(child) => self.encode_node(child)?,
            None => Vec::new(),
        };
        let den_els = match node.children.get(1) {
            Some(child) => self.encode_node(child)?,
            None => Vec::new(),
        };
        frac.push(m_el_children("num", num_els));
        frac.push(m_el_children("den", den_els));
        Ok(frac)
    }

    fn encode_msqrt(&mut self, node: &MathNode) -> Result<Element> {
        let mut radical = m_el("rad");
        let mut radical_pr = m_el("radPr");
        radical_pr.push(m_val_el("degHide", "1"));
        radical.push(radical_pr);
        radical.push(m_el("deg"));
        let body_els = self.encode_row_slice(&node.children)?;
        radical.push(m_el_children("e", body_els));
        Ok(radical)
    }

    fn encode_mroot(&mut self, node: &MathNode) -> Result<Element> {
        let body_els = match node.children.first() {
            Some(child) => self.encode_node(child)?,
            None => Vec::new(),
        };
        let deg_els = match node.children.get(1) {
            Some(child) if !is_empty_node(child) => self.encode_node(child)?,
            _ => Vec::new(),
        };

        let mut radical = m_el("rad");
        if deg_els.is_empty() {
            let mut radical_pr = m_el("radPr");
            radical_pr.push(m_val_el("degHide", "1"));
            radical.push(radical_pr);
        }
        radical.push(m_el_children("deg", deg_els));
        radical.push(m_el_children("e", body_els));
        Ok(radical)
    }

    fn encode_mtable(&mut self, node: &MathNode) -> Result<Element> {
        let mut matrix = m_el("m");
        let max_cols = node
            .children
            .iter()
            .filter(|row| row.tag == "mtr")
            .map(|row| row.children.iter().filter(|cell| cell.tag == "mtd").count())
            .max()
            .unwrap_or(0);

        if let Some(alignment) = node.attr("columnalign") {
            let mut cols_spec = m_el("mcs");
            let alignments: Vec<_> = alignment.split_whitespace().collect();
            for index in 0..max_cols {
                let mut col_pr = m_el("mcPr");
                col_pr.push(m_val_el("count", "1"));
                let align = alignments
                    .get(index)
                    .or_else(|| alignments.last())
                    .copied()
                    .unwrap_or("center");
                col_pr.push(m_val_el(
                    "mcJc",
                    match align {
                        "left" => "left",
                        "right" => "right",
                        _ => "center",
                    },
                ));
                let mut col = m_el("mc");
                col.push(col_pr);
                cols_spec.push(col);
            }
            let mut matrix_pr = m_el("mPr");
            matrix_pr.push(cols_spec);
            matrix.push(matrix_pr);
        }
        for row in node.children.iter().filter(|row| row.tag == "mtr") {
            let mut mrow = m_el("mr");
            let mut col_count = 0;
            for cell in row.children.iter().filter(|cell| cell.tag == "mtd") {
                let cell_els = self.encode_row_slice(&cell.children)?;
                mrow.push(m_el_children("e", cell_els));
                col_count += 1;
            }
            while col_count < max_cols {
                mrow.push(m_el("e"));
                col_count += 1;
            }
            matrix.push(mrow);
        }
        Ok(matrix)
    }

    fn encode_menclose(&mut self, node: &MathNode) -> Result<Element> {
        let notation = node.attr("notation").unwrap_or("box");
        let parts: Vec<&str> = notation.split_whitespace().collect();

        let mut box_pr = m_el("borderBoxPr");
        let is_strike = parts.iter().any(|&part| {
            matches!(
                part,
                "horizontalstrike" | "verticalstrike" | "updiagonalstrike" | "downdiagonalstrike"
            )
        });

        if is_strike {
            box_pr.push(m_val_el("hideTop", "1"));
            box_pr.push(m_val_el("hideBot", "1"));
            box_pr.push(m_val_el("hideLeft", "1"));
            box_pr.push(m_val_el("hideRight", "1"));
            if parts.contains(&"horizontalstrike") {
                box_pr.push(m_val_el("strikeH", "1"));
            }
            if parts.contains(&"verticalstrike") {
                box_pr.push(m_val_el("strikeV", "1"));
            }
            if parts.contains(&"updiagonalstrike") {
                box_pr.push(m_val_el("strikeBLTR", "1"));
            }
            if parts.contains(&"downdiagonalstrike") {
                box_pr.push(m_val_el("strikeTLBR", "1"));
            }
        } else if !parts.is_empty() && !parts.contains(&"box") && !parts.contains(&"roundedbox") {
            let has_top = parts.contains(&"top")
                || parts.contains(&"actuarial")
                || parts.contains(&"longdiv");
            let has_bot = parts.contains(&"bottom");
            let has_left = parts.contains(&"left") || parts.contains(&"longdiv");
            let has_right = parts.contains(&"right") || parts.contains(&"actuarial");
            if !has_top {
                box_pr.push(m_val_el("hideTop", "1"));
            }
            if !has_bot {
                box_pr.push(m_val_el("hideBot", "1"));
            }
            if !has_left {
                box_pr.push(m_val_el("hideLeft", "1"));
            }
            if !has_right {
                box_pr.push(m_val_el("hideRight", "1"));
            }
        }

        let mut border_box = m_el("borderBox");
        if !box_pr.is_empty() {
            border_box.push(box_pr);
        }
        let body_els = self.encode_row_slice(&node.children)?;
        border_box.push(m_el_children("e", body_els));
        Ok(border_box)
    }

    fn encode_mphantom(&mut self, node: &MathNode) -> Result<Element> {
        let mut phantom = m_el("phant");
        let mut phantom_pr = m_el("phantPr");
        phantom_pr.push(m_val_el("show", "0"));
        phantom.push(phantom_pr);
        let body_els = self.encode_row_slice(&node.children)?;
        phantom.push(m_el_children("e", body_els));
        Ok(phantom)
    }
}

/// Create an empty OMML element `<m:{local}>`.
pub(super) fn m_el(local: &str) -> Element {
    Element::new(Some(NS_M), Some("m"), local)
}

/// Create an OMML element `<m:{local}>` populated with `children`.
pub(super) fn m_el_children(local: &str, children: Vec<Element>) -> Element {
    let mut element = m_el(local);
    for child in children {
        element.push(child);
    }
    element
}

/// Create an OMML property element `<m:{local} m:val="{val}"/>`.
pub(super) fn m_val_el(local: &str, val: &str) -> Element {
    let mut element = m_el(local);
    element.set_attr(Some(NS_M), Some("m"), "val", val);
    element
}

/// Return whether `node` has no visible text or structural leaf content.
pub(super) fn is_empty_node(node: &MathNode) -> bool {
    if !node.text.trim().is_empty() {
        return false;
    }
    if node.children.is_empty() {
        return !matches!(node.tag.as_str(), "mprescripts" | "mspace");
    }
    node.children.iter().all(is_empty_node)
}
