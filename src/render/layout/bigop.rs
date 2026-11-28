//! Summation, product, and set-theory big operators, plus the shared
//! `munder` / `mover` / `munderover` handler that also routes integrals,
//! labeled arrows, accents, and under-braces to their own modules.

use super::*;

impl Layout {
    /// Lay out a big operator glyph centered on the math axis.
    pub(super) fn big_glyph(&self, ch: char, face: Face, em: f32, scale: f32) -> LBox {
        let size = em * scale;
        let (asc, desc) = self.font_set.ink(face, ch, size);
        let axis = self.font_set.constants().axis_height * em;
        let y = (asc - desc) / 2.0 - axis;
        let mut layout_box = LBox {
            w: self.font_set.advance(face, ch, size),
            ..Default::default()
        };
        layout_box.items.push(Item::Glyph {
            ch,
            face,
            em: size,
            x: 0.0,
            y,
            ssty: 0,
        });
        layout_box.asc = asc - y;
        layout_box.desc = desc + y;
        layout_box
    }

    /// Lay out `munder`, `mover`, or `munderover` elements and specialized limit/accent forms.
    pub(super) fn under_over(
        &mut self,
        node: &MathNode,
        em: f32,
        ctx: MathContext,
    ) -> Result<LBox> {
        let first = node.children.first();
        let base_char = first
            .filter(|base| base.tag == "mo")
            .and_then(|base| base.text.trim().chars().next());
        let constants = self.font_set.constants();
        let lim_ctx = ctx.for_limits();
        let lim_em = em * lim_ctx.scale(constants);
        let (under_node, over_node) = match node.tag.as_str() {
            "munder" => (node.children.get(1), None),
            "mover" => (None, node.children.get(1)),
            _ => (node.children.get(1), node.children.get(2)),
        };

        // Integral sign: limits sit above/below the sign (see `integral.rs`).
        if base_char.is_some_and(is_integral) {
            if let Some(first_node) = first {
                if node.attr("data-limloc") == Some("subSup") {
                    return self.scripts(node, em, ctx);
                }
                return self.integral_with_limits(first_node, under_node, over_node, em, ctx);
            }
        }

        // Labeled arrow (MathType `tmARROW`): stretchy arrow with text above/below (see `arrow.rs`).
        if let Some(arrow_tmpl) = arrow::template_of(node) {
            return self.labeled_arrow(arrow_tmpl.dir, arrow_tmpl.top, arrow_tmpl.bottom, em, ctx);
        }

        let base = match first {
            Some(base_node) => self.node(base_node, em, ctx)?,
            None => LBox::default(),
        };
        let is_math_big_op = base_char.is_some_and(is_big_op);
        let is_big = is_math_big_op
            || first.is_some_and(|base_node| {
                base_node.tag == "mi" && is_function(base_node.text.trim())
            });

        // Accent over a normal base.
        let accent = over_node
            .filter(|over| over.tag == "mo" && !is_big)
            .and_then(|over| over.text.trim().chars().next())
            .filter(|ch| is_accent(*ch));
        let under_accent = under_node
            .filter(|under| {
                under.tag == "mo" && !is_big && node.attr("accentunder") == Some("true")
            })
            .and_then(|under| under.text.trim().chars().next())
            .filter(|ch| is_accent(*ch));
        let is_brace_under = under_node
            .filter(|under| under.tag == "mo" && under.text.trim() == "\u{23DF}")
            .is_some();
        let is_bar_under = under_node
            .filter(|under| under.tag == "mo" && !is_big)
            .and_then(|under| under.text.trim().chars().next())
            .is_some_and(|ch| {
                ch == '_'
                    || ch == '\u{0332}'
                    || ch == '\u{005F}'
                    || ch == '\u{00AF}'
                    || ch == '\u{203E}'
            });

        let under = match under_node {
            Some(under_child) if !is_brace_under && !is_bar_under && under_accent.is_none() => {
                Some(self.node(under_child, lim_em, lim_ctx)?)
            }
            _ => None,
        };
        let over = match over_node {
            Some(over_child) if accent.is_none() => Some(self.node(over_child, lim_em, lim_ctx)?),
            _ => None,
        };

        let (upper_gap, lower_gap) = if is_math_big_op {
            (
                constants.upper_limit_gap_min * em,
                constants.lower_limit_gap_min * em,
            )
        } else if is_big {
            (0.10 * em, 0.12 * em)
        } else {
            (0.12 * em, 0.12 * em)
        };
        let width = [
            Some(base.w),
            under.as_ref().map(|under_box| under_box.w),
            over.as_ref().map(|over_box| over_box.w),
        ]
        .into_iter()
        .flatten()
        .fold(0.0f32, f32::max);
        let mut layout_box = LBox::default();
        layout_box.place(&base, (width - base.w) / 2.0, 0.0);
        if let Some(over_box) = &over {
            layout_box.place(
                over_box,
                (width - over_box.w) / 2.0,
                -(base.asc + upper_gap + over_box.desc),
            );
        }
        if let Some(under_box) = &under {
            layout_box.place(
                under_box,
                (width - under_box.w) / 2.0,
                base.desc + lower_gap + under_box.asc,
            );
        }
        if let Some(ch) = accent {
            self.draw_accent(
                &mut layout_box,
                ch,
                width,
                (width - base.w) / 2.0,
                base.asc,
                em,
            );
        }
        if let Some(ch) = under_accent {
            self.draw_under_accent(
                &mut layout_box,
                ch,
                width,
                (width - base.w) / 2.0,
                base.desc,
                em,
            );
        } else if is_brace_under {
            self.draw_under_brace(&mut layout_box, width, base.desc, em);
        }
        if is_bar_under && under_accent.is_none() {
            let stroke_w = 0.045 * em;
            let y = (base.desc + 0.06 * em).max(0.10 * em);
            let x0 = (width - base.w) / 2.0;
            layout_box
                .items
                .push(Item::Poly(rect(x0, y, base.w, stroke_w)));
            layout_box.desc = layout_box.desc.max(y + stroke_w + 0.04 * em);
        }
        layout_box.w = width.max(layout_box.w);
        Ok(layout_box)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::parse_latex_to_mathml;

    #[test]
    fn bigop_limits_respect_math_limit_gap_constants() {
        let math = parse_latex_to_mathml(r"\sum_{i=1}^{n} x_i").unwrap();
        let (lbox, em) = layout_math(&math, 14.0).unwrap();
        let constants = fonts().constants();
        let upper_gap = constants.upper_limit_gap_min * em;
        let lower_gap = constants.lower_limit_gap_min * em;
        let sum_glyph = lbox
            .items
            .iter()
            .find_map(|item| match item {
                Item::Glyph {
                    ch: '∑',
                    face,
                    em: glyph_em,
                    y,
                    ..
                } => {
                    let (asc, desc) = fonts().ink(*face, '∑', *glyph_em);
                    Some((asc - y, desc + y))
                }
                _ => None,
            })
            .expect("missing ∑ glyph");
        let n_glyph_bottom = lbox
            .items
            .iter()
            .find_map(|item| match item {
                Item::Glyph {
                    ch: 'n',
                    face,
                    em: glyph_em,
                    y,
                    ..
                } => {
                    let (_, desc) = fonts().ink(*face, 'n', *glyph_em);
                    Some(y + desc)
                }
                _ => None,
            })
            .expect("missing n limit glyph");
        let eq_top = lbox
            .items
            .iter()
            .find_map(|item| match item {
                Item::Glyph {
                    ch: '=',
                    face,
                    em: glyph_em,
                    y,
                    ..
                } => {
                    let (asc, _) = fonts().ink(*face, '=', *glyph_em);
                    Some(y - asc)
                }
                _ => None,
            })
            .expect("missing '=' lower limit glyph");

        let actual_upper_gap = (-sum_glyph.0) - n_glyph_bottom;
        let actual_lower_gap = eq_top - sum_glyph.1;
        assert!(
            actual_upper_gap + 1e-3 >= upper_gap,
            "upper limit gap {actual_upper_gap} must satisfy upper_limit_gap_min {upper_gap}"
        );
        assert!(
            actual_lower_gap + 1e-3 >= lower_gap,
            "lower limit gap {actual_lower_gap} must satisfy lower_limit_gap_min {lower_gap}"
        );
    }
}
