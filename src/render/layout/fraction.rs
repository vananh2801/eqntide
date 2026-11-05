//! Fraction templates.

use super::*;


impl Layout {
    pub(super) fn frac(&mut self, n: &MathNode, em: f32, script: bool) -> Result<LBox> {
        let num = match n.children.first() {
            Some(c) => self.node(c, em, script)?,
            None => LBox::default(),
        };
        let den = match n.children.get(1) {
            Some(c) => self.node(c, em, script)?,
            None => LBox::default(),
        };
        let thick = if n.attr("linethickness") == Some("0") { 0.0 } else { (RULE * em).max(1.0) };
        let gap = 0.12 * em;
        let pad = 0.08 * em;
        let w = num.w.max(den.w) + 2.0 * pad;
        let axis = AXIS * em;
        let ny = -(axis + thick / 2.0 + gap + num.desc);
        let dy = -axis + thick / 2.0 + gap + den.asc;
        let mut b = LBox::default();
        b.place(&num, (w - num.w) / 2.0, ny);
        b.place(&den, (w - den.w) / 2.0, dy);
        if thick > 0.0 {
            b.items.push(Item::Poly(rect(0.0, -axis - thick / 2.0, w, thick)));
        }
        b.w = w;
        Ok(b)
    }
}
