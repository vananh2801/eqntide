//! Radical templates: square roots and n-th roots.

use super::*;


impl Layout {
    pub(super) fn radical(&self, c: &LBox, deg: Option<&LBox>, em: f32) -> LBox {
        let t = (RULE * em).max(1.0);
        let gap = 0.12 * em;
        let top = -(c.asc + gap) - t;
        let bot = c.desc + 0.04 * em;
        let h = bot - top;
        let rw = 0.62 * em;
        let off = deg.map_or(0.0, |d| (d.w - 0.40 * rw + 0.04 * em).max(0.0));
        let mut b = LBox::default();
        let pts = [
            (off, top + 0.60 * h),
            (off + 0.16 * rw, top + 0.54 * h),
            (off + 0.46 * rw, bot),
            (off + rw, top + t / 2.0),
        ];
        let widths = [0.04 * em, 0.09 * em, 0.04 * em];
        for (i, s) in pts.windows(2).enumerate() {
            for q in stroke(s, widths[i]) {
                b.items.push(Item::Poly(q));
            }
        }
        let cw = c.w + 0.08 * em;
        b.items.push(Item::Poly(rect(off + rw, top, cw, t)));
        b.asc = -top;
        b.desc = bot;
        b.w = off + rw + cw;
        b.place(c, off + rw + 0.03 * em, 0.0);
        if let Some(d) = deg {
            b.place(d, 0.0, top + 0.40 * h);
        }
        b.w = off + rw + cw;
        b
    }
}
