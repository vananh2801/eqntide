//! Caret placement, structural cursor movement, and selection commands.

use unicode_segmentation::UnicodeSegmentation;

use crate::editing::Editor;
use crate::node::NodeId;

/// Logical cursor position within the editor tree: a target [`NodeId`] and character or child offset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Caret {
    /// Target node containing the caret.
    pub node: NodeId,
    /// Character offset inside a token node, or child index inside a container node.
    pub offset: usize,
}

/// Logical horizontal and placeholder navigation commands for [`Editor::move_caret`].
#[derive(Clone, Copy, Debug)]
pub enum Move {
    /// Move one canonical stop backward.
    Left,
    /// Move one canonical stop forward.
    Right,
    /// Jump to the first canonical stop in the root expression.
    Home,
    /// Jump to the last canonical stop in the root expression.
    End,
    /// Jump to the next empty placeholder slot (wrapping around).
    NextPlaceholder,
    /// Jump to the previous empty placeholder slot (wrapping around).
    PreviousPlaceholder,
}

impl Editor {
    /// Moves the caret to `caret` if valid and snaps `<mtext>` offsets to grapheme boundaries.
    ///
    /// When `extend` is `true`, preserves or initializes the selection anchor at the prior caret.
    pub fn set_caret(&mut self, mut caret: Caret, extend: bool) {
        let Some(node) = self.node(caret.node) else {
            return;
        };
        let max_offset = if node.is_token() {
            node.text.chars().count()
        } else {
            node.children.len()
        };
        if caret.offset > max_offset {
            return;
        }
        if node.tag == "mtext" && caret.offset > 0 && caret.offset < max_offset {
            let mut grapheme_end = 0;
            for grapheme in node.text.graphemes(true) {
                grapheme_end += grapheme.chars().count();
                if grapheme_end >= caret.offset {
                    caret.offset = grapheme_end;
                    break;
                }
            }
        }
        if extend {
            self.state.anchor.get_or_insert(self.state.caret);
        } else {
            self.state.anchor = None;
        }
        self.state.caret = caret;
    }

    /// Selects the entire top-level `<math>` expression.
    pub fn select_all(&mut self) {
        self.state.anchor = Some(Caret {
            node: self.root().id,
            offset: 0,
        });
        self.state.caret = Caret {
            node: self.root().id,
            offset: self.root().children.len(),
        };
    }

    /// Selects the node identified by `id` as a span in its parent container, or selects all if root.
    pub fn select_node(&mut self, id: NodeId) {
        let Some(mut path) = self.path(id) else {
            return;
        };
        if let Some(child_idx) = path.pop() {
            let parent_id = self.root().at(&path).id;
            self.state.anchor = Some(Caret {
                node: parent_id,
                offset: child_idx,
            });
            self.state.caret = Caret {
                node: parent_id,
                offset: child_idx + 1,
            };
        } else {
            self.select_all();
        }
    }

    /// Advances or rewinds the caret along deduplicated stops according to `movement`.
    pub fn move_caret(&mut self, movement: Move, extend: bool) {
        let (stops, caret_to_idx) = self.root().canonical_stops();
        let current_idx = caret_to_idx
            .iter()
            .find(|(caret, _)| *caret == self.caret())
            .map(|(_, idx)| *idx)
            .unwrap_or(0);
        let target_idx = match movement {
            Move::Left => current_idx.saturating_sub(1),
            Move::Right => (current_idx + 1).min(stops.len().saturating_sub(1)),
            Move::Home => 0,
            Move::End => stops.len().saturating_sub(1),
            Move::NextPlaceholder | Move::PreviousPlaceholder => {
                let placeholders: Vec<usize> = stops
                    .iter()
                    .enumerate()
                    .filter(|(_, stop_caret)| {
                        self.node(stop_caret.node).is_some_and(|node| {
                            node.text.is_empty()
                                && node.children.is_empty()
                                && (node.is_row() || node.is_token())
                        })
                    })
                    .map(|(index, _)| index)
                    .collect();
                if matches!(movement, Move::NextPlaceholder) {
                    placeholders
                        .iter()
                        .copied()
                        .find(|index| *index > current_idx)
                        .or_else(|| placeholders.first().copied())
                        .unwrap_or(current_idx)
                } else {
                    placeholders
                        .iter()
                        .copied()
                        .rev()
                        .find(|index| *index < current_idx)
                        .or_else(|| placeholders.last().copied())
                        .unwrap_or(current_idx)
                }
            }
        };
        if let Some(target_caret) = stops.get(target_idx) {
            self.set_caret(*target_caret, extend);
        }
    }
}
