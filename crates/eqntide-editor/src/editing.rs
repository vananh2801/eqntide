//! Editor state, snapshot transactions, selection queries, and text/math insertion.

use eqntide::MathNode;

use crate::navigation::Caret;
use crate::node::{clear_as_tag, import_math_node, is_effectively_empty, Node, NodeId};
use crate::structures::{adjacent_table_row_caret, cell_edge_caret};

/// Structural MathML editor managing tree state, caret/selection, and undo history.
#[derive(Clone, Debug)]
pub struct Editor {
    pub(crate) state: Snapshot,
    pub(crate) next_id: NodeId,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    /// Monotonic counter incremented whenever the root tree or undo/redo state changes.
    pub revision: u64,
}

/// Immutable point-in-time editor state captured before and restored by undo/redo transactions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Snapshot {
    pub(crate) root: Node,
    pub(crate) caret: Caret,
    pub(crate) anchor: Option<Caret>,
}

impl Default for Editor {
    fn default() -> Self {
        Self::new(&MathNode::new("math"))
    }
}

impl Editor {
    /// Checks untrusted [`MathNode`] imports against depth and node-count limits before creating an editor.
    pub fn try_new(math: &MathNode) -> Result<Self, &'static str> {
        let mut stack = vec![(math, 0usize)];
        let mut node_count = 0;
        while let Some((node, depth)) = stack.pop() {
            node_count += 1;
            if depth >= 256 || node_count > 100_000 {
                return Err("Equation exceeds editor depth or node limit");
            }
            stack.extend(node.children.iter().map(|child| (child, depth + 1)));
        }
        Ok(Self::new(math))
    }

    /// Creates a new editor initialized with `math` and places the caret at the root start.
    pub fn new(math: &MathNode) -> Self {
        let mut next_id = 1;
        let root = import_math_node(math, &mut next_id);
        let caret = Caret {
            node: root.id,
            offset: 0,
        };
        Self {
            state: Snapshot {
                root,
                caret,
                anchor: None,
            },
            next_id,
            undo: vec![],
            redo: vec![],
            revision: 0,
        }
    }

    /// Returns the root `<math>` [`Node`].
    pub fn root(&self) -> &Node {
        &self.state.root
    }

    /// Exports the current editor tree as a [`MathNode`].
    pub fn math(&self) -> MathNode {
        self.state.root.to_math_node()
    }

    /// Returns the current [`Caret`].
    pub fn caret(&self) -> Caret {
        self.state.caret
    }

    /// Returns the current selection anchor, or `None` when no selection is active.
    pub fn anchor(&self) -> Option<Caret> {
        self.state.anchor
    }

    /// Looks up a node by [`NodeId`].
    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.state.root.find(id)
    }

    /// Resolves a child-index path to its [`NodeId`], if valid.
    pub fn id_at_path(&self, path: &[usize]) -> Option<NodeId> {
        let mut current = &self.state.root;
        for index in path {
            current = current.children.get(*index)?;
        }
        Some(current.id)
    }

    /// Returns the child-index path from the root to `id`, if `id` exists in the tree.
    pub fn path(&self, id: NodeId) -> Option<Vec<usize>> {
        let mut path = vec![];
        self.state.root.path(id, &mut path).then_some(path)
    }

    /// Returns the normalized `(container_id, start, end)` selection range, expanding
    /// cross-container selections to whole children of their lowest common ancestor.
    pub fn selection_span(&self) -> Option<(NodeId, usize, usize)> {
        let mut anchor = self.state.anchor?;
        let mut head = self.caret();
        let text_endpoint = |outside: Caret, inside: Caret| -> Option<Caret> {
            let text_node = self.node(inside.node).filter(|node| node.tag == "mtext")?;
            let row = self.node(outside.node).filter(|node| node.is_row())?;
            let index = row
                .children
                .iter()
                .position(|child| child.id == text_node.id)?;
            let offset = if outside.offset == index {
                0
            } else if outside.offset == index + 1 {
                text_node.text.chars().count()
            } else {
                return None;
            };
            Some(Caret {
                node: text_node.id,
                offset,
            })
        };
        if let Some(endpoint) = text_endpoint(anchor, head) {
            anchor = endpoint;
        } else if let Some(endpoint) = text_endpoint(head, anchor) {
            head = endpoint;
        }
        if anchor == head {
            return None;
        }
        if anchor.node == head.node {
            let node = self.node(head.node)?;
            let max_offset = if node.is_token() {
                node.text.chars().count()
            } else {
                node.children.len()
            };
            let start = anchor.offset.min(head.offset).min(max_offset);
            let end = anchor.offset.max(head.offset).min(max_offset);
            if start >= end {
                return None;
            }
            return Some((head.node, start, end));
        }
        let anchor_path = self.path(anchor.node)?;
        let head_path = self.path(head.node)?;
        let common_depth = anchor_path
            .iter()
            .zip(&head_path)
            .take_while(|(left, right)| left == right)
            .count();
        let parent = self.root().at(&anchor_path[..common_depth]);
        let anchor_idx = anchor_path
            .get(common_depth)
            .copied()
            .unwrap_or(anchor.offset);
        let head_idx = head_path.get(common_depth).copied().unwrap_or(head.offset);
        let (start, end) = if anchor_idx <= head_idx {
            (
                anchor_idx,
                head_idx + usize::from(head_path.len() > common_depth),
            )
        } else {
            (
                head_idx,
                anchor_idx + usize::from(anchor_path.len() > common_depth),
            )
        };
        let start = start.min(parent.children.len());
        let end = end.min(parent.children.len());
        if start >= end {
            return None;
        }
        Some((parent.id, start, end))
    }

    /// Exports the currently selected slice as a [`MathNode`], or `None` if nothing is selected.
    pub fn selected_math(&self) -> Option<MathNode> {
        let (id, start, end) = self.selection_span()?;
        let node = self.node(id)?;
        if node.is_token() {
            let char_count = node.text.chars().count();
            let start = start.min(char_count);
            let end = end.min(char_count);
            if start >= end {
                return None;
            }
            let mut result = node.to_math_node();
            result.text = node.text.chars().skip(start).take(end - start).collect();
            Some(result)
        } else {
            let start = start.min(node.children.len());
            let end = end.min(node.children.len());
            if start >= end {
                return None;
            }
            Some(MathNode::with_children(
                "mrow",
                node.children[start..end]
                    .iter()
                    .map(Node::to_math_node)
                    .collect(),
            ))
        }
    }

    /// Returns the [`NodeId`]s covered by the active selection for highlight rendering.
    pub fn selected_ids(&self) -> Vec<NodeId> {
        let Some((id, start, end)) = self.selection_span() else {
            return vec![];
        };
        let Some(node) = self.node(id) else {
            return vec![];
        };
        if node.is_token() {
            vec![id]
        } else {
            let start = start.min(node.children.len());
            let end = end.min(node.children.len());
            if start >= end {
                return vec![];
            }
            node.children[start..end]
                .iter()
                .map(|child| child.id)
                .collect()
        }
    }

    /// Restores the previous snapshot from the undo stack, if any.
    pub fn undo(&mut self) {
        if let Some(state) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.state, state));
            self.revision += 1;
        }
    }

    /// Reapplies a snapshot previously reverted by [`Editor::undo`].
    pub fn redo(&mut self) {
        if let Some(state) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.state, state));
            self.revision += 1;
        }
    }

    /// Replaces the entire equation tree with `math` as a single undoable transaction.
    pub fn replace(&mut self, math: &MathNode) {
        self.transact(|editor| {
            editor.state.root = import_math_node(math, &mut editor.next_id);
            editor.state.caret = Caret {
                node: editor.root().id,
                offset: editor.root().children.len(),
            };
            editor.state.anchor = None;
        });
    }

    /// Inserts `text` at the caret, extending compatible tokens or splicing new token nodes.
    pub fn insert_text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.transact(|editor| {
            editor.erase_selection();
            let caret = editor.caret();
            if let Some(node) = editor.state.root.find_mut(caret.node) {
                if node.is_token()
                    && !node.is_fence_delimiter()
                    && (node.tag == "mtext"
                        || text.chars().all(|ch| match node.tag.as_str() {
                            "mi" => ch.is_alphabetic(),
                            "mn" => ch.is_numeric() || ch == '.',
                            "mo" => !ch.is_alphanumeric() && !ch.is_whitespace(),
                            _ => true,
                        }))
                {
                    let byte_idx = char_byte_offset(&node.text, caret.offset);
                    node.text.insert_str(byte_idx, text);
                    editor.state.caret.offset += text.chars().count();
                    return;
                }
            }
            let nodes = text
                .chars()
                .filter(|ch| !ch.is_control())
                .map(|ch| {
                    MathNode::leaf(
                        if ch.is_numeric() {
                            "mn"
                        } else if ch.is_alphabetic() {
                            "mi"
                        } else if ch.is_whitespace() {
                            "mtext"
                        } else {
                            "mo"
                        },
                        &ch.to_string(),
                    )
                })
                .collect();
            editor.splice(nodes);
        });
    }

    /// Inserts `math` at the caret (unwrapping top-level `<math>` children) as one undoable step.
    pub fn insert_math(&mut self, math: &MathNode) {
        self.transact(|editor| {
            editor.erase_selection();
            let children = if math.tag == "math" {
                math.children.clone()
            } else {
                vec![math.clone()]
            };
            editor.splice(children);
        });
    }

    /// Executes `action` against a cloned [`Snapshot`], recording an undo entry only if the tree changed.
    pub(crate) fn transact(&mut self, action: impl FnOnce(&mut Self)) {
        let before = self.state.clone();
        action(self);
        if self.state.root != before.root {
            self.undo.push(before);
            if self.undo.len() > 200 {
                self.undo.remove(0);
            }
            self.redo.clear();
            self.revision += 1;
        }
    }

    /// Deletes the active selection while preserving matrix/row/fixed-arity structural invariants.
    pub(crate) fn erase_selection(&mut self) -> Option<MathNode> {
        let selected = self.selected_math()?;
        let (id, start, end) = self.selection_span()?;
        let tag = self.node(id)?.tag.clone();
        if tag == "mtable" {
            let len = self.node(id)?.children.len();
            let start = start.min(len);
            let end = end.min(len);
            if start < end && end - start < len {
                let table = self.state.root.find_mut(id)?;
                table.children.drain(start..end);
                let target_row = start.min(table.children.len() - 1);
                let target_caret = table
                    .children
                    .get(target_row)
                    .and_then(|row| row.children.first())
                    .map(|cell| cell_edge_caret(cell, false))
                    .unwrap_or(Caret {
                        node: id,
                        offset: target_row,
                    });
                self.state.caret = target_caret;
                self.state.anchor = None;
                return Some(selected);
            } else if start == 0 && end == len && len > 0 {
                let target = self.enclosing_structure_for_table(id);
                self.remove_node_and_place_caret(target);
                return Some(selected);
            }
        } else if tag == "mtr" {
            let len = self.node(id)?.children.len();
            let start = start.min(len);
            let end = end.min(len);
            if start == 0 && end == len && len > 0 {
                let mut path = self.path(id)?;
                if let Some(row_idx) = path.pop() {
                    let table_id = self.id_at_path(&path)?;
                    if self.node(table_id)?.tag == "mtable" {
                        let old_len = self.node(table_id)?.children.len();
                        if old_len >= 2 {
                            let table = self.state.root.find_mut(table_id)?;
                            table.children.remove(row_idx);
                            if let Some(caret) =
                                adjacent_table_row_caret(table, row_idx, old_len, true)
                            {
                                self.state.caret = caret;
                            }
                            self.state.anchor = None;
                            return Some(selected);
                        } else if self.node(id)?.children.iter().all(is_effectively_empty) {
                            let target = self.enclosing_structure_for_table(table_id);
                            self.remove_node_and_place_caret(target);
                            return Some(selected);
                        } else {
                            let row = self.state.root.find_mut(id)?;
                            for child in &mut row.children {
                                clear_as_tag(child, "mtd");
                            }
                            self.state.caret = Caret {
                                node: row.children[0].id,
                                offset: 0,
                            };
                            self.state.anchor = None;
                            return Some(selected);
                        }
                    }
                }
            } else if start < end {
                let row = self.state.root.find_mut(id)?;
                for child in &mut row.children[start..end] {
                    clear_as_tag(child, "mtd");
                }
                self.state.caret = Caret {
                    node: row.children[start].id,
                    offset: 0,
                };
                self.state.anchor = None;
                return Some(selected);
            }
        }
        let node = self.state.root.find_mut(id)?;
        if node.is_token() {
            let char_count = node.text.chars().count();
            let start = start.min(char_count);
            let end = end.min(char_count);
            node.text = node
                .text
                .chars()
                .take(start)
                .chain(node.text.chars().skip(end))
                .collect();
            let now_empty = node.text.is_empty() && !node.is_fence_delimiter();
            self.state.caret = Caret {
                node: id,
                offset: start,
            };
            self.state.anchor = None;
            if now_empty {
                self.prune_empty_token(id, true);
            }
            return Some(selected);
        } else if node.is_row() {
            let start = start.min(node.children.len());
            let end = end.min(node.children.len());
            if start < end {
                node.children.drain(start..end);
            }
            self.state.caret = Caret {
                node: id,
                offset: start,
            };
        } else {
            let start = start.min(node.children.len());
            let end = end.min(node.children.len());
            // Fixed arity structures retain their slots when a child is cleared.
            if start < end {
                for child in &mut node.children[start..end] {
                    clear_as_tag(child, "mrow");
                }
                self.state.caret = Caret {
                    node: node.children[start].id,
                    offset: 0,
                };
            } else {
                self.state.caret = Caret {
                    node: id,
                    offset: start,
                };
            }
        }
        self.state.anchor = None;
        Some(selected)
    }

    /// Splices `math_nodes` at the current caret, splitting tokens or inserting into rows,
    /// then focuses the first empty placeholder inside the last inserted node if present.
    pub(crate) fn splice(&mut self, math_nodes: Vec<MathNode>) {
        let caret = self.caret();
        let nodes: Vec<_> = math_nodes
            .iter()
            .map(|math_node| import_math_node(math_node, &mut self.next_id))
            .collect();
        let last_inserted_id = nodes.last().map(|node| node.id);
        let token_boundary_in_row = self.node(caret.node).and_then(|node| {
            if !node.is_token() {
                return None;
            }
            let char_len = node.text.chars().count();
            if caret.offset > 0 && caret.offset < char_len {
                return None;
            }
            let mut path = self.path(caret.node)?;
            let child_idx = path.pop()?;
            let parent = self.state.root.at(&path);
            parent.is_row().then_some((
                parent.id,
                child_idx,
                node.text.is_empty(),
                caret.offset == 0,
            ))
        });
        if let Some((parent_id, child_idx, empty_token, at_start)) = token_boundary_in_row {
            let count = nodes.len();
            let Some(parent) = self.state.root.find_mut(parent_id) else {
                return;
            };
            if empty_token {
                parent.children.splice(child_idx..=child_idx, nodes);
                self.state.caret = Caret {
                    node: parent_id,
                    offset: child_idx + count,
                };
            } else {
                let insert_at = if at_start { child_idx } else { child_idx + 1 };
                parent.children.splice(insert_at..insert_at, nodes);
                self.state.caret = Caret {
                    node: parent_id,
                    offset: insert_at + count,
                };
            }
        } else {
            let Some(node) = self.state.root.find_mut(caret.node) else {
                return;
            };
            if node.is_token() {
                let mut prefix = node.to_math_node();
                let mut suffix = prefix.clone();
                prefix.text = node.text.chars().take(caret.offset).collect();
                suffix.text = node.text.chars().skip(caret.offset).collect();
                node.tag = "mrow".into();
                node.text.clear();
                node.attrs.clear();
                node.children.clear();
                if !prefix.text.is_empty() {
                    node.children
                        .push(import_math_node(&prefix, &mut self.next_id));
                }
                node.children.extend(nodes);
                let offset = node.children.len();
                if !suffix.text.is_empty() {
                    node.children
                        .push(import_math_node(&suffix, &mut self.next_id));
                }
                self.state.caret = Caret {
                    node: node.id,
                    offset,
                };
            } else if node.is_row() {
                let offset = caret.offset.min(node.children.len());
                let count = nodes.len();
                node.children.splice(offset..offset, nodes);
                self.state.caret = Caret {
                    node: node.id,
                    offset: offset + count,
                };
            } else {
                // Enter the chosen fixed-arity slot instead of adding invalid children.
                let index = caret.offset.min(node.children.len().saturating_sub(1));
                if let Some(child) = node.children.get(index) {
                    self.state.caret = Caret {
                        node: child.id,
                        offset: 0,
                    };
                    self.splice(math_nodes);
                }
            }
        }
        if let Some(id) = last_inserted_id {
            if let Some(node) = self.node(id) {
                let mut stops = vec![];
                node.stops(&mut stops);
                if let Some(placeholder_caret) = stops.into_iter().find(|stop_caret| {
                    self.node(stop_caret.node)
                        .is_some_and(|slot| slot.text.is_empty() && slot.children.is_empty())
                }) {
                    self.state.caret = placeholder_caret;
                }
            }
        }
    }
}

/// Converts a character index into a UTF-8 byte offset within `text`.
pub(crate) fn char_byte_offset(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map_or(text.len(), |(byte_idx, _)| byte_idx)
}
