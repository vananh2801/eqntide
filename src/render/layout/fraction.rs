//! Fraction templates: TeX Rule 15 and OpenType MATH table standard layout.

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

        let c = self.f.constants();
        let axis = c.axis_height * em;
        let is_display = !script;

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
            let (mut shift_up, mut shift_down, num_gap_min, den_gap_min) = if is_display {
                (
                    c.fraction_num_display_style_shift_up * em,
                    c.fraction_den_display_style_shift_down * em,
                    c.fraction_num_display_style_gap_min * em,
                    c.fraction_den_display_style_gap_min * em,
                )
            } else {
                (
                    c.fraction_num_shift_up * em,
                    c.fraction_den_shift_down * em,
                    c.fraction_num_gap_min * em,
                    c.fraction_den_gap_min * em,
                )
            };

            // Numerator clearance: distance from numerator bottom (-shift_up + num.desc)
            // to the top edge of the fraction rule (-axis - thick / 2.0).
            // clearance = (-axis - thick / 2.0) - (-shift_up + num.desc)
            //           = shift_up - num.desc - axis - thick / 2.0
            let num_clearance = shift_up - num.desc - axis - thick / 2.0;
            if num_clearance < num_gap_min {
                shift_up += num_gap_min - num_clearance;
            }

            // Denominator clearance: distance from bottom edge of the fraction rule
            // (-axis + thick / 2.0) to denominator top (shift_down - den.asc).
            // clearance = (shift_down - den.asc) - (-axis + thick / 2.0)
            //           = shift_down - den.asc + axis - thick / 2.0
            let den_clearance = shift_down - den.asc + axis - thick / 2.0;
            if den_clearance < den_gap_min {
                shift_down += den_gap_min - den_clearance;
            }

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
        };

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fraction_baseline_standardization_tex_rule_15() {
        let frac_a_b = MathNode::parse("<math><mfrac><mi>a</mi><mi>b</mi></mfrac></math>").unwrap();
        let frac_x_y = MathNode::parse("<math><mfrac><mi>x</mi><mi>y</mi></mfrac></math>").unwrap();

        let mut l = Layout { f: fonts(), depth: 0 };
        let em = 20.0;

        let b1 = l.frac(&frac_a_b.children[0], em, false).unwrap();
        let b2 = l.frac(&frac_x_y.children[0], em, false).unwrap();

        // Numerator baseline (y position of the glyph)
        let get_glyph_y = |b: &LBox| -> f32 {
            b.items.iter().find_map(|it| match it {
                Item::Glyph { y, .. } => Some(*y),
                _ => None,
            }).unwrap()
        };

        let y1 = get_glyph_y(&b1);
        let y2 = get_glyph_y(&b2);

        // Under TeX Rule 15, both 'a' and 'x' (whose descenders fit comfortably within shift_up)
        // are placed on the exact same standardized baseline shift up!

        // Numerators without deep descenders (a and x) share the exact same baseline
        assert!((y1 - y2).abs() < 1e-4, "y1 ({}) and y2 ({}) must be equal", y1, y2);

        // A numerator with a deep descender (g) pushes shift_up just enough to clear min_gap
        let frac_g_y = MathNode::parse("<math><mfrac><mi>g</mi><mi>y</mi></mfrac></math>").unwrap();
        let b_g = l.frac(&frac_g_y.children[0], em, false).unwrap();
        let y_g = get_glyph_y(&b_g);
        assert!(y_g < y1, "g must be shifted up further to clear the fraction rule (y_g={}, y1={})", y_g, y1);

        // Verify binomial coefficient (linethickness = 0)
        let binom = MathNode::parse("<math><mfrac linethickness=\"0\"><mn>10</mn><mn>3</mn></mfrac></math>").unwrap();
        let b_binom = l.frac(&binom.children[0], em, false).unwrap();
        // Binomial should have no horizontal rule polygon
        assert!(!b_binom.items.iter().any(|it| matches!(it, Item::Poly(_))));
    }
}
