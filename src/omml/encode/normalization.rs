//! Lossless MathML cleanup performed before OMML element encoding.

use crate::mathml::MathNode;
use crate::symbols::{is_prime_char, prime_order};

use super::{is_empty_node, KNOWN_FUNCTIONS};

fn is_single_ascii_letter_mi(node: &MathNode) -> Option<char> {
    if node.tag != "mi" || node.attr("mathvariant") == Some("normal") {
        return None;
    }
    let mut chars = node.text.chars();
    let ch = chars.next()?;
    (chars.next().is_none() && ch.is_ascii_alphabetic()).then_some(ch)
}

fn merge_function_names_in_list(children: Vec<MathNode>) -> Vec<MathNode> {
    let mut out = Vec::with_capacity(children.len());
    let mut i = 0;
    while i < children.len() {
        if is_single_ascii_letter_mi(&children[i]).is_some() {
            let mut j = i;
            let mut combined = String::new();
            while j < children.len() {
                if let Some(ch) = is_single_ascii_letter_mi(&children[j]) {
                    combined.push(ch);
                    j += 1;
                } else {
                    break;
                }
            }
            if j < children.len()
                && matches!(
                    children[j].tag.as_str(),
                    "msup" | "msub" | "msubsup" | "munder" | "mover" | "munderover"
                )
                && !children[j].children.is_empty()
            {
                if let Some(base_ch) = is_single_ascii_letter_mi(&children[j].children[0]) {
                    let mut with_base = combined.clone();
                    with_base.push(base_ch);
                    if KNOWN_FUNCTIONS.contains(&with_base.as_str()) {
                        let mut script_node = children[j].clone();
                        let mut fn_mi = MathNode::leaf("mi", &with_base);
                        fn_mi.set_attr("mathvariant", "normal");
                        script_node.children[0] = fn_mi;
                        out.push(script_node);
                        i = j + 1;
                        continue;
                    }
                }
            }
            if combined.len() >= 2 && KNOWN_FUNCTIONS.contains(&combined.as_str()) {
                let mut fn_mi = MathNode::leaf("mi", &combined);
                fn_mi.set_attr("mathvariant", "normal");
                out.push(fn_mi);
                i = j;
                continue;
            }
            if combined.len() >= 3 {
                let prefix = &combined[..combined.len() - 1];
                if KNOWN_FUNCTIONS.contains(&prefix) {
                    let mut fn_mi = MathNode::leaf("mi", prefix);
                    fn_mi.set_attr("mathvariant", "normal");
                    out.push(fn_mi);
                    i = j - 1;
                    continue;
                }
            }
        }
        out.push(children[i].clone());
        i += 1;
    }
    out
}

pub(super) fn collect_text(node: &MathNode) -> String {
    let mut text = node.text.clone();
    for child in &node.children {
        text.push_str(&collect_text(child));
    }
    text
}

fn prime_text(order: u8) -> &'static str {
    match order {
        1 => "'",
        2 => "''",
        _ => "'''",
    }
}

fn nbsp_mi() -> MathNode {
    let mut space = MathNode::leaf("mi", "\u{00A0}");
    space.set_attr("mathvariant", "normal");
    space
}

/// Normalize equivalent MathML forms before structural OMML encoding.
pub(super) fn normalize_mathml(node: &MathNode) -> MathNode {
    if node.tag == "mo" && node.text.trim() == "\\" {
        let mut op = node.clone();
        op.text = "\u{2216}".to_string();
        return op;
    }

    let mut children: Vec<MathNode> = node.children.iter().map(normalize_mathml).collect();
    if matches!(
        node.tag.as_str(),
        "msub" | "msup" | "msubsup" | "munder" | "mover" | "munderover"
    ) && children.len() >= 2
        && is_empty_node(&children[0])
        && !children[1..].iter().all(is_empty_node)
    {
        children[0] = nbsp_mi();
    }
    if node.tag == "msup" && children.len() == 2 {
        if let Some(order) = prime_order(&collect_text(&children[1])) {
            let base = children.remove(0);
            let mut row = if base.tag == "mrow" {
                base.children
            } else if is_empty_node(&base) {
                Vec::new()
            } else {
                vec![base]
            };
            row.push(MathNode::leaf("mo", prime_text(order)));
            return MathNode::with_children("mrow", row);
        }
    }
    if node.tag == "mroot" && children.len() == 2 && is_empty_node(&children[1]) {
        return MathNode::with_children("msqrt", vec![children.remove(0)]);
    }
    if node.tag == "mmultiscripts" && !children.is_empty() && is_empty_node(&children[0]) {
        let non_empty: Vec<&MathNode> = children[1..]
            .iter()
            .filter(|child| {
                child.tag != "mprescripts" && child.tag != "none" && !is_empty_node(child)
            })
            .collect();
        if non_empty.len() == 1 {
            return non_empty[0].clone();
        }
    }

    let mut cleaned = Vec::with_capacity(children.len());
    for child in children {
        if child.tag == "msup" && child.children.len() == 2 {
            if let Some(order) = prime_order(&collect_text(&child.children[1])) {
                let base = child.children.into_iter().next().unwrap();
                if base.tag == "mrow" {
                    cleaned.extend(base.children);
                } else if !is_empty_node(&base) {
                    cleaned.push(base);
                }
                cleaned.push(MathNode::leaf("mo", prime_text(order)));
                continue;
            }
        }
        if child.tag == "msubsup" && child.children.len() == 3 {
            if let Some(order) = prime_order(&collect_text(&child.children[2])) {
                let mut parts = child.children.into_iter();
                let base = parts.next().unwrap();
                let sub = parts.next().unwrap();
                let primed = MathNode::with_children(
                    "mrow",
                    vec![base, MathNode::leaf("mo", prime_text(order))],
                );
                cleaned.push(MathNode::with_children("msub", vec![primed, sub]));
                continue;
            }
        }
        if child.tag == "mi" {
            let text = child.text.trim();
            if let Some(index) = text.find(|ch: char| is_prime_char(ch)) {
                if index == 0 {
                    cleaned.push(child);
                    continue;
                }
                let (name, primes) = text.split_at(index);
                if let Some(order) = prime_order(primes) {
                    let mut base = MathNode::leaf("mi", name);
                    base.attrs = child.attrs.clone();
                    cleaned.push(base);
                    cleaned.push(MathNode::leaf("mo", prime_text(order)));
                    continue;
                }
            }
        }
        if matches!(
            child.tag.as_str(),
            "msub" | "msup" | "msubsup" | "munder" | "mover" | "munderover"
        ) && child.children.len() >= 2
            && child.children[1..].iter().all(is_empty_node)
        {
            let base = child.children.into_iter().next().unwrap();
            if !is_empty_node(&base) {
                cleaned.push(base);
            }
            cleaned.push(nbsp_mi());
        } else {
            cleaned.push(child);
        }
    }
    MathNode {
        tag: node.tag.clone(),
        text: node.text.clone(),
        attrs: node.attrs.clone(),
        children: merge_function_names_in_list(cleaned),
    }
}
