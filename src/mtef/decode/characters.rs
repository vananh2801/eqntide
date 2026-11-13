//! MathML wrappers used for stretchy MathType character templates.

use crate::mathml::MathNode;

pub(super) fn append_prime(mut node: MathNode, prime_str: &str) -> MathNode {
    if node.tag == "mrow" {
        node.children.push(MathNode::leaf("mo", prime_str));
        node
    } else {
        MathNode::with_children("mrow", vec![node, MathNode::leaf("mo", prime_str)])
    }
}

pub(super) fn mover_stretchy_accent(base: MathNode, mo_str: &str) -> MathNode {
    let mut mo = MathNode::leaf("mo", mo_str);
    mo.set_attr("stretchy", "true");
    let mut mover = MathNode::with_children("mover", vec![base, mo]);
    mover.set_attr("accent", "true");
    mover
}

pub(super) fn munder_stretchy_accent(base: MathNode, mo_str: &str) -> MathNode {
    let mut mo = MathNode::leaf("mo", mo_str);
    mo.set_attr("stretchy", "true");
    let mut munder = MathNode::with_children("munder", vec![base, mo]);
    munder.set_attr("accentunder", "true");
    munder
}
