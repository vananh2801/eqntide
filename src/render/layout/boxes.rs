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
    /// A filled polygon (rules, radical strokes, brace outlines, …).
    Poly(Vec<(f32, f32)>),
}

/// A laid-out box: width, ascent, descent and the items to draw.
#[derive(Clone, Debug, Default)]
pub(crate) struct LBox {
    pub(crate) w: f32,
    pub(crate) asc: f32,
    pub(crate) desc: f32,
    pub(crate) items: Vec<Item>,
}

impl LBox {
    /// Put `o` into `self` with its origin at `(dx, dy)` (y down).
    pub(crate) fn place(&mut self, o: &LBox, dx: f32, dy: f32) {
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
            }
        }
        (min_x, max_x, min_y, max_y)
    }
}
