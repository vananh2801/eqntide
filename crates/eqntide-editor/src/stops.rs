//! Canonical caret stop collection and boundary deduplication.
//!
//! Nested `<mrow>` wrappers and adjacent tokens share logical cursor positions.
//! `collect_stops` tags raw positions with a monotonically advancing `group`
//! counter, and `canonical_stops` selects the highest-preference stop per group
//! while maintaining a reverse map from any valid [`Caret`] to its stop index.

use unicode_segmentation::UnicodeSegmentation;

use crate::navigation::Caret;
use crate::node::{is_effectively_empty, Node, NodeId};

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
    /// Priority key used to pick a single canonical caret when multiple tree positions share a visual boundary.
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

impl Node {
    /// Populates `stops_out` with the ordered canonical caret stops inside this subtree.
    pub(crate) fn stops(&self, stops_out: &mut Vec<Caret>) {
        let (canonical, _) = self.canonical_stops();
        *stops_out = canonical;
    }

    /// Computes deduplicated caret stops and a lookup mapping every raw caret in the subtree
    /// to its canonical index.
    pub(crate) fn canonical_stops(&self) -> (Vec<Caret>, Vec<(Caret, usize)>) {
        let mut raw_stops = Vec::new();
        let mut group = 0;
        self.collect_stops(0, &mut group, &mut raw_stops);
        let mut canonical = Vec::new();
        let mut caret_to_idx = Vec::with_capacity(raw_stops.len());
        let mut start_idx = 0;
        while start_idx < raw_stops.len() {
            let group_id = raw_stops[start_idx].group;
            let mut end_idx = start_idx + 1;
            while end_idx < raw_stops.len() && raw_stops[end_idx].group == group_id {
                end_idx += 1;
            }
            let Some(preferred) = raw_stops[start_idx..end_idx]
                .iter()
                .max_by_key(|entry| entry.preference())
            else {
                start_idx = end_idx;
                continue;
            };
            let canonical_idx = canonical.len();
            canonical.push(preferred.caret);
            for entry in &raw_stops[start_idx..end_idx] {
                caret_to_idx.push((entry.caret, canonical_idx));
            }
            start_idx = end_idx;
        }
        (canonical, caret_to_idx)
    }

    /// Recursively emits raw [`StopEntry`] records in visual left-to-right order,
    /// advancing `group` whenever crossing a visible character, delimiter, or slot boundary.
    fn collect_stops(&self, depth: usize, group: &mut usize, stops_out: &mut Vec<StopEntry>) {
        let last_in_group_is_token_or_placeholder = |entries: &[StopEntry], group_id: usize| {
            entries.last().is_some_and(|entry| {
                entry.group == group_id
                    && matches!(entry.kind, StopKind::TokenRest | StopKind::EmptyPlaceholder)
            })
        };
        if self.is_opaque() {
            if last_in_group_is_token_or_placeholder(stops_out, *group) {
                *group += 1;
            }
            *group += 1;
            return;
        }
        if self.is_token() {
            let char_len = self.text.chars().count();
            if self.tag == "mtext" {
                *group += 1;
                let mut offset = 0;
                for grapheme in self.text.graphemes(true) {
                    stops_out.push(StopEntry {
                        caret: Caret {
                            node: self.id,
                            offset,
                        },
                        group: *group,
                        depth,
                        kind: StopKind::TokenRest,
                    });
                    let next_offset = offset + grapheme.chars().count();
                    for inner_offset in offset + 1..next_offset {
                        stops_out.push(StopEntry {
                            caret: Caret {
                                node: self.id,
                                offset: inner_offset,
                            },
                            group: *group,
                            depth,
                            kind: StopKind::TokenStart,
                        });
                    }
                    offset = next_offset;
                    *group += 1;
                }
                stops_out.push(StopEntry {
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
                stops_out.push(StopEntry {
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
                        stops_out.push(StopEntry {
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
                if last_in_group_is_token_or_placeholder(stops_out, *group) {
                    *group += 1;
                }
                stops_out.push(StopEntry {
                    caret: Caret {
                        node: self.id,
                        offset: 0,
                    },
                    group: *group,
                    depth,
                    kind: StopKind::EmptyPlaceholder,
                });
            } else {
                if stops_out.last().is_some_and(|entry| {
                    entry.group == *group && entry.kind == StopKind::EmptyPlaceholder
                }) {
                    *group += 1;
                }
                stops_out.push(StopEntry {
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
                    stops_out.push(StopEntry {
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
        if self.is_row() {
            let is_radical = self.tag == "msqrt";
            if self.children.len() <= 1
                && is_effectively_empty(self)
                && self
                    .children
                    .first()
                    .is_none_or(|child| child.tag != "mtext")
            {
                if is_radical || last_in_group_is_token_or_placeholder(stops_out, *group) {
                    *group += 1;
                }
                let placeholder_id = deepest_empty_id(self);
                stops_out.push(StopEntry {
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
            stops_out.push(StopEntry {
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
            for (index, child) in self.children.iter().enumerate() {
                child.collect_stops(depth + 1, group, stops_out);
                stops_out.push(StopEntry {
                    caret: Caret {
                        node: self.id,
                        offset: index + 1,
                    },
                    group: *group,
                    depth,
                    kind: if child.tag == "mtext"
                        || self
                            .children
                            .get(index + 1)
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
                child.collect_stops(depth + 1, group, stops_out);
            }
            return;
        }
        if self.tag == "mspace" {
            if last_in_group_is_token_or_placeholder(stops_out, *group) {
                *group += 1;
            }
            *group += 1;
            return;
        }
        // Structural containers (`mfrac`, `mroot`, `msub`, `msup`, `mtable`, etc.)
        // have visible boundaries around and between their child slots.
        *group += 1;
        for child in &self.children {
            child.collect_stops(depth + 1, group, stops_out);
            *group += 1;
        }
    }
}

/// Descends through single-child empty wrappers to find the innermost placeholder [`NodeId`].
fn deepest_empty_id(node: &Node) -> NodeId {
    let mut current = node;
    while let Some(first) = current.children.first() {
        current = first;
    }
    current.id
}
