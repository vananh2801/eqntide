//! Primitive drawing boxes produced by the layout engine.
//!
//! Layout works in pixels with the origin at the left end of the baseline and
//! the y axis pointing *down*.

use crate::render::font::{Face, Fonts};
use crate::MathNode;

/// One drawable thing inside a box.
#[derive(Clone, Debug)]
pub(crate) enum Item {
    /// A single glyph at `(x, y)` (baseline origin) drawn at `em` pixels per em,
    /// with optical size script level `ssty` (`0` = normal, `1` = script, `2` = scriptscript).
    Glyph {
        ch: char,
        face: Face,
        em: f32,
        x: f32,
        y: f32,
        ssty: u8,
    },
    /// A single filled polygon (rules, brace outlines, clipped pieces, …).
    Poly(Vec<(f32, f32)>),
    /// Multiple contours forming one compound shape (such as a glyph outline with inner holes).
    /// Relative contour winding is preserved so holes stay hollow under nonzero fill.
    Contours(Vec<Vec<(f32, f32)>>),
}

/// A laid-out box: width, ascent, descent, hit targets, and the items to draw.
#[derive(Clone, Debug, Default)]
pub(crate) struct LBox {
    pub(crate) hits: Vec<RawHit>,
    pub(crate) w: f32,
    pub(crate) asc: f32,
    pub(crate) desc: f32,
    pub(crate) items: Vec<Item>,
}

/// Interactive hit-testing bounds recorded during box layout before coordinate normalization.
#[derive(Clone, Debug)]
pub(crate) struct RawHit {
    pub key: usize,
    pub carets: Vec<f32>,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub asc: f32,
    pub desc: f32,
}

impl LBox {
    pub(crate) fn mark(&mut self, node: &MathNode) {
        let mut carets: Vec<f32> = self
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Glyph { x, .. } => Some(*x),
                _ => None,
            })
            .collect();
        if node.text.is_empty() {
            carets.clear();
        } else {
            carets.push(self.w);
        }
        self.mark_with_carets(node, carets);
    }

    pub(crate) fn mark_with_carets(&mut self, node: &MathNode, carets: Vec<f32>) {
        self.hits.push(RawHit {
            carets,
            key: node as *const _ as usize,
            x: 0.0,
            y: 0.0,
            w: self.w,
            asc: self.asc,
            desc: self.desc,
        });
    }

    /// Place `child_box` into `self` with its origin at `(dx, dy)` (y down).
    pub(crate) fn place(&mut self, child_box: &LBox, dx: f32, dy: f32) {
        self.hits.extend(child_box.hits.iter().map(|hit| RawHit {
            x: hit.x + dx,
            y: hit.y + dy,
            ..hit.clone()
        }));
        for item in &child_box.items {
            self.items.push(match item {
                Item::Glyph {
                    ch,
                    face,
                    em,
                    x,
                    y,
                    ssty,
                } => Item::Glyph {
                    ch: *ch,
                    face: *face,
                    em: *em,
                    x: x + dx,
                    y: y + dy,
                    ssty: *ssty,
                },
                Item::Poly(points) => {
                    Item::Poly(points.iter().map(|(x, y)| (x + dx, y + dy)).collect())
                }
                Item::Contours(contours) => Item::Contours(
                    contours
                        .iter()
                        .map(|points| points.iter().map(|(x, y)| (x + dx, y + dy)).collect())
                        .collect(),
                ),
            });
        }
        self.asc = self.asc.max(child_box.asc - dy);
        self.desc = self.desc.max(child_box.desc + dy);
        self.w = self.w.max(dx + child_box.w);
    }

    /// Return the ink bounding box `(min_x, max_x, min_y, max_y)` in layout
    /// coordinates (where baseline is at `y = 0`, y positive downwards).
    pub(crate) fn ink_bounds(&self, font_set: &Fonts) -> (f32, f32, f32, f32) {
        let mut min_x: f32 = 0.0;
        let mut max_x: f32 = self.w;
        let mut min_y: f32 = -self.asc;
        let mut max_y: f32 = self.desc;

        for item in &self.items {
            match item {
                Item::Glyph {
                    ch,
                    face,
                    em,
                    x,
                    y,
                    ssty,
                } => {
                    let (gx0, gx1) = font_set.ink_x_styled(*face, *ch, *em, *ssty);
                    if gx0 != 0.0 || gx1 != 0.0 {
                        min_x = min_x.min(x + gx0);
                        max_x = max_x.max(x + gx1);
                    }
                    let (glyph_asc, glyph_desc) = font_set.ink_styled(*face, *ch, *em, *ssty);
                    if glyph_asc != 0.0 || glyph_desc != 0.0 {
                        min_y = min_y.min(y - glyph_asc);
                        max_y = max_y.max(y + glyph_desc);
                    }
                }
                Item::Poly(points) => {
                    for (px, py) in points {
                        min_x = min_x.min(*px);
                        max_x = max_x.max(*px);
                        min_y = min_y.min(*py);
                        max_y = max_y.max(*py);
                    }
                }
                Item::Contours(contours) => {
                    for points in contours {
                        for (px, py) in points {
                            min_x = min_x.min(*px);
                            max_x = max_x.max(*px);
                            min_y = min_y.min(*py);
                            max_y = max_y.max(*py);
                        }
                    }
                }
            }
        }
        (min_x, max_x, min_y, max_y)
    }
}
