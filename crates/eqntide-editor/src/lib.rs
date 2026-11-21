//! Structural editing independent of the window system and UI framework.
#![forbid(unsafe_code)]

use eqntide::MathNode;
use unicode_segmentation::UnicodeSegmentation;

pub type NodeId = u64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub id: NodeId,
    pub tag: String,
    pub text: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Node>,
}

impl Node {
    pub fn token(&self) -> bool {
        matches!(self.tag.as_str(), "mi" | "mn" | "mo" | "mtext" | "ms")
    }
    pub fn row(&self) -> bool {
        matches!(self.tag.as_str(), "math" | "mrow" | "mtd" | "msqrt")
    }
    pub fn opaque(&self) -> bool {
        is_opaque_tag(&self.tag)
    }
    pub fn is_fence_delimiter(&self) -> bool {
        self.tag == "mo"
            && self.attrs.iter().any(|(k, v)| {
                (k == "stretchy" && v == "true")
                    || (k == "form" && matches!(v.as_str(), "prefix" | "infix" | "postfix"))
            })
    }
    pub fn math(&self) -> MathNode {
        MathNode {
            tag: self.tag.clone(),
            text: self.text.clone(),
            attrs: self.attrs.clone(),
            children: self.children.iter().map(Node::math).collect(),
        }
    }
    fn find(&self, id: NodeId) -> Option<&Node> {
        if self.id == id {
            Some(self)
        } else {
            self.children.iter().find_map(|n| n.find(id))
        }
    }
    fn find_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        if self.id == id {
            Some(self)
        } else {
            self.children.iter_mut().find_map(|n| n.find_mut(id))
        }
    }
    fn path(&self, id: NodeId, path: &mut Vec<usize>) -> bool {
        if self.id == id {
            return true;
        }
        for (i, child) in self.children.iter().enumerate() {
            path.push(i);
            if child.path(id, path) {
                return true;
            }
            path.pop();
        }
        false
    }
    fn at(&self, path: &[usize]) -> &Node {
        path.iter().fold(self, |n, i| &n.children[*i])
    }
    fn stops(&self, out: &mut Vec<Caret>) {
        let (canonical, _) = self.canonical_stops();
        *out = canonical;
    }
    fn canonical_stops(&self) -> (Vec<Caret>, Vec<(Caret, usize)>) {
        let mut raw = Vec::new();
        let mut group = 0;
        self.collect_stops(0, &mut group, &mut raw);
        let mut canonical = Vec::new();
        let mut caret_to_idx = Vec::with_capacity(raw.len());
        let mut i = 0;
        while i < raw.len() {
            let g = raw[i].group;
            let mut j = i + 1;
            while j < raw.len() && raw[j].group == g {
                j += 1;
            }
            let best = raw[i..j]
                .iter()
                .max_by_key(|entry| entry.preference())
                .unwrap();
            let idx = canonical.len();
            canonical.push(best.caret);
            for entry in &raw[i..j] {
                caret_to_idx.push((entry.caret, idx));
            }
            i = j;
        }
        (canonical, caret_to_idx)
    }
    fn collect_stops(&self, depth: usize, group: &mut usize, out: &mut Vec<StopEntry>) {
        let last_in_group_is_token_or_placeholder = |out: &[StopEntry], group: usize| {
            out.last().is_some_and(|e| {
                e.group == group
                    && matches!(e.kind, StopKind::TokenRest | StopKind::EmptyPlaceholder)
            })
        };
        if self.opaque() {
            if last_in_group_is_token_or_placeholder(out, *group) {
                *group += 1;
            }
            *group += 1;
            return;
        }
        if self.token() {
            let char_len = self.text.chars().count();
            if self.tag == "mtext" {
                *group += 1;
                let mut offset = 0;
                for grapheme in self.text.graphemes(true) {
                    out.push(StopEntry {
                        caret: Caret {
                            node: self.id,
                            offset,
                        },
                        group: *group,
                        depth,
                        kind: StopKind::TokenRest,
                    });
                    let next = offset + grapheme.chars().count();
                    for offset in offset + 1..next {
                        out.push(StopEntry {
                            caret: Caret {
                                node: self.id,
                                offset,
                            },
                            group: *group,
                            depth,
                            kind: StopKind::TokenStart,
                        });
                    }
                    offset = next;
                    *group += 1;
                }
                out.push(StopEntry {
                    caret: Caret {
                        node: self.id,
                        offset,
                    },
                    group: *group,
                    depth,
                    kind: if char_len == 0 {
                        StopKind::EmptyPlaceholder
                    } else {
                        StopKind::TokenRest
                    },
                });
                *group += 1;
            } else if self.is_fence_delimiter() {
                out.push(StopEntry {
                    caret: Caret {
                        node: self.id,
                        offset: 0,
                    },
                    group: *group,
                    depth,
                    kind: StopKind::FenceDelimiter,
                });
                if !self.text.trim().is_empty() {
                    for offset in 1..=char_len {
                        *group += 1;
                        out.push(StopEntry {
                            caret: Caret {
                                node: self.id,
                                offset,
                            },
                            group: *group,
                            depth,
                            kind: StopKind::FenceDelimiter,
                        });
                    }
                }
            } else if char_len == 0 {
                if last_in_group_is_token_or_placeholder(out, *group) {
                    *group += 1;
                }
                out.push(StopEntry {
                    caret: Caret {
                        node: self.id,
                        offset: 0,
                    },
                    group: *group,
                    depth,
                    kind: StopKind::EmptyPlaceholder,
                });
            } else {
                if out
                    .last()
                    .is_some_and(|e| e.group == *group && e.kind == StopKind::EmptyPlaceholder)
                {
                    *group += 1;
                }
                out.push(StopEntry {
                    caret: Caret {
                        node: self.id,
                        offset: 0,
                    },
                    group: *group,
                    depth,
                    kind: StopKind::TokenStart,
                });
                for offset in 1..=char_len {
                    *group += 1;
                    out.push(StopEntry {
                        caret: Caret {
                            node: self.id,
                            offset,
                        },
                        group: *group,
                        depth,
                        kind: StopKind::TokenRest,
                    });
                }
            }
            return;
        }
        if self.row() {
            let is_radical = self.tag == "msqrt";
            if self.children.len() <= 1
                && is_effectively_empty(self)
                && self
                    .children
                    .first()
                    .is_none_or(|child| child.tag != "mtext")
            {
                if is_radical || last_in_group_is_token_or_placeholder(out, *group) {
                    *group += 1;
                }
                let placeholder_id = deepest_empty_id(self);
                out.push(StopEntry {
                    caret: Caret {
                        node: placeholder_id,
                        offset: 0,
                    },
                    group: *group,
                    depth,
                    kind: StopKind::EmptyPlaceholder,
                });
                if is_radical {
                    *group += 1;
                }
                return;
            }
            if is_radical {
                *group += 1;
            }
            out.push(StopEntry {
                caret: Caret {
                    node: self.id,
                    offset: 0,
                },
                group: *group,
                depth,
                kind: if self
                    .children
                    .first()
                    .is_some_and(|node| node.tag == "mtext")
                {
                    StopKind::TextBoundary
                } else {
                    StopKind::Row
                },
            });
            for (i, child) in self.children.iter().enumerate() {
                child.collect_stops(depth + 1, group, out);
                out.push(StopEntry {
                    caret: Caret {
                        node: self.id,
                        offset: i + 1,
                    },
                    group: *group,
                    depth,
                    kind: if child.tag == "mtext"
                        || self
                            .children
                            .get(i + 1)
                            .is_some_and(|node| node.tag == "mtext")
                    {
                        StopKind::TextBoundary
                    } else {
                        StopKind::Row
                    },
                });
            }
            if is_radical {
                *group += 1;
            }
            return;
        }
        if matches!(self.tag.as_str(), "mstyle" | "mpadded" | "mphantom") {
            for child in &self.children {
                child.collect_stops(depth + 1, group, out);
            }
            return;
        }
        if self.tag == "mspace" {
            if last_in_group_is_token_or_placeholder(out, *group) {
                *group += 1;
            }
            *group += 1;
            return;
        }
        // Structural containers (`mfrac`, `mroot`, `msub`, `msup`, `mtable`, etc.)
        // have visible boundaries around and between their child slots.
        *group += 1;
        for child in &self.children {
            child.collect_stops(depth + 1, group, out);
            *group += 1;
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum StopKind {
    FenceDelimiter,
    Row,
    TextBoundary,
    TokenStart,
    EmptyPlaceholder,
    TokenRest,
}

#[derive(Clone, Copy)]
struct StopEntry {
    caret: Caret,
    group: usize,
    depth: usize,
    kind: StopKind,
}

impl StopEntry {
    fn preference(&self) -> (u8, isize) {
        match self.kind {
            StopKind::TextBoundary => (5, self.depth as isize),
            StopKind::TokenRest => (4, self.depth as isize),
            StopKind::EmptyPlaceholder => (3, self.depth as isize),
            StopKind::TokenStart => (2, self.depth as isize),
            StopKind::Row => (1, -(self.depth as isize)),
            StopKind::FenceDelimiter => (0, 0),
        }
    }
}

fn deepest_empty_id(node: &Node) -> NodeId {
    let mut cur = node;
    while let Some(first) = cur.children.first() {
        cur = first;
    }
    cur.id
}

/// Elements without editing rules remain atomic and are preserved verbatim.
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Caret {
    pub node: NodeId,
    pub offset: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Snapshot {
    root: Node,
    caret: Caret,
    anchor: Option<Caret>,
}

#[derive(Clone, Debug)]
pub struct Editor {
    state: Snapshot,
    next_id: NodeId,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    pub revision: u64,
}

#[derive(Clone, Copy, Debug)]
pub enum Move {
    Left,
    Right,
    Home,
    End,
    NextPlaceholder,
    PreviousPlaceholder,
}

impl Default for Editor {
    fn default() -> Self {
        Self::new(&MathNode::new("math"))
    }
}

impl Editor {
    /// Check untrusted imports before recursive cloning or editing.
    pub fn try_new(math: &MathNode) -> Result<Self, &'static str> {
        let mut stack = vec![(math, 0usize)];
        let mut count = 0;
        while let Some((node, depth)) = stack.pop() {
            count += 1;
            if depth >= 256 || count > 100_000 {
                return Err("Equation exceeds editor depth or node limit");
            }
            stack.extend(node.children.iter().map(|n| (n, depth + 1)));
        }
        Ok(Self::new(math))
    }
    pub fn new(math: &MathNode) -> Self {
        let mut next = 1;
        let root = import(math, &mut next);
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
            next_id: next,
            undo: vec![],
            redo: vec![],
            revision: 0,
        }
    }
    pub fn root(&self) -> &Node {
        &self.state.root
    }
    pub fn math(&self) -> MathNode {
        self.state.root.math()
    }
    pub fn caret(&self) -> Caret {
        self.state.caret
    }
    pub fn anchor(&self) -> Option<Caret> {
        self.state.anchor
    }
    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.state.root.find(id)
    }
    pub fn id_at_path(&self, path: &[usize]) -> Option<NodeId> {
        let mut n = &self.state.root;
        for i in path {
            n = n.children.get(*i)?;
        }
        Some(n.id)
    }
    pub fn path(&self, id: NodeId) -> Option<Vec<usize>> {
        let mut p = vec![];
        self.state.root.path(id, &mut p).then_some(p)
    }
    pub fn set_caret(&mut self, mut caret: Caret, extend: bool) {
        let Some(node) = self.node(caret.node) else {
            return;
        };
        let max = if node.token() {
            node.text.chars().count()
        } else {
            node.children.len()
        };
        if caret.offset > max {
            return;
        }
        if node.tag == "mtext" && caret.offset > 0 && caret.offset < max {
            let mut end = 0;
            for grapheme in node.text.graphemes(true) {
                end += grapheme.chars().count();
                if end >= caret.offset {
                    caret.offset = end;
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
    pub fn select_node(&mut self, id: NodeId) {
        let Some(mut path) = self.path(id) else {
            return;
        };
        if let Some(i) = path.pop() {
            let parent = self.root().at(&path).id;
            self.state.anchor = Some(Caret {
                node: parent,
                offset: i,
            });
            self.state.caret = Caret {
                node: parent,
                offset: i + 1,
            };
        } else {
            self.select_all();
        }
    }
    pub fn move_caret(&mut self, movement: Move, extend: bool) {
        let (stops, caret_to_idx) = self.root().canonical_stops();
        let current = caret_to_idx
            .iter()
            .find(|(c, _)| *c == self.caret())
            .map(|(_, idx)| *idx)
            .unwrap_or(0);
        let target = match movement {
            Move::Left => current.saturating_sub(1),
            Move::Right => (current + 1).min(stops.len().saturating_sub(1)),
            Move::Home => 0,
            Move::End => stops.len().saturating_sub(1),
            Move::NextPlaceholder | Move::PreviousPlaceholder => {
                let placeholders: Vec<usize> = stops
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| {
                        self.node(c.node).is_some_and(|n| {
                            n.text.is_empty() && n.children.is_empty() && (n.row() || n.token())
                        })
                    })
                    .map(|(i, _)| i)
                    .collect();
                if matches!(movement, Move::NextPlaceholder) {
                    placeholders
                        .iter()
                        .copied()
                        .find(|i| *i > current)
                        .or_else(|| placeholders.first().copied())
                        .unwrap_or(current)
                } else {
                    placeholders
                        .iter()
                        .copied()
                        .rev()
                        .find(|i| *i < current)
                        .or_else(|| placeholders.last().copied())
                        .unwrap_or(current)
                }
            }
        };
        if let Some(c) = stops.get(target) {
            self.set_caret(*c, extend);
        }
    }
    fn transact(&mut self, action: impl FnOnce(&mut Self)) {
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
    pub fn undo(&mut self) {
        if let Some(state) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.state, state));
            self.revision += 1;
        }
    }
    pub fn redo(&mut self) {
        if let Some(state) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.state, state));
            self.revision += 1;
        }
    }
    pub fn replace(&mut self, math: &MathNode) {
        self.transact(|s| {
            s.state.root = import(math, &mut s.next_id);
            s.state.caret = Caret {
                node: s.root().id,
                offset: s.root().children.len(),
            };
            s.state.anchor = None;
        });
    }
    // Expand a cross-container selection to whole children of their common ancestor.
    pub fn selection_span(&self) -> Option<(NodeId, usize, usize)> {
        let mut anchor = self.state.anchor?;
        let mut head = self.caret();
        let text_endpoint = |outside: Caret, inside: Caret| -> Option<Caret> {
            let text = self.node(inside.node).filter(|node| node.tag == "mtext")?;
            let row = self.node(outside.node).filter(|node| node.row())?;
            let index = row.children.iter().position(|child| child.id == text.id)?;
            let offset = if outside.offset == index {
                0
            } else if outside.offset == index + 1 {
                text.text.chars().count()
            } else {
                return None;
            };
            Some(Caret {
                node: text.id,
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
            let max = if node.token() {
                node.text.chars().count()
            } else {
                node.children.len()
            };
            let start = anchor.offset.min(head.offset).min(max);
            let end = anchor.offset.max(head.offset).min(max);
            if start >= end {
                return None;
            }
            return Some((head.node, start, end));
        }
        let a = self.path(anchor.node)?;
        let b = self.path(head.node)?;
        let common = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
        let parent = self.root().at(&a[..common]);
        let ai = a.get(common).copied().unwrap_or(anchor.offset);
        let bi = b.get(common).copied().unwrap_or(head.offset);
        let (start, end) = if ai <= bi {
            (ai, bi + usize::from(b.len() > common))
        } else {
            (bi, ai + usize::from(a.len() > common))
        };
        let start = start.min(parent.children.len());
        let end = end.min(parent.children.len());
        if start >= end {
            return None;
        }
        Some((parent.id, start, end))
    }
    pub fn selected_math(&self) -> Option<MathNode> {
        let (id, start, end) = self.selection_span()?;
        let n = self.node(id)?;
        if n.token() {
            let count = n.text.chars().count();
            let start = start.min(count);
            let end = end.min(count);
            if start >= end {
                return None;
            }
            let mut result = n.math();
            result.text = n.text.chars().skip(start).take(end - start).collect();
            Some(result)
        } else {
            let start = start.min(n.children.len());
            let end = end.min(n.children.len());
            if start >= end {
                return None;
            }
            Some(MathNode::with_children(
                "mrow",
                n.children[start..end].iter().map(Node::math).collect(),
            ))
        }
    }
    pub fn selected_ids(&self) -> Vec<NodeId> {
        let Some((id, start, end)) = self.selection_span() else {
            return vec![];
        };
        let Some(n) = self.node(id) else {
            return vec![];
        };
        if n.token() {
            vec![id]
        } else {
            let start = start.min(n.children.len());
            let end = end.min(n.children.len());
            if start >= end {
                return vec![];
            }
            n.children[start..end].iter().map(|n| n.id).collect()
        }
    }
    fn erase_selection(&mut self) -> Option<MathNode> {
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
                    .and_then(|r| r.children.first())
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
        let n = self.state.root.find_mut(id)?;
        if n.token() {
            let count = n.text.chars().count();
            let start = start.min(count);
            let end = end.min(count);
            n.text = n
                .text
                .chars()
                .take(start)
                .chain(n.text.chars().skip(end))
                .collect();
            let now_empty = n.text.is_empty() && !n.is_fence_delimiter();
            self.state.caret = Caret {
                node: id,
                offset: start,
            };
            self.state.anchor = None;
            if now_empty {
                self.prune_empty_token(id, true);
            }
            return Some(selected);
        } else if n.row() {
            let start = start.min(n.children.len());
            let end = end.min(n.children.len());
            if start < end {
                n.children.drain(start..end);
            }
            self.state.caret = Caret {
                node: id,
                offset: start,
            };
        } else {
            let start = start.min(n.children.len());
            let end = end.min(n.children.len());
            // Fixed arity structures retain their slots when a child is cleared.
            if start < end {
                for child in &mut n.children[start..end] {
                    clear_as_tag(child, "mrow");
                }
                self.state.caret = Caret {
                    node: n.children[start].id,
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
    pub fn insert_text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.transact(|s| {
            s.erase_selection();
            let c = s.caret();
            if let Some(n) = s.state.root.find_mut(c.node) {
                if n.token()
                    && !n.is_fence_delimiter()
                    && (n.tag == "mtext"
                        || text.chars().all(|ch| match n.tag.as_str() {
                            "mi" => ch.is_alphabetic(),
                            "mn" => ch.is_numeric() || ch == '.',
                            "mo" => !ch.is_alphanumeric() && !ch.is_whitespace(),
                            _ => true,
                        }))
                {
                    let byte = char_byte(&n.text, c.offset);
                    n.text.insert_str(byte, text);
                    s.state.caret.offset += text.chars().count();
                    return;
                }
            }
            let nodes = text
                .chars()
                .filter(|c| !c.is_control())
                .map(|c| {
                    MathNode::leaf(
                        if c.is_numeric() {
                            "mn"
                        } else if c.is_alphabetic() {
                            "mi"
                        } else if c.is_whitespace() {
                            "mtext"
                        } else {
                            "mo"
                        },
                        &c.to_string(),
                    )
                })
                .collect();
            s.splice(nodes);
        });
    }
    pub fn insert_math(&mut self, math: &MathNode) {
        self.transact(|s| {
            s.erase_selection();
            let children = if math.tag == "math" {
                math.children.clone()
            } else {
                vec![math.clone()]
            };
            s.splice(children);
        });
    }
    fn splice(&mut self, math: Vec<MathNode>) {
        let c = self.caret();
        let nodes: Vec<_> = math.iter().map(|n| import(n, &mut self.next_id)).collect();
        let last = nodes.last().map(|n| n.id);
        let token_boundary_in_row = self.node(c.node).and_then(|n| {
            if !n.token() {
                return None;
            }
            let char_len = n.text.chars().count();
            if c.offset > 0 && c.offset < char_len {
                return None;
            }
            let mut path = self.path(c.node)?;
            let child_idx = path.pop()?;
            let parent = self.state.root.at(&path);
            parent
                .row()
                .then_some((parent.id, child_idx, n.text.is_empty(), c.offset == 0))
        });
        if let Some((parent_id, child_idx, empty_token, at_start)) = token_boundary_in_row {
            let count = nodes.len();
            let parent = self.state.root.find_mut(parent_id).unwrap();
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
            let Some(n) = self.state.root.find_mut(c.node) else {
                return;
            };
            if n.token() {
                let mut prefix = n.math();
                let mut suffix = prefix.clone();
                prefix.text = n.text.chars().take(c.offset).collect();
                suffix.text = n.text.chars().skip(c.offset).collect();
                n.tag = "mrow".into();
                n.text.clear();
                n.attrs.clear();
                n.children.clear();
                if !prefix.text.is_empty() {
                    n.children.push(import(&prefix, &mut self.next_id));
                }
                n.children.extend(nodes);
                let offset = n.children.len();
                if !suffix.text.is_empty() {
                    n.children.push(import(&suffix, &mut self.next_id));
                }
                self.state.caret = Caret { node: n.id, offset };
            } else if n.row() {
                let offset = c.offset.min(n.children.len());
                let count = nodes.len();
                n.children.splice(offset..offset, nodes);
                self.state.caret = Caret {
                    node: n.id,
                    offset: offset + count,
                };
            } else {
                // Enter the chosen fixed-arity slot instead of adding invalid children.
                let index = c.offset.min(n.children.len().saturating_sub(1));
                if let Some(child) = n.children.get(index) {
                    self.state.caret = Caret {
                        node: child.id,
                        offset: 0,
                    };
                    self.splice(math);
                }
            }
        }
        if let Some(id) = last {
            if let Some(node) = self.node(id) {
                let mut stops = vec![];
                node.stops(&mut stops);
                if let Some(caret) = stops.into_iter().find(|c| {
                    self.node(c.node)
                        .is_some_and(|n| n.text.is_empty() && n.children.is_empty())
                }) {
                    self.state.caret = caret;
                }
            }
        }
    }
    pub fn delete(&mut self, backward: bool) {
        self.transact(|s| {
            if s.erase_selection().is_some() {
                return;
            }
            let c = s.caret();
            let Some(n) = s.node(c.node) else { return };
            if n.token() {
                let count = n.text.chars().count();
                if (backward && c.offset > 0) || (!backward && c.offset < count) {
                    let start = if backward { c.offset - 1 } else { c.offset };
                    let is_fence = n.is_fence_delimiter();
                    let n = s.state.root.find_mut(c.node).unwrap();
                    n.text.replace_range(
                        char_byte(&n.text, start)..char_byte(&n.text, start + 1),
                        "",
                    );
                    let now_empty = n.text.is_empty() && !is_fence;
                    s.state.caret.offset = start;
                    if now_empty {
                        s.prune_empty_token(c.node, backward);
                    }
                    return;
                }
                if count == 0 && !n.is_fence_delimiter() {
                    if let Some(mut path) = s.path(c.node) {
                        if let Some(child_idx) = path.pop() {
                            let parent_id = s.id_at_path(&path).unwrap();
                            if s.node(parent_id).is_some_and(is_effectively_empty) {
                                s.handle_empty_slot_delete(parent_id, backward);
                                return;
                            }
                            if s.node(parent_id).is_some_and(Node::row) {
                                s.state
                                    .root
                                    .find_mut(parent_id)
                                    .unwrap()
                                    .children
                                    .remove(child_idx);
                                s.state.caret = Caret {
                                    node: parent_id,
                                    offset: child_idx,
                                };
                                s.delete_from_row_caret(parent_id, child_idx, backward);
                                return;
                            }
                        }
                    }
                }
                if let Some((parent_id, adj_idx)) = s.adjacent_row_sibling(c.node, backward) {
                    if !s.node(parent_id).unwrap().children[adj_idx].is_fence_delimiter() {
                        s.delete_adjacent_node_or_char(parent_id, adj_idx, backward);
                        return;
                    }
                }
                s.move_caret(if backward { Move::Left } else { Move::Right }, false);
            } else if n.row() {
                s.delete_from_row_caret(c.node, c.offset, backward);
            }
        });
    }
    fn delete_from_row_caret(&mut self, row_id: NodeId, offset: usize, backward: bool) {
        let Some(n) = self.node(row_id) else { return };
        let index = if backward {
            offset.checked_sub(1)
        } else {
            Some(offset)
        };
        if let Some(index) = index.filter(|i| *i < n.children.len()) {
            let child_is_fence = n.children[index].is_fence_delimiter();
            if child_is_fence {
                let empty_fence_content = n
                    .children
                    .iter()
                    .filter(|c| !c.is_fence_delimiter())
                    .all(is_effectively_empty);
                if empty_fence_content && is_fence_mrow(n) {
                    self.select_node(row_id);
                } else {
                    self.move_caret(if backward { Move::Left } else { Move::Right }, false);
                }
            } else {
                self.delete_adjacent_node_or_char(row_id, index, backward);
            }
            return;
        }
        if is_effectively_empty(n) {
            self.handle_empty_slot_delete(row_id, backward);
        } else if let Some((outer_row_id, outer_adj_idx)) =
            self.climb_plain_row_boundary(row_id, backward)
        {
            if !self.node(outer_row_id).unwrap().children[outer_adj_idx].is_fence_delimiter() {
                self.delete_adjacent_node_or_char(outer_row_id, outer_adj_idx, backward);
            } else {
                self.move_caret(if backward { Move::Left } else { Move::Right }, false);
            }
        } else {
            self.move_caret(if backward { Move::Left } else { Move::Right }, false);
        }
    }
    fn prune_empty_token(&mut self, token_id: NodeId, backward: bool) {
        let Some(mut path) = self.path(token_id) else {
            return;
        };
        let Some(child_idx) = path.pop() else {
            return;
        };
        let Some(parent_id) = self.id_at_path(&path) else {
            return;
        };
        if !self.node(parent_id).is_some_and(Node::row) {
            return;
        }
        let parent = self.state.root.find_mut(parent_id).unwrap();
        parent.children.remove(child_idx);
        let new_caret = if backward
            && child_idx > 0
            && parent
                .children
                .get(child_idx - 1)
                .is_some_and(|n| n.token() && !n.is_fence_delimiter())
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
                .is_some_and(|n| n.token() && !n.is_fence_delimiter())
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
    fn delete_adjacent_node_or_char(&mut self, row_id: NodeId, index: usize, backward: bool) {
        let Some(row) = self.state.root.find_mut(row_id) else {
            return;
        };
        let Some(child) = row.children.get_mut(index) else {
            return;
        };
        if child.token() && !child.is_fence_delimiter() {
            let count = child.text.chars().count();
            if count > 1 {
                let del_idx = if backward { count - 1 } else { 0 };
                child.text.replace_range(
                    char_byte(&child.text, del_idx)..char_byte(&child.text, del_idx + 1),
                    "",
                );
                self.state.caret = Caret {
                    node: child.id,
                    offset: if backward { count - 1 } else { 0 },
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
                .is_some_and(|n| n.token() && !n.is_fence_delimiter())
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
                .is_some_and(|n| n.token() && !n.is_fence_delimiter())
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
    fn adjacent_row_sibling(&self, node_id: NodeId, backward: bool) -> Option<(NodeId, usize)> {
        let mut path = self.path(node_id)?;
        let child_idx = path.pop()?;
        let parent_id = self.id_at_path(&path)?;
        let parent = self.node(parent_id)?;
        if !parent.row() {
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
    fn climb_plain_row_boundary(
        &self,
        mut row_id: NodeId,
        backward: bool,
    ) -> Option<(NodeId, usize)> {
        loop {
            let cur = self.node(row_id)?;
            if cur.tag != "mrow" || is_fence_mrow(cur) {
                return None;
            }
            let mut path = self.path(row_id)?;
            let child_idx = path.pop()?;
            let parent_id = self.id_at_path(&path)?;
            let parent = self.node(parent_id)?;
            if !parent.row() {
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
    fn enclosing_structure_for_table(&self, table_id: NodeId) -> NodeId {
        let Some(mut path) = self.path(table_id) else {
            return table_id;
        };
        if path.pop().is_some() {
            let parent = self.root().at(&path);
            if is_fence_mrow(parent)
                && parent
                    .children
                    .iter()
                    .filter(|c| !c.is_fence_delimiter())
                    .count()
                    == 1
            {
                return parent.id;
            }
        }
        table_id
    }
    fn remove_node_and_place_caret(&mut self, target_id: NodeId) {
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
        if parent.row() {
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
    /// Insert a palette template. The first placeholder receives the selection.
    pub fn insert_template(&mut self, template: &MathNode) {
        self.insert_template_impl(
            template,
            matches!(template.tag.as_str(), "msup" | "msub" | "msubsup"),
        );
    }
    /// Slash-style entry takes the preceding expression into a fraction numerator.
    pub fn insert_template_wrapping_previous(&mut self, template: &MathNode) {
        self.insert_template_impl(template, true);
    }
    fn insert_template_impl(&mut self, template: &MathNode, take_previous: bool) {
        self.transact(|s| {
            let mut selected = s.erase_selection();
            // Scripts attach to the expression before the caret when no range is selected.
            if selected.is_none() && take_previous {
                let caret = s.caret();
                if let Some(node) = s.state.root.find_mut(caret.node) {
                    if node.row() && caret.offset > 0 {
                        if !node.children[caret.offset - 1].is_fence_delimiter() {
                            selected = Some(node.children.remove(caret.offset - 1).math());
                            s.state.caret.offset -= 1;
                        }
                    } else if node.token() && caret.offset > 0 && !node.is_fence_delimiter() {
                        let mut prefix = node.math();
                        prefix.text = node.text.chars().take(caret.offset).collect();
                        node.text = node.text.chars().skip(caret.offset).collect();
                        selected = Some(prefix);
                        s.state.caret.offset = 0;
                    }
                }
            }
            let mut template = template.clone();
            if let Some(selected) = selected {
                fill_first(&mut template, &selected);
            }
            s.splice(vec![template]);
        });
    }
    /// Remove the last row/column, retaining at least one cell and a valid caret.
    pub fn shrink_matrix(&mut self, column: bool) -> bool {
        let Some(mut path) = self.path(self.caret().node) else {
            return false;
        };
        loop {
            let n = self.root().at(&path);
            if n.tag == "mtable" {
                let id = n.id;
                let possible = if column {
                    n.children.iter().any(|r| r.children.len() > 1)
                } else {
                    n.children.len() > 1
                };
                if !possible {
                    return false;
                }
                self.transact(|s| {
                    let table = s.state.root.find_mut(id).unwrap();
                    if column {
                        for row in &mut table.children {
                            if row.children.len() > 1 {
                                row.children.pop();
                            }
                        }
                    } else {
                        table.children.pop();
                    }
                    let fallback = table
                        .children
                        .first()
                        .and_then(|r| r.children.first())
                        .map(|c| c.id);
                    if s.node(s.caret().node).is_none() {
                        if let Some(node) = fallback {
                            s.state.caret = Caret { node, offset: 0 };
                        }
                    }
                    s.state.anchor = None;
                });
                return true;
            }
            if path.pop().is_none() {
                return false;
            }
        }
    }
    /// Add a row or column to the nearest matrix. Existing cells retain IDs.
    pub fn grow_matrix(&mut self, column: bool) -> bool {
        let Some(mut path) = self.path(self.caret().node) else {
            return false;
        };
        loop {
            let n = self.root().at(&path);
            if n.tag == "mtable" {
                let id = n.id;
                self.transact(|s| {
                    let table = s.state.root.find_mut(id).unwrap();
                    if column {
                        for row in &mut table.children {
                            row.children
                                .push(import(&MathNode::new("mtd"), &mut s.next_id));
                        }
                    } else {
                        let cols = table.children.first().map_or(2, |r| r.children.len());
                        let row = MathNode::with_children(
                            "mtr",
                            (0..cols).map(|_| MathNode::new("mtd")).collect(),
                        );
                        table.children.push(import(&row, &mut s.next_id));
                    }
                });
                return true;
            }
            if path.pop().is_none() {
                return false;
            }
        }
    }
    /// Insert a new row below the caret's current table row, or promote a
    /// one-sided `{` / `[` fence slot into a two-row left-aligned system.
    pub fn insert_row_or_promote_fence(&mut self) -> bool {
        if let Some((table_id, row_idx, cols)) = self.enclosing_table_row() {
            self.transact(|s| {
                let row = MathNode::with_children(
                    "mtr",
                    (0..cols).map(|_| MathNode::new("mtd")).collect(),
                );
                let imported_row = import(&row, &mut s.next_id);
                let first_cell_id = imported_row.children[0].id;
                let table = s.state.root.find_mut(table_id).unwrap();
                table.children.insert(row_idx + 1, imported_row);
                s.state.caret = Caret {
                    node: first_cell_id,
                    offset: 0,
                };
                s.state.anchor = None;
            });
            return true;
        }
        if let Some((fence_id, content_end, has_empty_close)) =
            self.enclosing_one_sided_left_fence()
        {
            self.transact(|s| {
                let fence_mrow = s.state.root.find_mut(fence_id).unwrap();
                let slot_nodes: Vec<Node> = fence_mrow.children.drain(1..content_end).collect();
                let row1_children: Vec<MathNode> =
                    if slot_nodes.len() == 1 && slot_nodes[0].tag == "mrow" {
                        slot_nodes[0].children.iter().map(Node::math).collect()
                    } else {
                        slot_nodes.iter().map(Node::math).collect()
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
                let imported_table = import(&table_math, &mut s.next_id);
                let row2_cell_id = imported_table.children[1].children[0].id;
                let fence_mrow = s.state.root.find_mut(fence_id).unwrap();
                fence_mrow.children.insert(1, imported_table);
                if !has_empty_close {
                    let mut right = MathNode::leaf("mo", "");
                    right.set_attr("stretchy", "true");
                    right.set_attr("form", "postfix");
                    fence_mrow.children.push(import(&right, &mut s.next_id));
                }
                s.state.caret = Caret {
                    node: row2_cell_id,
                    offset: 0,
                };
                s.state.anchor = None;
            });
            return true;
        }
        false
    }
    fn try_remove_empty_table_row(&mut self, backward: bool) -> bool {
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
        let table = self.state.root.find_mut(table_id).unwrap();
        table.children.remove(row_idx);
        if let Some(target_caret) = adjacent_table_row_caret(table, row_idx, old_len, backward) {
            self.state.caret = target_caret;
        }
        self.state.anchor = None;
        true
    }
    fn enclosing_table_row(&self) -> Option<(NodeId, usize, usize)> {
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
                            .map_or(1, |r| r.children.len())
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

fn is_effectively_empty(node: &Node) -> bool {
    (node.token() && !node.is_fence_delimiter() && node.text.is_empty())
        || (matches!(node.tag.as_str(), "math" | "mrow" | "mtd" | "msqrt")
            && node.text.is_empty()
            && node.children.iter().all(is_effectively_empty))
}

fn is_structure_tag(tag: &str) -> bool {
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

fn is_fence_mrow(node: &Node) -> bool {
    node.tag == "mrow"
        && node.children.first().is_some_and(|c| {
            c.is_fence_delimiter() && !c.attrs.iter().any(|(k, v)| k == "form" && v == "postfix")
        })
}

fn clear_as_tag(node: &mut Node, tag: &str) {
    node.tag = tag.into();
    node.text.clear();
    node.attrs.clear();
    node.children.clear();
}

fn cell_edge_caret(cell: &Node, at_end: bool) -> Caret {
    if at_end {
        if let Some(last) = cell.children.last().filter(|n| n.token()) {
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
    } else if let Some(first) = cell.children.first().filter(|n| n.token()) {
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

fn row_end_caret(row: &Node) -> Caret {
    row.children
        .last()
        .map(|cell| cell_edge_caret(cell, true))
        .unwrap_or(Caret {
            node: row.id,
            offset: row.children.len(),
        })
}

fn adjacent_table_row_caret(
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

fn one_sided_left_fence_bounds(mrow: &Node) -> Option<(usize, bool)> {
    let first = mrow.children.first()?;
    if first.tag != "mo"
        || !first.is_fence_delimiter()
        || !matches!(first.text.trim(), "{" | "[")
        || first
            .attrs
            .iter()
            .any(|(k, v)| k == "form" && v == "postfix")
    {
        return None;
    }
    if mrow.children.iter().any(|c| c.tag == "mtable") {
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

fn char_byte(text: &str, index: usize) -> usize {
    text.char_indices()
        .nth(index)
        .map_or(text.len(), |(i, _)| i)
}
fn import(math: &MathNode, next: &mut NodeId) -> Node {
    let id = *next;
    *next += 1;
    let mut node = Node {
        id,
        tag: math.tag.clone(),
        text: math.text.clone(),
        attrs: math.attrs.clone(),
        children: Vec::with_capacity(math.children.len()),
    };
    let needs_text_rows = !node.row() && !node.opaque() && !node.token();
    for child in &math.children {
        let child = import(child, next);
        if needs_text_rows && child.tag == "mtext" {
            // A row gives the text slot Math-mode insertion points on either side.
            node.children.push(Node {
                id: *next,
                tag: "mrow".into(),
                text: String::new(),
                attrs: Vec::new(),
                children: vec![child],
            });
            *next += 1;
        } else {
            node.children.push(child);
        }
    }
    node
}
fn fill_first(template: &mut MathNode, selected: &MathNode) -> bool {
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
        .any(|n| fill_first(n, selected))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn import_is_lossless_including_unknown_nodes() {
        let math = MathNode::parse("<math display=\"inline\"><mfrac bevelled=\"true\"><mi mathvariant=\"bold\">αβ</mi><unknown foo=\"bar\"><mi>x</mi></unknown></mfrac></math>").unwrap();
        assert_eq!(Editor::new(&math).math(), math);
    }
    #[test]
    fn template_wraps_selection_and_undo_restores_caret() {
        let mut e = Editor::default();
        e.insert_text("x+1");
        e.select_all();
        let before = e.state.clone();
        e.insert_template(&MathNode::with_children(
            "mfrac",
            vec![MathNode::new("mrow"), MathNode::new("mrow")],
        ));
        assert_eq!(e.math().children[0].children[0].children.len(), 3);
        assert!(e.node(e.caret().node).unwrap().children.is_empty());
        e.undo();
        assert_eq!(e.state, before);
        e.redo();
        assert_eq!(e.math().children[0].tag, "mfrac");
    }
    #[test]
    fn unicode_edits_use_char_boundaries() {
        let mut e = Editor::new(&MathNode::with_children(
            "math",
            vec![MathNode::leaf("mtext", "αβγδε ζηθι")],
        ));
        let id = e.root().children[0].id;
        e.set_caret(
            Caret {
                node: id,
                offset: 5,
            },
            false,
        );
        e.insert_text("α");
        e.delete(true);
        assert_eq!(e.math().children[0].text, "αβγδε ζηθι");
    }
    #[test]
    fn insert_structure_splits_token_without_losing_style() {
        let mut m = MathNode::leaf("mi", "ab");
        m.set_attr("mathvariant", "bold");
        let mut e = Editor::new(&MathNode::with_children("math", vec![m]));
        let id = e.root().children[0].id;
        e.set_caret(
            Caret {
                node: id,
                offset: 1,
            },
            false,
        );
        e.insert_template(&MathNode::with_children(
            "msqrt",
            vec![MathNode::new("mrow")],
        ));
        let m = e.math();
        let parts = &m.children[0].children;
        assert_eq!(parts[0].attr("mathvariant"), Some("bold"));
        assert_eq!(parts[2].text, "b");
    }
    #[test]
    fn matrix_grows_and_redo_is_cleared_after_edit() {
        let math = MathNode::parse("<math><mtable><mtr><mtd/></mtr></mtable></math>").unwrap();
        let mut e = Editor::new(&math);
        let id = e.root().children[0].children[0].children[0].id;
        e.set_caret(
            Caret {
                node: id,
                offset: 0,
            },
            false,
        );
        assert!(e.grow_matrix(true));
        assert!(e.grow_matrix(false));
        assert_eq!(e.math().children[0].children.len(), 2);
        e.undo();
        e.insert_text("z");
        let m = e.math();
        e.redo();
        assert_eq!(e.math(), m);
    }
    #[test]
    fn deleting_a_fixed_slot_keeps_valid_arity() {
        let mut e = Editor::new(
            &MathNode::parse("<math><mfrac><mi>a</mi><mi>b</mi></mfrac></math>").unwrap(),
        );
        let numerator = e.root().children[0].children[0].id;
        e.select_node(numerator);
        e.delete(true);
        assert_eq!(e.math().children[0].children.len(), 2);
        assert_eq!(e.math().children[0].children[1].text, "b");
    }
}

#[cfg(test)]
mod regression_tests {
    use super::*;
    #[test]
    fn scripts_take_the_previous_base_and_undo_is_one_transaction() {
        let mut e = Editor::default();
        e.insert_text("x");
        let before = e.math();
        e.insert_template(&MathNode::with_children(
            "msup",
            vec![MathNode::new("mrow"), MathNode::new("mrow")],
        ));
        e.insert_text("2");
        assert_eq!(e.math().children.len(), 1);
        assert_eq!(e.math().children[0].tag, "msup");
        assert_eq!(e.math().children[0].children[0].children[0].text, "x");
        e.undo();
        e.undo();
        assert_eq!(e.math(), before);
    }
    #[test]
    fn operators_pasted_inside_an_identifier_become_operator_nodes() {
        let mut e = Editor::new(&MathNode::with_children(
            "math",
            vec![MathNode::leaf("mi", "xy")],
        ));
        let id = e.root().children[0].id;
        e.set_caret(
            Caret {
                node: id,
                offset: 1,
            },
            false,
        );
        e.insert_text("+2");
        let math = e.math();
        let parts = &math.children[0].children;
        assert_eq!(
            parts.iter().map(|n| n.tag.as_str()).collect::<Vec<_>>(),
            ["mi", "mo", "mn", "mi"]
        );
        e.undo();
        assert_eq!(e.math().children[0].text, "xy");
    }
    #[test]
    fn shrinking_matrix_restores_caret_to_a_surviving_cell() {
        let mut e = Editor::new(
            &MathNode::parse(
                "<math><mtable><mtr><mtd/><mtd/></mtr><mtr><mtd/><mtd/></mtr></mtable></math>",
            )
            .unwrap(),
        );
        let id = e.root().children[0].children[1].children[1].id;
        e.set_caret(
            Caret {
                node: id,
                offset: 0,
            },
            false,
        );
        assert!(e.shrink_matrix(false));
        assert!(e.node(e.caret().node).is_some());
        assert!(e.shrink_matrix(true));
        assert!(!e.shrink_matrix(true));
        assert!(!e.shrink_matrix(false));
        e.undo();
        e.undo();
        assert_eq!(e.math().children[0].children.len(), 2);
    }
    #[test]
    fn untrusted_imports_have_a_depth_limit() {
        let mut math = MathNode::leaf("mi", "x");
        for _ in 0..260 {
            math = MathNode::with_children("mrow", vec![math]);
        }
        assert!(Editor::try_new(&math).is_err());
    }
    #[test]
    fn slash_fraction_takes_previous_expression_into_numerator() {
        let mut e = Editor::default();
        e.insert_text("x");
        e.insert_template_wrapping_previous(&MathNode::with_children(
            "mfrac",
            vec![MathNode::new("mrow"), MathNode::new("mrow")],
        ));
        let math = e.math();
        assert_eq!(math.children.len(), 1);
        assert_eq!(math.children[0].children[0].children[0].text, "x");
        assert!(e.node(e.caret().node).unwrap().children.is_empty());
    }
    #[test]
    fn empty_or_out_of_bounds_selection_never_panics() {
        let mut e = Editor::default();
        let root = e.root().id;
        // Simulating selection on empty root with out-of-bounds offset
        e.state.anchor = Some(Caret {
            node: root,
            offset: 0,
        });
        e.state.caret = Caret {
            node: root,
            offset: 1,
        };
        assert!(e.selected_ids().is_empty());
        assert!(e.selected_math().is_none());
        assert!(e.erase_selection().is_none());
    }
    #[test]
    fn editing_at_fence_boundaries_keeps_delimiters_as_direct_row_siblings() {
        let fence = |left: &str, right: &str| {
            let mut l = MathNode::leaf("mo", left);
            l.set_attr("stretchy", "true");
            l.set_attr("form", "prefix");
            let mut r = MathNode::leaf("mo", right);
            r.set_attr("stretchy", "true");
            r.set_attr("form", "postfix");
            MathNode::with_children(
                "math",
                vec![MathNode::with_children(
                    "mrow",
                    vec![l, MathNode::new("mrow"), r],
                )],
            )
        };

        for (left, right) in [("∥", "∥"), ("(", ")")] {
            let mut e = Editor::new(&fence(left, right));
            let inner_id = e.root().children[0].children[1].id;
            e.set_caret(
                Caret {
                    node: inner_id,
                    offset: 0,
                },
                false,
            );
            e.insert_template(&MathNode::with_children(
                "mfrac",
                vec![MathNode::new("mrow"), MathNode::new("mrow")],
            ));
            e.insert_text("1");
            e.move_caret(Move::NextPlaceholder, false);
            e.insert_text("2");

            // Typing `x` at the left boundary of the closing fence must insert into the fence mrow.
            let right_id = e.root().children[0].children[2].id;
            e.set_caret(
                Caret {
                    node: right_id,
                    offset: 0,
                },
                false,
            );
            e.insert_text("x");
            // Typing `+` at the right boundary of the closing fence must not corrupt the delimiter text.
            e.set_caret(
                Caret {
                    node: right_id,
                    offset: 1,
                },
                false,
            );
            e.insert_text("+");
            // Typing `a` at the left boundary of the opening fence must stay in the same row.
            let left_id = e.root().children[0].children[0].id;
            e.set_caret(
                Caret {
                    node: left_id,
                    offset: 0,
                },
                false,
            );
            e.insert_text("a");

            let math = e.math();
            let row = &math.children[0].children;
            assert_eq!(row[0].tag, "mi");
            assert_eq!(row[0].text, "a");
            assert_eq!(row[1].tag, "mo");
            assert_eq!(row[1].text, left);
            assert_eq!(row[1].attr("form"), Some("prefix"));
            assert_eq!(row[2].tag, "mrow");
            assert_eq!(row[3].tag, "mi");
            assert_eq!(row[3].text, "x");
            assert_eq!(row[4].tag, "mo");
            assert_eq!(row[4].text, right);
            assert_eq!(row[4].attr("form"), Some("postfix"));
            assert_eq!(row[5].tag, "mo");
            assert_eq!(row[5].text, "+");
        }
    }
    #[test]
    fn wrapping_scripts_or_fractions_at_fence_boundaries_does_not_steal_delimiter() {
        let mut l = MathNode::leaf("mo", "∥");
        l.set_attr("stretchy", "true");
        l.set_attr("form", "prefix");
        let mut r = MathNode::leaf("mo", "∥");
        r.set_attr("stretchy", "true");
        r.set_attr("form", "postfix");
        let mut e = Editor::new(&MathNode::with_children(
            "math",
            vec![MathNode::with_children(
                "mrow",
                vec![l, MathNode::leaf("mi", "x"), r],
            )],
        ));
        let row_id = e.root().children[0].id;
        let right_id = e.root().children[0].children[2].id;
        let msup =
            MathNode::with_children("msup", vec![MathNode::new("mrow"), MathNode::new("mrow")]);
        let mfrac =
            MathNode::with_children("mfrac", vec![MathNode::new("mrow"), MathNode::new("mrow")]);

        // Caret on closing delimiter offset 1: pressing `^` must not steal `∥` into the base.
        e.set_caret(
            Caret {
                node: right_id,
                offset: 1,
            },
            false,
        );
        e.insert_template(&msup);
        // Caret right after opening delimiter in parent row: `/` must not steal `∥` into numerator.
        e.set_caret(
            Caret {
                node: row_id,
                offset: 1,
            },
            false,
        );
        e.insert_template_wrapping_previous(&mfrac);

        let math = e.math();
        let row = &math.children[0].children;
        assert_eq!(row[0].tag, "mo");
        assert_eq!(row[0].text, "∥");
        assert_eq!(row[0].attr("form"), Some("prefix"));
        assert_eq!(row[1].tag, "mfrac");
        assert!(row[1].children[0].children.is_empty());
        assert_eq!(row[2].tag, "mi");
        assert_eq!(row[2].text, "x");
        assert_eq!(row[3].tag, "mo");
        assert_eq!(row[3].text, "∥");
        assert_eq!(row[3].attr("form"), Some("postfix"));
        assert_eq!(row[4].tag, "msup");
        assert!(row[4].children[0].children.is_empty());
    }
    #[test]
    fn insert_row_or_promote_fence_adds_row_below_caret_and_undoes_in_one_step() {
        let mut e = Editor::new(
            &MathNode::parse(
                "<math><mtable columnalign=\"left\"><mtr><mtd><mi>a</mi></mtd></mtr><mtr><mtd><mi>b</mi></mtd></mtr></mtable></math>",
            )
            .unwrap(),
        );
        // Place caret inside row 0 cell (`a`).
        let row0_mi = e.root().children[0].children[0].children[0].children[0].id;
        e.set_caret(
            Caret {
                node: row0_mi,
                offset: 1,
            },
            false,
        );
        assert!(e.insert_row_or_promote_fence());

        let math = e.math();
        assert_eq!(math.children[0].children.len(), 3);
        assert_eq!(
            math.children[0].children[0].children[0].children[0].text,
            "a"
        );
        assert!(math.children[0].children[1].children[0].children.is_empty());
        assert_eq!(
            math.children[0].children[2].children[0].children[0].text,
            "b"
        );
        assert_eq!(
            e.caret().node,
            e.root().children[0].children[1].children[0].id
        );

        e.undo();
        assert_eq!(e.math().children[0].children.len(), 2);
        e.redo();
        assert_eq!(e.math().children[0].children.len(), 3);

        // Multi-column table and nested fraction inside a cell preserve column count.
        let mut multi = Editor::new(
            &MathNode::parse(
                "<math><mtable><mtr><mtd><mfrac><mi>x</mi><mi>y</mi></mfrac></mtd><mtd><mi>z</mi></mtd></mtr></mtable></math>",
            )
            .unwrap(),
        );
        let frac_num = multi.root().children[0].children[0].children[0].children[0].children[0].id;
        multi.set_caret(
            Caret {
                node: frac_num,
                offset: 0,
            },
            false,
        );
        assert!(multi.insert_row_or_promote_fence());
        assert_eq!(multi.math().children[0].children.len(), 2);
        assert_eq!(multi.math().children[0].children[1].children.len(), 2);

        // Outside a table or left fence: no-op.
        let mut plain = Editor::default();
        plain.insert_text("x");
        assert!(!plain.insert_row_or_promote_fence());
    }
    #[test]
    fn insert_row_or_promote_fence_promotes_single_sided_brace_and_bracket_only() {
        for open in ["{", "["] {
            let mut l = MathNode::leaf("mo", open);
            l.set_attr("stretchy", "true");
            l.set_attr("form", "prefix");
            let mut r = MathNode::leaf("mo", "");
            r.set_attr("stretchy", "true");
            r.set_attr("form", "postfix");
            let tpl = MathNode::with_children("mrow", vec![l, MathNode::new("mrow"), r]);

            // Non-empty single-slot left fence promotes to 2x1 left-aligned table.
            let mut e = Editor::default();
            e.insert_template(&tpl);
            e.insert_text("x+2=1");
            assert!(e.insert_row_or_promote_fence());
            let math = e.math();
            let fence_row = &math.children[0].children;
            assert_eq!(fence_row[0].text, open);
            assert_eq!(fence_row[1].tag, "mtable");
            assert_eq!(fence_row[1].attr("columnalign"), Some("left"));
            assert_eq!(fence_row[1].children.len(), 2);
            assert_eq!(fence_row[1].children[0].children[0].children.len(), 5);
            assert!(fence_row[1].children[1].children[0].children.is_empty());
            assert_eq!(fence_row[2].text, "");
            assert_eq!(
                e.caret().node,
                e.root().children[0].children[1].children[1].children[0].id
            );

            // Undo restores the single-slot fenced expression in one step.
            e.undo();
            assert_eq!(e.math().children[0].children[1].tag, "mrow");

            // Empty slot also promotes cleanly.
            let mut empty = Editor::default();
            empty.insert_template(&tpl);
            assert!(empty.insert_row_or_promote_fence());
            assert_eq!(empty.math().children[0].children[1].tag, "mtable");
            assert_eq!(empty.math().children[0].children[1].children.len(), 2);
        }

        // Paired fences `( x )` or `{ x }` and one-sided `(` must not promote.
        for (open, close) in [("{", "}"), ("[", "]"), ("(", "")] {
            let mut l = MathNode::leaf("mo", open);
            l.set_attr("stretchy", "true");
            l.set_attr("form", "prefix");
            let mut r = MathNode::leaf("mo", close);
            r.set_attr("stretchy", "true");
            r.set_attr("form", "postfix");
            let mut e = Editor::default();
            e.insert_template(&MathNode::with_children(
                "mrow",
                vec![l, MathNode::new("mrow"), r],
            ));
            e.insert_text("x");
            assert!(!e.insert_row_or_promote_fence());
        }
    }
    #[test]
    fn backspace_and_delete_remove_empty_table_rows_and_guard_nonempty_or_single_row() {
        // 3-row system: row 0 = `a`, row 1 = empty, row 2 = `b`.
        let mut e = Editor::new(
            &MathNode::parse(
                "<math><mtable columnalign=\"left\"><mtr><mtd><mi>a</mi></mtd></mtr><mtr><mtd/></mtr><mtr><mtd><mi>b</mi></mtd></mtr></mtable></math>",
            )
            .unwrap(),
        );
        // Backspace in middle empty row removes row 1 and moves caret to end of row 0 (`a|`).
        let row1_cell = e.root().children[0].children[1].children[0].id;
        e.set_caret(
            Caret {
                node: row1_cell,
                offset: 0,
            },
            false,
        );
        e.delete(true);
        assert_eq!(e.math().children[0].children.len(), 2);
        assert_eq!(
            e.math().children[0].children[0].children[0].children[0].text,
            "a"
        );
        assert_eq!(
            e.math().children[0].children[1].children[0].children[0].text,
            "b"
        );
        let row0_mi = e.root().children[0].children[0].children[0].children[0].id;
        assert_eq!(
            e.caret(),
            Caret {
                node: row0_mi,
                offset: 1
            }
        );

        // Undo restores row 1 in one step; Delete removes row 1 and moves caret to start of row 2 (`|b`).
        e.undo();
        assert_eq!(e.math().children[0].children.len(), 3);
        e.delete(false);
        assert_eq!(e.math().children[0].children.len(), 2);
        let row1_mi_b = e.root().children[0].children[1].children[0].children[0].id;
        assert_eq!(
            e.caret(),
            Caret {
                node: row1_mi_b,
                offset: 0
            }
        );
        e.redo();
        assert_eq!(e.math().children[0].children.len(), 2);

        // Removing the first row when empty moves caret to start of new first row;
        // removing the last row when empty moves caret to end of previous row.
        let mut edges = Editor::new(
            &MathNode::parse(
                "<math><mtable><mtr><mtd/></mtr><mtr><mtd><mi>m</mi></mtd></mtr><mtr><mtd/></mtr></mtable></math>",
            )
            .unwrap(),
        );
        let first_cell = edges.root().children[0].children[0].children[0].id;
        edges.set_caret(
            Caret {
                node: first_cell,
                offset: 0,
            },
            false,
        );
        edges.delete(true);
        assert_eq!(edges.math().children[0].children.len(), 2);
        let new_first_mi = edges.root().children[0].children[0].children[0].children[0].id;
        assert_eq!(
            edges.caret(),
            Caret {
                node: new_first_mi,
                offset: 0
            }
        );
        let last_cell = edges.root().children[0].children[1].children[0].id;
        edges.set_caret(
            Caret {
                node: last_cell,
                offset: 0,
            },
            false,
        );
        edges.delete(false);
        assert_eq!(edges.math().children[0].children.len(), 1);
        assert_eq!(
            edges.caret(),
            Caret {
                node: new_first_mi,
                offset: 1
            }
        );

        // Single-row non-empty table: Backspace at start of cell moves out without removing row;
        // single-row empty table uses A2 (selects table first, then deletes on second press).
        let only_cell = edges.root().children[0].children[0].children[0].id;
        edges.set_caret(
            Caret {
                node: only_cell,
                offset: 0,
            },
            false,
        );
        edges.delete(true);
        assert_eq!(edges.math().children[0].children.len(), 1);

        let mut single_empty = Editor::new(
            &MathNode::parse("<math><mtable><mtr><mtd/></mtr></mtable></math>").unwrap(),
        );
        let table_id = single_empty.root().children[0].id;
        let single_cell = single_empty.root().children[0].children[0].children[0].id;
        single_empty.set_caret(
            Caret {
                node: single_cell,
                offset: 0,
            },
            false,
        );
        single_empty.delete(true);
        assert_eq!(single_empty.selected_ids(), vec![table_id]);
        assert_eq!(single_empty.math().children[0].children.len(), 1);
        single_empty.delete(true);
        assert!(single_empty.math().children.is_empty());

        // Multi-column row where another cell has content: first Backspace/Delete selects the row
        // with caret at row end; second press removes the row.
        let mut multi = Editor::new(
            &MathNode::parse(
                "<math><mtable><mtr><mtd><mi>x</mi></mtd><mtd><mi>y</mi></mtd></mtr><mtr><mtd/><mtd><mi>z</mi></mtd></mtr></mtable></math>",
            )
            .unwrap(),
        );
        let empty_left = multi.root().children[0].children[1].children[0].id;
        let right_cell = multi.root().children[0].children[1].children[1].id;
        let right_mi = multi.root().children[0].children[1].children[1].children[0].id;
        multi.set_caret(
            Caret {
                node: empty_left,
                offset: 0,
            },
            false,
        );
        multi.delete(true);
        assert_eq!(multi.math().children[0].children.len(), 2);
        assert_eq!(multi.selected_ids(), vec![empty_left, right_cell]);
        assert_eq!(
            multi.caret(),
            Caret {
                node: right_mi,
                offset: 1
            }
        );
        multi.delete(false);
        assert_eq!(multi.math().children[0].children.len(), 1);
    }

    #[test]
    fn horizontal_arrow_stops_skip_redundant_wrappers_and_boundaries() {
        // Flat expression `x+1` has 4 stops (1 step per character/operator).
        let mut flat = Editor::new(&MathNode::with_children(
            "math",
            vec![
                MathNode::leaf("mi", "x"),
                MathNode::leaf("mo", "+"),
                MathNode::leaf("mn", "1"),
            ],
        ));
        let mut stops = Vec::new();
        flat.root().stops(&mut stops);
        assert_eq!(stops.len(), 4);
        // Even starting from non-canonical (math, 0), Right moves in 1 step past 'x'.
        flat.move_caret(Move::Right, false);
        assert_eq!(
            flat.caret(),
            Caret {
                node: flat.root().children[0].id,
                offset: 1
            }
        );
        flat.move_caret(Move::Right, false);
        assert_eq!(
            flat.caret(),
            Caret {
                node: flat.root().children[1].id,
                offset: 1
            }
        );
        flat.move_caret(Move::Right, false);
        assert_eq!(
            flat.caret(),
            Caret {
                node: flat.root().children[2].id,
                offset: 1
            }
        );

        // Nested mrow wrappers around `x` still produce only 2 stops.
        let nested = Editor::new(
            &MathNode::parse("<math><mrow><mrow><mi>x</mi></mrow></mrow></math>").unwrap(),
        );
        nested.root().stops(&mut stops);
        assert_eq!(stops.len(), 2);

        // Fraction `\frac{1}{2}` has 6 stops: before frac, |1, 1|, |2, 2|, after frac.
        let mut frac = Editor::new(
            &MathNode::parse(
                "<math><mfrac><mrow><mn>1</mn></mrow><mrow><mn>2</mn></mrow></mfrac></math>",
            )
            .unwrap(),
        );
        frac.root().stops(&mut stops);
        assert_eq!(stops.len(), 6);
        // From end of numerator `1|`, one Right enters start of denominator `|2`.
        let num_mn = frac.root().children[0].children[0].children[0].id;
        let den_mn = frac.root().children[0].children[1].children[0].id;
        frac.set_caret(
            Caret {
                node: num_mn,
                offset: 1,
            },
            false,
        );
        frac.move_caret(Move::Right, false);
        assert_eq!(
            frac.caret(),
            Caret {
                node: den_mn,
                offset: 0
            }
        );
        // From end of denominator `2|`, one Right exits fraction.
        frac.move_caret(Move::Right, false);
        frac.move_caret(Move::Right, false);
        assert_eq!(
            frac.caret(),
            Caret {
                node: frac.root().id,
                offset: 1
            }
        );

        // Fenced `(x)` has 4 stops: |(x), (|x), (x|), (x)|.
        let mut l = MathNode::leaf("mo", "(");
        l.set_attr("stretchy", "true");
        l.set_attr("form", "prefix");
        let mut r = MathNode::leaf("mo", ")");
        r.set_attr("stretchy", "true");
        r.set_attr("form", "postfix");
        let mut fence = Editor::new(&MathNode::with_children(
            "math",
            vec![MathNode::with_children(
                "mrow",
                vec![
                    l,
                    MathNode::with_children("mrow", vec![MathNode::leaf("mi", "x")]),
                    r,
                ],
            )],
        ));
        fence.root().stops(&mut stops);
        assert_eq!(stops.len(), 4);
        let x_id = fence.root().children[0].children[1].children[0].id;
        fence.set_caret(
            Caret {
                node: x_id,
                offset: 1,
            },
            false,
        );
        fence.move_caret(Move::Right, false);
        assert_eq!(
            fence.caret(),
            Caret {
                node: fence.root().id,
                offset: 1
            }
        );
    }

    #[test]
    fn deleting_last_token_char_prunes_empty_token_and_forward_delete_crosses_tokens() {
        let mut e = Editor::new(
            &MathNode::parse(
                "<math><mfrac><mrow><mn>1</mn></mrow><mrow><mn>2</mn></mrow></mfrac></math>",
            )
            .unwrap(),
        );
        let den_mrow = e.root().children[0].children[1].id;
        let den_mn = e.root().children[0].children[1].children[0].id;
        e.set_caret(
            Caret {
                node: den_mn,
                offset: 1,
            },
            false,
        );
        e.delete(true);
        assert!(e.node(den_mrow).unwrap().children.is_empty());
        assert_eq!(
            e.caret(),
            Caret {
                node: den_mrow,
                offset: 0
            }
        );

        // Multi-char token followed by another token: Backspace then Backspace deletes char-by-char.
        let mut seq = Editor::new(&MathNode::with_children(
            "math",
            vec![
                MathNode::leaf("mi", "ab"),
                MathNode::leaf("mo", "+"),
                MathNode::leaf("mn", "1"),
            ],
        ));
        let one_id = seq.root().children[2].id;
        seq.set_caret(
            Caret {
                node: one_id,
                offset: 1,
            },
            false,
        );
        seq.delete(true);
        assert_eq!(seq.math().children.len(), 2);
        seq.delete(true);
        assert_eq!(seq.math().children.len(), 1);
        seq.delete(true);
        assert_eq!(seq.math().children[0].text, "a");

        // Forward Delete at `a|+` deletes `+` in one step.
        let mut fwd = Editor::new(&MathNode::with_children(
            "math",
            vec![
                MathNode::leaf("mi", "a"),
                MathNode::leaf("mo", "+"),
                MathNode::leaf("mn", "1"),
            ],
        ));
        let a_id = fwd.root().children[0].id;
        fwd.set_caret(
            Caret {
                node: a_id,
                offset: 1,
            },
            false,
        );
        fwd.delete(false);
        assert_eq!(fwd.math().children.len(), 2);
        assert_eq!(fwd.math().children[1].text, "1");
    }

    #[test]
    fn empty_slot_backspace_and_delete_use_a2_selection_and_nonempty_boundaries_step() {
        // Empty denominator in `\frac{1}{□}`: 1st Backspace selects fraction, 2nd deletes it.
        let mut frac = Editor::new(
            &MathNode::parse("<math><mfrac><mrow><mn>1</mn></mrow><mrow/></mfrac></math>").unwrap(),
        );
        let frac_id = frac.root().children[0].id;
        let den_id = frac.root().children[0].children[1].id;
        frac.set_caret(
            Caret {
                node: den_id,
                offset: 0,
            },
            false,
        );
        frac.delete(true);
        assert_eq!(frac.selected_ids(), vec![frac_id]);
        assert_eq!(frac.math().children.len(), 1);
        frac.delete(true);
        assert!(frac.math().children.is_empty());
        frac.undo();
        assert_eq!(frac.math().children.len(), 1);

        // Nested `1 + \frac{2}{□}` selects only inner fraction, not outer expression.
        let mut nested = Editor::new(
            &MathNode::parse(
                "<math><mn>1</mn><mo>+</mo><mfrac><mrow><mn>2</mn></mrow><mrow/></mfrac></math>",
            )
            .unwrap(),
        );
        let inner_frac_id = nested.root().children[2].id;
        let inner_den_id = nested.root().children[2].children[1].id;
        nested.set_caret(
            Caret {
                node: inner_den_id,
                offset: 0,
            },
            false,
        );
        nested.delete(false);
        assert_eq!(nested.selected_ids(), vec![inner_frac_id]);
        nested.delete(false);
        assert_eq!(nested.math().children.len(), 2);

        // Non-empty `\frac{1}{|2}` + Backspace moves to `1|` without selecting/deleting.
        let mut nonempty = Editor::new(
            &MathNode::parse(
                "<math><mfrac><mrow><mn>1</mn></mrow><mrow><mn>2</mn></mrow></mfrac></math>",
            )
            .unwrap(),
        );
        let num_mn = nonempty.root().children[0].children[0].children[0].id;
        let den_mn = nonempty.root().children[0].children[1].children[0].id;
        nonempty.set_caret(
            Caret {
                node: den_mn,
                offset: 0,
            },
            false,
        );
        nonempty.delete(true);
        assert!(nonempty.selected_ids().is_empty());
        assert_eq!(
            nonempty.caret(),
            Caret {
                node: num_mn,
                offset: 1
            }
        );
        // Forward Delete from `1|` moves to `|2`.
        nonempty.delete(false);
        assert!(nonempty.selected_ids().is_empty());
        assert_eq!(
            nonempty.caret(),
            Caret {
                node: den_mn,
                offset: 0
            }
        );

        // Empty fenced `(□)` and fenced 1-row empty `{□` select the enclosing fence first.
        let mut l = MathNode::leaf("mo", "(");
        l.set_attr("stretchy", "true");
        l.set_attr("form", "prefix");
        let mut r = MathNode::leaf("mo", ")");
        r.set_attr("stretchy", "true");
        r.set_attr("form", "postfix");
        let mut fence = Editor::new(&MathNode::with_children(
            "math",
            vec![MathNode::with_children(
                "mrow",
                vec![l, MathNode::new("mrow"), r],
            )],
        ));
        let fence_id = fence.root().children[0].id;
        let inner_id = fence.root().children[0].children[1].id;
        fence.set_caret(
            Caret {
                node: inner_id,
                offset: 0,
            },
            false,
        );
        fence.delete(true);
        assert_eq!(fence.selected_ids(), vec![fence_id]);
        fence.delete(true);
        assert!(fence.math().children.is_empty());
    }

    #[test]
    fn erasing_selected_table_row_removes_row_and_partial_selection_preserves_mtd() {
        // Drag-selecting an empty row across 2 cells in a 2x2 matrix and pressing Backspace removes the row.
        let mut mat = Editor::new(
            &MathNode::parse(
                "<math><mtable><mtr><mtd><mi>a</mi></mtd><mtd><mi>b</mi></mtd></mtr><mtr><mtd/><mtd/></mtr></mtable></math>",
            )
            .unwrap(),
        );
        let r1_c0 = mat.root().children[0].children[1].children[0].id;
        let r1_c1 = mat.root().children[0].children[1].children[1].id;
        mat.set_caret(
            Caret {
                node: r1_c0,
                offset: 0,
            },
            false,
        );
        mat.set_caret(
            Caret {
                node: r1_c1,
                offset: 0,
            },
            true,
        );
        mat.delete(true);
        let math = mat.math();
        assert_eq!(math.children[0].children.len(), 1);
        assert_eq!(math.children[0].children[0].tag, "mtr");
        assert_eq!(math.to_latex(), r"\begin{matrix} a & b \end{matrix}");

        // Partial selection inside a row preserves `mtd` tags (never converts to `mrow`).
        let mut partial = Editor::new(
            &MathNode::parse(
                "<math><mtable><mtr><mtd><mi>a</mi></mtd><mtd><mi>b</mi></mtd></mtr></mtable></math>",
            )
            .unwrap(),
        );
        let c0 = partial.root().children[0].children[0].children[0].id;
        partial.select_node(c0);
        partial.delete(true);
        let cell0 = &partial.math().children[0].children[0].children[0];
        assert_eq!(cell0.tag, "mtd");
        assert!(cell0.children.is_empty());
    }
}
