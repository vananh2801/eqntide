//! Subscript and superscript templates.

use super::*;


impl Layout {
    pub(super) fn scripts(&mut self, n: &MathNode, em: f32, script: bool) -> Result<LBox> {
        let base = match n.children.first() {
            Some(c) => self.node(c, em, script)?,
            None => LBox::default(),
        };
        let (sub_n, sup_n) = match n.tag.as_str() {
            "msub" => (n.children.get(1), None),
            "msup" => (None, n.children.get(1)),
            _ => (n.children.get(1), n.children.get(2)),
        };
        let sem = em * SCRIPT;
        let sub = match sub_n {
            Some(c) => Some(self.node(c, sem, true)?),
            None => None,
        };
        let sup = match sup_n {
            Some(c) => Some(self.node(c, sem, true)?),
            None => None,
        };
        Ok(self.attach_scripts(base, sub, sup, em))
    }

    pub(super) fn prescripts(&mut self, n: &MathNode, em: f32, script: bool) -> Result<LBox> {
        let base = match n.children.first() {
            Some(c) => self.node(c, em, script)?,
            None => LBox::default(),
        };
        let sem = em * SCRIPT;
        let mut pre_sub: Option<LBox> = None;
        let mut pre_sup: Option<LBox> = None;
        if let Some(mp_idx) = n.children.iter().position(|c| c.tag == "mprescripts") {
            if let Some(sub_n) = n.children.get(mp_idx + 1) {
                if sub_n.tag != "none" && (!sub_n.children.is_empty() || !sub_n.text.trim().is_empty()) {
                    pre_sub = Some(self.node(sub_n, sem, true)?);
                }
            }
            if let Some(sup_n) = n.children.get(mp_idx + 2) {
                if sup_n.tag != "none" && (!sup_n.children.is_empty() || !sup_n.text.trim().is_empty()) {
                    pre_sup = Some(self.node(sup_n, sem, true)?);
                }
            }
        }
        let sub_w = pre_sub.as_ref().map_or(0.0, |s| s.w);
        let sup_w = pre_sup.as_ref().map_or(0.0, |s| s.w);
        let pre_w = sub_w.max(sup_w);
        if pre_w == 0.0 {
            return Ok(base);
        }

        let mut sub_y = pre_sub.as_ref().map(|s| (0.22 * em).max(base.desc + 0.05 * em).max(s.asc - 0.32 * em));
        let sup_y = pre_sup.as_ref().map(|s| -(0.40 * em).max(base.asc - 0.39 * sem).max(s.desc + 0.25 * 0.45 * em));
        if let (Some(s1), Some(s2), Some(sy), Some(py)) = (&pre_sub, &pre_sup, sub_y, sup_y) {
            let gap = (sy - s1.asc) - (py + s2.desc);
            let need = 0.18 * em;
            if gap < need {
                sub_y = Some(sy + (need - gap));
            }
        }

        let mut b = LBox::default();
        if let (Some(s), Some(y)) = (&pre_sub, sub_y) {
            b.place(s, pre_w - s.w, y);
        }
        if let (Some(s), Some(y)) = (&pre_sup, sup_y) {
            b.place(s, pre_w - s.w, y);
        }
        let base_x = pre_w + 0.03 * em;
        b.place(&base, base_x, 0.0);
        b.w = base_x + base.w;
        Ok(b)
    }

    pub(super) fn attach_scripts(&self, base: LBox, sub: Option<LBox>, sup: Option<LBox>, em: f32) -> LBox {
        let sem = em * SCRIPT;
        let mut sub_y = sub.as_ref().map(|s| (0.22 * em).max(base.desc + 0.05 * em).max(s.asc - 0.32 * em));

        // Check if the superscript is a prime mark (', ′)
        let is_prime = sup.as_ref().map_or(false, |s| {
            s.items.first().map_or(false, |it| match it {
                Item::Glyph { ch, .. } => matches!(*ch, '\'' | '\u{2032}' | '\u{2033}' | '\u{2034}'),
                _ => false,
            })
        });

        let mut sup_y = sup.as_ref().map(|s| {
            let y = -(0.40 * em).max(base.asc - 0.39 * sem).max(s.desc + 0.25 * 0.45 * em);
            if is_prime {
                // Lower the prime mark so it hugs the top of the base letter
                y + 0.12 * em
            } else {
                y
            }
        });

        if let (Some(s1), Some(s2), Some(sy), Some(py)) = (&sub, &sup, sub_y, sup_y) {
            let gap = (sy - s1.asc) - (py + s2.desc);
            let need = 0.18 * em;
            if gap < need {
                sub_y = Some(sy + (need - gap));
            }
            let _ = &mut sup_y;
        }

        // Shift right slightly to clear the overhanging hook of italic 'f'
        let italic_corr = match base.items.last() {
            Some(Item::Glyph { ch: 'f', face: Face::Variable, .. }) => 0.10 * em,
            _ => 0.0,
        };

        let x = base.w + 0.02 * em;
        let sup_x = x + italic_corr;

        let mut b = LBox::default();
        b.place(&base, 0.0, 0.0);
        let mut w = base.w;

        if let (Some(s), Some(y)) = (&sub, sub_y) {
            b.place(s, x, y);
            w = w.max(x + s.w);
        }
        if let (Some(s), Some(y)) = (&sup, sup_y) {
            b.place(s, sup_x, y);
            if is_prime {
                // Tighten the trailing space after prime so '(' doesn't sit too far away
                w = w.max(sup_x + 0.08 * em);
            } else {
                w = w.max(sup_x + s.w);
            }
        }

        b.w = if is_prime { w } else { w + 0.03 * em };
        b
    }
}
