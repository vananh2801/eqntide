//! Structural editing independent of the window system and UI framework.
#![forbid(unsafe_code)]

use eqntide::MathNode;

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
        if self.token() {
            for offset in 0..=self.text.chars().count() {
                out.push(Caret {
                    node: self.id,
                    offset,
                });
            }
        } else if !self.opaque() {
            if self.row() {
                out.push(Caret {
                    node: self.id,
                    offset: 0,
                });
            }
            for (i, child) in self.children.iter().enumerate() {
                if !child.opaque() {
                    child.stops(out);
                }
                if self.row() {
                    out.push(Caret {
                        node: self.id,
                        offset: i + 1,
                    });
                }
            }
        }
    }
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
    pub fn set_caret(&mut self, caret: Caret, extend: bool) {
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
        let mut stops = vec![];
        self.root().stops(&mut stops);
        let current = stops.iter().position(|c| *c == self.caret()).unwrap_or(0);
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
        let anchor = self.state.anchor?;
        let head = self.caret();
        if anchor == head {
            return None;
        }
        if anchor.node == head.node {
            return Some((
                head.node,
                anchor.offset.min(head.offset),
                anchor.offset.max(head.offset),
            ));
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
        Some((
            parent.id,
            start.min(parent.children.len()),
            end.min(parent.children.len()),
        ))
    }
    pub fn selected_math(&self) -> Option<MathNode> {
        let (id, start, end) = self.selection_span()?;
        let n = self.node(id)?;
        if n.token() {
            let mut result = n.math();
            result.text = n.text.chars().skip(start).take(end - start).collect();
            Some(result)
        } else {
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
            n.children[start..end].iter().map(|n| n.id).collect()
        }
    }
    fn erase_selection(&mut self) -> Option<MathNode> {
        let selected = self.selected_math()?;
        let (id, start, end) = self.selection_span()?;
        let n = self.state.root.find_mut(id)?;
        if n.token() {
            n.text = n
                .text
                .chars()
                .take(start)
                .chain(n.text.chars().skip(end))
                .collect();
        } else if n.row() {
            n.children.drain(start..end);
        } else {
            // Fixed arity structures retain their slots when a child is cleared.
            for child in &mut n.children[start..end] {
                child.tag = "mrow".into();
                child.text.clear();
                child.attrs.clear();
                child.children.clear();
            }
        }
        self.state.caret = Caret {
            node: id,
            offset: start,
        };
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
                    let n = s.state.root.find_mut(c.node).unwrap();
                    n.text.replace_range(
                        char_byte(&n.text, start)..char_byte(&n.text, start + 1),
                        "",
                    );
                    s.state.caret.offset = start;
                } else {
                    s.move_caret(if backward { Move::Left } else { Move::Right }, false);
                }
            } else if n.row() {
                let index = if backward {
                    c.offset.checked_sub(1)
                } else {
                    Some(c.offset)
                };
                if let Some(index) = index.filter(|i| *i < n.children.len()) {
                    s.state
                        .root
                        .find_mut(c.node)
                        .unwrap()
                        .children
                        .remove(index);
                    s.state.caret.offset = index;
                }
            }
        });
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
                        selected = Some(node.children.remove(caret.offset - 1).math());
                        s.state.caret.offset -= 1;
                    } else if node.token() && caret.offset > 0 {
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
}

fn char_byte(text: &str, index: usize) -> usize {
    text.char_indices()
        .nth(index)
        .map_or(text.len(), |(i, _)| i)
}
fn import(math: &MathNode, next: &mut NodeId) -> Node {
    let id = *next;
    *next += 1;
    Node {
        id,
        tag: math.tag.clone(),
        text: math.text.clone(),
        attrs: math.attrs.clone(),
        children: math.children.iter().map(|n| import(n, next)).collect(),
    }
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
            vec![MathNode::leaf("mtext", "Tiếng Việt")],
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
        assert_eq!(e.math().children[0].text, "Tiếng Việt");
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
}
