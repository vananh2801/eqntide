//! Structural equation editor independent of the window system and UI framework.
//!
//! # Architecture & Data Flow
//!
//! ```text
//!   MathNode (eqntide AST)
//!        |
//!        | import_math_node() -> assigns monotonic NodeId per node
//!        v
//!   +---------------------------------------------------------------+
//!   | Editor                                                        |
//!   |   state: Snapshot { root: Node, caret: Caret, anchor }        |
//!   |   undo / redo: Vec<Snapshot>                                  |
//!   +---------------------------------------------------------------+
//!        |                                    |
//!        | Edit actions via transact()        | Navigation via canonical_stops()
//!        v                                    v
//!   +-----------------------------+      +--------------------------+
//!   | Snapshot Transaction        |      | Stop Deduplication       |
//!   | 1. Clone `state`            |      | 1. Walk `root` tree      |
//!   | 2. Mutate tree & caret      |      | 2. Group shared boundaries|
//!   | 3. If `root` changed:       |      | 3. Pick highest-priority |
//!   |    push undo, clear redo,   |      |    stop per group        |
//!   |    bump `revision`          |      | 4. Map any caret -> idx  |
//!   +-----------------------------+      +--------------------------+
//!        |
//!        v
//!   Node::to_math_node() -> MathNode (lossless round-trip export)
//! ```
#![forbid(unsafe_code)]

mod deletion;
mod editing;
mod navigation;
mod node;
mod stops;
mod structures;

#[cfg(test)]
mod tests;

pub use editing::Editor;
pub use navigation::{Caret, Move};
pub use node::{is_opaque_tag, Node, NodeId};
