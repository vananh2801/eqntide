//! Primitive drawing boxes produced by the layout engine.
//!
//! Layout works in pixels with the origin at the left end of the baseline and
//! the y axis pointing *down*.

use crate::render::font::Face;

/// One drawable thing inside a box.
#[derive(Clone, Debug)]
pub(crate) enum Item {
    /// A single glyph at `(x, y)` (baseline origin) drawn at `em` pixels per em.
    Glyph { ch: char, face: Face, em: f32, x: f32, y: f32 },
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
                Item::Glyph { ch, face, em, x, y } => Item::Glyph {
                    ch: *ch,
                    face: *face,
                    em: *em,
                    x: x + dx,
                    y: y + dy,
                },
                Item::Poly(p) => Item::Poly(p.iter().map(|(x, y)| (x + dx, y + dy)).collect()),
            });
        }
        self.asc = self.asc.max(o.asc - dy);
        self.desc = self.desc.max(o.desc + dy);
        self.w = self.w.max(dx + o.w);
    }
}
