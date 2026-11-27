//! Integral templates: the sign is assembled from glyph pieces and its limits
//! can sit above/below it; explicit side limits use the script layout.

use super::*;

const STEM_TOP: char = '⌠';
const STEM_BOTTOM: char = '⌡';
const STIX_UPEM: f32 = 1000.0;
// Centers of the STIX stem connector and closed circle arrows in font units.
const STEM_CENTER_X: f32 = 347.5;
const RING_CENTER_X: f32 = 474.0;
const RING_CENTER_Y: f32 = 261.0;
const RING_DIAMETER: f32 = 760.0;

impl Layout {
    /// Assemble an integral sign from vertical glyph pieces and an optional loop.
    pub(super) fn integral_without_limits(&self, ch: char, target_h: f32, em: f32) -> Option<LBox> {
        if !is_integral(ch) {
            return None;
        }
        let num_stems = match ch {
            '\u{222C}' | '\u{222F}' => 2,
            '\u{222D}' | '\u{2230}' => 3,
            _ => 1,
        };
        let eff_em = em * (target_h / (2.0 * em)).clamp(0.60, 1.0);
        let y_mid = -AXIS * em;
        let stem = self.integral_stem(target_h, eff_em, y_mid)?;
        let stem_spacing = 0.45 * eff_em;
        let stem_span = (num_stems - 1) as f32 * stem_spacing;
        let mut layout_box = LBox::default();
        for i in 0..num_stems {
            layout_box.place(&stem, i as f32 * stem_spacing, 0.0);
        }
        let loop_center_x = stem_span / 2.0 + STEM_CENTER_X * eff_em / STIX_UPEM;
        match ch {
            '∮' | '∯' | '∰' => {
                append_plain_loop(&mut layout_box, num_stems, eff_em, loop_center_x, y_mid);
            }
            '∲' | '∳' => {
                let ring = if ch == '∲' { '⥁' } else { '⥀' };
                let contours = self.directional_ring(ring, eff_em, loop_center_x, y_mid)?;
                layout_box.items.push(Item::Contours(contours));
            }
            _ => {}
        }

        let (min_x, max_x, min_y, max_y) = layout_box.ink_bounds(self.font_set);
        layout_box.w = max_x;
        layout_box.asc = -min_y;
        layout_box.desc = max_y;
        if min_x < 0.0 {
            let mut shifted = LBox::default();
            shifted.place(&layout_box, -min_x, 0.0);
            return Some(shifted);
        }
        Some(layout_box)
    }

    /// Lay out an integral sign with its lower/upper limits placed above and below it.
    pub(super) fn integral_with_limits(
        &mut self,
        base_node: &MathNode,
        under_node: Option<&MathNode>,
        over_node: Option<&MathNode>,
        em: f32,
        ctx: MathContext,
    ) -> Result<LBox> {
        let constants = self.font_set.constants();
        let sub_ctx = ctx.for_subscript();
        let sup_ctx = ctx.for_superscript();
        let sub_em = em * sub_ctx.scale(constants);
        let sup_em = em * sup_ctx.scale(constants);

        // Reuse the integral assembly through the normal node path, retaining
        // the operator's hit region for caret/selection in the native editor.
        let base = self.node(base_node, em, ctx)?;
        let sub = match under_node {
            Some(node) => Some(self.node(node, sub_em, sub_ctx)?),
            None => None,
        };
        let sup = match over_node {
            Some(node) => Some(self.node(node, sup_em, sup_ctx)?),
            None => None,
        };

        let mut layout_box = LBox::default();
        let w_base = base.w;
        let w_sub = sub.as_ref().map_or(0.0, |sub_box| sub_box.w);
        let w_sup = sup.as_ref().map_or(0.0, |sup_box| sup_box.w);
        let w_center = w_base.max(w_sub).max(w_sup);

        let offset_x = 0.12 * em;
        layout_box.place(&base, offset_x + (w_center - w_base) / 2.0, 0.0);

        let gap = 0.08 * em;

        if let Some(sup_box) = &sup {
            let sup_x = offset_x + (w_center - w_sup) / 2.0 + 0.14 * em;
            layout_box.place(sup_box, sup_x, -(base.asc + gap + sup_box.desc));
        }

        if let Some(sub_box) = &sub {
            let sub_x = offset_x + (w_center - w_sub) / 2.0 - 0.14 * em;
            layout_box.place(sub_box, sub_x, base.desc + gap + sub_box.asc);
        }

        layout_box.w += 0.12 * em;
        Ok(layout_box)
    }
}

/// Placement metrics for the repeated OpenType MATH stem connector.
struct StemExtension {
    glyph_id: u16,
    count: usize,
    advance: f32,
    overlap: f32,
}

impl Layout {
    fn integral_stem(&self, target_h: f32, em: f32, axis_y: f32) -> Option<LBox> {
        let face = Face::Symbol;
        let (top_asc, _) = self.font_set.ink(face, STEM_TOP, em);
        let (bottom_asc, _) = self.font_set.ink(face, STEM_BOTTOM, em);
        let body_h = top_asc + bottom_asc;
        let cap_overlap = 0.03 * em;
        let extension = if target_h > body_h - cap_overlap {
            Some(self.integral_extension(target_h, body_h, em)?)
        } else {
            None
        };
        let overlap = extension.as_ref().map_or(cap_overlap, |ext| ext.overlap);
        let extender_span = extension
            .as_ref()
            .map_or(0.0, |ext| ext.count as f32 * (ext.advance - overlap));
        let height = body_h + extender_span - overlap;
        let top_y = axis_y - height / 2.0;
        let bottom_y = top_y + height;
        let mut stem = LBox {
            w: self.font_set.advance(face, STEM_TOP, em),
            asc: -top_y,
            desc: bottom_y,
            ..Default::default()
        };

        let top_baseline = top_y + top_asc;
        stem.items.push(Item::Glyph {
            ch: STEM_TOP,
            face,
            em,
            x: 0.0,
            y: top_baseline,
            ssty: 0,
        });
        if let Some(ext) = extension {
            for i in 1..=ext.count {
                let y = top_baseline + i as f32 * (ext.advance - overlap);
                let contours = self.font_set.math_glyph_polys(ext.glyph_id, em, em, 0.0, y);
                stem.items.push(Item::Contours(contours));
            }
        }
        stem.items.push(Item::Glyph {
            ch: STEM_BOTTOM,
            face,
            em,
            x: 0.0,
            y: bottom_y,
            ssty: 0,
        });
        Some(stem)
    }

    fn integral_extension(&self, target_h: f32, body_h: f32, em: f32) -> Option<StemExtension> {
        let assembly = self.font_set.integral_assembly()?;
        // OpenType lists the parts from bottom to top.
        let [bottom, extender, top] = assembly.parts.as_slice() else {
            return None;
        };
        let unit = em / STIX_UPEM;
        let min_overlap = f32::from(assembly.min_connector_overlap) * unit;
        let max_overlap = f32::from(
            bottom
                .end_connector
                .min(extender.start_connector)
                .min(extender.end_connector)
                .min(top.start_connector),
        ) * unit;
        let advance = f32::from(extender.full_advance) * unit;
        let count = ((target_h - body_h + min_overlap) / (advance - min_overlap))
            .ceil()
            .max(1.0) as usize;
        let overlap = ((body_h + count as f32 * advance - target_h) / (count + 1) as f32)
            .clamp(min_overlap, max_overlap);
        Some(StemExtension {
            glyph_id: extender.glyph_id,
            count,
            advance,
            overlap,
        })
    }

    fn directional_ring(
        &self,
        ring: char,
        em: f32,
        center_x: f32,
        axis_y: f32,
    ) -> Option<Vec<Vec<(f32, f32)>>> {
        let glyph_id = self.font_set.glyph_id_for_math(Face::Symbol, ring)?;
        let unit = 0.60 * em / RING_DIAMETER;
        let ring_em = unit * STIX_UPEM;
        let mut contours = self.font_set.math_glyph_polys(
            glyph_id,
            ring_em,
            ring_em,
            -RING_CENTER_X * unit,
            RING_CENTER_Y * unit,
        );
        // Move the arrowhead from the top of the ring to the right of the stem.
        for (x, y) in contours.iter_mut().flatten() {
            (*x, *y) = (center_x - *y, axis_y + *x);
        }
        Some(contours)
    }
}

fn append_plain_loop(
    layout_box: &mut LBox,
    num_stems: usize,
    em: f32,
    center_x: f32,
    center_y: f32,
) {
    let radius_x = 0.3 * em * num_stems as f32;
    let radius_y = 0.3 * em;
    let steps = 24;
    let points: Vec<_> = (0..=steps)
        .map(|i| {
            let angle = i as f32 / steps as f32 * std::f32::consts::TAU;
            (
                center_x + radius_x * angle.cos(),
                center_y + radius_y * angle.sin(),
            )
        })
        .collect();
    layout_box
        .items
        .extend(stroke(&points, 0.045 * em).into_iter().map(Item::Poly));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contour_integral_ink_fits_its_operator_hit_between_neighbors() {
        let mut layout = Layout {
            font_set: fonts(),
            depth: 0,
        };
        let em = 24.0;
        for ch in ['∯', '∰', '∲', '∳'] {
            let operator = ch.to_string();
            let nodes = [
                MathNode::leaf("mi", "x"),
                MathNode::leaf("mo", &operator),
                MathNode::leaf("mi", "y"),
            ];
            let row = layout.row(&nodes, em, MathContext::DISPLAY).unwrap();
            let [left, sign, right] = row.hits.as_slice() else {
                panic!("expected a hit for each token");
            };
            assert!(left.x + left.w <= sign.x);
            assert!(sign.x + sign.w <= right.x);

            let operator_box = layout.integral_without_limits(ch, 1.45 * em, em).unwrap();
            let (min_x, max_x, min_y, max_y) = operator_box.ink_bounds(fonts());
            assert!(min_x >= -0.001 && max_x <= sign.w + 0.001, "{ch}");
            assert!(
                min_y >= -sign.asc - 0.001 && max_y <= sign.desc + 0.001,
                "{ch}"
            );
        }
    }

    #[test]
    fn contour_integral_rings_preserve_direction_and_limit_placement() {
        let em = 100.0;
        for (suffix, limit_kind) in [
            ("", ""),
            (r"\nolimits_{C}", "msub"),
            (r"\limits_{C}", "munder"),
        ] {
            for (command, direction) in [
                (r"\oint", 0.0),
                (r"\varointclockwise", -1.0),
                (r"\ointctrclockwise", 1.0),
            ] {
                let math =
                    crate::service::parse_latex_to_mathml(&format!("{command}{suffix}")).unwrap();
                let mut layout = Layout {
                    font_set: fonts(),
                    depth: 0,
                };
                let sign = layout.node(&math, em, MathContext::DISPLAY).unwrap();
                let ring = sign.items.iter().find_map(|item| match item {
                    Item::Contours(contours) => Some(contours),
                    _ => None,
                });
                if direction == 0.0 {
                    assert!(ring.is_none());
                    assert!(sign.items.iter().any(|item| matches!(item, Item::Poly(_))));
                } else {
                    let ring = ring.unwrap();
                    assert_eq!(ring.len(), 2, "ring must retain its inner hole");
                    let left_edge = |outline: &[(f32, f32)]| {
                        outline
                            .iter()
                            .map(|point| point.0)
                            .fold(f32::INFINITY, f32::min)
                    };
                    let stroke = (left_edge(&ring[1]) - left_edge(&ring[0])).abs();
                    assert!(stroke < 0.06 * 0.725 * em, "directional loop is too heavy");
                    let rightmost = ring[0]
                        .iter()
                        .max_by(|left, right| left.0.total_cmp(&right.0))
                        .unwrap();
                    // The clockwise glyph's outer arrow wing is above the axis.
                    assert_eq!((rightmost.1 + AXIS * em).signum(), direction);
                }
                if !limit_kind.is_empty() {
                    let scripted = &math.children[0];
                    assert_eq!(scripted.tag, limit_kind);
                    let operator = &scripted.children[0];
                    let limit = &scripted.children[1];
                    let hit = |node: &MathNode| {
                        sign.hits
                            .iter()
                            .find(|hit| hit.key == node as *const _ as usize)
                            .unwrap()
                    };
                    let operator = hit(operator);
                    let limit = hit(limit);
                    if limit_kind == "msub" {
                        assert!(limit.x >= operator.x + operator.w);
                    } else {
                        assert!(limit.x < operator.x + operator.w);
                        assert!(limit.y - limit.asc > operator.y + operator.desc);
                    }
                }
            }
        }
    }

    #[test]
    fn extended_integral_stem_has_continuous_connector_width() {
        let layout = Layout {
            font_set: fonts(),
            depth: 0,
        };
        let em = 100.0;
        let stem = layout.integral_without_limits('∫', 5.0 * em, em).unwrap();
        assert!((stem.asc + stem.desc - 5.0 * em).abs() < 0.01);

        let [Item::Glyph { y: top_base, .. }, Item::Contours(first), Item::Contours(second), Item::Glyph { y: bottom_base, .. }] =
            stem.items.as_slice()
        else {
            panic!("expected a stem with two extenders");
        };
        let stem_left = 28.1;
        let stem_right = 41.4;
        for connector in [first, second] {
            let [outline] = connector.as_slice() else {
                panic!("expected a solid extender");
            };
            assert!(outline
                .iter()
                .all(|(x, _)| { (x - stem_left).abs() < 0.01 || (x - stem_right).abs() < 0.01 }));
        }

        let (width, _, alpha) =
            crate::render::rasterize_alpha(&stem, 0.0, 0.0, 0.0, 0.0, stem.asc, stem.desc).unwrap();
        let middle_rows = ((*top_base + stem.asc - 8.0) as usize)
            ..((*bottom_base + stem.asc - 189.5 + 8.0) as usize);
        for row in middle_rows {
            let coverage: u32 = alpha[row * width..(row + 1) * width]
                .iter()
                .map(|value| u32::from(*value))
                .sum();
            assert!((3320..=3470).contains(&coverage), "row {row}: {coverage}");
        }
    }
}
