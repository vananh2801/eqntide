//! Fraction templates: TeX Rule 15 and OpenType MATH table standard layout.

use super::*;

impl Layout {
    pub(super) fn frac(&mut self, n: &MathNode, em: f32, ctx: MathContext) -> Result<LBox> {
        let small = n.attr("displaystyle") == Some("false");
        let ctx = match n.attr("displaystyle") {
            Some("false") => MathContext::TEXT,
            Some("true") => MathContext::DISPLAY,
            _ => ctx,
        };
        if n.attr("bevelled") == Some("true") {
            return self.bevelled_fraction(n, em, ctx, small);
        }
        let c = self.f.constants();
        let (num_ctx, den_ctx) = if small {
            (ctx.for_superscript(), ctx.for_subscript())
        } else {
            (ctx.for_numerator(), ctx.for_denominator())
        };
        let num_em = em * ctx.rel_scale(num_ctx, c);
        let den_em = em * ctx.rel_scale(den_ctx, c);

        let num = match n.children.first() {
            Some(k) => self.node(k, num_em, num_ctx)?,
            None => LBox::default(),
        };
        let den = match n.children.get(1) {
            Some(k) => self.node(k, den_em, den_ctx)?,
            None => LBox::default(),
        };

        let axis = c.axis_height * em;
        let is_display = ctx.is_display();

        let has_bar = n.attr("linethickness") != Some("0");
        let thick = if has_bar {
            (c.fraction_rule_thickness * em).max(1.0)
        } else {
            0.0
        };

        // Horizontal overhang padding beyond the wider of numerator and denominator
        let pad = 0.08 * em;
        let w = num.w.max(den.w) + 2.0 * pad;

        let (ny, dy) = if has_bar {
            // TeX Rule 15b & 15d: Fraction with a horizontal rule
            let (mut shift_up, num_gap_min, den_gap_min) = if is_display {
                (
                    c.fraction_num_display_style_shift_up * em,
                    c.fraction_num_display_style_gap_min * em,
                    c.fraction_den_display_style_gap_min * em,
                )
            } else if small || ctx.is_script() {
                // Small / script fraction (e.g. \tfrac or nested script): compact shift
                (
                    (axis + thick / 2.0 + num.desc + c.fraction_num_gap_min * em).max(0.42 * em),
                    c.fraction_num_gap_min * em,
                    c.fraction_den_gap_min * em,
                )
            } else {
                (
                    c.fraction_num_shift_up * em,
                    c.fraction_num_gap_min * em,
                    c.fraction_den_gap_min * em,
                )
            };

            // Numerator clearance: distance from numerator bottom (-shift_up + num.desc)
            // to the top edge of the fraction rule (-axis - thick / 2.0).
            let num_clearance = shift_up - num.desc - axis - thick / 2.0;
            if num_clearance < num_gap_min {
                shift_up += num_gap_min - num_clearance;
            }

            // Actual clearance between bottom of numerator and top of fraction rule
            let actual_num_clearance = shift_up - num.desc - axis - thick / 2.0;

            // Denominator clearance: symmetrical with numerator clearance,
            // bounded by [den_gap_min, actual_num_clearance]
            let den_clearance = if ctx.is_script() {
                actual_num_clearance.max(den_gap_min)
            } else {
                actual_num_clearance.min(den_gap_min * 2.2).max(den_gap_min)
            };

            // Denominator baseline: top edge of denominator sits at (-axis + thick / 2.0 + den_clearance)
            let shift_down = -axis + thick / 2.0 + den_clearance + den.asc;

            (-shift_up, shift_down)
        } else {
            // TeX Rule 15c: Stack without a rule (e.g. binomial coefficients \binom{n}{k})
            let (mut shift_up, mut shift_down, min_gap) = if is_display {
                (
                    c.fraction_num_display_style_shift_up * em,
                    c.fraction_den_display_style_shift_down * em,
                    7.0 * c.fraction_rule_thickness * em,
                )
            } else {
                (
                    c.fraction_num_shift_up * em,
                    c.fraction_den_shift_down * em,
                    3.0 * c.fraction_rule_thickness * em,
                )
            };

            // Clearance between numerator bottom and denominator top:
            // actual_gap = (shift_down - den.asc) - (-shift_up + num.desc)
            //            = shift_up - num.desc + shift_down - den.asc
            let actual_gap = shift_up - num.desc + shift_down - den.asc;
            if actual_gap < min_gap {
                let diff = (min_gap - actual_gap) / 2.0;
                shift_up += diff;
                shift_down += diff;
            }

            (-shift_up, shift_down)
        } as (f32, f32);

        let mut b = LBox::default();
        b.place(&num, (w - num.w) / 2.0, ny);
        b.place(&den, (w - den.w) / 2.0, dy);
        if thick > 0.0 {
            b.items
                .push(Item::Poly(rect(0.0, -axis - thick / 2.0, w, thick)));
        }
        b.w = w;
        Ok(b)
    }
    fn bevelled_fraction(
        &mut self,
        n: &MathNode,
        em: f32,
        ctx: MathContext,
        small: bool,
    ) -> Result<LBox> {
        let part_ctx = if small {
            ctx.for_superscript()
        } else {
            ctx.for_numerator()
        };
        let part_em = em * ctx.rel_scale(part_ctx, self.f.constants());
        let num = n
            .children
            .first()
            .map(|c| self.node(c, part_em, part_ctx))
            .transpose()?
            .unwrap_or_default();
        let den = n
            .children
            .get(1)
            .map(|c| self.node(c, part_em, part_ctx))
            .transpose()?
            .unwrap_or_default();
        let gap = 0.12 * em;
        let slash_w = 0.4 * em;
        let up = 0.25 * em;
        let down = 0.20 * em;
        let mut b = LBox::default();
        b.place(&num, 0.0, -up);
        let sx = num.w + gap;
        let top = (-num.asc - up).min(-0.7 * em);
        let bottom = (den.desc + down).max(0.2 * em);
        for poly in stroke(&[(sx, bottom), (sx + slash_w, top)], 0.045 * em) {
            b.items.push(Item::Poly(poly));
        }
        b.place(&den, sx + slash_w + gap, down);
        b.asc = b.asc.max(-top);
        b.desc = b.desc.max(bottom);
        Ok(b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fraction_vertical_clearance_and_limit_symmetry() {
        let frac_str = "<math><msubsup><mo>∫</mo><mi>π</mi><mrow><mn>100</mn><mo>+</mo><mfrac><mi>π</mi><mn>2</mn></mfrac></mrow></msubsup><mrow><mfrac><mn>1</mn><mn>2</mn></mfrac><mi>sin</mi><mi>x</mi><mi>d</mi><mi>x</mi></mrow></math>";
        let n = MathNode::parse(frac_str).unwrap();
        let (b, em) = layout_math(&n, 12.0).unwrap();
        assert!(em > 0.0);
        assert!(!b.items.is_empty());

        let poly_bars: Vec<_> = b
            .items
            .iter()
            .filter_map(|it| match it {
                Item::Poly(pts) => Some(pts),
                _ => None,
            })
            .collect();
        assert!(!poly_bars.is_empty(), "Fraction bar must be present");
    }

    #[test]
    fn test_fraction_baseline_standardization_tex_rule_15() {
        let frac_a_b = MathNode::parse("<math><mfrac><mi>a</mi><mi>b</mi></mfrac></math>").unwrap();
        let frac_x_y = MathNode::parse("<math><mfrac><mi>x</mi><mi>y</mi></mfrac></math>").unwrap();

        let mut l = Layout {
            f: fonts(),
            depth: 0,
        };
        let em = 20.0;

        let b1 = l
            .frac(&frac_a_b.children[0], em, MathContext::DISPLAY)
            .unwrap();
        let b2 = l
            .frac(&frac_x_y.children[0], em, MathContext::DISPLAY)
            .unwrap();

        // Numerator baseline (y position of the glyph)
        let get_glyph_y = |b: &LBox| -> f32 {
            b.items
                .iter()
                .find_map(|it| match it {
                    Item::Glyph { y, .. } => Some(*y),
                    _ => None,
                })
                .unwrap()
        };

        let y1 = get_glyph_y(&b1);
        let y2 = get_glyph_y(&b2);

        // Under TeX Rule 15, both 'a' and 'x' (whose descenders fit comfortably within shift_up)
        // are placed on the exact same standardized baseline shift up!
        assert!(
            (y1 - y2).abs() < 1e-4,
            "y1 ({}) and y2 ({}) must be equal",
            y1,
            y2
        );

        // A numerator with a deep descender (g) pushes shift_up just enough to clear min_gap
        let frac_g_y = MathNode::parse("<math><mfrac><mi>g</mi><mi>y</mi></mfrac></math>").unwrap();
        let b_g = l
            .frac(&frac_g_y.children[0], em, MathContext::DISPLAY)
            .unwrap();
        let y_g = get_glyph_y(&b_g);
        assert!(
            y_g < y1,
            "g must be shifted up further to clear the fraction rule (y_g={}, y1={})",
            y_g,
            y1
        );

        // Verify binomial coefficient (linethickness = 0)
        let binom = MathNode::parse(
            "<math><mfrac linethickness=\"0\"><mn>10</mn><mn>3</mn></mfrac></math>",
        )
        .unwrap();
        let b_binom = l
            .frac(&binom.children[0], em, MathContext::DISPLAY)
            .unwrap();
        // Binomial should have no horizontal rule polygon
        assert!(!b_binom.items.iter().any(|it| matches!(it, Item::Poly(_))));
    }

    #[test]
    fn test_fraction_parts_full_size_in_text_and_table() {
        let mut l = Layout {
            f: fonts(),
            depth: 0,
        };
        let em = 24.0;
        let frac = MathNode::parse("<math><mfrac><mn>1</mn><mn>2</mn></mfrac></math>").unwrap();

        // In DISPLAY context:
        let b_disp = l.frac(&frac.children[0], em, MathContext::DISPLAY).unwrap();
        // In TEXT context (e.g. inside mtable or inline):
        let b_text = l.frac(&frac.children[0], em, MathContext::TEXT).unwrap();

        let get_em = |b: &LBox| -> f32 {
            b.items
                .iter()
                .find_map(|it| match it {
                    Item::Glyph { ch: '1', em, .. } => Some(*em),
                    _ => None,
                })
                .unwrap()
        };

        let em_disp = get_em(&b_disp);
        let em_text = get_em(&b_text);

        // In MathType / OMML, fraction numerator & denominator stay at full size in both display and text/table!
        assert_eq!(em_disp, em);
        assert_eq!(em_text, em);
    }

    #[test]
    fn test_small_fraction_displaystyle_false_shrinks_parts() {
        let mut l = Layout {
            f: fonts(),
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

        let get_em = |b: &LBox| -> f32 {
            b.items
                .iter()
                .find_map(|it| match it {
                    Item::Glyph { ch: '1', em, .. } => Some(*em),
                    _ => None,
                })
                .unwrap()
        };

        let em_small = get_em(
            &l.frac(&small_stacked.children[0], em, MathContext::DISPLAY)
                .unwrap(),
        );
        let em_small_bevel = get_em(
            &l.frac(&small_bevel.children[0], em, MathContext::DISPLAY)
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
