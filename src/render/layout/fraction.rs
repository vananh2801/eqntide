//! Fraction templates: TeX Rule 15 and OpenType MATH table standard layout.

use super::*;

impl Layout {
    /// Lay out an `<mfrac>` (stacked rule, binomial stack, or bevelled slash).
    pub(super) fn frac(&mut self, node: &MathNode, em: f32, ctx: MathContext) -> Result<LBox> {
        let small = node.attr("displaystyle") == Some("false");
        let ctx = match node.attr("displaystyle") {
            Some("false") => MathContext::TEXT,
            Some("true") => MathContext::DISPLAY,
            _ => ctx,
        };
        if node.attr("bevelled") == Some("true") {
            return self.bevelled_fraction(node, em, ctx, small);
        }
        let constants = self.font_set.constants();
        let (num_ctx, den_ctx) = if small {
            (ctx.for_superscript(), ctx.for_subscript())
        } else {
            (ctx.for_numerator(), ctx.for_denominator())
        };
        let num_em = em * ctx.rel_scale(num_ctx, constants);
        let den_em = em * ctx.rel_scale(den_ctx, constants);

        let num = match node.children.first() {
            Some(child) => self.node(child, num_em, num_ctx)?,
            None => LBox::default(),
        };
        let den = match node.children.get(1) {
            Some(child) => self.node(child, den_em, den_ctx)?,
            None => LBox::default(),
        };

        let axis = constants.axis_height * em;
        let is_display = ctx.is_display();

        let has_bar = node.attr("linethickness") != Some("0");
        let thick = if has_bar {
            (constants.fraction_rule_thickness * em).max(1.0)
        } else {
            0.0
        };

        // Horizontal overhang padding beyond the wider of numerator and denominator.
        let pad = 0.08 * em;
        let width = num.w.max(den.w) + 2.0 * pad;

        let (num_y, den_y) = if has_bar {
            // TeX Rule 15b & 15d: Fraction with a horizontal rule.
            let (mut shift_up, num_gap_min, den_gap_min) = if is_display {
                (
                    constants.fraction_num_display_style_shift_up * em,
                    constants.fraction_num_display_style_gap_min * em,
                    constants.fraction_den_display_style_gap_min * em,
                )
            } else if small || ctx.is_script() {
                (
                    (axis + thick / 2.0 + num.desc + constants.fraction_num_gap_min * em)
                        .max(0.42 * em),
                    constants.fraction_num_gap_min * em,
                    constants.fraction_den_gap_min * em,
                )
            } else {
                (
                    constants.fraction_num_shift_up * em,
                    constants.fraction_num_gap_min * em,
                    constants.fraction_den_gap_min * em,
                )
            };

            let num_clearance = shift_up - num.desc - axis - thick / 2.0;
            if num_clearance < num_gap_min {
                shift_up += num_gap_min - num_clearance;
            }

            let actual_num_clearance = shift_up - num.desc - axis - thick / 2.0;
            let den_clearance = if ctx.is_script() {
                actual_num_clearance.max(den_gap_min)
            } else {
                actual_num_clearance.min(den_gap_min * 2.2).max(den_gap_min)
            };

            let shift_down = -axis + thick / 2.0 + den_clearance + den.asc;
            (-shift_up, shift_down)
        } else {
            // TeX Rule 15c: Stack without a rule (e.g. binomial coefficients).
            let (mut shift_up, mut shift_down, min_gap) = if is_display {
                (
                    constants.fraction_num_display_style_shift_up * em,
                    constants.fraction_den_display_style_shift_down * em,
                    7.0 * constants.fraction_rule_thickness * em,
                )
            } else {
                (
                    constants.fraction_num_shift_up * em,
                    constants.fraction_den_shift_down * em,
                    3.0 * constants.fraction_rule_thickness * em,
                )
            };

            let actual_gap = shift_up - num.desc + shift_down - den.asc;
            if actual_gap < min_gap {
                let diff = (min_gap - actual_gap) / 2.0;
                shift_up += diff;
                shift_down += diff;
            }

            (-shift_up, shift_down)
        };

        let mut layout_box = LBox::default();
        layout_box.place(&num, (width - num.w) / 2.0, num_y);
        layout_box.place(&den, (width - den.w) / 2.0, den_y);
        if thick > 0.0 {
            layout_box
                .items
                .push(Item::Poly(rect(0.0, -axis - thick / 2.0, width, thick)));
        }
        layout_box.w = width;
        Ok(layout_box)
    }

    fn bevelled_fraction(
        &mut self,
        node: &MathNode,
        em: f32,
        ctx: MathContext,
        small: bool,
    ) -> Result<LBox> {
        let part_ctx = if small {
            ctx.for_superscript()
        } else {
            ctx.for_numerator()
        };
        let part_em = em * ctx.rel_scale(part_ctx, self.font_set.constants());
        let num = node
            .children
            .first()
            .map(|child| self.node(child, part_em, part_ctx))
            .transpose()?
            .unwrap_or_default();
        let den = node
            .children
            .get(1)
            .map(|child| self.node(child, part_em, part_ctx))
            .transpose()?
            .unwrap_or_default();
        let gap = 0.12 * em;
        let slash_w = 0.4 * em;
        let up = 0.25 * em;
        let down = 0.20 * em;
        let mut layout_box = LBox::default();
        layout_box.place(&num, 0.0, -up);
        let slash_x = num.w + gap;
        let top = (-num.asc - up).min(-0.7 * em);
        let bottom = (den.desc + down).max(0.2 * em);
        for poly in stroke(&[(slash_x, bottom), (slash_x + slash_w, top)], 0.045 * em) {
            layout_box.items.push(Item::Poly(poly));
        }
        layout_box.place(&den, slash_x + slash_w + gap, down);
        layout_box.asc = layout_box.asc.max(-top);
        layout_box.desc = layout_box.desc.max(bottom);
        Ok(layout_box)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fraction_vertical_clearance_and_limit_symmetry() {
        let frac_str = "<math><msubsup><mo>∫</mo><mi>π</mi><mrow><mn>100</mn><mo>+</mo><mfrac><mi>π</mi><mn>2</mn></mfrac></mrow></msubsup><mrow><mfrac><mn>1</mn><mn>2</mn></mfrac><mi>sin</mi><mi>x</mi><mi>d</mi><mi>x</mi></mrow></math>";
        let node = MathNode::parse(frac_str).unwrap();
        let (layout_box, em) = layout_math(&node, 12.0).unwrap();
        assert!(em > 0.0);
        assert!(!layout_box.items.is_empty());

        let poly_bars: Vec<_> = layout_box
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Poly(points) => Some(points),
                _ => None,
            })
            .collect();
        assert!(!poly_bars.is_empty(), "Fraction bar must be present");
    }

    #[test]
    fn test_fraction_baseline_standardization_tex_rule_15() {
        let frac_a_b = MathNode::parse("<math><mfrac><mi>a</mi><mi>b</mi></mfrac></math>").unwrap();
        let frac_x_y = MathNode::parse("<math><mfrac><mi>x</mi><mi>y</mi></mfrac></math>").unwrap();

        let mut layout = Layout {
            font_set: fonts(),
            depth: 0,
        };
        let em = 20.0;

        let box_ab = layout
            .frac(&frac_a_b.children[0], em, MathContext::DISPLAY)
            .unwrap();
        let box_xy = layout
            .frac(&frac_x_y.children[0], em, MathContext::DISPLAY)
            .unwrap();

        let get_glyph_y = |layout_box: &LBox| -> f32 {
            layout_box
                .items
                .iter()
                .find_map(|item| match item {
                    Item::Glyph { y, .. } => Some(*y),
                    _ => None,
                })
                .unwrap()
        };

        let y1 = get_glyph_y(&box_ab);
        let y2 = get_glyph_y(&box_xy);

        // Under TeX Rule 15, 'a' and 'x' fit within shift_up and share the same baseline shift.
        assert!(
            (y1 - y2).abs() < 1e-4,
            "y1 ({}) and y2 ({}) must be equal",
            y1,
            y2
        );

        // A numerator with a deep descender (g) pushes shift_up to clear min_gap.
        let frac_g_y = MathNode::parse("<math><mfrac><mi>g</mi><mi>y</mi></mfrac></math>").unwrap();
        let box_gy = layout
            .frac(&frac_g_y.children[0], em, MathContext::DISPLAY)
            .unwrap();
        let y_g = get_glyph_y(&box_gy);
        assert!(
            y_g < y1,
            "g must be shifted up further to clear the fraction rule (y_g={}, y1={})",
            y_g,
            y1
        );

        let binom = MathNode::parse(
            "<math><mfrac linethickness=\"0\"><mn>10</mn><mn>3</mn></mfrac></math>",
        )
        .unwrap();
        let box_binom = layout
            .frac(&binom.children[0], em, MathContext::DISPLAY)
            .unwrap();
        assert!(!box_binom
            .items
            .iter()
            .any(|item| matches!(item, Item::Poly(_))));
    }

    #[test]
    fn test_fraction_parts_full_size_in_text_and_table() {
        let mut layout = Layout {
            font_set: fonts(),
            depth: 0,
        };
        let em = 24.0;
        let frac = MathNode::parse("<math><mfrac><mn>1</mn><mn>2</mn></mfrac></math>").unwrap();

        let box_disp = layout
            .frac(&frac.children[0], em, MathContext::DISPLAY)
            .unwrap();
        let box_text = layout
            .frac(&frac.children[0], em, MathContext::TEXT)
            .unwrap();

        let get_em = |layout_box: &LBox| -> f32 {
            layout_box
                .items
                .iter()
                .find_map(|item| match item {
                    Item::Glyph { ch: '1', em, .. } => Some(*em),
                    _ => None,
                })
                .unwrap()
        };

        let em_disp = get_em(&box_disp);
        let em_text = get_em(&box_text);

        // In MathType / OMML, numerator & denominator stay at full size in display and text/table.
        assert_eq!(em_disp, em);
        assert_eq!(em_text, em);
    }

    #[test]
    fn test_small_fraction_displaystyle_false_shrinks_parts() {
        let mut layout = Layout {
            font_set: fonts(),
            depth: 0,
        };
        let em = 24.0;
        let small_stacked = MathNode::parse(
            r#"<math><mfrac displaystyle="false"><mn>1</mn><mn>2</mn></mfrac></math>"#,
        )
        .unwrap();
        let small_bevel = MathNode::parse(
            r#"<math><mfrac bevelled="true" displaystyle="false"><mn>1</mn><mn>2</mn></mfrac></math>"#,
        )
        .unwrap();

        let get_em = |layout_box: &LBox| -> f32 {
            layout_box
                .items
                .iter()
                .find_map(|item| match item {
                    Item::Glyph { ch: '1', em, .. } => Some(*em),
                    _ => None,
                })
                .unwrap()
        };

        let em_small = get_em(
            &layout
                .frac(&small_stacked.children[0], em, MathContext::DISPLAY)
                .unwrap(),
        );
        let em_small_bevel = get_em(
            &layout
                .frac(&small_bevel.children[0], em, MathContext::DISPLAY)
                .unwrap(),
        );
        assert!(
            em_small < 0.8 * em,
            "expected script-size tfrac, got {em_small}"
        );
        assert!(
            em_small_bevel < 0.8 * em,
            "expected script-size tsfrac, got {em_small_bevel}"
        );
    }
}
