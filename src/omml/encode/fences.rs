//! Fence recognition, Dirac bra-kets, and stack-based OMML delimiter (`<m:d>`) encoding.

use crate::error::Result;
use crate::mathml::MathNode;
use crate::xml::Element;

use super::{m_el, m_el_children, m_val_el, MathmlEncoder};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FenceKind {
    Open,
    Close,
    Neutral,
}

#[derive(Clone)]
enum MixedItem<'a> {
    Raw(&'a MathNode),
    Built(Element),
}

impl MathmlEncoder {
    /// Encode `[optional <mo>⟨</mo>, left, <mo>|</mo>, right, optional <mo>⟩</mo>]`
    /// into a 2-slot OMML `<m:d>` delimiter with `sepChr="|"`.
    pub(super) fn try_encode_dirac(&mut self, nodes: &[MathNode]) -> Result<Option<Element>> {
        if nodes.len() < 3 || nodes.len() > 5 {
            return Ok(None);
        }
        let has_left = nodes.first().is_some_and(|node| {
            is_stretchy_fence_mo(node) && matches!(node.text.trim(), "\u{27E8}" | "\u{2329}")
        });
        let has_right = nodes.last().is_some_and(|node| {
            is_stretchy_fence_mo(node) && matches!(node.text.trim(), "\u{27E9}" | "\u{232A}")
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
            .filter_map(|(idx, node)| {
                (is_stretchy_fence_mo(node) && node.text.trim() == "|").then_some(idx)
            })
            .collect();
        if sep_positions.len() != 1 {
            return Ok(None);
        }
        let sep_idx = sep_positions[0];
        let left_els = self.encode_nary_and_items(&inner[..sep_idx])?;
        let right_els = self.encode_nary_and_items(&inner[sep_idx + 1..])?;
        let beg_chr = if has_left { "\u{27E8}" } else { "" };
        let end_chr = if has_right { "\u{27E9}" } else { "" };
        Ok(Some(build_delimiter(
            beg_chr,
            end_chr,
            Some("|"),
            vec![left_els, right_els],
        )))
    }

    /// Match nested/unpaired stretchy fence `<mo>` siblings using a stack and wrap each
    /// delimited span in `<m:d>`.
    pub(super) fn encode_row_with_fences(&mut self, nodes: &[MathNode]) -> Result<Vec<Element>> {
        let mut root_items: Vec<MixedItem<'_>> = Vec::new();
        let mut stack: Vec<(String, Vec<MixedItem<'_>>)> = Vec::new();

        for node in nodes {
            if is_stretchy_fence_mo(node) {
                let fence_text = node.text.trim();
                match classify_fence_char(fence_text).unwrap_or(FenceKind::Neutral) {
                    FenceKind::Open => {
                        stack.push((fence_text.to_owned(), Vec::new()));
                    }
                    FenceKind::Close => {
                        if let Some((open_ch, inner_items)) = stack.pop() {
                            let inner_els = self.encode_mixed_items(&inner_items)?;
                            let delimiter =
                                build_delimiter(&open_ch, fence_text, None, vec![inner_els]);
                            if let Some((_, parent_items)) = stack.last_mut() {
                                parent_items.push(MixedItem::Built(delimiter));
                            } else {
                                root_items.push(MixedItem::Built(delimiter));
                            }
                        } else {
                            let taken = std::mem::take(&mut root_items);
                            let inner_els = self.encode_mixed_items(&taken)?;
                            let delimiter = build_delimiter("", fence_text, None, vec![inner_els]);
                            root_items.push(MixedItem::Built(delimiter));
                        }
                    }
                    FenceKind::Neutral => {
                        if stack
                            .last()
                            .is_some_and(|(open_ch, _)| open_ch == fence_text)
                        {
                            let Some((open_ch, inner_items)) = stack.pop() else {
                                continue;
                            };
                            let inner_els = self.encode_mixed_items(&inner_items)?;
                            let delimiter =
                                build_delimiter(&open_ch, fence_text, None, vec![inner_els]);
                            if let Some((_, parent_items)) = stack.last_mut() {
                                parent_items.push(MixedItem::Built(delimiter));
                            } else {
                                root_items.push(MixedItem::Built(delimiter));
                            }
                        } else {
                            stack.push((fence_text.to_owned(), Vec::new()));
                        }
                    }
                }
            } else if let Some((_, top_items)) = stack.last_mut() {
                top_items.push(MixedItem::Raw(node));
            } else {
                root_items.push(MixedItem::Raw(node));
            }
        }

        // Close any remaining unmatched opening fences (e.g. `{ mtable` system of equations).
        while let Some((open_ch, inner_items)) = stack.pop() {
            let inner_els = self.encode_mixed_items(&inner_items)?;
            let delimiter = build_delimiter(&open_ch, "", None, vec![inner_els]);
            if let Some((_, parent_items)) = stack.last_mut() {
                parent_items.push(MixedItem::Built(delimiter));
            } else {
                root_items.push(MixedItem::Built(delimiter));
            }
        }

        self.encode_mixed_items(&root_items)
    }

    fn encode_mixed_items(&mut self, items: &[MixedItem<'_>]) -> Result<Vec<Element>> {
        let mut out = Vec::new();
        let mut idx = 0;
        while idx < items.len() {
            match &items[idx] {
                MixedItem::Built(element) => {
                    out.push(element.clone());
                    idx += 1;
                }
                MixedItem::Raw(_) => {
                    let mut raw_run = Vec::new();
                    while idx < items.len() {
                        if let MixedItem::Raw(node) = items[idx] {
                            raw_run.push(node.clone());
                            idx += 1;
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
}

/// Classify a delimiter string as opening, closing, or neutral.
pub(super) fn classify_fence_char(value: &str) -> Option<FenceKind> {
    match value {
        "(" | "[" | "{" | "\u{2308}" | "\u{230A}" | "\u{2329}" | "\u{27E8}" | "\u{27E6}"
        | "\u{301A}" => Some(FenceKind::Open),
        ")" | "]" | "}" | "\u{2309}" | "\u{230B}" | "\u{232A}" | "\u{27E9}" | "\u{27E7}"
        | "\u{301B}" => Some(FenceKind::Close),
        "|" | "\u{2016}" | "\u{2223}" | "\u{2225}" => Some(FenceKind::Neutral),
        _ => None,
    }
}

/// Return whether `node` is an `<mo>` delimiter that participates in `<m:d>` grouping.
pub(super) fn is_stretchy_fence_mo(node: &MathNode) -> bool {
    node.tag == "mo"
        && node.attr("stretchy") != Some("false")
        && classify_fence_char(node.text.trim()).is_some()
}

/// Construct an `<m:d>` OMML delimiter element around one or more operand `<m:e>` slots.
pub(super) fn build_delimiter(
    beg_chr: &str,
    end_chr: &str,
    sep_chr: Option<&str>,
    e_groups: Vec<Vec<Element>>,
) -> Element {
    let mut delimiter = m_el("d");
    let mut delim_pr = m_el("dPr");
    delim_pr.push(m_val_el("begChr", beg_chr));
    if let Some(sep) = sep_chr {
        delim_pr.push(m_val_el("sepChr", sep));
    }
    delim_pr.push(m_val_el("endChr", end_chr));
    delimiter.push(delim_pr);
    if e_groups.is_empty() {
        delimiter.push(m_el("e"));
    } else {
        for group in e_groups {
            delimiter.push(m_el_children("e", group));
        }
    }
    delimiter
}
