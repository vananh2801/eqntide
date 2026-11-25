//! Editor tree nodes, identifiers, and structural node classification.
//!
//! Each [`Node`] carries a stable [`NodeId`] across snapshots so the UI can
//! correlate rendered layout regions with editing targets and preserve undo state.

use eqntide::MathNode;

/// Monotonic identifier uniquely assigned to each node in an [`crate::Editor`] tree.
pub type NodeId = u64;

/// Editable MathML-shaped node stored inside [`crate::Editor`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    /// Stable node identifier within the editor session.
    pub id: NodeId,
    /// MathML element tag name (e.g., `"mi"`, `"mfrac"`, `"mtable"`).
    pub tag: String,
    /// Character content for token elements; empty for structural containers.
    pub text: String,
    /// Preserved MathML attributes in `(name, value)` order.
    pub attrs: Vec<(String, String)>,
    /// Ordered child nodes.
    pub children: Vec<Node>,
}

impl Node {
    /// Returns `true` if this node is a MathML token element (`mi`, `mn`, `mo`, `mtext`, `ms`).
    pub fn is_token(&self) -> bool {
        matches!(self.tag.as_str(), "mi" | "mn" | "mo" | "mtext" | "ms")
    }

    /// Returns `true` if this node is a linear row-like container (`math`, `mrow`, `mtd`, `msqrt`).
    pub fn is_row(&self) -> bool {
        matches!(self.tag.as_str(), "math" | "mrow" | "mtd" | "msqrt")
    }

    /// Returns `true` if this node has an unrecognized tag preserved atomically by the editor.
    pub fn is_opaque(&self) -> bool {
        is_opaque_tag(&self.tag)
    }

    /// Returns `true` if this `<mo>` acts as a stretchy or prefix/infix/postfix fence delimiter.
    pub fn is_fence_delimiter(&self) -> bool {
        self.tag == "mo"
            && self.attrs.iter().any(|(key, value)| {
                (key == "stretchy" && value == "true")
                    || (key == "form" && matches!(value.as_str(), "prefix" | "infix" | "postfix"))
            })
    }

    /// Converts this editor subtree back into a [`MathNode`] AST without losing attributes or opaque nodes.
    pub fn to_math_node(&self) -> MathNode {
        MathNode {
            tag: self.tag.clone(),
            text: self.text.clone(),
            attrs: self.attrs.clone(),
            children: self.children.iter().map(Node::to_math_node).collect(),
        }
    }

    /// Recursively finds an immutable reference to the node matching `id`.
    pub(crate) fn find(&self, id: NodeId) -> Option<&Node> {
        if self.id == id {
            Some(self)
        } else {
            self.children.iter().find_map(|child| child.find(id))
        }
    }

    /// Recursively finds a mutable reference to the node matching `id`.
    pub(crate) fn find_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        if self.id == id {
            Some(self)
        } else {
            self.children
                .iter_mut()
                .find_map(|child| child.find_mut(id))
        }
    }

    /// Populates `path` with child indices from `self` down to `id`, returning `true` if found.
    pub(crate) fn path(&self, id: NodeId, path: &mut Vec<usize>) -> bool {
        if self.id == id {
            return true;
        }
        for (index, child) in self.children.iter().enumerate() {
            path.push(index);
            if child.path(id, path) {
                return true;
            }
            path.pop();
        }
        false
    }

    /// Follows a validated child-index `path` from `self` to a descendant node.
    pub(crate) fn at(&self, path: &[usize]) -> &Node {
        path.iter().fold(self, |node, index| &node.children[*index])
    }
}

/// Returns `true` for MathML tags without editor rules, which remain atomic and are preserved verbatim.
pub fn is_opaque_tag(tag: &str) -> bool {
    !matches!(
        tag,
        "math"
            | "mrow"
            | "mtd"
            | "msqrt"
            | "mi"
            | "mn"
            | "mo"
            | "mtext"
            | "ms"
            | "mfrac"
            | "mroot"
            | "msub"
            | "msup"
            | "msubsup"
            | "munder"
            | "mover"
            | "munderover"
            | "mtable"
            | "mtr"
            | "mlabeledtr"
            | "mfenced"
            | "menclose"
            | "mstyle"
            | "mpadded"
            | "mphantom"
            | "mspace"
            | "mmultiscripts"
            | "mprescripts"
            | "none"
    )
}

/// Imports a [`MathNode`] subtree into [`Node`]s with fresh monotonic [`NodeId`]s,
/// wrapping direct `<mtext>` children of structural containers in `<mrow>` slots.
pub(crate) fn import_math_node(math: &MathNode, next_id: &mut NodeId) -> Node {
    let id = *next_id;
    *next_id += 1;
    let mut node = Node {
        id,
        tag: math.tag.clone(),
        text: math.text.clone(),
        attrs: math.attrs.clone(),
        children: Vec::with_capacity(math.children.len()),
    };
    let needs_text_rows = !node.is_row() && !node.is_opaque() && !node.is_token();
    for child in &math.children {
        let child = import_math_node(child, next_id);
        if needs_text_rows && child.tag == "mtext" {
            // A row gives the text slot Math-mode insertion points on either side.
            node.children.push(Node {
                id: *next_id,
                tag: "mrow".into(),
                text: String::new(),
                attrs: Vec::new(),
                children: vec![child],
            });
            *next_id += 1;
        } else {
            node.children.push(child);
        }
    }
    node
}

/// Returns `true` if `node` is an empty token (excluding fences) or contains only effectively empty nodes.
pub(crate) fn is_effectively_empty(node: &Node) -> bool {
    (node.is_token() && !node.is_fence_delimiter() && node.text.is_empty())
        || (matches!(node.tag.as_str(), "math" | "mrow" | "mtd" | "msqrt")
            && node.text.is_empty()
            && node.children.iter().all(is_effectively_empty))
}

/// Returns `true` if `tag` denotes a structural MathML container that wraps child slots.
pub(crate) fn is_structure_tag(tag: &str) -> bool {
    matches!(
        tag,
        "mfrac"
            | "mroot"
            | "msqrt"
            | "msub"
            | "msup"
            | "msubsup"
            | "munder"
            | "mover"
            | "munderover"
            | "menclose"
            | "mfenced"
    )
}

/// Returns `true` if `node` is an `<mrow>` whose leading child is an opening/infix fence delimiter.
pub(crate) fn is_fence_mrow(node: &Node) -> bool {
    node.tag == "mrow"
        && node.children.first().is_some_and(|child| {
            child.is_fence_delimiter()
                && !child
                    .attrs
                    .iter()
                    .any(|(key, value)| key == "form" && value == "postfix")
        })
}

/// Resets `node` in place to an empty element with the given `tag`, preserving its [`NodeId`].
pub(crate) fn clear_as_tag(node: &mut Node, tag: &str) {
    node.tag = tag.into();
    node.text.clear();
    node.attrs.clear();
    node.children.clear();
}
