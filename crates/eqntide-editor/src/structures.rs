//! Template insertion, matrix resizing, and fence-to-system promotion.

use eqntide::MathNode;

use crate::editing::Editor;
use crate::navigation::Caret;
use crate::node::{import_math_node, is_effectively_empty, Node, NodeId};

impl Editor {
    /// Inserts a palette template, placing any active selection (or script base) in the first placeholder.
    pub fn insert_template(&mut self, template: &MathNode) {
        self.insert_template_impl(
            template,
            matches!(template.tag.as_str(), "msup" | "msub" | "msubsup"),
        );
    }

    /// Inserts `template` while wrapping the preceding expression into its first placeholder
    /// (used for slash-style fraction entry).
    pub fn insert_template_wrapping_previous(&mut self, template: &MathNode) {
        self.insert_template_impl(template, true);
    }

    /// Removes the last row or column of the nearest matrix, retaining at least one cell and a valid caret.
    pub fn shrink_matrix(&mut self, column: bool) -> bool {
        let Some(mut path) = self.path(self.caret().node) else {
            return false;
        };
        loop {
            let node = self.root().at(&path);
            if node.tag == "mtable" {
                let table_id = node.id;
                let can_shrink = if column {
                    node.children.iter().any(|row| row.children.len() > 1)
                } else {
                    node.children.len() > 1
                };
                if !can_shrink {
                    return false;
                }
                self.transact(|editor| {
                    let Some(table) = editor.state.root.find_mut(table_id) else {
                        return;
                    };
                    if column {
                        for row in &mut table.children {
                            if row.children.len() > 1 {
                                row.children.pop();
                            }
                        }
                    } else {
                        table.children.pop();
                    }
                    let fallback_cell_id = table
                        .children
                        .first()
                        .and_then(|row| row.children.first())
                        .map(|cell| cell.id);
                    if editor.node(editor.caret().node).is_none() {
                        if let Some(node_id) = fallback_cell_id {
                            editor.state.caret = Caret {
                                node: node_id,
                                offset: 0,
                            };
                        }
                    }
                    editor.state.anchor = None;
                });
                return true;
            }
            if path.pop().is_none() {
                return false;
            }
        }
    }

    /// Appends a row or column to the nearest matrix while preserving existing cell [`NodeId`]s.
    pub fn grow_matrix(&mut self, column: bool) -> bool {
        let Some(mut path) = self.path(self.caret().node) else {
            return false;
        };
        loop {
            let node = self.root().at(&path);
            if node.tag == "mtable" {
                let table_id = node.id;
                self.transact(|editor| {
                    let Some(table) = editor.state.root.find_mut(table_id) else {
                        return;
                    };
                    if column {
                        for row in &mut table.children {
                            row.children
                                .push(import_math_node(&MathNode::new("mtd"), &mut editor.next_id));
                        }
                    } else {
                        let cols = table.children.first().map_or(2, |row| row.children.len());
                        let row = MathNode::with_children(
                            "mtr",
                            (0..cols).map(|_| MathNode::new("mtd")).collect(),
                        );
                        table
                            .children
                            .push(import_math_node(&row, &mut editor.next_id));
                    }
                });
                return true;
            }
            if path.pop().is_none() {
                return false;
            }
        }
    }

    /// Inserts a new row below the caret's current table row, or promotes a
    /// one-sided `{` / `[` fence slot into a two-row left-aligned system.
    pub fn insert_row_or_promote_fence(&mut self) -> bool {
        if let Some((table_id, row_idx, cols)) = self.enclosing_table_row() {
            self.transact(|editor| {
                let row = MathNode::with_children(
                    "mtr",
                    (0..cols).map(|_| MathNode::new("mtd")).collect(),
                );
                let imported_row = import_math_node(&row, &mut editor.next_id);
                let Some(first_cell_id) = imported_row.children.first().map(|cell| cell.id) else {
                    return;
                };
                let Some(table) = editor.state.root.find_mut(table_id) else {
                    return;
                };
                let insert_at = (row_idx + 1).min(table.children.len());
                table.children.insert(insert_at, imported_row);
                editor.state.caret = Caret {
                    node: first_cell_id,
                    offset: 0,
                };
                editor.state.anchor = None;
            });
            return true;
        }
        if let Some((fence_id, content_end, has_empty_close)) =
            self.enclosing_one_sided_left_fence()
        {
            self.transact(|editor| {
                let Some(fence_mrow) = editor.state.root.find_mut(fence_id) else {
                    return;
                };
                if content_end < 1 || content_end > fence_mrow.children.len() {
                    return;
                }
                let slot_nodes: Vec<Node> = fence_mrow.children.drain(1..content_end).collect();
                let row1_children: Vec<MathNode> =
                    if slot_nodes.len() == 1 && slot_nodes[0].tag == "mrow" {
                        slot_nodes[0]
                            .children
                            .iter()
                            .map(Node::to_math_node)
                            .collect()
                    } else {
                        slot_nodes.iter().map(Node::to_math_node).collect()
                    };
                let mut table_math = MathNode::with_children(
                    "mtable",
                    vec![
                        MathNode::with_children(
                            "mtr",
                            vec![MathNode::with_children("mtd", row1_children)],
                        ),
                        MathNode::with_children("mtr", vec![MathNode::new("mtd")]),
                    ],
                );
                table_math.set_attr("columnalign", "left");
                let imported_table = import_math_node(&table_math, &mut editor.next_id);
                let row2_cell_id = imported_table.children[1].children[0].id;
                let Some(fence_mrow) = editor.state.root.find_mut(fence_id) else {
                    return;
                };
                fence_mrow.children.insert(1, imported_table);
                if !has_empty_close {
                    let mut right_delim = MathNode::leaf("mo", "");
                    right_delim.set_attr("stretchy", "true");
                    right_delim.set_attr("form", "postfix");
                    fence_mrow
                        .children
                        .push(import_math_node(&right_delim, &mut editor.next_id));
                }
                editor.state.caret = Caret {
                    node: row2_cell_id,
                    offset: 0,
                };
                editor.state.anchor = None;
            });
            return true;
        }
        false
    }

    /// Removes the caret's enclosing `<mtr>` if all of its cells are empty and the table has multiple rows.
    pub(crate) fn try_remove_empty_table_row(&mut self, backward: bool) -> bool {
        let Some((table_id, row_idx, _)) = self.enclosing_table_row() else {
            return false;
        };
        let Some(table) = self.node(table_id) else {
            return false;
        };
        let old_len = table.children.len();
        if old_len <= 1 {
            return false;
        }
        let Some(row) = table.children.get(row_idx) else {
            return false;
        };
        if !row.children.iter().all(is_effectively_empty) {
            return false;
        }
        let Some(table) = self.state.root.find_mut(table_id) else {
            return false;
        };
        if row_idx >= table.children.len() {
            return false;
        }
        table.children.remove(row_idx);
        if let Some(target_caret) = adjacent_table_row_caret(table, row_idx, old_len, backward) {
            self.state.caret = target_caret;
        }
        self.state.anchor = None;
        true
    }

    /// Returns `(table_id, row_index, column_count)` for the `<mtable>` enclosing the current caret.
    pub(crate) fn enclosing_table_row(&self) -> Option<(NodeId, usize, usize)> {
        let mut path = self.path(self.caret().node)?;
        while let Some(child_idx) = path.pop() {
            let parent = self.root().at(&path);
            if parent.tag == "mtr" {
                if let Some(row_idx) = path.pop() {
                    let table = self.root().at(&path);
                    if table.tag == "mtable" {
                        let cols = table
                            .children
                            .get(row_idx)
                            .map_or(1, |row| row.children.len())
                            .max(1);
                        return Some((table.id, row_idx, cols));
                    }
                }
                return None;
            }
            let _ = child_idx;
        }
        None
    }

    /// Shared template insertion implementation supporting selection wrapping and preceding-item capture.
    fn insert_template_impl(&mut self, template: &MathNode, take_previous: bool) {
        self.transact(|editor| {
            let mut selected = editor.erase_selection();
            // Scripts attach to the expression before the caret when no range is selected.
            if selected.is_none() && take_previous {
                let caret = editor.caret();
                if let Some(node) = editor.state.root.find_mut(caret.node) {
                    if node.is_row() && caret.offset > 0 {
                        if !node.children[caret.offset - 1].is_fence_delimiter() {
                            selected = Some(node.children.remove(caret.offset - 1).to_math_node());
                            editor.state.caret.offset -= 1;
                        }
                    } else if node.is_token() && caret.offset > 0 && !node.is_fence_delimiter() {
                        let mut prefix = node.to_math_node();
                        prefix.text = node.text.chars().take(caret.offset).collect();
                        node.text = node.text.chars().skip(caret.offset).collect();
                        selected = Some(prefix);
                        editor.state.caret.offset = 0;
                    }
                }
            }
            let mut template = template.clone();
            if let Some(selected) = selected {
                fill_first_empty_slot(&mut template, &selected);
            }
            editor.splice(vec![template]);
        });
    }

    /// Checks whether the caret sits inside a single-sided `{` or `[` fence ready for system promotion.
    fn enclosing_one_sided_left_fence(&self) -> Option<(NodeId, usize, bool)> {
        let caret = self.caret();
        let mut path = self.path(caret.node)?;
        let mut child_idx_from_below: Option<usize> = None;
        loop {
            let candidate = self.root().at(&path);
            if candidate.tag == "mrow" && !candidate.children.is_empty() {
                if let Some((content_end, has_empty_close)) = one_sided_left_fence_bounds(candidate)
                {
                    let inside = match child_idx_from_below {
                        Some(idx) => idx >= 1 && idx < content_end,
                        None => caret.offset >= 1 && caret.offset <= content_end,
                    };
                    if inside {
                        return Some((candidate.id, content_end, has_empty_close));
                    }
                }
            }
            child_idx_from_below = path.pop();
            child_idx_from_below?;
        }
    }
}

/// Computes the caret at the start (`at_end = false`) or end (`at_end = true`) of table cell `cell`.
pub(crate) fn cell_edge_caret(cell: &Node, at_end: bool) -> Caret {
    if at_end {
        if let Some(last) = cell.children.last().filter(|node| node.is_token()) {
            Caret {
                node: last.id,
                offset: last.text.chars().count(),
            }
        } else {
            let mut stops = Vec::new();
            cell.stops(&mut stops);
            stops.last().copied().unwrap_or(Caret {
                node: cell.id,
                offset: 0,
            })
        }
    } else if let Some(first) = cell.children.first().filter(|node| node.is_token()) {
        Caret {
            node: first.id,
            offset: 0,
        }
    } else {
        let mut stops = Vec::new();
        cell.stops(&mut stops);
        stops.first().copied().unwrap_or(Caret {
            node: cell.id,
            offset: 0,
        })
    }
}

/// Returns the caret at the trailing edge of the last cell in `row`.
pub(crate) fn row_end_caret(row: &Node) -> Caret {
    row.children
        .last()
        .map(|cell| cell_edge_caret(cell, true))
        .unwrap_or(Caret {
            node: row.id,
            offset: row.children.len(),
        })
}

/// Computes the destination caret after removing row `removed_row_idx` from `table`.
pub(crate) fn adjacent_table_row_caret(
    table: &Node,
    removed_row_idx: usize,
    old_len: usize,
    backward: bool,
) -> Option<Caret> {
    let move_to_prev_end =
        (backward && removed_row_idx > 0) || (!backward && removed_row_idx + 1 == old_len);
    let target_row_idx = if move_to_prev_end {
        removed_row_idx - 1
    } else {
        removed_row_idx
    };
    let target_cell = if move_to_prev_end {
        table.children.get(target_row_idx)?.children.last()
    } else {
        table.children.get(target_row_idx)?.children.first()
    };
    target_cell.map(|cell| cell_edge_caret(cell, move_to_prev_end))
}

/// Validates that `mrow` is a promotable one-sided `{` / `[` fence and returns `(content_end, has_empty_close)`.
fn one_sided_left_fence_bounds(mrow: &Node) -> Option<(usize, bool)> {
    let first = mrow.children.first()?;
    if first.tag != "mo"
        || !first.is_fence_delimiter()
        || !matches!(first.text.trim(), "{" | "[")
        || first
            .attrs
            .iter()
            .any(|(key, value)| key == "form" && value == "postfix")
    {
        return None;
    }
    if mrow.children.iter().any(|child| child.tag == "mtable") {
        return None;
    }
    let last = mrow.children.last()?;
    let has_empty_close = mrow.children.len() >= 2
        && last.tag == "mo"
        && last.is_fence_delimiter()
        && last.text.trim().is_empty();
    let has_non_empty_close = mrow.children.len() >= 2
        && last.tag == "mo"
        && last.is_fence_delimiter()
        && !last.text.trim().is_empty();
    if has_non_empty_close {
        return None;
    }
    let content_end = if has_empty_close {
        mrow.children.len() - 1
    } else {
        mrow.children.len()
    };
    (content_end >= 2).then_some((content_end, has_empty_close))
}

/// Fills the first empty `<mrow>` or `<mtd>` placeholder inside `template` with `selected`.
fn fill_first_empty_slot(template: &mut MathNode, selected: &MathNode) -> bool {
    if matches!(template.tag.as_str(), "mrow" | "mtd")
        && template.text.is_empty()
        && template.children.is_empty()
    {
        template.children = if matches!(selected.tag.as_str(), "math" | "mrow") {
            selected.children.clone()
        } else {
            vec![selected.clone()]
        };
        return true;
    }
    template
        .children
        .iter_mut()
        .any(|child| fill_first_empty_slot(child, selected))
}
