//! Explicit MathML enclosures and cancellation marks.
use super::*;

impl Layout {
    pub(super) fn enclosure(&mut self, n: &MathNode, em: f32, ctx: MathContext) -> Result<LBox> {
        let body = self.row(&n.children, em, ctx)?;
        let notation = n.attr("notation").unwrap_or("box");
        let has = |part: &str| notation.split_whitespace().any(|s| s == part);
        let boxed = has("box") || has("roundedbox");
        let pad_left = if has("longdiv") {
            0.22 * em
        } else if boxed || has("actuarial") || has("left") || has("right") {
            0.15 * em
        } else {
            0.07 * em
        };
        let pad_right =
            if boxed || has("longdiv") || has("actuarial") || has("left") || has("right") {
                0.15 * em
            } else {
                0.07 * em
            };
        let mut b = LBox::default();
        b.place(&body, pad_left, 0.0);
        let w = body.w + pad_left + pad_right;
        let top = -body.asc - 0.12 * em;
        let bottom = body.desc + 0.12 * em;
        let st = 0.045 * em;
        let mut paths = Vec::new();
        if has("roundedbox") {
            let r = (0.12 * em).min(w / 3.0).min((bottom - top) / 3.0);
            let mut path = Vec::new();
            for (cx, cy, start) in [
                (r, top + r, std::f32::consts::PI),
                (w - r, top + r, 1.5 * std::f32::consts::PI),
                (w - r, bottom - r, 0.0),
                (r, bottom - r, 0.5 * std::f32::consts::PI),
            ] {
                for i in 0..=6 {
                    let a = start + i as f32 / 6.0 * std::f32::consts::FRAC_PI_2;
                    path.push((cx + r * a.cos(), cy + r * a.sin()));
                }
            }
            path.push(path[0]);
            paths.push(path);
        } else {
            if boxed || has("top") || has("actuarial") || has("longdiv") {
                paths.push(vec![(0.0, top), (w, top)]);
            }
            if boxed || has("bottom") {
                paths.push(vec![(0.0, bottom), (w, bottom)]);
            }
            if boxed || has("left") {
                paths.push(vec![(0.0, top), (0.0, bottom)]);
            }
            if boxed || has("right") || has("actuarial") {
                paths.push(vec![(w, top), (w, bottom)]);
            }
            if has("longdiv") {
                paths.push(quad_bezier(
                    (0.0, top),
                    (0.20 * em, (top + bottom) / 2.0),
                    (0.0, bottom),
                    16,
                ));
            }
        }
        if has("horizontalstrike") {
            paths.push(vec![(0.0, -AXIS * em), (w, -AXIS * em)]);
        }
        if has("verticalstrike") {
            paths.push(vec![(w / 2.0, top), (w / 2.0, bottom)]);
        }
        if has("updiagonalstrike") {
            paths.push(vec![(0.0, bottom), (w, top)]);
        }
        if has("downdiagonalstrike") {
            paths.push(vec![(0.0, top), (w, bottom)]);
        }
        for path in paths {
            b.items
                .extend(stroke(&path, st).into_iter().map(Item::Poly));
        }
        b.w = w;
        b.asc = -top + st / 2.0;
        b.desc = bottom + st / 2.0;
        Ok(b)
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
            let b = layout_math(&MathNode::with_children("math", vec![node]), 12.0)
                .unwrap()
                .0;
            assert!(
                b.items
                    .iter()
                    .filter(|i| matches!(i, Item::Poly(_)))
                    .count()
                    >= minimum,
                "{notation}"
            );
            assert!(b.w > plain.w && b.asc > plain.asc);
            assert!(b.hits.len() > plain.hits.len());
        }
    }
    #[test]
    fn dots_below_and_above_have_distinct_visible_geometry() {
        for (ch, count) in [('˙', 1), ('¨', 2), ('⃛', 3), ('⃜', 4)] {
            let xml =
                format!("<math><mover accent=\"true\"><mi>x</mi><mo>{ch}</mo></mover></math>");
            let b = layout_math(&MathNode::parse(&xml).unwrap(), 12.0)
                .unwrap()
                .0;
            assert_eq!(
                b.items
                    .iter()
                    .filter(|i| matches!(i, Item::Poly(_)))
                    .count(),
                count
            );
            let xml = format!(
                "<math><munder accentunder=\"true\"><mi>x</mi><mo>{ch}</mo></munder></math>"
            );
            let b = layout_math(&MathNode::parse(&xml).unwrap(), 12.0)
                .unwrap()
                .0;
            let dots: Vec<_> = b
                .items
                .iter()
                .filter_map(|i| if let Item::Poly(p) = i { Some(p) } else { None })
                .collect();
            assert_eq!(dots.len(), count);
            assert!(dots.iter().flat_map(|p| p.iter()).all(|(_, y)| *y > 0.0));
        }
    }
    #[test]
    fn diagonal_fraction_and_explicit_spacing_change_geometry() {
        let stacked = MathNode::parse("<math><mfrac><mn>1</mn><mn>2</mn></mfrac></math>").unwrap();
        let bevelled =
            MathNode::parse("<math><mfrac bevelled=\"true\"><mn>1</mn><mn>2</mn></mfrac></math>")
                .unwrap();
        let a = layout_math(&stacked, 12.0).unwrap().0;
        let b = layout_math(&bevelled, 12.0).unwrap().0;
        assert!(b.w > a.w);
        assert!(b.items.iter().any(|i| match i {
            Item::Poly(p) =>
                p.len() == 4 && (p[0].0 - p[1].0).abs() > 1.0 && (p[0].1 - p[1].1).abs() > 1.0,
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
