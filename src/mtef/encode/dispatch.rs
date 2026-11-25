//! Recursion-depth guard and leaf-unwrapping helpers for MTEF encoding.

use crate::error::{Error, Result};
use crate::mathml::MathNode;

use super::{MathmlEncoder, MAX_DEPTH};

impl MathmlEncoder {
    /// Dispatch one MathML node while enforcing [`MAX_DEPTH`].
    pub(super) fn dispatch(&mut self, node: &MathNode) -> Result<Vec<u8>> {
        if self.depth >= MAX_DEPTH {
            return Err(Error::TooDeep);
        }
        self.depth += 1;
        let result = self.dispatch_inner(node);
        self.depth -= 1;
        result
    }

    /// Encode child `idx` of `node`, returning empty bytes when absent.
    pub(super) fn nth(&mut self, node: &MathNode, idx: usize) -> Result<Vec<u8>> {
        match node.children.get(idx) {
            Some(child) => self.dispatch(child),
            None => Ok(Vec::new()),
        }
    }
}

/// Unwrap single-child `<mstyle>` / `<mrow>` / `<mpadded>` wrappers to get trimmed leaf text.
pub(super) fn unwrap_leaf_text(mut node: &MathNode) -> Option<&str> {
    loop {
        if !node.text.trim().is_empty() {
            return Some(node.text.trim());
        }
        if matches!(node.tag.as_str(), "mstyle" | "mrow" | "mpadded") && node.children.len() == 1 {
            node = &node.children[0];
        } else {
            return Some(node.text.trim());
        }
    }
}
