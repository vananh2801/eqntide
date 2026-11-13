//! Recursion-depth and child-dispatch policy for MTEF encoding.

use crate::error::{Error, Result};
use crate::mathml::MathNode;

use super::{MathmlEncoder, MAX_DEPTH};

impl MathmlEncoder {
    /// Dispatch one MathML node while enforcing the encoder recursion limit.
    pub(super) fn dispatch(&mut self, node: &MathNode) -> Result<Vec<u8>> {
        if self.depth >= MAX_DEPTH {
            return Err(Error::TooDeep);
        }
        self.depth += 1;
        let result = self.dispatch_inner(node);
        self.depth -= 1;
        result
    }

    /// Encode child `idx` of `node`, or nothing when it does not exist.
    pub(super) fn nth(&mut self, node: &MathNode, idx: usize) -> Result<Vec<u8>> {
        match node.children.get(idx) {
            Some(child) => self.dispatch(child),
            None => Ok(Vec::new()),
        }
    }
}
