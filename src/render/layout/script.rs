//! Script templates: sub/superscripts, tensor prescripts, and limits.
//!
//! Applies OpenType MATH script shifts, italic corrections, math kerning,
//! and optical size selection (`ssty` feature).

use super::*;

impl Layout {
    pub(super) fn scripts(&mut self, n: &MathNode, em: f32, ctx: MathContext) -> Result<LBox> {
        let (sub_n, sup_n) = match n.tag.as_str() {
            "msub" => (n.children.get(1), None),
            "msup" => (None, n.children.get(1)),
            _ => (n.children.get(1), n.children.get(2)),
        };

        if let Some(base_node) = n.children.first() {
            if delimiter::is_eval_bar_base(base_node) {
                return self.evaluation_bar(sub_n, sup_n, 0.85 * em, 0.65 * em, em, ctx);
            }
        }

        let base = match n.children.first() {
            Some(c) => self.node(c, em, ctx)?,
            None => LBox::default(),
        };
        let c = self.f.constants();
        let sub_ctx = ctx.for_subscript();
        let sup_ctx = ctx.for_superscript();
        let sub_em = em * ctx.rel_scale(sub_ctx, c);
        let sup_em = em * ctx.rel_scale(sup_ctx, c);

        let sub = match sub_n {
            Some(c) => Some(self.node(c, sub_em, sub_ctx)?),
            None => None,
        };
        let sup = match sup_n {
            Some(c) => Some(self.node(c, sup_em, sup_ctx)?),
            None => None,
        };
        Ok(self.attach_scripts(base, sub, sup, em, sup_em, ctx))
    }

    pub(super) fn prescripts(&mut self, n: &MathNode, em: f32, ctx: MathContext) -> Result<LBox> {
        let base = match n.children.first() {
            Some(c) => self.node(c, em, ctx)?,
            None => LBox::default(),
        };
        let c = self.f.constants();
        let sub_ctx = ctx.for_subscript();
        let sup_ctx = ctx.for_superscript();
        let sub_em = em * ctx.rel_scale(sub_ctx, c);
        let sup_em = em * ctx.rel_scale(sup_ctx, c);

        let mut pre_sub: Option<LBox> = None;
        let mut pre_sup: Option<LBox> = None;
        if let Some(mp_idx) = n.children.iter().position(|c| c.tag == "mprescripts") {
            if let Some(sub_n) = n.children.get(mp_idx + 1) {
                if sub_n.tag != "none" && (!sub_n.children.is_empty() || !sub_n.text.trim().is_empty()) {
                    pre_sub = Some(self.node(sub_n, sub_em, sub_ctx)?);
                }
            }
            if let Some(sup_n) = n.children.get(mp_idx + 2) {
                if sup_n.tag != "none" && (!sup_n.children.is_empty() || !sup_n.text.trim().is_empty()) {
                    pre_sup = Some(self.node(sup_n, sup_em, sup_ctx)?);
                }
            }
        }
        let sub_w = pre_sub.as_ref().map_or(0.0, |s| s.w);
        let sup_w = pre_sup.as_ref().map_or(0.0, |s| s.w);
        let pre_w = sub_w.max(sup_w);
        if pre_w == 0.0 {
            return Ok(base);
        }

        let shift_up = if ctx.cramped {
            c.superscript_shift_up_cramped * em
        } else {
            c.superscript_shift_up * em
        };
        let shift_down = c.subscript_shift_down * em;
        let gap_min = c.sub_superscript_gap_min * em;

        let mut sub_y = pre_sub.as_ref().map(|s| shift_down.max(base.desc + 0.05 * em).max(s.asc - 0.32 * em));
        let sup_y = pre_sup.as_ref().map(|s| {
            -(shift_up.max(base.asc - 0.39 * sup_em).max(s.desc + 0.11 * em))
        });
        if let (Some(s1), Some(s2), Some(sy), Some(py)) = (&pre_sub, &pre_sup, sub_y, sup_y) {
            let gap = (sy - s1.asc) - (py + s2.desc);
            if gap < gap_min {
                sub_y = Some(sy + (gap_min - gap));
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

    pub(super) fn attach_scripts(
        &self,
        base: LBox,
        sub: Option<LBox>,
        sup: Option<LBox>,
        em: f32,
        sup_em: f32,
        ctx: MathContext,
    ) -> LBox {
        let c = self.f.constants();
        let shift_up = if ctx.cramped {
            c.superscript_shift_up_cramped * em
        } else {
            c.superscript_shift_up * em
        };
        let shift_down = c.subscript_shift_down * em;
        let gap_min = c.sub_superscript_gap_min * em;

        let mut sub_y = sub.as_ref().map(|s| shift_down.max(base.desc + 0.05 * em).max(s.asc - 0.32 * em));

        // Check if the superscript is a prime mark (', ′)
        let is_prime = sup.as_ref().map_or(false, |s| {
            s.items.first().map_or(false, |it| match it {
                Item::Glyph { ch, .. } => matches!(*ch, '\'' | '\u{2032}' | '\u{2033}' | '\u{2034}'),
                _ => false,
            })
        });

        let sup_y = sup.as_ref().map(|s| {
            let y = -(shift_up.max(base.asc - 0.39 * sup_em).max(s.desc + 0.11 * em));
            if is_prime {
                // Lower the prime mark moderately to align with letter tops,
                // without colliding into outward-slanted strokes.
                y + 0.04 * em
            } else {
                y
            }
        });

        if let (Some(s1), Some(s2), Some(sy), Some(py)) = (&sub, &sup, sub_y, sup_y) {
            let gap = (sy - s1.asc) - (py + s2.desc);
            if gap < gap_min {
                sub_y = Some(sy + (gap_min - gap));
            }
        }

        let ic = self.italic_correction_of_box(&base);
        let base_glyph = self.last_glyph_of_box(&base);
        let sub_glyph = sub.as_ref().and_then(|s| self.first_glyph_of_box(s));
        let sup_glyph = sup.as_ref().and_then(|s| self.first_glyph_of_box(s));

        // Subscript horizontal placement: tucked in under slanted letters (math_kern or TeX heuristic)
        let sub_kern = match (base_glyph, sub_glyph, sub_y) {
            (Some((b_face, b_ch, _)), Some((s_face, s_ch, _)), Some(sy)) => {
                let k = self.f.math_kern(b_face, b_ch, s_face, s_ch, true, sy, em);
                if k != 0.0 {
                    k
                } else if ic > 0.0 {
                    -(0.35 * ic).min(0.04 * em)
                } else {
                    0.0
                }
            }
            _ => if ic > 0.0 { -(0.35 * ic).min(0.04 * em) } else { 0.0 },
        };
        let sub_x = (base.w + 0.02 * em + sub_kern).max(0.0);

        // Superscript horizontal placement: shifted right by italic correction and math kerning
        let sup_kern = match (base_glyph, sup_glyph, sup_y) {
            (Some((b_face, b_ch, _)), Some((s_face, s_ch, _)), Some(py)) => {
                self.f.math_kern(b_face, b_ch, s_face, s_ch, false, py, em)
            }
            _ => 0.0,
        };
        let sup_x = if is_prime {
            // Prime mark needs sufficient clearance from the base glyph, especially for
            // letters like 'y' or 'x' where OpenType italic correction is very small.
            base.w + ic.max(0.04 * em) + 0.03 * em
        } else {
            base.w + ic + sup_kern + 0.02 * em
        };

        let mut b = LBox::default();
        b.place(&base, 0.0, 0.0);
        let mut w = base.w;

        if let (Some(s), Some(y)) = (&sub, sub_y) {
            b.place(s, sub_x, y);
            w = w.max(sub_x + s.w);
        }
        if let (Some(s), Some(y)) = (&sup, sup_y) {
            b.place(s, sup_x, y);
            w = w.max(sup_x + s.w);
        }
        b.w = if is_prime { w + 0.02 * em } else { w + 0.03 * em };
        b
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    fn layout() -> Layout {
        Layout { f: fonts(), depth: 0 }
    }

    #[test]
    fn test_math_style_transitions() {
        assert_eq!(Face::auto_detect('x'), Face::Variable);
        assert_eq!(Face::auto_detect('2'), Face::Number);
        assert_eq!(Face::auto_detect('+'), Face::Symbol);
        assert_eq!(Face::auto_detect('α'), Face::LcGreek);
        assert_eq!(Face::auto_detect('Γ'), Face::UcGreek);
        assert_eq!(Face::auto_detect('∑'), Face::ExtraMath);
    }

    #[test]
    fn test_script_optical_size_ssty_feature() {
        let mut l = Layout { f: fonts(), depth: 0 };
        let em = 20.0;

        // In x^2: base 'x' is at level 0 (Display/Text), superscript '2' is at level 1 (Script style)
        let node_x2 = MathNode::parse("<math><msup><mi>x</mi><mn>2</mn></msup></math>").unwrap();
        let b_x2 = l.scripts(&node_x2.children[0], em, MathContext::DISPLAY).unwrap();

        let ssty_base = b_x2.items.iter().find_map(|it| match it {
            Item::Glyph { ch: 'x', ssty, .. } => Some(*ssty),
            _ => None,
        }).unwrap();
        assert_eq!(ssty_base, 0, "Base 'x' should have ssty = 0");

        let ssty_sup = b_x2.items.iter().find_map(|it| match it {
            Item::Glyph { ch: '2', ssty, .. } => Some(*ssty),
            _ => None,
        }).unwrap();
        assert_eq!(ssty_sup, 1, "Superscript '2' should have ssty = 1");

        // In nested script x^{y^z}:
        // 'x' has ssty = 0
        // 'y' has ssty = 1
        // 'z' has ssty = 2 (ScriptScript style)
        let node_nested = MathNode::parse("<math><msup><mi>x</mi><msup><mi>y</mi><mi>z</mi></msup></msup></math>").unwrap();
        let b_nested = l.scripts(&node_nested.children[0], em, MathContext::DISPLAY).unwrap();

        let ssty_z = b_nested.items.iter().find_map(|it| match it {
            Item::Glyph { ch: 'z', ssty, .. } => Some(*ssty),
            _ => None,
        }).unwrap();
        assert_eq!(ssty_z, 2, "Nested superscript 'z' must have ssty = 2 (ScriptScript)");
    }

    #[test]
    fn test_italic_correction_and_math_kerning_attachment() {
        let mut l = Layout { f: fonts(), depth: 0 };
        let em = 20.0;

        // Base 'V' has substantial italic correction (100 units in STIX Two Math = 0.10 * em)
        let ic_v = l.f.italic_correction(Face::Variable, 'V', em);
        assert!(ic_v > 0.08 * em, "Italic correction for 'V' should exceed 0.08 * em, got {}", ic_v);

        // Base '1' (upright digit) has zero italic correction
        let ic_1 = l.f.italic_correction(Face::Number, '1', em);
        assert_eq!(ic_1, 0.0, "Italic correction for '1' must be 0.0");

        // When placing superscript '2' on 'V':
        let node_v2 = MathNode::parse("<math><msup><mi>V</mi><mn>2</mn></msup></math>").unwrap();
        let b_v2 = l.scripts(&node_v2.children[0], em, MathContext::DISPLAY).unwrap();
        let base_v_w = b_v2.items.iter().find_map(|it| match it {
            Item::Glyph { ch: 'V', .. } => Some(l.f.advance(Face::Variable, 'V', em)),
            _ => None,
        }).unwrap();
        let sup_v_x = b_v2.items.iter().find_map(|it| match it {
            Item::Glyph { ch: '2', x, .. } => Some(*x),
            _ => None,
        }).unwrap();
        assert!(sup_v_x > base_v_w + 0.08 * em, "Superscript on 'V' should be shifted right by italic correction, got {}", sup_v_x);

        // Base 'P' with subscript '1': STIX Two Math has bottom-right kerning for 'P' (negative),
        // tucking the subscript in under the overhang.
        let node_p1 = MathNode::parse("<math><msub><mi>P</mi><mn>1</mn></msub></math>").unwrap();
        let b_p1 = l.scripts(&node_p1.children[0], em, MathContext::DISPLAY).unwrap();
        let base_p_w = b_p1.items.iter().find_map(|it| match it {
            Item::Glyph { ch: 'P', .. } => Some(l.f.advance(Face::Variable, 'P', em)),
            _ => None,
        }).unwrap();
        let sub_p_x = b_p1.items.iter().find_map(|it| match it {
            Item::Glyph { ch: '1', x, .. } => Some(*x),
            _ => None,
        }).unwrap();
        // Since bottom-right kern is negative, sub_p_x < base_p_w + 0.02 * em
        assert!(sub_p_x < base_p_w + 0.02 * em, "Subscript under 'P' should be kerned inward, got sub_p_x = {}, base_p_w = {}", sub_p_x, base_p_w);

        // Base 'y' with prime '\'': prime mark has sufficient clearance to avoid colliding into y
        let node_y_prime = MathNode::parse("<math><msup><mi>y</mi><mo>′</mo></msup></math>").unwrap();
        let b_y_prime = l.scripts(&node_y_prime.children[0], em, MathContext::DISPLAY).unwrap();
        let base_y_w = b_y_prime.items.iter().find_map(|it| match it {
            Item::Glyph { ch: 'y', .. } => Some(l.f.advance(Face::Variable, 'y', em)),
            _ => None,
        }).unwrap();
        let sup_prime_x = b_y_prime.items.iter().find_map(|it| match it {
            Item::Glyph { ch: '′', x, .. } => Some(*x),
            _ => None,
        }).unwrap();
        assert!(sup_prime_x >= base_y_w + 0.06 * em, "Prime on 'y' should have sufficient clearance, got sup_prime_x = {}, base_y_w = {}", sup_prime_x, base_y_w);
    }

    #[test]
    fn test_cramped_superscript_shift_down() {
        let mut l = Layout { f: fonts(), depth: 0 };
        let em = 20.0;
        let node = MathNode::parse("<math><msup><mi>x</mi><mn>2</mn></msup></math>").unwrap();
        let b_normal = l.scripts(&node.children[0], em, MathContext::DISPLAY).unwrap();
        let b_cramped = l.scripts(&node.children[0], em, MathContext::DISPLAY_CRAMPED).unwrap();

        let sup_y_normal = b_normal.items.iter().find_map(|it| match it {
            Item::Glyph { ch: '2', y, .. } => Some(*y),
            _ => None,
        }).unwrap();
        let sup_y_cramped = b_cramped.items.iter().find_map(|it| match it {
            Item::Glyph { ch: '2', y, .. } => Some(*y),
            _ => None,
        }).unwrap();

        assert!(sup_y_cramped > sup_y_normal, "In cramped style, superscript y-coordinate must be lower (shifted down) than in un-cramped style");
    }
}
