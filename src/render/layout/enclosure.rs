//! Explicit MathML `<menclose>` borders and cancellation strikes.

use super::*;

impl Layout {
    /// Lay out an `<menclose>` box with notation borders, arcs, and cancellation strokes.
    pub(super) fn enclosure(&mut self, node: &MathNode, em: f32, ctx: MathContext) -> Result<LBox> {
        let body = self.row(&node.children, em, ctx)?;
        let notation = node.attr("notation").unwrap_or("box");
        let has_notation = |part: &str| notation.split_whitespace().any(|token| token == part);
        let boxed = has_notation("box") || has_notation("roundedbox");
        let pad_left = if has_notation("longdiv") {
            0.22 * em
        } else if boxed
            || has_notation("actuarial")
            || has_notation("left")
            || has_notation("right")
        {
            0.15 * em
        } else {
            0.07 * em
        };
        let pad_right = if boxed
            || has_notation("longdiv")
            || has_notation("actuarial")
            || has_notation("left")
            || has_notation("right")
        {
            0.15 * em
        } else {
            0.07 * em
        };
        let mut layout_box = LBox::default();
        layout_box.place(&body, pad_left, 0.0);
        let width = body.w + pad_left + pad_right;
        let top = -body.asc - 0.12 * em;
        let bottom = body.desc + 0.12 * em;
        let stroke_w = 0.045 * em;
        let mut paths = Vec::new();
        if has_notation("roundedbox") {
            let radius = (0.12 * em).min(width / 3.0).min((bottom - top) / 3.0);
            let mut path = Vec::new();
            for (cx, cy, start_angle) in [
                (radius, top + radius, std::f32::consts::PI),
                (width - radius, top + radius, 1.5 * std::f32::consts::PI),
                (width - radius, bottom - radius, 0.0),
                (radius, bottom - radius, 0.5 * std::f32::consts::PI),
            ] {
                for i in 0..=6 {
                    let angle = start_angle + i as f32 / 6.0 * std::f32::consts::FRAC_PI_2;
                    path.push((cx + radius * angle.cos(), cy + radius * angle.sin()));
                }
            }
            path.push(path[0]);
            paths.push(path);
        } else {
            if boxed || has_notation("top") || has_notation("actuarial") || has_notation("longdiv")
            {
                paths.push(vec![(0.0, top), (width, top)]);
            }
            if boxed || has_notation("bottom") {
                paths.push(vec![(0.0, bottom), (width, bottom)]);
            }
            if boxed || has_notation("left") {
                paths.push(vec![(0.0, top), (0.0, bottom)]);
            }
            if boxed || has_notation("right") || has_notation("actuarial") {
                paths.push(vec![(width, top), (width, bottom)]);
            }
            if has_notation("longdiv") {
                paths.push(quad_bezier(
                    (0.0, top),
                    (0.20 * em, (top + bottom) / 2.0),
                    (0.0, bottom),
                    16,
                ));
            }
        }
        if has_notation("horizontalstrike") {
            paths.push(vec![(0.0, -AXIS * em), (width, -AXIS * em)]);
        }
        if has_notation("verticalstrike") {
            paths.push(vec![(width / 2.0, top), (width / 2.0, bottom)]);
        }
        if has_notation("updiagonalstrike") {
            paths.push(vec![(0.0, bottom), (width, top)]);
        }
        if has_notation("downdiagonalstrike") {
            paths.push(vec![(0.0, top), (width, bottom)]);
        }
        for path in paths {
            layout_box
                .items
                .extend(stroke(&path, stroke_w).into_iter().map(Item::Poly));
        }
        layout_box.w = width;
        layout_box.asc = -top + stroke_w / 2.0;
        layout_box.desc = bottom + stroke_w / 2.0;
        Ok(layout_box)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enclosure_rules_and_strikes_are_visible_and_keep_body_hits() {
        let base = MathNode::leaf("mi", "x");
        let plain = layout_math(&MathNode::with_children("math", vec![base.clone()]), 12.0)
            .unwrap()
            .0;
        for (notation, minimum) in [
            ("box", 4),
            ("top left", 2),
            ("horizontalstrike", 1),
            ("updiagonalstrike downdiagonalstrike", 2),
            ("roundedbox", 20),
        ] {
            let mut node = MathNode::with_children("menclose", vec![base.clone()]);
            node.set_attr("notation", notation);
            let layout_box = layout_math(&MathNode::with_children("math", vec![node]), 12.0)
                .unwrap()
                .0;
            assert!(
                layout_box
                    .items
                    .iter()
                    .filter(|item| matches!(item, Item::Poly(_)))
                    .count()
                    >= minimum,
                "{notation}"
            );
            assert!(layout_box.w > plain.w && layout_box.asc > plain.asc);
            assert!(layout_box.hits.len() > plain.hits.len());
        }
    }

    #[test]
    fn dots_below_and_above_have_distinct_visible_geometry() {
        for (ch, count) in [('˙', 1), ('¨', 2), ('⃛', 3), ('⃜', 4)] {
            let xml =
                format!("<math><mover accent=\"true\"><mi>x</mi><mo>{ch}</mo></mover></math>");
            let over_box = layout_math(&MathNode::parse(&xml).unwrap(), 12.0)
                .unwrap()
                .0;
            assert_eq!(
                over_box
                    .items
                    .iter()
                    .filter(|item| matches!(item, Item::Poly(_)))
                    .count(),
                count
            );
            let xml = format!(
                "<math><munder accentunder=\"true\"><mi>x</mi><mo>{ch}</mo></munder></math>"
            );
            let under_box = layout_math(&MathNode::parse(&xml).unwrap(), 12.0)
                .unwrap()
                .0;
            let dots: Vec<_> = under_box
                .items
                .iter()
                .filter_map(|item| {
                    if let Item::Poly(points) = item {
                        Some(points)
                    } else {
                        None
                    }
                })
                .collect();
            assert_eq!(dots.len(), count);
            assert!(dots
                .iter()
                .flat_map(|poly| poly.iter())
                .all(|(_, y)| *y > 0.0));
        }
    }

    #[test]
    fn diagonal_fraction_and_explicit_spacing_change_geometry() {
        let stacked = MathNode::parse("<math><mfrac><mn>1</mn><mn>2</mn></mfrac></math>").unwrap();
        let bevelled =
            MathNode::parse("<math><mfrac bevelled=\"true\"><mn>1</mn><mn>2</mn></mfrac></math>")
                .unwrap();
        let stacked_box = layout_math(&stacked, 12.0).unwrap().0;
        let bevelled_box = layout_math(&bevelled, 12.0).unwrap().0;
        assert!(bevelled_box.w > stacked_box.w);
        assert!(bevelled_box.items.iter().any(|item| match item {
            Item::Poly(poly) => {
                poly.len() == 4
                    && (poly[0].0 - poly[1].0).abs() > 1.0
                    && (poly[0].1 - poly[1].1).abs() > 1.0
            }
            _ => false,
        }));
        let plain = layout_math(
            &MathNode::parse("<math><mi>x</mi><mi>y</mi></math>").unwrap(),
            12.0,
        )
        .unwrap()
        .0;
        let spaced = layout_math(
            &MathNode::parse("<math><mi>x</mi><mspace width=\"1em\"/><mi>y</mi></math>").unwrap(),
            12.0,
        )
        .unwrap()
        .0;
        assert!((spaced.w - plain.w - 96.0).abs() < 0.01);
    }

    #[test]
    fn extended_arrow_and_accent_glyphs_are_present_in_embedded_font() {
        for ch in "⇒⇐⇔⇀↼⇌⇋↦⇄⇆⎴⎵⏜⏝".chars() {
            assert!(fonts().has_glyph(ch), "missing glyph: {ch}");
        }
    }
}
