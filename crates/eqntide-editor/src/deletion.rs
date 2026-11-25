//! Structural backspace, forward delete, empty-token pruning, and slot deletion.
//!
//! Deletion follows two-step ("A2") safety for empty structural slots: pressing
//! Backspace/Delete inside an empty slot first selects the enclosing structure
//! (or removes an empty multi-row table row), and a second press deletes it.

use crate::editing::{char_byte_offset, Editor};
use crate::navigation::{Caret, Move};
use crate::node::{
    clear_as_tag, is_effectively_empty, is_fence_mrow, is_structure_tag, Node, NodeId,
};
use crate::structures::row_end_caret;

impl Editor {
    /// Deletes the active selection or the character/element adjacent to the caret.
    ///
    /// When `backward` is `true` (Backspace), deletes to the left; when `false` (Delete),
    /// deletes to the right.
    pub fn delete(&mut self, backward: bool) {
        self.transact(|editor| {
            if editor.erase_selection().is_some() {
                return;
            }
            let caret = editor.caret();
            let Some(node) = editor.node(caret.node) else {
                return;
            };
            if node.is_token() {
                let char_count = node.text.chars().count();
                if (backward && caret.offset > 0) || (!backward && caret.offset < char_count) {
                    let delete_idx = if backward {
                        caret.offset - 1
                    } else {
                        caret.offset
                    };
                    let is_fence = node.is_fence_delimiter();
                    let Some(node) = editor.state.root.find_mut(caret.node) else {
                        return;
                    };
                    node.text.replace_range(
                        char_byte_offset(&node.text, delete_idx)
                            ..char_byte_offset(&node.text, delete_idx + 1),
                        "",
                    );
                    let now_empty = node.text.is_empty() && !is_fence;
                    editor.state.caret.offset = delete_idx;
                    if now_empty {
                        editor.prune_empty_token(caret.node, backward);
                    }
                    return;
                }
                if char_count == 0 && !node.is_fence_delimiter() {
                    if let Some(mut path) = editor.path(caret.node) {
                        if let Some(child_idx) = path.pop() {
                            if let Some(parent_id) = editor.id_at_path(&path) {
                                if editor.node(parent_id).is_some_and(is_effectively_empty) {
                                    editor.handle_empty_slot_delete(parent_id, backward);
                                    return;
                                }
                                if editor.node(parent_id).is_some_and(Node::is_row) {
                                    if let Some(parent) = editor.state.root.find_mut(parent_id) {
                                        if child_idx < parent.children.len() {
                                            parent.children.remove(child_idx);
                                            editor.state.caret = Caret {
                                                node: parent_id,
                                                offset: child_idx,
                                            };
                                            editor.delete_from_row_caret(
                                                parent_id, child_idx, backward,
                                            );
                                            return;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if let Some((parent_id, adj_idx)) =
                    editor.adjacent_row_sibling(caret.node, backward)
                {
                    if editor
                        .node(parent_id)
                        .and_then(|parent| parent.children.get(adj_idx))
                        .is_some_and(|child| !child.is_fence_delimiter())
                    {
                        editor.delete_adjacent_node_or_char(parent_id, adj_idx, backward);
                        return;
                    }
                }
                editor.move_caret(if backward { Move::Left } else { Move::Right }, false);
            } else if node.is_row() {
                editor.delete_from_row_caret(caret.node, caret.offset, backward);
            }
        });
    }

    /// Removes an emptied non-fence token from its parent row and places the caret on an adjacent token or row slot.
    pub(crate) fn prune_empty_token(&mut self, token_id: NodeId, backward: bool) {
        let Some(mut path) = self.path(token_id) else {
            return;
        };
        let Some(child_idx) = path.pop() else {
            return;
        };
        let Some(parent_id) = self.id_at_path(&path) else {
            return;
        };
        if !self.node(parent_id).is_some_and(Node::is_row) {
            return;
        }
        let Some(parent) = self.state.root.find_mut(parent_id) else {
            return;
        };
        if child_idx >= parent.children.len() {
            return;
        }
        parent.children.remove(child_idx);
        let new_caret = if backward
            && child_idx > 0
            && parent
                .children
                .get(child_idx - 1)
                .is_some_and(|node| node.is_token() && !node.is_fence_delimiter())
        {
            let prev = &parent.children[child_idx - 1];
            Caret {
                node: prev.id,
                offset: prev.text.chars().count(),
            }
        } else if !backward
            && parent
                .children
                .get(child_idx)
                .is_some_and(|node| node.is_token() && !node.is_fence_delimiter())
        {
            Caret {
                node: parent.children[child_idx].id,
                offset: 0,
            }
        } else {
            Caret {
                node: parent_id,
                offset: child_idx.min(parent.children.len()),
            }
        };
        self.state.caret = new_caret;
    }

    /// Returns the enclosing single-child fence `<mrow>` if `table_id` is wrapped in one, otherwise `table_id`.
    pub(crate) fn enclosing_structure_for_table(&self, table_id: NodeId) -> NodeId {
        let Some(mut path) = self.path(table_id) else {
            return table_id;
        };
        if path.pop().is_some() {
            let parent = self.root().at(&path);
            if is_fence_mrow(parent)
                && parent
                    .children
                    .iter()
                    .filter(|child| !child.is_fence_delimiter())
                    .count()
                    == 1
            {
                return parent.id;
            }
        }
        table_id
    }

    /// Removes `target_id` from a row parent or clears it to an empty `<mrow>` slot in a fixed-arity parent.
    pub(crate) fn remove_node_and_place_caret(&mut self, target_id: NodeId) {
        let Some(mut path) = self.path(target_id) else {
            return;
        };
        let Some(child_idx) = path.pop() else {
            return;
        };
        let Some(parent_id) = self.id_at_path(&path) else {
            return;
        };
        let Some(parent) = self.state.root.find_mut(parent_id) else {
            return;
        };
        if parent.is_row() {
            parent.children.remove(child_idx);
            self.state.caret = Caret {
                node: parent_id,
                offset: child_idx.min(parent.children.len()),
            };
        } else {
            clear_as_tag(&mut parent.children[child_idx], "mrow");
            self.state.caret = Caret {
                node: parent.children[child_idx].id,
                offset: 0,
            };
        }
        self.state.anchor = None;
    }

    /// Handles deletion when the caret sits directly on a row container (`math`, `mrow`, `mtd`, `msqrt`).
    fn delete_from_row_caret(&mut self, row_id: NodeId, offset: usize, backward: bool) {
        let Some(row) = self.node(row_id) else {
            return;
        };
        let index = if backward {
            offset.checked_sub(1)
        } else {
            Some(offset)
        };
        if let Some(index) = index.filter(|idx| *idx < row.children.len()) {
            let child_is_fence = row.children[index].is_fence_delimiter();
            if child_is_fence {
                let empty_fence_content = row
                    .children
                    .iter()
                    .filter(|child| !child.is_fence_delimiter())
                    .all(is_effectively_empty);
                if empty_fence_content && is_fence_mrow(row) {
                    self.select_node(row_id);
                } else {
                    self.move_caret(if backward { Move::Left } else { Move::Right }, false);
                }
            } else {
                self.delete_adjacent_node_or_char(row_id, index, backward);
            }
            return;
        }
        if is_effectively_empty(row) {
            self.handle_empty_slot_delete(row_id, backward);
        } else if let Some((outer_row_id, outer_adj_idx)) =
            self.climb_plain_row_boundary(row_id, backward)
        {
            if self
                .node(outer_row_id)
                .and_then(|parent| parent.children.get(outer_adj_idx))
                .is_some_and(|child| !child.is_fence_delimiter())
            {
                self.delete_adjacent_node_or_char(outer_row_id, outer_adj_idx, backward);
            } else {
                self.move_caret(if backward { Move::Left } else { Move::Right }, false);
            }
        } else {
            self.move_caret(if backward { Move::Left } else { Move::Right }, false);
        }
    }

    /// Deletes one character from an adjacent multi-char token, or removes the whole child node.
    fn delete_adjacent_node_or_char(&mut self, row_id: NodeId, index: usize, backward: bool) {
        let Some(row) = self.state.root.find_mut(row_id) else {
            return;
        };
        let Some(child) = row.children.get_mut(index) else {
            return;
        };
        if child.is_token() && !child.is_fence_delimiter() {
            let char_count = child.text.chars().count();
            if char_count > 1 {
                let delete_idx = if backward { char_count - 1 } else { 0 };
                child.text.replace_range(
                    char_byte_offset(&child.text, delete_idx)
                        ..char_byte_offset(&child.text, delete_idx + 1),
                    "",
                );
                self.state.caret = Caret {
                    node: child.id,
                    offset: if backward { char_count - 1 } else { 0 },
                };
                return;
            }
        }
        row.children.remove(index);
        let new_caret = if backward
            && index > 0
            && row
                .children
                .get(index - 1)
                .is_some_and(|node| node.is_token() && !node.is_fence_delimiter())
        {
            let prev = &row.children[index - 1];
            Caret {
                node: prev.id,
                offset: prev.text.chars().count(),
            }
        } else if !backward
            && row
                .children
                .get(index)
                .is_some_and(|node| node.is_token() && !node.is_fence_delimiter())
        {
            Caret {
                node: row.children[index].id,
                offset: 0,
            }
        } else {
            Caret {
                node: row_id,
                offset: index.min(row.children.len()),
            }
        };
        self.state.caret = new_caret;
    }

    /// Finds the immediate row sibling adjacent to `node_id` in the delete direction, climbing plain `<mrow>` wrappers.
    fn adjacent_row_sibling(&self, node_id: NodeId, backward: bool) -> Option<(NodeId, usize)> {
        let mut path = self.path(node_id)?;
        let child_idx = path.pop()?;
        let parent_id = self.id_at_path(&path)?;
        let parent = self.node(parent_id)?;
        if !parent.is_row() {
            return None;
        }
        let adj = if backward {
            child_idx.checked_sub(1)
        } else {
            (child_idx + 1 < parent.children.len()).then_some(child_idx + 1)
        };
        if let Some(idx) = adj {
            return Some((parent_id, idx));
        }
        self.climb_plain_row_boundary(parent_id, backward)
    }

    /// Walks up through non-fence `<mrow>` wrappers until finding a sibling in an enclosing row.
    fn climb_plain_row_boundary(
        &self,
        mut row_id: NodeId,
        backward: bool,
    ) -> Option<(NodeId, usize)> {
        loop {
            let current = self.node(row_id)?;
            if current.tag != "mrow" || is_fence_mrow(current) {
                return None;
            }
            let mut path = self.path(row_id)?;
            let child_idx = path.pop()?;
            let parent_id = self.id_at_path(&path)?;
            let parent = self.node(parent_id)?;
            if !parent.is_row() {
                return None;
            }
            let adj = if backward {
                child_idx.checked_sub(1)
            } else {
                (child_idx + 1 < parent.children.len()).then_some(child_idx + 1)
            };
            if let Some(idx) = adj {
                return Some((parent_id, idx));
            }
            row_id = parent_id;
        }
    }

    /// Applies empty-slot deletion rules: removes empty matrix rows, selects non-empty rows, or selects enclosing structures.
    fn handle_empty_slot_delete(&mut self, slot_id: NodeId, backward: bool) {
        if let Some((table_id, row_idx, cols)) = self.enclosing_table_row() {
            let Some(table) = self.node(table_id) else {
                return;
            };
            let rows_len = table.children.len();
            let Some(row) = table.children.get(row_idx) else {
                return;
            };
            let row_empty = row.children.iter().all(is_effectively_empty);
            if rows_len >= 2 {
                if row_empty {
                    self.try_remove_empty_table_row(backward);
                } else if cols >= 2 {
                    let first_cell_id = row.children[0].id;
                    let end_caret = row_end_caret(row);
                    self.state.anchor = Some(Caret {
                        node: first_cell_id,
                        offset: 0,
                    });
                    self.state.caret = end_caret;
                }
            } else if row_empty {
                let target = self.enclosing_structure_for_table(table_id);
                self.select_node(target);
            } else if cols >= 2 {
                let first_cell_id = row.children[0].id;
                let end_caret = row_end_caret(row);
                self.state.anchor = Some(Caret {
                    node: first_cell_id,
                    offset: 0,
                });
                self.state.caret = end_caret;
            }
            return;
        }
        if let Some(struct_id) = self.enclosing_structure(slot_id) {
            self.select_node(struct_id);
        }
    }

    /// Finds the nearest enclosing structural container, table/fence, or `<msqrt>` above `slot_id`.
    fn enclosing_structure(&self, slot_id: NodeId) -> Option<NodeId> {
        let node = self.node(slot_id)?;
        if node.tag == "msqrt" {
            return Some(node.id);
        }
        let mut path = self.path(slot_id)?;
        while path.pop().is_some() {
            let ancestor = self.root().at(&path);
            if ancestor.tag == "math" {
                return None;
            }
            if ancestor.tag == "mtable" {
                return Some(self.enclosing_structure_for_table(ancestor.id));
            }
            if is_structure_tag(&ancestor.tag) || is_fence_mrow(ancestor) {
                return Some(ancestor.id);
            }
        }
        None
    }
}
