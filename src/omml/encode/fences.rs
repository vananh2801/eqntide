//! Fence recognition used by row and delimiter encoding.

use crate::mathml::MathNode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FenceKind {
    Open,
    Close,
    Neutral,
}

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

pub(super) fn is_stretchy_fence_mo(node: &MathNode) -> bool {
    node.tag == "mo"
        && node.attr("stretchy") != Some("false")
        && classify_fence_char(node.text.trim()).is_some()
}
