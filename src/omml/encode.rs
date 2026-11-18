//! MathML ([`MathNode`]) → OMML ([`Element`]) converter.
//!
//! Converts the intermediate [`MathNode`] tree (produced by [`crate::mtef::decode`]
//! or [`crate::omml::decode`]) into an Office Open XML Math `<m:oMath>` element
//! tree suitable for direct insertion into a `.docx` document.

use crate::error::{Error, Result};
use crate::mathml::MathNode;
use crate::omml::decode::NS_M;
use crate::symbols::prime_order;
use crate::xml::{Element, Node};

mod fences;
mod normalization;
mod scripts;

use fences::{classify_fence_char, is_stretchy_fence_mo, FenceKind};
use normalization::{collect_text, normalize_mathml};

const MAX_DEPTH: usize = 256;

const NS_W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

/// Known multi-letter mathematical function names that MathType stores as
/// individual `<mi>` character records.
pub(super) const KNOWN_FUNCTIONS: &[&str] = &[
    "arcsin", "arccos", "arctan", "arccot", "arcsec", "arccsc", "arsinh", "arcosh", "artanh",
    "arcoth", "sinh", "cosh", "tanh", "coth", "sech", "csch", "cotg", "cosec", "sin", "cos", "tan",
    "cot", "sec", "csc", "tg", "ctg", "log", "ln", "lg", "exp", "lim", "limsup", "liminf", "max",
    "min", "sup", "inf", "det", "dim", "deg", "gcd", "lcm", "arg", "ker", "hom", "mod", "Pr",
];

// ── XML Element Helpers ──────────────────────────────────────────────────────

pub(super) fn m_el(local: &str) -> Element {
    Element::new(Some(NS_M), Some("m"), local)
}

pub(super) fn m_el_children(local: &str, children: Vec<Element>) -> Element {
    let mut el = m_el(local);
    for c in children {
        el.push(c);
    }
    el
}

fn m_val_el(local: &str, val: &str) -> Element {
    let mut el = m_el(local);
    el.set_attr(Some(NS_M), Some("m"), "val", val);
    el
}

// ── Classification Helpers ───────────────────────────────────────────────────

pub(super) fn is_empty_node(node: &MathNode) -> bool {
    if !node.text.trim().is_empty() {
        return false;
    }
    if node.children.is_empty() {
        return !matches!(node.tag.as_str(), "mprescripts" | "mspace");
    }
    node.children.iter().all(is_empty_node)
}

fn is_nary_char(s: &str) -> bool {
    matches!(
        s,
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

/// Unwrap single-child `<mstyle>` / `<mrow>` / `<mpadded>` wrappers to reach an `<mo>` leaf.
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

fn normalize_nary_chr(s: &str, has_scripts_or_big_style: bool) -> Option<&str> {
    if is_nary_char(s) {
        Some(s)
    } else if has_scripts_or_big_style {
        match s {
            "\u{222A}" => Some("⋃"),
            "\u{2229}" => Some("⋂"),
            _ => None,
        }
    } else {
        None
    }
}

/// Extract `(chr, lim_loc, sub_node, sup_node)` if `node` represents an N-ary operator
/// (either a bare `<mo>` with a large operator character, or a script/underover node
/// whose base is such an `<mo>`).
pub(super) fn as_nary_op(
    node: &MathNode,
) -> Option<(&str, &'static str, Option<&MathNode>, Option<&MathNode>)> {
    if let Some((mo, big_style)) = unwrap_single_mo(node) {
        let t = mo.text.trim();
        if mo.attr("stretchy") != Some("false") {
            if let Some(chr) = normalize_nary_chr(t, big_style) {
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

fn is_arrow_mo(node: &MathNode) -> bool {
    node.tag == "mo"
        && matches!(
            node.text.trim(),
            "→" | "←"
                | "↔"
                | "⇄"
                | "⇌"
                | "⇋"
                | "⇆"
                | "⇀"
                | "↼"
                | "⇁"
                | "↽"
                | "↦"
                | "⟵"
                | "⟶"
                | "⟷"
                | "⇐"
                | "⇒"
                | "⇔"
                | "⟸"
                | "⟹"
                | "⟺"
        )
}

fn is_bar_char(s: &str) -> bool {
    matches!(
        s,
        "\u{00AF}"
            | "\u{203E}"
            | "\u{2015}"
            | "\u{0304}"
            | "\u{0305}"
            | "\u{0332}"
            | "\u{02C9}"
            | "_"
            | "\u{2212}"
            | "-"
    )
}

fn map_top_group_char(s: &str) -> Option<&'static str> {
    match s {
        "\u{FE37}" | "\u{23DE}" => Some("\u{23DE}"),
        "\u{23B4}" | "\u{FE39}" => Some("\u{23B4}"),
        "\u{2322}" | "\u{2312}" | "\u{23DC}" | "\u{FE35}" => Some("\u{23DC}"),
        _ => None,
    }
}

fn map_bot_group_char(s: &str) -> Option<&'static str> {
    match s {
        "\u{FE38}" | "\u{23DF}" => Some("\u{23DF}"),
        "\u{23B5}" | "\u{FE3A}" => Some("\u{23B5}"),
        "\u{2322}" | "\u{2323}" | "\u{23DD}" | "\u{FE36}" | "\u{032F}" => Some("\u{23DD}"),
        _ => None,
    }
}

fn map_accent_char(s: &str) -> Option<&'static str> {
    match s {
        "^" | "\u{02C6}" | "\u{0302}" => Some("\u{0302}"),
        "~" | "\u{02DC}" | "\u{0303}" | "\u{223C}" => Some("\u{0303}"),
        "\u{02D9}" | "\u{0307}" | "." | "\u{00B7}" => Some("\u{0307}"),
        "\u{00A8}" | "\u{0308}" | ".." => Some("\u{0308}"),
        "\u{20DB}" | "..." | "\u{2026}" => Some("\u{20DB}"),
        "\u{20DC}" => Some("\u{20DC}"),
        "\u{00B4}" | "\u{0301}" => Some("\u{0301}"),
        "`" | "\u{0300}" => Some("\u{0300}"),
        "\u{02D8}" | "\u{0306}" => Some("\u{0306}"),
        "\u{02C7}" | "\u{030C}" => Some("\u{030C}"),
        "\u{00B0}" | "\u{02DA}" | "\u{030A}" => Some("\u{030A}"),
        "\u{2192}" | "\u{20D7}" | "\u{27F6}" => Some("\u{20D7}"),
        "\u{2190}" | "\u{20D6}" | "\u{27F5}" => Some("\u{20D6}"),
        "\u{2194}" | "\u{20E1}" | "\u{27F7}" => Some("\u{20E1}"),
        "\u{21C0}" | "\u{21C1}" | "\u{20D1}" => Some("\u{20D1}"),
        "\u{21BC}" | "\u{21BD}" | "\u{20D0}" => Some("\u{20D0}"),
        "\u{0311}" => Some("\u{0311}"),
        "\u{033F}" | "==" | "═" => Some("\u{033F}"),
        _ => None,
    }
}

// ── MathML -> OMML Encoder ───────────────────────────────────────────────────

#[derive(Clone)]
enum MixedItem<'a> {
    Raw(&'a MathNode),
    Built(Element),
}

pub(super) struct MathmlEncoder {
    depth: usize,
}

impl MathmlEncoder {
    fn new() -> Self {
        Self { depth: 0 }
    }

    fn encode_root(&mut self, mathml: &MathNode) -> Result<Element> {
        let norm = normalize_mathml(mathml);
        let mut omath = m_el("oMath").with_nsdecl(Some("m"), NS_M);
        let children = if norm.tag == "math" {
            self.encode_row_slice(&norm.children)?
        } else {
            self.encode_node(&norm)?
        };
        for c in children {
            omath.push(c);
        }
        Ok(omath)
    }

    pub(super) fn encode_node(&mut self, node: &MathNode) -> Result<Vec<Element>> {
        if self.depth >= MAX_DEPTH {
            return Err(Error::TooDeep);
        }
        self.depth += 1;
        let res = self.encode_node_inner(node);
        self.depth -= 1;
        res
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
                    .encode_run(&MathNode::leaf("mtext", text))
                    .into_iter()
                    .collect())
            }
            "mfrac" => self.encode_mfrac(node).map(|e| vec![e]),
            "msqrt" => self.encode_msqrt(node).map(|e| vec![e]),
            "mroot" => self.encode_mroot(node).map(|e| vec![e]),
            "msub" => self.encode_msub(node).map(|e| vec![e]),
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
                    self.encode_msup(node).map(|e| vec![e])
                }
            }
            "msubsup" => self.encode_msubsup(node).map(|e| vec![e]),
            "mmultiscripts" => self.encode_mmultiscripts(node),
            "munder" => self.encode_munder(node).map(|e| vec![e]),
            "mover" => self.encode_mover(node).map(|e| vec![e]),
            "munderover" => self.encode_munderover(node).map(|e| vec![e]),
            "mtable" => self.encode_mtable(node).map(|e| vec![e]),
            "menclose" => self.encode_menclose(node).map(|e| vec![e]),
            "mphantom" => self.encode_mphantom(node).map(|e| vec![e]),
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

        // 2. Check for a dedicated fence `<mrow>` where both ends are stretchy fence `<mo>`
        //    (including interval fences like `]a, b[`, `[a, b[`, `]a, b]`) and there are no
        //    stretchy fences in the interior.
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

        // 3. Check for trailing non-stretchy closer right after `<mtable>`
        //    (e.g. right-handed system of equations `mtable }` where `}` had stretchy="false").
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

        // 4. General stack-based fence matching if any stretchy fence `<mo>` is present
        if nodes.iter().any(is_stretchy_fence_mo) {
            return self.encode_row_with_fences(nodes);
        }

        // 5. No stretchy fences at this level: process N-ary operators and normal items
        self.encode_nary_and_items(nodes)
    }

    fn try_encode_dirac(&mut self, nodes: &[MathNode]) -> Result<Option<Element>> {
        if nodes.len() < 3 || nodes.len() > 5 {
            return Ok(None);
        }
        let has_left = nodes.first().is_some_and(|n| {
            is_stretchy_fence_mo(n) && matches!(n.text.trim(), "\u{27E8}" | "\u{2329}")
        });
        let has_right = nodes.last().is_some_and(|n| {
            is_stretchy_fence_mo(n) && matches!(n.text.trim(), "\u{27E9}" | "\u{232A}")
        });
        if !has_left && !has_right {
            return Ok(None);
        }
        let start = usize::from(has_left);
        let end = nodes.len() - usize::from(has_right);
        let inner = &nodes[start..end];
        let sep_positions: Vec<usize> = inner
            .iter()
            .enumerate()
            .filter_map(|(idx, n)| (is_stretchy_fence_mo(n) && n.text.trim() == "|").then_some(idx))
            .collect();
        if sep_positions.len() != 1 {
            return Ok(None);
        }
        let sep_idx = sep_positions[0];
        let left_els = self.encode_nary_and_items(&inner[..sep_idx])?;
        let right_els = self.encode_nary_and_items(&inner[sep_idx + 1..])?;
        let beg = if has_left { "\u{27E8}" } else { "" };
        let end_chr = if has_right { "\u{27E9}" } else { "" };
        Ok(Some(build_delimiter(
            beg,
            end_chr,
            Some("|"),
            vec![left_els, right_els],
        )))
    }

    fn encode_row_with_fences(&mut self, nodes: &[MathNode]) -> Result<Vec<Element>> {
        let mut root_items: Vec<MixedItem<'_>> = Vec::new();
        let mut stack: Vec<(String, Vec<MixedItem<'_>>)> = Vec::new();

        for node in nodes {
            if is_stretchy_fence_mo(node) {
                let ch = node.text.trim();
                match classify_fence_char(ch).unwrap_or(FenceKind::Neutral) {
                    FenceKind::Open => {
                        stack.push((ch.to_owned(), Vec::new()));
                    }
                    FenceKind::Close => {
                        if let Some((open_ch, inner_items)) = stack.pop() {
                            let inner_els = self.encode_mixed_items(&inner_items)?;
                            let d = build_delimiter(&open_ch, ch, None, vec![inner_els]);
                            if let Some((_, parent_items)) = stack.last_mut() {
                                parent_items.push(MixedItem::Built(d));
                            } else {
                                root_items.push(MixedItem::Built(d));
                            }
                        } else {
                            // Unmatched closing fence (e.g. `mtable }`): wrap preceding items
                            let taken = std::mem::take(&mut root_items);
                            let inner_els = self.encode_mixed_items(&taken)?;
                            let d = build_delimiter("", ch, None, vec![inner_els]);
                            root_items.push(MixedItem::Built(d));
                        }
                    }
                    FenceKind::Neutral => {
                        if stack.last().is_some_and(|(open_ch, _)| open_ch == ch) {
                            let (open_ch, inner_items) = stack.pop().unwrap();
                            let inner_els = self.encode_mixed_items(&inner_items)?;
                            let d = build_delimiter(&open_ch, ch, None, vec![inner_els]);
                            if let Some((_, parent_items)) = stack.last_mut() {
                                parent_items.push(MixedItem::Built(d));
                            } else {
                                root_items.push(MixedItem::Built(d));
                            }
                        } else {
                            stack.push((ch.to_owned(), Vec::new()));
                        }
                    }
                }
            } else if let Some((_, top_items)) = stack.last_mut() {
                top_items.push(MixedItem::Raw(node));
            } else {
                root_items.push(MixedItem::Raw(node));
            }
        }

        // Close any remaining unmatched opening fences (e.g. `{ mtable` system of equations)
        while let Some((open_ch, inner_items)) = stack.pop() {
            let inner_els = self.encode_mixed_items(&inner_items)?;
            let d = build_delimiter(&open_ch, "", None, vec![inner_els]);
            if let Some((_, parent_items)) = stack.last_mut() {
                parent_items.push(MixedItem::Built(d));
            } else {
                root_items.push(MixedItem::Built(d));
            }
        }

        self.encode_mixed_items(&root_items)
    }

    fn encode_mixed_items(&mut self, items: &[MixedItem<'_>]) -> Result<Vec<Element>> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < items.len() {
            match &items[i] {
                MixedItem::Built(el) => {
                    out.push(el.clone());
                    i += 1;
                }
                MixedItem::Raw(_) => {
                    let mut raw_run = Vec::new();
                    while i < items.len() {
                        if let MixedItem::Raw(n) = items[i] {
                            raw_run.push(n.clone());
                            i += 1;
                        } else {
                            break;
                        }
                    }
                    out.extend(self.encode_nary_and_items(&raw_run)?);
                }
            }
        }
        Ok(out)
    }

    /// Process a slice of `MathNode`s without stretchy fences, grouping N-ary operators
    /// with their operand body `<m:e>`.
    fn encode_nary_and_items(&mut self, nodes: &[MathNode]) -> Result<Vec<Element>> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < nodes.len() {
            if let Some((chr, lim_loc, sub_opt, sup_opt)) = as_nary_op(&nodes[i]) {
                let (body_els, next_i) = if i + 1 >= nodes.len() {
                    (Vec::new(), i + 1)
                } else if (i == 0
                    && nodes.len() > 2
                    && !nodes[1..]
                        .iter()
                        .any(|n| n.tag == "mo" && n.text.trim() == "="))
                    || as_nary_op(&nodes[i + 1]).is_some()
                {
                    (self.encode_nary_and_items(&nodes[i + 1..])?, nodes.len())
                } else {
                    (self.encode_node(&nodes[i + 1])?, i + 2)
                };

                let nary_el = self.build_nary(chr, lim_loc, sub_opt, sup_opt, body_els)?;
                out.push(nary_el);
                i = next_i;
            } else {
                out.extend(self.encode_node(&nodes[i])?);
                i += 1;
            }
        }
        Ok(out)
    }

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
            Some(s) if !is_empty_node(s) => self.encode_node(s)?,
            _ => Vec::new(),
        };
        let sup_els = match sup_opt {
            Some(p) if !is_empty_node(p) => self.encode_node(p)?,
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

    // ── Leaf Runs (<mi>, <mn>, <mo>, <mtext>, <ms>) ──────────────────────────

    fn encode_run(&self, node: &MathNode) -> Option<Element> {
        let clean: String = node
            .text
            .chars()
            .filter(|&c| !('\u{2061}'..='\u{2064}').contains(&c))
            .collect();
        if clean.is_empty() {
            return None;
        }

        let mut r = m_el("r");
        let mut r_pr = m_el("rPr");

        let variant = node.attr("mathvariant");
        let (scr, sty) = match variant {
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
                // OMML defaults to italic, unlike MathML token defaults.
                // Spell out each token's style so WPS does not italicize
                // digits/operators or inherit the surrounding text's style.
                let style = if node.tag == "mi" && clean.chars().count() == 1 {
                    "i"
                } else {
                    "p"
                };
                (None, Some(style))
            }
        };

        if let Some(scr_val) = scr {
            r_pr.push(m_val_el("scr", scr_val));
        }
        if let Some(sty_val) = sty {
            r_pr.push(m_val_el("sty", sty_val));
        }
        if !r_pr.is_empty() {
            r.push(r_pr);
        }

        let mut t = m_el("t");
        t.children.push(Node::Text(clean));
        r.push(t);
        Some(r)
    }

    // ── Fractions & Radicals ─────────────────────────────────────────────────

    fn encode_mfrac(&mut self, node: &MathNode) -> Result<Element> {
        let mut f = m_el("f");
        let mut f_pr = m_el("fPr");
        if node.attr("bevelled") == Some("true") {
            f_pr.push(m_val_el("type", "skw"));
        } else if let Some(lt) = node.attr("linethickness") {
            let lt_trim = lt.trim();
            if lt_trim == "0" || lt_trim.starts_with("0pt") || lt_trim.starts_with("0px") {
                f_pr.push(m_val_el("type", "noBar"));
            } else {
                f_pr.push(m_val_el("type", "bar"));
            }
        } else {
            f_pr.push(m_val_el("type", "bar"));
        }
        f.push(f_pr);

        let num_els = match node.children.first() {
            Some(c) => self.encode_node(c)?,
            None => Vec::new(),
        };
        let den_els = match node.children.get(1) {
            Some(c) => self.encode_node(c)?,
            None => Vec::new(),
        };
        f.push(m_el_children("num", num_els));
        f.push(m_el_children("den", den_els));
        Ok(f)
    }

    fn encode_msqrt(&mut self, node: &MathNode) -> Result<Element> {
        let mut rad = m_el("rad");
        let mut rad_pr = m_el("radPr");
        rad_pr.push(m_val_el("degHide", "1"));
        rad.push(rad_pr);
        rad.push(m_el("deg"));
        let body_els = self.encode_row_slice(&node.children)?;
        rad.push(m_el_children("e", body_els));
        Ok(rad)
    }

    fn encode_mroot(&mut self, node: &MathNode) -> Result<Element> {
        let body_els = match node.children.first() {
            Some(c) => self.encode_node(c)?,
            None => Vec::new(),
        };
        let deg_els = match node.children.get(1) {
            Some(c) if !is_empty_node(c) => self.encode_node(c)?,
            _ => Vec::new(),
        };

        let mut rad = m_el("rad");
        if deg_els.is_empty() {
            let mut rad_pr = m_el("radPr");
            rad_pr.push(m_val_el("degHide", "1"));
            rad.push(rad_pr);
        }
        rad.push(m_el_children("deg", deg_els));
        rad.push(m_el_children("e", body_els));
        Ok(rad)
    }

    // ── Under / Over / Accents / Bars / GroupChr ─────────────────────────────

    fn encode_mover(&mut self, node: &MathNode) -> Result<Element> {
        if let Some((chr, lim_loc, sub_opt, sup_opt)) = as_nary_op(node) {
            return self.build_nary(chr, lim_loc, sub_opt, sup_opt, Vec::new());
        }
        let Some(base) = node.children.first() else {
            return Ok(m_el("limUpp"));
        };
        let Some(over) = node.children.get(1) else {
            let mut el = m_el("limUpp");
            el.push(m_el_children("e", self.encode_node(base)?));
            el.push(m_el("lim"));
            return Ok(el);
        };

        if over.tag == "mo" && !is_arrow_mo(base) {
            let over_str = over.text.trim();

            // 1. Overbar (<m:bar pos="top">)
            if is_bar_char(over_str) {
                let mut bar = m_el("bar");
                let mut bar_pr = m_el("barPr");
                bar_pr.push(m_val_el("pos", "top"));
                bar.push(bar_pr);
                bar.push(m_el_children("e", self.encode_node(base)?));
                return Ok(bar);
            }

            // 2. Top horizontal brace / bracket (<m:groupChr pos="top">)
            if let Some(gchr) = map_top_group_char(over_str) {
                let mut g = m_el("groupChr");
                let mut g_pr = m_el("groupChrPr");
                g_pr.push(m_val_el("chr", gchr));
                g_pr.push(m_val_el("pos", "top"));
                g_pr.push(m_val_el("vertJc", "bot"));
                g.push(g_pr);
                g.push(m_el_children("e", self.encode_node(base)?));
                return Ok(g);
            }

            // 3. Top accent (<m:acc>)
            if let Some(acc_chr) = map_accent_char(over_str) {
                let mut acc = m_el("acc");
                let mut acc_pr = m_el("accPr");
                acc_pr.push(m_val_el("chr", acc_chr));
                acc.push(acc_pr);
                acc.push(m_el_children("e", self.encode_node(base)?));
                return Ok(acc);
            }
            if node.attr("accent") == Some("true") && !over_str.is_empty() {
                let mut acc = m_el("acc");
                let mut acc_pr = m_el("accPr");
                acc_pr.push(m_val_el("chr", over_str));
                acc.push(acc_pr);
                acc.push(m_el_children("e", self.encode_node(base)?));
                return Ok(acc);
            }
        }

        // 4. Extensible arrow with label above (<m:groupChr pos="bot" vertJc="bot">)
        if is_arrow_mo(base) {
            return Ok(build_arrow_group_chr(
                base.text.trim(),
                "bot",
                "bot",
                self.encode_node(over)?,
            ));
        }

        // 5. General upper limit (<m:limUpp>)
        let mut lim_upp = m_el("limUpp");
        lim_upp.push(m_el_children("e", self.encode_node(base)?));
        lim_upp.push(m_el_children("lim", self.encode_node(over)?));
        Ok(lim_upp)
    }

    fn encode_munder(&mut self, node: &MathNode) -> Result<Element> {
        if let Some((chr, lim_loc, sub_opt, sup_opt)) = as_nary_op(node) {
            return self.build_nary(chr, lim_loc, sub_opt, sup_opt, Vec::new());
        }
        let Some(base) = node.children.first() else {
            return Ok(m_el("limLow"));
        };
        let Some(under) = node.children.get(1) else {
            let mut el = m_el("limLow");
            el.push(m_el_children("e", self.encode_node(base)?));
            el.push(m_el("lim"));
            return Ok(el);
        };

        if under.tag == "mo" && !is_arrow_mo(base) {
            let under_str = under.text.trim();

            // 1. Underbar (<m:bar pos="bot">)
            if is_bar_char(under_str) {
                let mut bar = m_el("bar");
                let mut bar_pr = m_el("barPr");
                bar_pr.push(m_val_el("pos", "bot"));
                bar.push(bar_pr);
                bar.push(m_el_children("e", self.encode_node(base)?));
                return Ok(bar);
            }

            // 2. Bottom horizontal brace / bracket (<m:groupChr pos="bot">)
            if let Some(gchr) = map_bot_group_char(under_str) {
                let mut g = m_el("groupChr");
                let mut g_pr = m_el("groupChrPr");
                g_pr.push(m_val_el("chr", gchr));
                g_pr.push(m_val_el("pos", "bot"));
                g_pr.push(m_val_el("vertJc", "top"));
                g.push(g_pr);
                g.push(m_el_children("e", self.encode_node(base)?));
                return Ok(g);
            }

            // 3. Under-accent / under-arrow / under-arc (<m:groupChr pos="bot">)
            if node.attr("accentunder") == Some("true") && !under_str.is_empty() {
                let mut g = m_el("groupChr");
                let mut g_pr = m_el("groupChrPr");
                g_pr.push(m_val_el("chr", under_str));
                g_pr.push(m_val_el("pos", "bot"));
                g_pr.push(m_val_el("vertJc", "top"));
                g.push(g_pr);
                g.push(m_el_children("e", self.encode_node(base)?));
                return Ok(g);
            }
        }

        // 4. Extensible arrow with label below (<m:groupChr pos="top" vertJc="top">)
        if is_arrow_mo(base) {
            return Ok(build_arrow_group_chr(
                base.text.trim(),
                "top",
                "top",
                self.encode_node(under)?,
            ));
        }

        // 5. General lower limit (<m:limLow>)
        let mut lim_low = m_el("limLow");
        lim_low.push(m_el_children("e", self.encode_node(base)?));
        lim_low.push(m_el_children("lim", self.encode_node(under)?));
        Ok(lim_low)
    }

    fn encode_munderover(&mut self, node: &MathNode) -> Result<Element> {
        if let Some((chr, lim_loc, sub_opt, sup_opt)) = as_nary_op(node) {
            return self.build_nary(chr, lim_loc, sub_opt, sup_opt, Vec::new());
        }
        if let (Some(base), Some(under), Some(over)) = (
            node.children.first(),
            node.children.get(1),
            node.children.get(2),
        ) {
            if is_arrow_mo(base) {
                let chr = base.text.trim();
                let under_empty = is_empty_node(under);
                let over_empty = is_empty_node(over);
                if under_empty && !over_empty {
                    return Ok(build_arrow_group_chr(
                        chr,
                        "bot",
                        "bot",
                        self.encode_node(over)?,
                    ));
                }
                if over_empty && !under_empty {
                    return Ok(build_arrow_group_chr(
                        chr,
                        "top",
                        "top",
                        self.encode_node(under)?,
                    ));
                }
                let under_els = self.encode_node(under)?;
                let over_els = self.encode_node(over)?;
                if collect_text(under).chars().count() > collect_text(over).chars().count() {
                    let g = build_arrow_group_chr(chr, "top", "top", under_els);
                    let mut lim_upp = m_el("limUpp");
                    lim_upp.push(m_el_children("e", vec![g]));
                    lim_upp.push(m_el_children("lim", over_els));
                    return Ok(lim_upp);
                }
                let g = build_arrow_group_chr(chr, "bot", "bot", over_els);
                let mut lim_low = m_el("limLow");
                lim_low.push(m_el_children("e", vec![g]));
                lim_low.push(m_el_children("lim", under_els));
                return Ok(lim_low);
            }
        }
        let base_els = match node.children.first() {
            Some(c) => self.encode_node(c)?,
            None => Vec::new(),
        };
        let under_els = match node.children.get(1) {
            Some(c) => self.encode_node(c)?,
            None => Vec::new(),
        };
        let over_els = match node.children.get(2) {
            Some(c) => self.encode_node(c)?,
            None => Vec::new(),
        };

        let mut lim_upp = m_el("limUpp");
        lim_upp.push(m_el_children("e", base_els));
        lim_upp.push(m_el_children("lim", over_els));

        let mut lim_low = m_el("limLow");
        lim_low.push(m_el_children("e", vec![lim_upp]));
        lim_low.push(m_el_children("lim", under_els));
        Ok(lim_low)
    }

    // ── Table / Matrix / Pile (<mtable>) ─────────────────────────────────────

    fn encode_mtable(&mut self, node: &MathNode) -> Result<Element> {
        let mut m = m_el("m");
        let max_cols = node
            .children
            .iter()
            .filter(|r| r.tag == "mtr")
            .map(|r| r.children.iter().filter(|c| c.tag == "mtd").count())
            .max()
            .unwrap_or(0);

        if let Some(alignment) = node.attr("columnalign") {
            let mut mcs = m_el("mcs");
            let alignments: Vec<_> = alignment.split_whitespace().collect();
            for index in 0..max_cols {
                let mut mc_pr = m_el("mcPr");
                mc_pr.push(m_val_el("count", "1"));
                let align = alignments
                    .get(index)
                    .or_else(|| alignments.last())
                    .copied()
                    .unwrap_or("center");
                mc_pr.push(m_val_el(
                    "mcJc",
                    match align {
                        "left" => "left",
                        "right" => "right",
                        _ => "center",
                    },
                ));
                let mut mc = m_el("mc");
                mc.push(mc_pr);
                mcs.push(mc);
            }
            let mut pr = m_el("mPr");
            pr.push(mcs);
            m.push(pr);
        }
        for row in node.children.iter().filter(|r| r.tag == "mtr") {
            let mut mr = m_el("mr");
            let mut col_count = 0;
            for cell in row.children.iter().filter(|c| c.tag == "mtd") {
                let cell_els = self.encode_row_slice(&cell.children)?;
                mr.push(m_el_children("e", cell_els));
                col_count += 1;
            }
            while col_count < max_cols {
                mr.push(m_el("e"));
                col_count += 1;
            }
            m.push(mr);
        }
        Ok(m)
    }

    // ── Enclosures & Phantoms (<menclose>, <mphantom>) ───────────────────────

    fn encode_menclose(&mut self, node: &MathNode) -> Result<Element> {
        let notation = node.attr("notation").unwrap_or("box");
        let parts: Vec<&str> = notation.split_whitespace().collect();

        let mut b_pr = m_el("borderBoxPr");
        let is_strike = parts.iter().any(|&p| {
            matches!(
                p,
                "horizontalstrike" | "verticalstrike" | "updiagonalstrike" | "downdiagonalstrike"
            )
        });

        if is_strike {
            b_pr.push(m_val_el("hideTop", "1"));
            b_pr.push(m_val_el("hideBot", "1"));
            b_pr.push(m_val_el("hideLeft", "1"));
            b_pr.push(m_val_el("hideRight", "1"));
            if parts.contains(&"horizontalstrike") {
                b_pr.push(m_val_el("strikeH", "1"));
            }
            if parts.contains(&"verticalstrike") {
                b_pr.push(m_val_el("strikeV", "1"));
            }
            if parts.contains(&"updiagonalstrike") {
                b_pr.push(m_val_el("strikeBLTR", "1"));
            }
            if parts.contains(&"downdiagonalstrike") {
                b_pr.push(m_val_el("strikeTLBR", "1"));
            }
        } else if !parts.is_empty() && !parts.contains(&"box") && !parts.contains(&"roundedbox") {
            let has_top = parts.contains(&"top")
                || parts.contains(&"actuarial")
                || parts.contains(&"longdiv");
            let has_bot = parts.contains(&"bottom");
            let has_left = parts.contains(&"left") || parts.contains(&"longdiv");
            let has_right = parts.contains(&"right") || parts.contains(&"actuarial");
            if !has_top {
                b_pr.push(m_val_el("hideTop", "1"));
            }
            if !has_bot {
                b_pr.push(m_val_el("hideBot", "1"));
            }
            if !has_left {
                b_pr.push(m_val_el("hideLeft", "1"));
            }
            if !has_right {
                b_pr.push(m_val_el("hideRight", "1"));
            }
        }

        let mut bbox = m_el("borderBox");
        if !b_pr.is_empty() {
            bbox.push(b_pr);
        }
        let body_els = self.encode_row_slice(&node.children)?;
        bbox.push(m_el_children("e", body_els));
        Ok(bbox)
    }

    fn encode_mphantom(&mut self, node: &MathNode) -> Result<Element> {
        let mut phant = m_el("phant");
        let mut phant_pr = m_el("phantPr");
        phant_pr.push(m_val_el("show", "0"));
        phant.push(phant_pr);
        let body_els = self.encode_row_slice(&node.children)?;
        phant.push(m_el_children("e", body_els));
        Ok(phant)
    }
}

fn build_arrow_group_chr(chr: &str, pos: &str, vert_jc: &str, label_els: Vec<Element>) -> Element {
    let mut g = m_el("groupChr");
    let mut g_pr = m_el("groupChrPr");
    g_pr.push(m_val_el("chr", chr));
    g_pr.push(m_val_el("pos", pos));
    g_pr.push(m_val_el("vertJc", vert_jc));
    g.push(g_pr);
    g.push(m_el_children("e", label_els));
    g
}

fn build_delimiter(
    beg_chr: &str,
    end_chr: &str,
    sep_chr: Option<&str>,
    e_groups: Vec<Vec<Element>>,
) -> Element {
    let mut d = m_el("d");
    let mut d_pr = m_el("dPr");
    d_pr.push(m_val_el("begChr", beg_chr));
    if let Some(sep) = sep_chr {
        d_pr.push(m_val_el("sepChr", sep));
    }
    d_pr.push(m_val_el("endChr", end_chr));
    d.push(d_pr);
    if e_groups.is_empty() {
        d.push(m_el("e"));
    } else {
        for group in e_groups {
            d.push(m_el_children("e", group));
        }
    }
    d
}

// ── Public API ───────────────────────────────────────────────────────────────

fn encode_mathml_root(mathml: &MathNode) -> Result<Element> {
    MathmlEncoder::new().encode_root(mathml)
}

/// Convert a MathML [`MathNode`] tree into an OMML `<m:oMath>` [`Element`].
pub fn mathml_to_omml(mathml: &MathNode) -> Result<Element> {
    encode_mathml_root(mathml)
}

/// Convert a MathML [`MathNode`] tree into an OMML `<m:oMath>` XML string.
pub fn mathml_to_omml_xml(mathml: &MathNode) -> Result<String> {
    let el = mathml_to_omml(mathml)?;
    let bytes = crate::xml::to_bytes(&el);
    String::from_utf8(bytes).map_err(|e| Error::Xml(e.to_string()))
}

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

/// Inject `<w:rPr><w:rFonts w:ascii="font" w:hAnsi="font" w:cs="font"/></w:rPr>`
/// into every `<m:r>` element and `<m:ctrlPr><w:rPr><w:rFonts .../></w:rPr></m:ctrlPr>`
/// into every structural `*Pr` element inside `omml`.
pub fn apply_math_font(omml: &mut Element, font: &str) {
    fn walk(el: &mut Element, font: &str) {
        if el.is(NS_M, "r") {
            if let Some(w_rpr) = el
                .children
                .iter_mut()
                .filter_map(|n| match n {
                    Node::Element(c) if c.is(NS_W, "rPr") => Some(c),
                    _ => None,
                })
                .next()
            {
                if !w_rpr.elements().any(|c| c.is(NS_W, "rFonts")) {
                    w_rpr.push(make_w_rfonts(font));
                }
            } else {
                let insert_pos = el
                    .children
                    .iter()
                    .position(|n| matches!(n, Node::Element(c) if c.is(NS_M, "t")))
                    .unwrap_or(el.children.len());
                el.children
                    .insert(insert_pos, Node::Element(make_w_rpr(font)));
            }
        } else if el.ns.as_deref() == Some(NS_M) && CTRL_PR_PARENTS.contains(&el.local.as_str()) {
            if let Some(ctrl_pr) = el
                .children
                .iter_mut()
                .filter_map(|n| match n {
                    Node::Element(c) if c.is(NS_M, "ctrlPr") => Some(c),
                    _ => None,
                })
                .next()
            {
                if !ctrl_pr.elements().any(|c| c.is(NS_W, "rPr")) {
                    ctrl_pr.push(make_w_rpr(font));
                }
            } else {
                let mut ctrl_pr = m_el("ctrlPr");
                ctrl_pr.push(make_w_rpr(font));
                el.push(ctrl_pr);
            }
        }
        for child in &mut el.children {
            if let Node::Element(c) = child {
                walk(c, font);
            }
        }
    }
    walk(omml, font);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_token_styles_are_explicit_in_nested_equations() {
        let math = MathNode::parse("<math><mfrac><mn>1</mn><mn>2</mn></mfrac><mi>x</mi><mo>+</mo><mn>3</mn><mo>=</mo><mn>0</mn><mi>sin</mi><mtext>text</mtext><ms>string</ms></math>").unwrap();
        let omml = mathml_to_omml(&math).unwrap();
        for run in omml.iter().into_iter().filter(|e| e.is(NS_M, "r")) {
            let text = run.find(NS_M, "t").unwrap().text();
            let style = run.find(NS_M, "rPr").unwrap().find(NS_M, "sty").unwrap();
            assert_eq!(
                style.attr(Some(NS_M), "val"),
                Some(if text == "x" { "i" } else { "p" }),
                "run {text}"
            );
        }
    }

    #[test]
    fn explicit_token_variants_override_the_default_style() {
        for (tag, variant, expected) in [
            ("mn", "italic", "i"),
            ("mn", "bold", "b"),
            ("mo", "bold-italic", "bi"),
            ("mi", "normal", "p"),
            ("mi", "bold-italic", "bi"),
        ] {
            let mut token = MathNode::leaf(tag, if tag == "mi" { "x" } else { "1" });
            token.set_attr("mathvariant", variant);
            let omml = mathml_to_omml(&token).unwrap();
            let style = omml
                .find(NS_M, "r")
                .unwrap()
                .find(NS_M, "rPr")
                .unwrap()
                .find(NS_M, "sty")
                .unwrap();
            assert_eq!(style.attr(Some(NS_M), "val"), Some(expected));
        }
    }
    use crate::omml::decode::omml_to_mathml;

    #[test]
    fn test_fraction_and_roots_to_omml() {
        let mml = MathNode::parse(
            "<math><mrow><mfrac><mi>a</mi><mi>b</mi></mfrac><mo>+</mo><msqrt><mi>x</mi></msqrt><mo>+</mo><mroot><mi>y</mi><mn>3</mn></mroot></mrow></math>",
        )
        .unwrap();
        let omml = mathml_to_omml(&mml).unwrap();
        assert!(omml.is(NS_M, "oMath"));
        assert!(omml.find(NS_M, "f").is_some());
        let rads = omml.find_all(NS_M, "rad");
        assert_eq!(rads.len(), 2);

        // Round-trip OMML -> MathML -> OMML
        let mml_rt = omml_to_mathml(&omml).unwrap();
        let omml_rt = mathml_to_omml(&mml_rt).unwrap();
        assert_eq!(
            omml.find_all(NS_M, "rad").len(),
            omml_rt.find_all(NS_M, "rad").len()
        );
    }

    #[test]
    fn test_function_name_merging_and_nary() {
        let mml = MathNode::parse(
            "<math><mrow><munderover><mo>∑</mo><mrow><mi>k</mi><mo>=</mo><mn>0</mn></mrow><mi>n</mi></munderover><mi>s</mi><mi>i</mi><msup><mi>n</mi><mn>2</mn></msup><mi>k</mi></mrow></math>",
        )
        .unwrap();
        let omml = mathml_to_omml(&mml).unwrap();
        let nary = omml.find(NS_M, "nary").expect("should have m:nary");
        let e = nary.find(NS_M, "e").expect("nary should have m:e");
        let ssup = e
            .find(NS_M, "sSup")
            .expect("sin^2 should be merged into sSup");
        let ssup_e = ssup.find(NS_M, "e").unwrap();
        let r = ssup_e.find(NS_M, "r").unwrap();
        assert_eq!(r.find(NS_M, "t").unwrap().text(), "sin");
    }

    #[test]
    fn test_single_sided_brace_and_trailing_closer() {
        // Left brace `{` + mtable
        let left_sys = MathNode::parse(
            "<math><mrow><mo>{</mo><mtable><mtr><mtd><mi>x</mi><mo>=</mo><mn>1</mn></mtd></mtr><mtr><mtd><mi>y</mi><mo>=</mo><mn>2</mn></mtd></mtr></mtable></mrow></math>",
        )
        .unwrap();
        let omml_left = mathml_to_omml(&left_sys).unwrap();
        let d_left = omml_left.find(NS_M, "d").expect("should wrap in m:d");
        let d_pr_left = d_left.find(NS_M, "dPr").unwrap();
        assert_eq!(
            d_pr_left
                .find(NS_M, "begChr")
                .unwrap()
                .attr(Some(NS_M), "val"),
            Some("{")
        );
        assert_eq!(
            d_pr_left
                .find(NS_M, "endChr")
                .unwrap()
                .attr(Some(NS_M), "val"),
            Some("")
        );

        // Right brace `mtable` + `}` (even with stretchy="false")
        let right_sys = MathNode::parse(
            "<math><mrow><mtable><mtr><mtd><mi>x</mi><mo>=</mo><mn>1</mn></mtd></mtr></mtable><mo stretchy=\"false\">}</mo></mrow></math>",
        )
        .unwrap();
        let omml_right = mathml_to_omml(&right_sys).unwrap();
        let d_right = omml_right.find(NS_M, "d").expect("should wrap in m:d");
        let d_pr_right = d_right.find(NS_M, "dPr").unwrap();
        assert_eq!(
            d_pr_right
                .find(NS_M, "begChr")
                .unwrap()
                .attr(Some(NS_M), "val"),
            Some("")
        );
        assert_eq!(
            d_pr_right
                .find(NS_M, "endChr")
                .unwrap()
                .attr(Some(NS_M), "val"),
            Some("}")
        );
    }

    #[test]
    fn test_accents_bars_and_prescripts() {
        let mml = MathNode::parse(
            "<math><mrow><mover accent=\"true\"><mi>x</mi><mo>^</mo></mover><mo>+</mo><mover accent=\"true\"><mrow><mi>A</mi><mi>B</mi></mrow><mo>¯</mo></mover><mo>+</mo><mmultiscripts><mi>U</mi><mprescripts/><mn>92</mn><mn>235</mn></mmultiscripts></mrow></math>",
        )
        .unwrap();
        let omml = mathml_to_omml(&mml).unwrap();
        let acc = omml.find(NS_M, "acc").expect("should have m:acc");
        assert_eq!(
            acc.find(NS_M, "accPr")
                .unwrap()
                .find(NS_M, "chr")
                .unwrap()
                .attr(Some(NS_M), "val"),
            Some("\u{0302}")
        );
        assert!(omml.find(NS_M, "bar").is_some());
        assert!(omml.find(NS_M, "sPre").is_some());
    }

    #[test]
    fn test_vector_arrows_embellishments_nary_and_boxes_roundtrip() {
        use crate::mtef::decode::mtef_to_mathml;
        use crate::mtef::encode::mathml_to_mtef_bytes;
        use crate::mtef::{build_mtef, TM_BOX, TM_LDIV, TM_UNION, TM_VEC};

        // 1. Left & bidirectional vector arrows over/under characters -> TM_VEC (31)
        let vec_mml = MathNode::parse(
            "<math><mrow><mover><mrow><mi>A</mi><mi>B</mi></mrow><mo>←</mo></mover><mo>+</mo><mover><mrow><mi>C</mi><mi>D</mi></mrow><mo>↔</mo></mover><mo>+</mo><munder><mrow><mi>E</mi><mi>F</mi></mrow><mo>→</mo></munder></mrow></math>",
        )
        .unwrap();
        let vec_bytes = mathml_to_mtef_bytes(&vec_mml).unwrap();
        assert_eq!(
            vec_bytes
                .windows(3)
                .filter(|w| w[0] == 0x03 && w[2] == TM_VEC)
                .count(),
            3
        );
        let decoded_vec = mtef_to_mathml(&build_mtef(&vec_bytes)).unwrap();
        let omml_vec = mathml_to_omml(&decoded_vec).unwrap();
        assert_eq!(omml_vec.find_all(NS_M, "acc").len(), 2);
        assert_eq!(omml_vec.find_all(NS_M, "groupChr").len(), 1);

        // 2. Combining marks on <mi>/<mo> -> EMB_* record
        let comb_mml = MathNode::parse(
            "<math><mrow><mi>x\u{0307}</mi><mo>=\u{0338}</mo><mn>0</mn></mrow></math>",
        )
        .unwrap();
        let comb_bytes = mathml_to_mtef_bytes(&comb_mml).unwrap();
        // Should contain CHAR records with OPT_CHAR_EMBELL (0x01) followed by EMBELL (0x06)
        assert!(comb_bytes
            .windows(2)
            .any(|w| w[0] == 0x02 && (w[1] & 0x01) != 0));
        let decoded_comb = mtef_to_mathml(&build_mtef(&comb_bytes)).unwrap();
        let omml_comb = mathml_to_omml(&decoded_comb).unwrap();
        assert!(omml_comb.find(NS_M, "acc").is_some());
        assert!(omml_comb.find(NS_M, "borderBox").is_some());

        // 3. Union / Intersection big operators -> TM_UNION -> m:nary
        let union_mml = MathNode::parse(
            "<math><mrow><munderover><mo>⋃</mo><mrow><mi>i</mi><mo>=</mo><mn>1</mn></mrow><mi>n</mi></munderover><msub><mi>A</mi><mi>i</mi></msub></mrow></math>",
        )
        .unwrap();
        let union_bytes = mathml_to_mtef_bytes(&union_mml).unwrap();
        assert!(union_bytes
            .windows(3)
            .any(|w| w[0] == 0x03 && w[2] == TM_UNION));
        let decoded_union = mtef_to_mathml(&build_mtef(&union_bytes)).unwrap();
        let omml_union = mathml_to_omml(&decoded_union).unwrap();
        let nary = omml_union
            .find(NS_M, "nary")
            .expect("tmUNION should map to m:nary");
        assert_eq!(
            nary.find(NS_M, "naryPr")
                .unwrap()
                .find(NS_M, "chr")
                .unwrap()
                .attr(Some(NS_M), "val"),
            Some("⋃")
        );

        // 4. Partial border box (top + left) & long division with quotient
        let box_mml = MathNode::parse(
            "<math><mrow><menclose notation=\"top left\"><mi>x</mi></menclose><mo>+</mo><mover><menclose notation=\"longdiv\"><mn>144</mn></menclose><mn>12</mn></mover></mrow></math>",
        )
        .unwrap();
        let box_bytes = mathml_to_mtef_bytes(&box_mml).unwrap();
        assert!(box_bytes.windows(3).any(|w| w[0] == 0x03 && w[2] == TM_BOX));
        assert!(box_bytes
            .windows(4)
            .any(|w| w[0] == 0x03 && w[2] == TM_LDIV && (w[3] & 0x01) != 0));
        let decoded_box = mtef_to_mathml(&build_mtef(&box_bytes)).unwrap();
        let roundtrip_box_bytes = mathml_to_mtef_bytes(&decoded_box).unwrap();
        assert!(roundtrip_box_bytes
            .windows(4)
            .any(|w| w[0] == 0x03 && w[2] == TM_LDIV && (w[3] & 0x01) != 0));
    }

    #[test]
    fn test_derivative_primes_direct_runs_and_mtef_embellishments() {
        use crate::mtef::decode::mtef_to_mathml;
        use crate::mtef::encode::mathml_to_mtef_bytes;
        use crate::mtef::{build_mtef, EMB_1PRIME, EMB_2PRIME, EMB_3PRIME};

        // 1. msup with prime (1st, 2nd, 3rd) encodes directly to OMML without m:sSup
        let primes = [("'", "'"), ("''", "''"), ("'''", "'''"), ("\u{2032}", "'")];
        for (sup_char, expected_prime) in primes {
            let mml = MathNode::parse(&format!(
                "<math><msup><mi>y</mi><mo>{}</mo></msup></math>",
                sup_char
            ))
            .unwrap();
            let omml = mathml_to_omml(&mml).unwrap();
            assert!(
                omml.find(NS_M, "sSup").is_none(),
                "msup with prime '{}' must not emit m:sSup",
                sup_char
            );
            let runs: Vec<String> = omml
                .find_all(NS_M, "r")
                .into_iter()
                .filter_map(|r| r.find(NS_M, "t").map(|t| t.text()))
                .collect();
            assert_eq!(runs, vec!["y".to_string(), expected_prime.to_string()]);
        }

        // 2. Direct sequence [y, '] encodes to MathType EMB_1PRIME, [y, ''] to EMB_2PRIME, [y, '''] to EMB_3PRIME
        let orders = [("'", EMB_1PRIME), ("''", EMB_2PRIME), ("'''", EMB_3PRIME)];
        for (prime_str, expected_emb) in orders {
            let mml = MathNode::parse(&format!(
                "<math><mrow><mi>y</mi><mo>{}</mo></mrow></math>",
                prime_str
            ))
            .unwrap();
            let mtef_bytes = mathml_to_mtef_bytes(&mml).unwrap();
            // Should contain embellishment record: [0x06, 0x00, expected_emb]
            assert!(
                mtef_bytes
                    .windows(3)
                    .any(|w| w[0] == 0x06 && w[1] == 0x00 && w[2] == expected_emb),
                "Should attach embellishment code {} for prime {}",
                expected_emb,
                prime_str
            );

            // Decode MathType back to OMML: should produce direct runs without m:sSup
            let decoded_mml = mtef_to_mathml(&build_mtef(&mtef_bytes)).unwrap();
            let roundtrip_omml = mathml_to_omml(&decoded_mml).unwrap();
            assert!(
                roundtrip_omml.find(NS_M, "sSup").is_none(),
                "Roundtrip OMML must not contain m:sSup"
            );
            let runs: Vec<String> = roundtrip_omml
                .find_all(NS_M, "r")
                .into_iter()
                .filter_map(|r| r.find(NS_M, "t").map(|t| t.text()))
                .collect();
            assert_eq!(runs, vec!["y".to_string(), prime_str.to_string()]);
        }
    }
    #[test]
    fn test_tuple_with_comma_does_not_become_prime() {
        use crate::mtef::decode::mtef_to_mathml;
        use crate::mtef::encode::mathml_to_mtef_bytes;
        use crate::mtef::{build_mtef, EMB_1PRIME};
        let mml = MathNode::parse("<math><mrow><mo>(</mo><mi>O</mi><mi>u</mi><mo>,</mo><mi>O</mi><mi>v</mi><mo>)</mo></mrow></math>").unwrap();
        let mtef = mathml_to_mtef_bytes(&mml).unwrap();
        assert!(
            !mtef.windows(3).any(|w| w[0] == 0x06 && w[2] == EMB_1PRIME),
            "Comma must not be converted to prime embellishment"
        );
        let decoded = mtef_to_mathml(&build_mtef(&mtef)).unwrap();
        let latex = decoded.to_latex();
        assert_eq!(latex, "(Ou,Ov)");
    }
    #[test]
    fn extended_palette_attributes_survive_omml_roundtrip() {
        for notation in [
            "top left",
            "bottom right",
            "horizontalstrike",
            "verticalstrike",
            "updiagonalstrike downdiagonalstrike",
        ] {
            let math = MathNode::parse(&format!(
                "<math><menclose notation=\"{notation}\"><mi>x</mi></menclose></math>"
            ))
            .unwrap();
            let out = crate::omml::omml_to_mathml(&mathml_to_omml(&math).unwrap()).unwrap();
            let enclosure = out
                .iter()
                .into_iter()
                .find(|n| n.tag == "menclose")
                .unwrap();
            let result = enclosure.attr("notation").unwrap();
            for part in notation.split_whitespace() {
                assert!(result.contains(part), "{notation}: {result}");
            }
        }
        for width in ["0.166667em", "0.222222em", "0.277778em", "1em", "2em"] {
            let math = MathNode::parse(&format!(
                "<math><mi>x</mi><mspace width=\"{width}\"/><mi>y</mi></math>"
            ))
            .unwrap();
            let out = crate::omml::omml_to_mathml(&mathml_to_omml(&math).unwrap()).unwrap();
            let space = out
                .iter()
                .into_iter()
                .find(|n| n.tag == "mspace")
                .expect("explicit spacing must survive");
            assert!(
                (crate::symbols::misc::space_width_em(space.attr("width").unwrap())
                    - crate::symbols::misc::space_width_em(width))
                .abs()
                    < 0.00001
            );
        }
        let matrix=MathNode::parse("<math><mtable columnalign=\"left right\"><mtr><mtd><mi>x</mi></mtd><mtd><mi>y</mi></mtd></mtr></mtable></math>").unwrap();
        let out = crate::omml::omml_to_mathml(&mathml_to_omml(&matrix).unwrap()).unwrap();
        assert!(out
            .iter()
            .iter()
            .any(|n| n.tag == "mtable" && n.attr("columnalign") == Some("left right")));
    }

    #[test]
    fn apply_math_font_sets_runs_and_ctrl_pr_on_structural_properties() {
        let mml = MathNode::parse(
            "<math><mrow><mfrac><mi>a</mi><mi>b</mi></mfrac><mo>+</mo><msqrt><mi>x</mi></msqrt><mo>+</mo><mrow><mo>(</mo><mi>y</mi><mo>)</mo></mrow><mo>+</mo><mover accent=\"true\"><mi>z</mi><mo>^</mo></mover><mo>+</mo><mover><mi>w</mi><mo>¯</mo></mover><mo>+</mo><munderover><mo>∑</mo><mrow><mi>i</mi><mo>=</mo><mn>1</mn></mrow><mi>n</mi></munderover><msub><mi>x</mi><mi>i</mi></msub></mrow></math>",
        )
        .unwrap();
        let mut omml = mathml_to_omml(&mml).unwrap();
        apply_math_font(&mut omml, "Cambria Math");

        for run in omml.iter().into_iter().filter(|e| e.is(NS_M, "r")) {
            let r_fonts = run
                .find(NS_W, "rPr")
                .and_then(|pr| pr.find(NS_W, "rFonts"))
                .expect("every m:r must have w:rPr/w:rFonts");
            assert_eq!(r_fonts.attr(Some(NS_W), "ascii"), Some("Cambria Math"));
            assert_eq!(r_fonts.attr(Some(NS_W), "hAnsi"), Some("Cambria Math"));
            assert_eq!(r_fonts.attr(Some(NS_W), "cs"), Some("Cambria Math"));
        }

        // Also cover all structural *Pr element names specified for ctrlPr injection.
        let mut synthetic = m_el("oMath");
        for pr_name in [
            "fPr",
            "dPr",
            "sSupPr",
            "sSubPr",
            "sSubSupPr",
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
        ] {
            synthetic.push(m_el(pr_name));
        }
        apply_math_font(&mut synthetic, "STIX Two Math");
        for pr_el in synthetic.elements() {
            let r_fonts = pr_el
                .find(NS_M, "ctrlPr")
                .and_then(|c| c.find(NS_W, "rPr"))
                .and_then(|r| r.find(NS_W, "rFonts"))
                .unwrap_or_else(|| panic!("{} missing m:ctrlPr/w:rPr/w:rFonts", pr_el.local));
            assert_eq!(r_fonts.attr(Some(NS_W), "ascii"), Some("STIX Two Math"));
            assert_eq!(r_fonts.attr(Some(NS_W), "hAnsi"), Some("STIX Two Math"));
            assert_eq!(r_fonts.attr(Some(NS_W), "cs"), Some("STIX Two Math"));
        }

        // Calling apply_math_font twice must not duplicate w:rPr or m:ctrlPr.
        apply_math_font(&mut omml, "Cambria Math");
        assert!(omml
            .find_all(NS_M, "fPr")
            .into_iter()
            .all(|pr| pr.find_all(NS_M, "ctrlPr").len() == 1));
    }

    #[test]
    fn set_minus_backslash_normalizes_to_u2216_and_empty_script_base_avoids_empty_e() {
        let set_mml = MathNode::parse(
            "<math><mi>D</mi><mo>=</mo><mi>ℝ</mi><mo>\\</mo><mrow><mo>{</mo><mrow><mo>-</mo><mn>1</mn></mrow><mo>}</mo></mrow><mtext>\\</mtext></math>",
        )
        .unwrap();
        let set_omml = mathml_to_omml(&set_mml).unwrap();
        let texts: Vec<String> = set_omml
            .iter()
            .into_iter()
            .filter(|e| e.is(NS_M, "t"))
            .map(|t| t.text())
            .collect();
        assert!(
            texts.iter().any(|t| t == "∖"),
            "expected U+2216 in {texts:?}"
        );
        // <mtext>\</mtext> remains literal '\', while <mo>\</mo> was converted to U+2216.
        assert_eq!(texts.iter().filter(|t| *t == "\\").count(), 1);

        // Empty script base on y_{CT} (where _{CT} has an empty <mrow/> base)
        let sub_mml = MathNode::parse(
            "<math><mrow><mi>y</mi><msub><mrow/><mrow><mi>C</mi><mi>T</mi></mrow></msub></mrow></math>",
        )
        .unwrap();
        let sub_omml = mathml_to_omml(&sub_mml).unwrap();
        let s_sub = sub_omml.find(NS_M, "sSub").expect("should emit m:sSub");
        let e = s_sub.find(NS_M, "e").expect("m:sSub must have m:e");
        assert!(
            !e.children.is_empty(),
            "empty script base must not produce <m:e/>"
        );
        assert_eq!(
            e.find(NS_M, "r")
                .and_then(|r| r.find(NS_M, "t"))
                .map(|t| t.text()),
            Some("\u{00A0}".to_string())
        );
    }

    #[test]
    fn encode_munderover_with_arrow_base_emits_stretchy_group_chr_and_roundtrips() {
        let mml = crate::service::parse_latex_to_mathml(
            r"A \overset{f}{\underset{g}{\leftrightharpoons}} B",
        )
        .unwrap();
        let omml = mathml_to_omml(&mml).unwrap();
        let group_chr = omml
            .iter()
            .into_iter()
            .find(|e| e.is(NS_M, "groupChr"))
            .expect("arrow munderover should emit m:groupChr");
        let chr = group_chr
            .find(NS_M, "groupChrPr")
            .and_then(|pr| pr.find(NS_M, "chr"))
            .and_then(|c| c.attr(Some(NS_M), "val"));
        assert_eq!(chr, Some("⇋"));

        let decoded = omml_to_mathml(&omml).unwrap();
        assert!(
            decoded
                .iter()
                .into_iter()
                .any(|n| n.tag == "munderover" && n.children[0].text == "⇋"),
            "roundtrip must preserve <munderover>: {}",
            decoded.to_xml()
        );

        // When under-label is longer than over-label, groupChr wraps the under-label inside limUpp
        let wide_under =
            crate::service::parse_latex_to_mathml(r"\overset{f}{\underset{catalyst}{\rightarrow}}")
                .unwrap();
        let omml_wide = mathml_to_omml(&wide_under).unwrap();
        let lim_upp = omml_wide.find(NS_M, "limUpp").expect("should use limUpp");
        assert!(lim_upp
            .find(NS_M, "e")
            .unwrap()
            .find(NS_M, "groupChr")
            .is_some());
        let decoded_wide = omml_to_mathml(&omml_wide).unwrap();
        assert!(decoded_wide
            .iter()
            .into_iter()
            .any(|n| n.tag == "munderover"));
    }
}
