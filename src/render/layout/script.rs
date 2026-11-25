//! Script templates: sub/superscripts, tensor prescripts, and limits.
//!
//! Applies OpenType MATH script shifts, italic corrections, math kerning,
//! and optical size selection (`ssty` feature).

use super::*;

impl Layout {
    /// Lay out `msub`, `msup`, `msubsup`, or script-positioned integral/eval-bar forms.
    pub(super) fn scripts(&mut self, node: &MathNode, em: f32, ctx: MathContext) -> Result<LBox> {
        let (sub_node, sup_node) = match node.tag.as_str() {
            "msub" | "munder" => (node.children.get(1), None),
            "msup" | "mover" => (None, node.children.get(1)),
            _ => (node.children.get(1), node.children.get(2)),
        };

        if let Some(base_node) = node.children.first() {
            if node.attr("data-limloc") == Some("undOvr")
                && base_node.tag == "mo"
                && base_node
                    .text
                    .trim()
                    .chars()
                    .next()
                    .is_some_and(is_integral)
            {
                return self.integral_with_limits(base_node, sub_node, sup_node, em, ctx);
            }
            if delimiter::is_eval_bar_base(base_node) {
                return self.evaluation_bar(sub_node, sup_node, 0.85 * em, 0.65 * em, em, ctx);
            }
            if base_node.tag == "mrow" && base_node.children.len() >= 2 {
                let first = &base_node.children[0];
                let last = &base_node.children[base_node.children.len() - 1];
                if first.tag == "mo"
                    && first.text.trim().is_empty()
                    && delimiter::is_eval_bar_base(last)
                {
                    let inner_nodes = &base_node.children[1..base_node.children.len() - 1];
                    let inner = self.row(inner_nodes, em, ctx)?;
                    let half_span = (inner.asc - AXIS * em)
                        .max(inner.desc + AXIS * em)
                        .max(0.675 * em);
                    let bar_asc = AXIS * em + half_span;
                    let bar_desc = half_span - AXIS * em;
                    let mut eval_box =
                        self.evaluation_bar(sub_node, sup_node, bar_asc, bar_desc, em, ctx)?;
                    eval_box.mark(last);
                    let mut layout_box = LBox::default();
                    layout_box.place(&inner, 0.0, 0.0);
                    let gap = if inner.w > 0.0 { 0.06 * em } else { 0.0 };
                    layout_box.place(&eval_box, inner.w + gap, 0.0);
                    layout_box.w = inner.w + gap + eval_box.w;
                    return Ok(layout_box);
                }
            }
        }

        let base = match node.children.first() {
            Some(child) => self.node(child, em, ctx)?,
            None => LBox::default(),
        };
        let constants = self.font_set.constants();
        let sub_ctx = ctx.for_subscript();
        let sup_ctx = ctx.for_superscript();
        let sub_em = em * ctx.rel_scale(sub_ctx, constants);
        let sup_em = em * ctx.rel_scale(sup_ctx, constants);

        let sub = match sub_node {
            Some(child) => Some(self.node(child, sub_em, sub_ctx)?),
            None => None,
        };
        let sup = match sup_node {
            Some(child) => Some(self.node(child, sup_em, sup_ctx)?),
            None => None,
        };
        Ok(self.attach_scripts(base, sub, sup, em, sup_em, ctx))
    }

    /// Lay out `<mmultiscripts>` tensor prescripts before `base`.
    pub(super) fn prescripts(
        &mut self,
        node: &MathNode,
        em: f32,
        ctx: MathContext,
    ) -> Result<LBox> {
        let base = match node.children.first() {
            Some(child) => self.node(child, em, ctx)?,
            None => LBox::default(),
        };
        let constants = self.font_set.constants();
        let sub_ctx = ctx.for_subscript();
        let sup_ctx = ctx.for_superscript();
        let sub_em = em * ctx.rel_scale(sub_ctx, constants);
        let sup_em = em * ctx.rel_scale(sup_ctx, constants);

        let mut pre_sub: Option<LBox> = None;
        let mut pre_sup: Option<LBox> = None;
        if let Some(mp_idx) = node
            .children
            .iter()
            .position(|child| child.tag == "mprescripts")
        {
            if let Some(sub_node) = node.children.get(mp_idx + 1) {
                if sub_node.tag != "none"
                    && (!sub_node.children.is_empty() || !sub_node.text.trim().is_empty())
                {
                    pre_sub = Some(self.node(sub_node, sub_em, sub_ctx)?);
                }
            }
            if let Some(sup_node) = node.children.get(mp_idx + 2) {
                if sup_node.tag != "none"
                    && (!sup_node.children.is_empty() || !sup_node.text.trim().is_empty())
                {
                    pre_sup = Some(self.node(sup_node, sup_em, sup_ctx)?);
                }
            }
        }
        let sub_w = pre_sub.as_ref().map_or(0.0, |sub_box| sub_box.w);
        let sup_w = pre_sup.as_ref().map_or(0.0, |sup_box| sup_box.w);
        let pre_w = sub_w.max(sup_w);
        if pre_w == 0.0 {
            return Ok(base);
        }

        let shift_up = if ctx.cramped {
            constants.superscript_shift_up_cramped * em
        } else {
            constants.superscript_shift_up * em
        };
        let shift_down = constants.subscript_shift_down * em;
        let gap_min = constants.sub_superscript_gap_min * em;

        let mut sub_y = pre_sub.as_ref().map(|sub_box| {
            shift_down
                .max(base.desc + 0.05 * em)
                .max(sub_box.asc - 0.32 * em)
        });
        let sup_y = pre_sup.as_ref().map(|sup_box| {
            -(shift_up
                .max(base.asc - 0.39 * sup_em)
                .max(sup_box.desc + 0.11 * em))
        });
        if let (Some(sub_box), Some(sup_box), Some(sub_pos), Some(sup_pos)) =
            (&pre_sub, &pre_sup, sub_y, sup_y)
        {
            let gap = (sub_pos - sub_box.asc) - (sup_pos + sup_box.desc);
            if gap < gap_min {
                sub_y = Some(sub_pos + (gap_min - gap));
            }
        }

        let mut layout_box = LBox::default();
        if let (Some(sub_box), Some(y)) = (&pre_sub, sub_y) {
            layout_box.place(sub_box, pre_w - sub_box.w, y);
        }
        if let (Some(sup_box), Some(y)) = (&pre_sup, sup_y) {
            layout_box.place(sup_box, pre_w - sup_box.w, y);
        }
        let base_x = pre_w + 0.03 * em;
        layout_box.place(&base, base_x, 0.0);
        layout_box.w = base_x + base.w;
        Ok(layout_box)
    }

    /// Position optional `sub` and `sup` boxes relative to `base` with OpenType kerning.
    pub(super) fn attach_scripts(
        &self,
        base: LBox,
        sub: Option<LBox>,
        sup: Option<LBox>,
        em: f32,
        sup_em: f32,
        ctx: MathContext,
    ) -> LBox {
        let constants = self.font_set.constants();
        let shift_up = if ctx.cramped {
            constants.superscript_shift_up_cramped * em
        } else {
            constants.superscript_shift_up * em
        };
        let shift_down = constants.subscript_shift_down * em;
        let gap_min = constants.sub_superscript_gap_min * em;

        let mut sub_y = sub.as_ref().map(|sub_box| {
            shift_down
                .max(base.desc + 0.05 * em)
                .max(sub_box.asc - 0.32 * em)
        });

        let is_prime = sup.as_ref().is_some_and(|sup_box| {
            sup_box.items.first().is_some_and(|item| match item {
                Item::Glyph { ch, .. } => {
                    matches!(*ch, '\'' | '\u{2032}' | '\u{2033}' | '\u{2034}')
                }
                _ => false,
            })
        });

        let sup_y = sup.as_ref().map(|sup_box| {
            let y = -(shift_up
                .max(base.asc - 0.39 * sup_em)
                .max(sup_box.desc + 0.11 * em));
            if is_prime {
                // Lower primes moderately to align with letter tops without colliding.
                y + 0.04 * em
            } else {
                y
            }
        });

        if let (Some(sub_box), Some(sup_box), Some(sub_pos), Some(sup_pos)) =
            (&sub, &sup, sub_y, sup_y)
        {
            let gap = (sub_pos - sub_box.asc) - (sup_pos + sup_box.desc);
            if gap < gap_min {
                sub_y = Some(sub_pos + (gap_min - gap));
            }
        }

        let ic = self.italic_correction_of_box(&base);
        let base_glyph = self.last_glyph_of_box(&base);
        let sub_glyph = sub
            .as_ref()
            .and_then(|sub_box| self.first_glyph_of_box(sub_box));
        let sup_glyph = sup
            .as_ref()
            .and_then(|sup_box| self.first_glyph_of_box(sup_box));

        let overhang = base_glyph.map_or(0.0, |(face, ch, glyph_em)| {
            let (_, x_max) = self.font_set.ink_x(face, ch, glyph_em);
            let adv = self.font_set.advance(face, ch, glyph_em);
            (x_max - adv).max(0.0)
        });
        let clearance = ic.max(overhang);

        // Tuck subscript under slanted bases using OpenType math kerning or TeX heuristic.
        let sub_kern = match (base_glyph, sub_glyph, sub_y) {
            (Some((b_face, b_ch, _)), Some((s_face, s_ch, _)), Some(sub_pos)) => {
                let kern = self
                    .font_set
                    .math_kern(b_face, b_ch, s_face, s_ch, true, sub_pos, em);
                if kern != 0.0 {
                    kern
                } else if ic > 0.0 {
                    -(0.35 * ic).min(0.04 * em)
                } else {
                    0.0
                }
            }
            _ => {
                if ic > 0.0 {
                    -(0.35 * ic).min(0.04 * em)
                } else {
                    0.0
                }
            }
        };
        let sub_x = (base.w + 0.02 * em + sub_kern).max(0.0);

        let sup_kern = match (base_glyph, sup_glyph, sup_y) {
            (Some((b_face, b_ch, _)), Some((s_face, s_ch, _)), Some(sup_pos)) => self
                .font_set
                .math_kern(b_face, b_ch, s_face, s_ch, false, sup_pos, em),
            _ => 0.0,
        };
        let sup_x = if is_prime {
            base.w + clearance.max(0.04 * em) + 0.03 * em
        } else {
            base.w + clearance + sup_kern + 0.02 * em
        };

        let mut layout_box = LBox::default();
        layout_box.place(&base, 0.0, 0.0);
        let mut width = base.w;

        if let (Some(sub_box), Some(y)) = (&sub, sub_y) {
            layout_box.place(sub_box, sub_x, y);
            width = width.max(sub_x + sub_box.w);
        }
        if let (Some(sup_box), Some(y)) = (&sup, sup_y) {
            layout_box.place(sup_box, sup_x, y);
            width = width.max(sup_x + sup_box.w);
        }
        layout_box.w = if is_prime {
            width + 0.02 * em
        } else {
            width + 0.03 * em
        };
        layout_box
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    fn layout() -> Layout {
        Layout {
            font_set: fonts(),
            depth: 0,
        }
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
        let mut layout = Layout {
            font_set: fonts(),
            depth: 0,
        };
        let em = 20.0;

        let node_x2 = MathNode::parse("<math><msup><mi>x</mi><mn>2</mn></msup></math>").unwrap();
        let box_x2 = layout
            .scripts(&node_x2.children[0], em, MathContext::DISPLAY)
            .unwrap();

        let ssty_base = box_x2
            .items
            .iter()
            .find_map(|item| match item {
                Item::Glyph { ch: 'x', ssty, .. } => Some(*ssty),
                _ => None,
            })
            .unwrap();
        assert_eq!(ssty_base, 0, "Base 'x' should have ssty = 0");

        let ssty_sup = box_x2
            .items
            .iter()
            .find_map(|item| match item {
                Item::Glyph { ch: '2', ssty, .. } => Some(*ssty),
                _ => None,
            })
            .unwrap();
        assert_eq!(ssty_sup, 1, "Superscript '2' should have ssty = 1");

        let node_nested = MathNode::parse(
            "<math><msup><mi>x</mi><msup><mi>y</mi><mi>z</mi></msup></msup></math>",
        )
        .unwrap();
        let box_nested = layout
            .scripts(&node_nested.children[0], em, MathContext::DISPLAY)
            .unwrap();

        let ssty_z = box_nested
            .items
            .iter()
            .find_map(|item| match item {
                Item::Glyph { ch: 'z', ssty, .. } => Some(*ssty),
                _ => None,
            })
            .unwrap();
        assert_eq!(
            ssty_z, 2,
            "Nested superscript 'z' must have ssty = 2 (ScriptScript)"
        );
    }

    #[test]
    fn test_italic_correction_and_math_kerning_attachment() {
        let mut layout = Layout {
            font_set: fonts(),
            depth: 0,
        };
        let em = 20.0;

        let ic_v = layout.font_set.italic_correction(Face::Variable, 'V', em);
        assert!(
            ic_v > 0.08 * em,
            "Italic correction for 'V' should exceed 0.08 * em, got {}",
            ic_v
        );

        let ic_1 = layout.font_set.italic_correction(Face::Number, '1', em);
        assert_eq!(ic_1, 0.0, "Italic correction for '1' must be 0.0");

        let node_v2 = MathNode::parse("<math><msup><mi>V</mi><mn>2</mn></msup></math>").unwrap();
        let box_v2 = layout
            .scripts(&node_v2.children[0], em, MathContext::DISPLAY)
            .unwrap();
        let base_v_w = box_v2
            .items
            .iter()
            .find_map(|item| match item {
                Item::Glyph { ch: 'V', .. } => {
                    Some(layout.font_set.advance(Face::Variable, 'V', em))
                }
                _ => None,
            })
            .unwrap();
        let sup_v_x = box_v2
            .items
            .iter()
            .find_map(|item| match item {
                Item::Glyph { ch: '2', x, .. } => Some(*x),
                _ => None,
            })
            .unwrap();
        assert!(
            sup_v_x > base_v_w + 0.08 * em,
            "Superscript on 'V' should be shifted right by italic correction, got {}",
            sup_v_x
        );

        let node_p1 = MathNode::parse("<math><msub><mi>P</mi><mn>1</mn></msub></math>").unwrap();
        let box_p1 = layout
            .scripts(&node_p1.children[0], em, MathContext::DISPLAY)
            .unwrap();
        let base_p_w = box_p1
            .items
            .iter()
            .find_map(|item| match item {
                Item::Glyph { ch: 'P', .. } => {
                    Some(layout.font_set.advance(Face::Variable, 'P', em))
                }
                _ => None,
            })
            .unwrap();
        let sub_p_x = box_p1
            .items
            .iter()
            .find_map(|item| match item {
                Item::Glyph { ch: '1', x, .. } => Some(*x),
                _ => None,
            })
            .unwrap();
        assert!(
            sub_p_x < base_p_w + 0.02 * em,
            "Subscript under 'P' should be kerned inward, got sub_p_x = {}, base_p_w = {}",
            sub_p_x,
            base_p_w
        );

        let node_y_prime =
            MathNode::parse("<math><msup><mi>y</mi><mo>′</mo></msup></math>").unwrap();
        let box_y_prime = layout
            .scripts(&node_y_prime.children[0], em, MathContext::DISPLAY)
            .unwrap();
        let base_y_w = box_y_prime
            .items
            .iter()
            .find_map(|item| match item {
                Item::Glyph { ch: 'y', .. } => {
                    Some(layout.font_set.advance(Face::Variable, 'y', em))
                }
                _ => None,
            })
            .unwrap();
        let sup_prime_x = box_y_prime
            .items
            .iter()
            .find_map(|item| match item {
                Item::Glyph { ch: '′', x, .. } => Some(*x),
                _ => None,
            })
            .unwrap();
        assert!(
            sup_prime_x >= base_y_w + 0.06 * em,
            "Prime on 'y' should have sufficient clearance, got sup_prime_x = {}, base_y_w = {}",
            sup_prime_x,
            base_y_w
        );

        let node_f_prime =
            MathNode::parse("<math><msup><mi>f</mi><mo>′</mo></msup></math>").unwrap();
        let box_f_prime = layout
            .scripts(&node_f_prime.children[0], em, MathContext::DISPLAY)
            .unwrap();
        let (_, f_xmax) = layout.font_set.ink_x(Face::Variable, 'f', em);
        let f_prime_x = box_f_prime
            .items
            .iter()
            .find_map(|item| match item {
                Item::Glyph { ch: '′', x, .. } => Some(*x),
                _ => None,
            })
            .unwrap();
        let (prime_xmin, _) = layout.font_set.ink_x(Face::Symbol, '′', em);
        assert!(
            f_prime_x + prime_xmin > f_xmax,
            "Prime mark must clear 'f' top hook ink: f_prime_x + prime_xmin = {}, f_xmax = {}",
            f_prime_x + prime_xmin,
            f_xmax
        );

        let node_row_f_prime =
            MathNode::parse("<math><mrow><mi>f</mi><mo>′</mo></mrow></math>").unwrap();
        let (box_row, _) = layout_math(&node_row_f_prime, 12.0).unwrap();
        let row_prime_x = box_row
            .items
            .iter()
            .find_map(|item| match item {
                Item::Glyph { ch: '′', x, .. } => Some(*x),
                _ => None,
            })
            .unwrap();
        let em12 = 12.0 * PX_PER_PT;
        let (_, f_xmax12) = layout.font_set.ink_x(Face::Variable, 'f', em12);
        let (prime_xmin12, _) = layout.font_set.ink_x(Face::Symbol, '′', em12);
        assert!(
            row_prime_x + prime_xmin12 > f_xmax12,
            "Row prime mark must clear 'f' top hook ink"
        );
    }

    #[test]
    fn test_cramped_superscript_shift_down() {
        let mut layout = Layout {
            font_set: fonts(),
            depth: 0,
        };
        let em = 20.0;
        let node = MathNode::parse("<math><msup><mi>x</mi><mn>2</mn></msup></math>").unwrap();
        let box_normal = layout
            .scripts(&node.children[0], em, MathContext::DISPLAY)
            .unwrap();
        let box_cramped = layout
            .scripts(&node.children[0], em, MathContext::DISPLAY_CRAMPED)
            .unwrap();

        let sup_y_normal = box_normal
            .items
            .iter()
            .find_map(|item| match item {
                Item::Glyph { ch: '2', y, .. } => Some(*y),
                _ => None,
            })
            .unwrap();
        let sup_y_cramped = box_cramped
            .items
            .iter()
            .find_map(|item| match item {
                Item::Glyph { ch: '2', y, .. } => Some(*y),
                _ => None,
            })
            .unwrap();

        assert!(
            sup_y_cramped > sup_y_normal,
            "In cramped style, superscript y-coordinate must be lower (shifted down) than in un-cramped style"
        );
    }

    #[test]
    fn test_evaluation_bar_with_fenced_base_scales_to_inner_height() {
        let math_short =
            crate::service::parse_latex_to_mathml(r"\left. x \right|_{0}^{1}").unwrap();
        let math_tall =
            crate::service::parse_latex_to_mathml(r"\left. \frac{1}{2} \right|_{0}^{1}").unwrap();
        let (box_short, _) = layout_math(&math_short, 14.0).unwrap();
        let (box_tall, _) = layout_math(&math_tall, 14.0).unwrap();

        let (min_y_short, max_y_short) = (
            -box_short.ink_bounds(fonts()).2,
            box_short.ink_bounds(fonts()).3,
        );
        let (min_y_tall, max_y_tall) = (
            -box_tall.ink_bounds(fonts()).2,
            box_tall.ink_bounds(fonts()).3,
        );
        assert!(
            min_y_tall + max_y_tall > min_y_short + max_y_short + 20.0,
            "Evaluation bar for \\frac{{1}}{{2}} ({:.1}) must stretch taller than for x ({:.1})",
            min_y_tall + max_y_tall,
            min_y_short + max_y_short
        );
        let x_pos = box_short
            .items
            .iter()
            .find_map(|item| match item {
                Item::Glyph { ch: 'x', x, .. } => Some(*x),
                _ => None,
            })
            .unwrap();
        let lim_x = box_short
            .items
            .iter()
            .find_map(|item| match item {
                Item::Glyph { ch: '0', x, .. } => Some(*x),
                _ => None,
            })
            .unwrap();
        assert!(lim_x > x_pos + 10.0);
    }

    #[test]
    fn test_double_and_triple_prime_templates_render_level_without_staircase() {
        for source in [r"{#?}''", r"{#?}'''"] {
            let math = crate::catalog::preview_template(source).unwrap();
            let (lbox, _) = layout_math(&math, 14.0).unwrap();
            let primes: Vec<_> = lbox
                .items
                .iter()
                .filter_map(|item| match item {
                    Item::Glyph { ch, y, em, .. }
                        if matches!(*ch, '\'' | '\u{2032}' | '\u{2033}' | '\u{2034}') =>
                    {
                        Some((*ch, *y, *em))
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(
                primes.len(),
                1,
                "{source} should lay out as a single combined prime glyph without nested msup stairs: {primes:?}"
            );
        }
    }
}
