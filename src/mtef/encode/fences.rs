//! Fence, interval, and Dirac bra-ket encoding for MTEF 5.0.

use crate::error::Result;
use crate::mathml::MathNode;
use crate::mtef::{
    dirac, fence, interval, TM_ANGLE, TM_BAR, TM_BRACE, TM_BRACK, TM_CEILING, TM_DBAR, TM_FLOOR,
    TM_OBRACK, TM_PAREN,
};

pub(super) type ScriptedCloseFence = (usize, Vec<u8>, Vec<u8>);

impl super::MathmlEncoder {
    /// Look ahead from an opening fence at `open_idx` for a matching closing fence
    /// that is the base of `<msup>`, `<msub>`, or `<msubsup>`.
    pub(super) fn find_scripted_close_fence(
        &mut self,
        children: &[MathNode],
        open_idx: usize,
        selector: u8,
    ) -> Result<Option<ScriptedCloseFence>> {
        let mut depth = 0usize;
        for (idx, child) in children.iter().enumerate().skip(open_idx + 1) {
            if child.tag == "mo" {
                let text = child.text.trim();
                if open_fence_selector(text) == Some(selector)
                    && !close_fence_matches(selector, text)
                {
                    depth += 1;
                } else if close_fence_matches(selector, text) {
                    if depth == 0 {
                        return Ok(None);
                    }
                    depth -= 1;
                }
            } else if depth == 0 && matches!(child.tag.as_str(), "msup" | "msub" | "msubsup") {
                if let Some(base) = child.children.first() {
                    if base.tag == "mo" && close_fence_matches(selector, base.text.trim()) {
                        let (sub_bytes, sup_bytes) = match child.tag.as_str() {
                            "msub" => (self.nth(child, 1)?, Vec::new()),
                            "msup" => (Vec::new(), self.nth(child, 1)?),
                            _ => (self.nth(child, 1)?, self.nth(child, 2)?),
                        };
                        return Ok(Some((idx, sub_bytes, sup_bytes)));
                    }
                }
            }
        }
        Ok(None)
    }

    /// Encode an `<mrow data-omml-d="true">` (or stretchy-fence `<mrow>`) into a delimiter template.
    pub(super) fn try_encode_omml_delimiter(&mut self, node: &MathNode) -> Result<Option<Vec<u8>>> {
        let children = &node.children;
        if children.is_empty() {
            return Ok(Some(Vec::new()));
        }

        let first_node = children
            .first()
            .filter(|child| child.tag == "mo" && child.attr("stretchy") == Some("true"));
        let last_node = if children.len() > 1 {
            children
                .last()
                .filter(|child| child.tag == "mo" && child.attr("stretchy") == Some("true"))
        } else {
            None
        };
        let first_mo = first_node
            .map(|child| child.text.trim())
            .filter(|text| !text.is_empty());
        let last_mo = last_node
            .map(|child| child.text.trim())
            .filter(|text| !text.is_empty());

        let start = usize::from(first_node.is_some());
        let end = if last_node.is_some() {
            children.len() - 1
        } else {
            children.len()
        };
        if start > end {
            return Ok(None);
        }
        let inner_slice = &children[start..end];

        // 1. Dirac bra-ket `<a | b>`, `|b>`, `<a|`.
        let is_left_angle = matches!(first_mo, Some("\u{2329}" | "\u{27E8}" | "<"));
        let is_right_angle = matches!(last_mo, Some("\u{232A}" | "\u{27E9}" | ">"));
        if is_left_angle && is_right_angle {
            if let Some(bar_pos) = inner_slice.iter().position(|child| {
                child.tag == "mo"
                    && child.attr("stretchy") == Some("true")
                    && child.text.trim() == "|"
            }) {
                let left_bytes = self.encode_children_slice(&inner_slice[..bar_pos])?;
                let right_bytes = self.encode_children_slice(&inner_slice[bar_pos + 1..])?;
                return Ok(Some(dirac(
                    Some(&left_bytes),
                    Some(&right_bytes),
                    &[],
                    &[],
                    &[],
                )));
            }
        }
        if first_mo == Some("|") && is_right_angle {
            let right_bytes = self.encode_children_slice(inner_slice)?;
            return Ok(Some(dirac(None, Some(&right_bytes), &[], &[], &[])));
        }
        if is_left_angle && last_mo == Some("|") {
            let left_bytes = self.encode_children_slice(inner_slice)?;
            return Ok(Some(dirac(Some(&left_bytes), None, &[], &[], &[])));
        }

        // 2. Unmatched interval fences (`[a, b)`, `(a, b]`, `]a, b[`, etc.).
        if let (Some(left_str), Some(right_str)) = (first_mo, last_mo) {
            if let (Some(left_kind), Some(right_kind)) = (
                interval_fence_kind(left_str),
                interval_fence_kind(right_str),
            ) {
                let is_matched_paren = left_str == "(" && right_str == ")";
                let is_matched_brack = left_str == "[" && right_str == "]";
                if !is_matched_paren && !is_matched_brack {
                    let inner_bytes = self.encode_children_slice(inner_slice)?;
                    return Ok(Some(interval(
                        left_kind,
                        right_kind,
                        &inner_bytes,
                        &[],
                        &[],
                    )));
                }
            }
        }

        // 3. Standard matched or single-sided fence.
        match (
            first_mo.and_then(open_fence_selector),
            last_mo.and_then(close_fence_selector),
        ) {
            (Some(left_sel), Some(right_sel)) if left_sel == right_sel => {
                let inner_bytes = self.encode_children_slice(inner_slice)?;
                Ok(Some(fence(left_sel, true, true, &inner_bytes, &[], &[])))
            }
            (Some(left_sel), None) if last_mo.is_none() => {
                let inner_bytes = self.encode_children_slice(inner_slice)?;
                Ok(Some(fence(left_sel, true, false, &inner_bytes, &[], &[])))
            }
            (None, Some(right_sel)) if first_mo.is_none() => {
                let inner_bytes = self.encode_children_slice(inner_slice)?;
                Ok(Some(fence(right_sel, false, true, &inner_bytes, &[], &[])))
            }
            _ => Ok(None),
        }
    }
}

/// Return whether `node` is an `<mrow>` bounded by prefix/postfix stretchy fence operators.
pub(super) fn is_stretchy_fence_mrow(node: &MathNode) -> bool {
    let children = &node.children;
    children.len() >= 2
        && children.first().is_some_and(|child| {
            child.tag == "mo"
                && child.attr("stretchy") == Some("true")
                && child.attr("form") == Some("prefix")
        })
        && children.last().is_some_and(|child| {
            child.tag == "mo"
                && child.attr("stretchy") == Some("true")
                && child.attr("form") == Some("postfix")
        })
        && !children[1..children.len() - 1].iter().any(|child| {
            child.tag == "mo"
                && child.attr("stretchy") == Some("true")
                && matches!(child.attr("form"), Some("prefix" | "postfix"))
        })
}

/// Map an opening fence text to its MTEF fence selector (`TM_*`).
pub(super) fn open_fence_selector(text: &str) -> Option<u8> {
    match text {
        "(" => Some(TM_PAREN),
        "[" => Some(TM_BRACK),
        "{" => Some(TM_BRACE),
        "|" => Some(TM_BAR),
        "\u{2016}" | "\u{2225}" => Some(TM_DBAR),
        "\u{2329}" | "\u{27E8}" => Some(TM_ANGLE),
        "\u{230A}" => Some(TM_FLOOR),
        "\u{2308}" => Some(TM_CEILING),
        "\u{301A}" | "\u{27E6}" => Some(TM_OBRACK),
        _ => None,
    }
}

/// Map a closing fence text to its MTEF fence selector (`TM_*`).
pub(super) fn close_fence_selector(text: &str) -> Option<u8> {
    match text {
        ")" => Some(TM_PAREN),
        "]" => Some(TM_BRACK),
        "}" => Some(TM_BRACE),
        "|" => Some(TM_BAR),
        "\u{2016}" | "\u{2225}" => Some(TM_DBAR),
        "\u{232A}" | "\u{27E9}" => Some(TM_ANGLE),
        "\u{230B}" => Some(TM_FLOOR),
        "\u{2309}" => Some(TM_CEILING),
        "\u{301B}" | "\u{27E7}" => Some(TM_OBRACK),
        _ => None,
    }
}

/// Return whether `close_str` matches `open_selector`.
pub(super) fn close_fence_matches(open_selector: u8, close_str: &str) -> bool {
    close_fence_selector(close_str) == Some(open_selector)
}

fn interval_fence_kind(text: &str) -> Option<u8> {
    match text {
        "(" => Some(0),
        ")" => Some(1),
        "[" => Some(2),
        "]" => Some(3),
        _ => None,
    }
}
