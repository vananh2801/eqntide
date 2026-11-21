//! Primitive drawing boxes produced by the layout engine.
//!
//! Layout works in pixels with the origin at the left end of the baseline and
//! the y axis pointing *down*.

use crate::render::font::Face;

/// One drawable thing inside a box.
#[derive(Clone, Debug)]
pub(crate) enum Item {
    /// A single glyph at `(x, y)` (baseline origin) drawn at `em` pixels per em,
    /// with optical size script level `ssty` (0 = normal, 1 = script, 2 = scriptscript).
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
    /// Multiple contours forming one compound shape (e.g. a glyph outline with inner holes).
    /// Relative contour winding is preserved so holes stay hollow under nonzero fill.
    Contours(Vec<Vec<(f32, f32)>>),
}

/// A laid-out box: width, ascent, descent and the items to draw.
#[derive(Clone, Debug, Default)]
pub(crate) struct LBox {
    pub(crate) hits: Vec<RawHit>,
    pub(crate) w: f32,
    pub(crate) asc: f32,
    pub(crate) desc: f32,
    pub(crate) items: Vec<Item>,
}

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
    pub(crate) fn mark(&mut self, node: &crate::MathNode) {
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

    pub(crate) fn mark_with_carets(&mut self, node: &crate::MathNode, carets: Vec<f32>) {
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

    /// Put `o` into `self` with its origin at `(dx, dy)` (y down).
    pub(crate) fn place(&mut self, o: &LBox, dx: f32, dy: f32) {
        self.hits.extend(o.hits.iter().map(|h| RawHit {
            x: h.x + dx,
            y: h.y + dy,
            ..h.clone()
        }));
        for it in &o.items {
            self.items.push(match it {
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
                Item::Poly(p) => Item::Poly(p.iter().map(|(x, y)| (x + dx, y + dy)).collect()),
                Item::Contours(cs) => Item::Contours(
                    cs.iter()
                        .map(|p| p.iter().map(|(x, y)| (x + dx, y + dy)).collect())
                        .collect(),
                ),
            });
        }
        self.asc = self.asc.max(o.asc - dy);
        self.desc = self.desc.max(o.desc + dy);
        self.w = self.w.max(dx + o.w);
    }

    /// Returns the ink bounding box `(min_x, max_x, min_y, max_y)` of the box in layout
    /// coordinates (where baseline is at y = 0, y positive downwards).
    pub(crate) fn ink_bounds(&self, f: &crate::render::font::Fonts) -> (f32, f32, f32, f32) {
        let mut min_x: f32 = 0.0;
        let mut max_x: f32 = self.w;
        let mut min_y: f32 = -self.asc;
        let mut max_y: f32 = self.desc;

        for it in &self.items {
            match it {
                Item::Glyph {
                    ch,
                    face,
                    em,
                    x,
                    y,
                    ssty,
                } => {
                    let (gx0, gx1) = f.ink_x_styled(*face, *ch, *em, *ssty);
                    if gx0 != 0.0 || gx1 != 0.0 {
                        min_x = min_x.min(x + gx0);
                        max_x = max_x.max(x + gx1);
                    }
                    let (ga, gd) = f.ink_styled(*face, *ch, *em, *ssty);
                    if ga != 0.0 || gd != 0.0 {
                        min_y = min_y.min(y - ga);
                        max_y = max_y.max(y + gd);
                    }
                }
                Item::Poly(pts) => {
                    for (px, py) in pts {
                        min_x = min_x.min(*px);
                        max_x = max_x.max(*px);
                        min_y = min_y.min(*py);
                        max_y = max_y.max(*py);
                    }
                }
                Item::Contours(cs) => {
                    for pts in cs {
                        for (px, py) in pts {
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
