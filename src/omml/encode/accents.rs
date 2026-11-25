//! OMML accent (`<m:acc>`), over/underbar (`<m:bar>`), group character (`<m:groupChr>`),
//! and upper/lower limit (`<m:limUpp>`, `<m:limLow>`) encoders.

use crate::error::Result;
use crate::mathml::MathNode;
use crate::xml::Element;

use super::nary::as_nary_op;
use super::normalization::collect_text;
use super::{is_empty_node, m_el, m_el_children, m_val_el, MathmlEncoder};

impl MathmlEncoder {
    /// Encode `<mover>` as `<m:nary>`, `<m:bar>`, `<m:groupChr>`, `<m:acc>`, or `<m:limUpp>`.
    pub(super) fn encode_mover(&mut self, node: &MathNode) -> Result<Element> {
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

            if is_bar_char(over_str) {
                let mut bar = m_el("bar");
                let mut bar_pr = m_el("barPr");
                bar_pr.push(m_val_el("pos", "top"));
                bar.push(bar_pr);
                bar.push(m_el_children("e", self.encode_node(base)?));
                return Ok(bar);
            }

            if let Some(group_chr) = map_top_group_char(over_str) {
                let mut group = m_el("groupChr");
                let mut group_pr = m_el("groupChrPr");
                group_pr.push(m_val_el("chr", group_chr));
                group_pr.push(m_val_el("pos", "top"));
                group_pr.push(m_val_el("vertJc", "bot"));
                group.push(group_pr);
                group.push(m_el_children("e", self.encode_node(base)?));
                return Ok(group);
            }

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

        if is_arrow_mo(base) {
            return Ok(build_arrow_group_chr(
                base.text.trim(),
                "bot",
                "bot",
                self.encode_node(over)?,
            ));
        }

        let mut lim_upp = m_el("limUpp");
        lim_upp.push(m_el_children("e", self.encode_node(base)?));
        lim_upp.push(m_el_children("lim", self.encode_node(over)?));
        Ok(lim_upp)
    }

    /// Encode `<munder>` as `<m:nary>`, `<m:bar>`, `<m:groupChr>`, or `<m:limLow>`.
    pub(super) fn encode_munder(&mut self, node: &MathNode) -> Result<Element> {
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

            if is_bar_char(under_str) {
                let mut bar = m_el("bar");
                let mut bar_pr = m_el("barPr");
                bar_pr.push(m_val_el("pos", "bot"));
                bar.push(bar_pr);
                bar.push(m_el_children("e", self.encode_node(base)?));
                return Ok(bar);
            }

            if let Some(group_chr) = map_bot_group_char(under_str) {
                let mut group = m_el("groupChr");
                let mut group_pr = m_el("groupChrPr");
                group_pr.push(m_val_el("chr", group_chr));
                group_pr.push(m_val_el("pos", "bot"));
                group_pr.push(m_val_el("vertJc", "top"));
                group.push(group_pr);
                group.push(m_el_children("e", self.encode_node(base)?));
                return Ok(group);
            }

            if node.attr("accentunder") == Some("true") && !under_str.is_empty() {
                let mut group = m_el("groupChr");
                let mut group_pr = m_el("groupChrPr");
                group_pr.push(m_val_el("chr", under_str));
                group_pr.push(m_val_el("pos", "bot"));
                group_pr.push(m_val_el("vertJc", "top"));
                group.push(group_pr);
                group.push(m_el_children("e", self.encode_node(base)?));
                return Ok(group);
            }
        }

        if is_arrow_mo(base) {
            return Ok(build_arrow_group_chr(
                base.text.trim(),
                "top",
                "top",
                self.encode_node(under)?,
            ));
        }

        let mut lim_low = m_el("limLow");
        lim_low.push(m_el_children("e", self.encode_node(base)?));
        lim_low.push(m_el_children("lim", self.encode_node(under)?));
        Ok(lim_low)
    }

    /// Encode `<munderover>` as `<m:nary>`, labeled extensible arrow `<m:groupChr>`, or nested `<m:limLow>`/`<m:limUpp>`.
    pub(super) fn encode_munderover(&mut self, node: &MathNode) -> Result<Element> {
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
                    let group = build_arrow_group_chr(chr, "top", "top", under_els);
                    let mut lim_upp = m_el("limUpp");
                    lim_upp.push(m_el_children("e", vec![group]));
                    lim_upp.push(m_el_children("lim", over_els));
                    return Ok(lim_upp);
                }
                let group = build_arrow_group_chr(chr, "bot", "bot", over_els);
                let mut lim_low = m_el("limLow");
                lim_low.push(m_el_children("e", vec![group]));
                lim_low.push(m_el_children("lim", under_els));
                return Ok(lim_low);
            }
        }
        let base_els = match node.children.first() {
            Some(child) => self.encode_node(child)?,
            None => Vec::new(),
        };
        let under_els = match node.children.get(1) {
            Some(child) => self.encode_node(child)?,
            None => Vec::new(),
        };
        let over_els = match node.children.get(2) {
            Some(child) => self.encode_node(child)?,
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
}

fn build_arrow_group_chr(chr: &str, pos: &str, vert_jc: &str, label_els: Vec<Element>) -> Element {
    let mut group = m_el("groupChr");
    let mut group_pr = m_el("groupChrPr");
    group_pr.push(m_val_el("chr", chr));
    group_pr.push(m_val_el("pos", pos));
    group_pr.push(m_val_el("vertJc", vert_jc));
    group.push(group_pr);
    group.push(m_el_children("e", label_els));
    group
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

fn is_bar_char(text: &str) -> bool {
    matches!(
        text,
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

fn map_top_group_char(text: &str) -> Option<&'static str> {
    match text {
        "\u{FE37}" | "\u{23DE}" => Some("\u{23DE}"),
        "\u{23B4}" | "\u{FE39}" => Some("\u{23B4}"),
        "\u{2322}" | "\u{2312}" | "\u{23DC}" | "\u{FE35}" => Some("\u{23DC}"),
        _ => None,
    }
}

fn map_bot_group_char(text: &str) -> Option<&'static str> {
    match text {
        "\u{FE38}" | "\u{23DF}" => Some("\u{23DF}"),
        "\u{23B5}" | "\u{FE3A}" => Some("\u{23B5}"),
        "\u{2322}" | "\u{2323}" | "\u{23DD}" | "\u{FE36}" | "\u{032F}" => Some("\u{23DD}"),
        _ => None,
    }
}

fn map_accent_char(text: &str) -> Option<&'static str> {
    match text {
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
