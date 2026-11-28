//! Embellishments: accents drawn over or under a base (hat, tilde, dot, arrow, bar, …).

use super::*;

impl Layout {
    /// Draw an over-accent above a laid-out base contained within `layout_box`.
    pub(super) fn draw_accent(
        &self,
        layout_box: &mut LBox,
        ch: char,
        w: f32,
        x0: f32,
        base_asc: f32,
        em: f32,
    ) {
        self.draw_accent_oriented(layout_box, ch, w, x0, base_asc, em, false);
    }

    /// Reflect an accent below the baseline, preserving natural glyph proportions.
    pub(super) fn draw_under_accent(
        &self,
        layout_box: &mut LBox,
        ch: char,
        w: f32,
        x0: f32,
        desc: f32,
        em: f32,
    ) {
        let mut accent = LBox::default();
        let ch = match ch {
            '⏟' => '⏞',
            '⎵' => '⎴',
            '⏝' => '⏜',
            _ => ch,
        };
        self.draw_accent_oriented(&mut accent, ch, w, x0, desc, em, true);
        for item in accent.items {
            match item {
                Item::Poly(poly) => layout_box
                    .items
                    .push(Item::Poly(poly.into_iter().map(|(x, y)| (x, -y)).collect())),
                Item::Contours(contours) => layout_box.items.push(Item::Contours(
                    contours
                        .into_iter()
                        .map(|poly| poly.into_iter().map(|(x, y)| (x, -y)).collect())
                        .collect(),
                )),
                Item::Glyph {
                    ch,
                    face,
                    em: glyph_em,
                    x,
                    y,
                    ssty,
                } if matches!(
                    ch,
                    '\u{2192}' | '\u{2190}' | '\u{2194}' | '⇒' | '⇐' | '⇔' | '=' | '\u{23AF}'
                ) =>
                {
                    // These glyphs are vertically symmetric around +259 font units above
                    // baseline, so keeping Item::Glyph avoids raster seam notches.
                    layout_box.items.push(Item::Glyph {
                        ch,
                        face,
                        em: glyph_em,
                        x,
                        y: -y + 518.0 * (glyph_em / 1000.0),
                        ssty,
                    });
                }
                Item::Glyph {
                    ch,
                    face,
                    em: glyph_em,
                    x,
                    y,
                    ..
                } => {
                    let contours: Vec<Vec<(f32, f32)>> = self
                        .font_set
                        .glyph_polys(face, ch, glyph_em, glyph_em, x, y)
                        .into_iter()
                        .map(|poly| poly.into_iter().map(|(px, py)| (px, -py)).collect())
                        .collect();
                    if !contours.is_empty() {
                        layout_box.items.push(Item::Contours(contours));
                    }
                }
            }
        }
        layout_box.desc = layout_box.desc.max(accent.asc);
        layout_box.w = layout_box.w.max(accent.w);
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_accent_oriented(
        &self,
        layout_box: &mut LBox,
        ch: char,
        w: f32,
        x0: f32,
        base_asc: f32,
        em: f32,
        is_under: bool,
    ) {
        let ch = match ch {
            '\u{0308}' => '¨',
            '\u{030A}' | '°' => '˚',
            '\u{030C}' => 'ˇ',
            '\u{0306}' => '˘',
            '\u{0300}' => 'ˋ',
            '\u{0301}' => 'ˊ',
            '\u{20D6}' => '←',
            '\u{20E1}' => '↔',
            '\u{20D0}' => '↼',
            '\u{20D1}' => '⇀',
            _ => ch,
        };
        // `top_accent_attachment_of_box` already includes the base glyph's placed x offset (`x0`).
        let center_x = if let Some(att) = self.top_accent_attachment_of_box(layout_box) {
            att
        } else {
            let ic = self.italic_correction_of_box(layout_box);
            if ic > 0.0 {
                w / 2.0 + 0.35 * ic
            } else {
                w / 2.0
            }
        };

        let effective_asc = if is_under {
            base_asc
        } else {
            base_asc.max(self.font_set.constants().accent_base_height * em)
        };
        let y = -(effective_asc + 0.10 * em);
        let stroke_w = 0.045 * em;
        let mut polys: Vec<Vec<(f32, f32)>> = Vec::new();
        match ch {
            '¨' | '⃛' | '⃜' => {
                let count = match ch {
                    '¨' => 2,
                    '⃛' => 3,
                    _ => 4,
                };
                let step = if count == 4 { 0.14 * em } else { 0.16 * em };
                let radius = if count == 4 { 0.040 * em } else { 0.045 * em };
                let mut max_dot_x = w;
                for j in 0..count {
                    let dot_cx = center_x + (j as f32 - (count - 1) as f32 / 2.0) * step;
                    max_dot_x = max_dot_x.max(dot_cx + radius + 0.02 * em);
                    polys.push(
                        (0..12)
                            .map(|i| {
                                let angle = i as f32 / 12.0 * std::f32::consts::TAU;
                                (dot_cx + radius * angle.cos(), y + radius * angle.sin())
                            })
                            .collect(),
                    );
                }
                layout_box.asc = layout_box.asc.max(-y + radius + 0.04 * em);
                layout_box.w = layout_box.w.max(max_dot_x);
            }
            '⏞' | '⏟' => {
                let content_w = w - 2.0 * x0;
                let span = content_w.max(0.55 * em);
                let brace_x0 = x0 - (span - content_w) / 2.0;
                let depth = (span * 0.28).clamp(0.12 * em, 0.20 * em);
                polys.extend(stroke(
                    &bar::brace_h(span, y, depth, ch == '⏟')
                        .into_iter()
                        .map(|(x, y)| (x + brace_x0, y))
                        .collect::<Vec<_>>(),
                    stroke_w,
                ));
                layout_box.asc = layout_box.asc.max(effective_asc + depth + 0.15 * em);
            }
            '⎴' | '⎵' => {
                let dir_sign = if ch == '⎴' { -1.0 } else { 1.0 };
                let bracket_h = 0.16 * em;
                polys.extend(stroke(
                    &[
                        (x0, y),
                        (x0, y + dir_sign * bracket_h),
                        (w - x0, y + dir_sign * bracket_h),
                        (w - x0, y),
                    ],
                    stroke_w,
                ));
                layout_box.asc = layout_box.asc.max(effective_asc + 0.32 * em);
            }
            'ˊ' | 'ˋ' => {
                let (left_y, right_y) = if ch == 'ˊ' {
                    (y, y - 0.14 * em)
                } else {
                    (y - 0.14 * em, y)
                };
                polys.extend(stroke(
                    &[
                        (center_x - 0.07 * em, left_y),
                        (center_x + 0.07 * em, right_y),
                    ],
                    stroke_w,
                ));
                layout_box.asc = layout_box.asc.max(effective_asc + 0.30 * em);
            }
            'ˇ' => {
                polys.extend(stroke(
                    &[
                        (center_x - 0.20 * em, y - 0.12 * em),
                        (center_x, y),
                        (center_x + 0.20 * em, y - 0.12 * em),
                    ],
                    stroke_w,
                ));
                layout_box.asc = layout_box.asc.max(effective_asc + 0.28 * em);
            }
            '˘' | '⌣' | '⏝' => {
                polys.extend(stroke(
                    &quad_bezier(
                        (x0, y - 0.12 * em),
                        (w / 2.0, y + 0.08 * em),
                        (w - x0, y - 0.12 * em),
                        16,
                    ),
                    stroke_w,
                ));
                layout_box.asc = layout_box.asc.max(effective_asc + 0.28 * em);
            }
            '˚' => {
                polys.extend(stroke(
                    &(0..=20)
                        .map(|i| {
                            let angle = i as f32 / 20.0 * std::f32::consts::TAU;
                            (
                                center_x + 0.07 * em * angle.cos(),
                                y - 0.05 * em + 0.07 * em * angle.sin(),
                            )
                        })
                        .collect::<Vec<_>>(),
                    stroke_w * 0.65,
                ));
                layout_box.asc = layout_box.asc.max(effective_asc + 0.25 * em);
            }
            '_' | '═' | '̿' | '̳' => {
                let y_bar = -(base_asc + 0.10 * em);
                polys.push(rect(
                    x0,
                    y_bar - stroke_w / 2.0,
                    (w - 2.0 * x0).max(0.3 * em),
                    stroke_w.max(1.0),
                ));
                if ch != '_' {
                    polys.push(rect(
                        x0,
                        y_bar - 0.12 * em - stroke_w / 2.0,
                        (w - 2.0 * x0).max(0.3 * em),
                        stroke_w.max(1.0),
                    ));
                    layout_box.asc = layout_box.asc.max(base_asc + 0.29 * em);
                }
            }
            _ if arrow::arrow_dir(ch).is_some() => {
                if let Some(dir) = arrow::arrow_dir(ch) {
                    let is_double_or_stacked = matches!(
                        dir,
                        arrow::ArrowDir::DoubleRight
                            | arrow::ArrowDir::DoubleLeft
                            | arrow::ArrowDir::DoubleBoth
                            | arrow::ArrowDir::EquilibriumRightLeft
                            | arrow::ArrowDir::EquilibriumLeftRight
                            | arrow::ArrowDir::OpposingRightLeft
                            | arrow::ArrowDir::OpposingLeftRight
                    );
                    let (eff_em, x_left, span) = if is_double_or_stacked {
                        let content_w = w - 2.0 * x0;
                        let target_span = content_w.max(0.45 * em);
                        (em, x0 - (target_span - content_w) / 2.0, target_span)
                    } else {
                        let ic = self.italic_correction_of_box(layout_box);
                        let left = x0 + 0.35 * ic;
                        let right = (w - x0 + ic).max(left + 0.40 * em);
                        (0.65 * em, left, right - left)
                    };
                    let unit = eff_em / 1000.0;
                    let (y_min_font, _) = arrow::glyph_font_y_bounds(dir);
                    let y_glyph = (-(effective_asc + 0.10 * em) + y_min_font * unit)
                        .min(-(effective_asc + 0.18 * em) + 259.0 * unit);
                    let (min_y, _) = self.assemble_horizontal_arrow(
                        dir,
                        x_left,
                        span,
                        y_glyph,
                        eff_em,
                        &mut layout_box.items,
                    );
                    layout_box.asc = layout_box.asc.max(-min_y + 0.04 * em);
                }
            }
            '^' | '\u{0302}' | '\u{02C6}' => {
                // Determine content width (such as single letter 'B' vs multi-letter 'ABC').
                let content_w = w - 2.0 * x0;
                let (half_w, height, y_base) = if content_w > 0.65 * em {
                    // Wide angle hat spanning multiple letters (such as \widehat{ABC}).
                    let span = content_w.max(0.60 * em);
                    let half_w = span / 2.0;
                    let height = (0.08 * em + 0.04 * span).min(0.22 * em);
                    let base_y = -(effective_asc + 0.06 * em);
                    (half_w, height, base_y)
                } else {
                    // Standard compact hat for a single letter (such as \hat{x}, \hat{B}).
                    let half_w = (content_w * 0.65).clamp(0.13 * em, 0.22 * em);
                    let height = 0.10 * em;
                    let base_y = -(effective_asc + 0.03 * em);
                    (half_w, height, base_y)
                };
                // Stroke thickness measured perpendicular to the arms:
                // thick at the apex, thin at the two wingtips.
                let stroke_apex = 0.08 * em;
                let stroke_end = 0.015 * em;
                let slope_len = (height * height + half_w * half_w).sqrt();
                let apex_thick = (stroke_apex * slope_len / half_w).min(height * 0.8);
                let dx_inner = (stroke_end * slope_len / height).clamp(0.006 * em, half_w * 0.5);
                let y_apex = y_base - height;
                let y_inner = y_apex + apex_thick;
                polys.push(vec![
                    (center_x, y_apex),
                    (center_x + half_w, y_base),
                    (center_x + half_w - dx_inner, y_base),
                    (center_x, y_inner),
                    (center_x - half_w + dx_inner, y_base),
                    (center_x - half_w, y_base),
                ]);
                layout_box.asc = layout_box.asc.max(-y_apex + 0.04 * em);
            }
            '\u{2312}' | '\u{23DC}' | '\u{2322}' => {
                let natural = self
                    .font_set
                    .glyph_polys(Face::Symbol, '\u{2322}', em, em, 0.0, 0.0);
                if !natural.is_empty() {
                    let (x_lo, x_hi) = natural
                        .iter()
                        .flatten()
                        .fold((f32::MAX, f32::MIN), |(lo, hi), point| {
                            (lo.min(point.0), hi.max(point.0))
                        });
                    let (y_lo, y_hi) = natural
                        .iter()
                        .flatten()
                        .fold((f32::MAX, f32::MIN), |(lo, hi), point| {
                            (lo.min(point.1), hi.max(point.1))
                        });
                    let natural_w = (x_hi - x_lo).max(1.0);
                    let content_w = w - 2.0 * x0;
                    let span = content_w.max(0.58 * em);
                    let em_x = em * span / natural_w;
                    let em_y = if content_w < 0.7 * em { 0.78 * em } else { em };
                    let y_target = -(effective_asc + 0.04 * em);
                    let origin_y = y_target - y_hi * (em_y / em);
                    let origin_x = center_x - (x_lo + x_hi) / 2.0 * (em_x / em);
                    let polys = self.font_set.glyph_polys(
                        Face::Symbol,
                        '\u{2322}',
                        em_x,
                        em_y,
                        origin_x,
                        origin_y,
                    );
                    layout_box.items.push(Item::Contours(polys));
                    layout_box.asc = layout_box
                        .asc
                        .max(-(origin_y + y_lo * (em_y / em)) + 0.04 * em);
                }
            }
            '\u{00AF}' | '\u{0304}' | '\u{203E}' | '\u{2015}' | '\u{02C9}' => {
                let y_bar = -(base_asc + 0.08 * em);
                polys.push(rect(
                    x0,
                    y_bar - stroke_w / 2.0,
                    (w - 2.0 * x0).max(0.3 * em),
                    stroke_w.max(1.0),
                ));
            }
            '~' | '\u{0303}' | '\u{02DC}' => {
                let content_w = w - 2.0 * x0;
                let (span, amp, steps) = if content_w > 0.65 * em {
                    let span = content_w.max(0.55 * em);
                    let amp = (0.04 * em + 0.015 * span).min(0.08 * em);
                    (span, amp, 24usize)
                } else {
                    let span = (content_w * 0.75).clamp(0.34 * em, 0.48 * em);
                    (span, 0.04 * em, 16usize)
                };
                let points: Vec<(f32, f32)> = (0..=steps)
                    .map(|i| {
                        let ratio = i as f32 / steps as f32;
                        (
                            center_x - span / 2.0 + span * ratio,
                            y - amp * (ratio * std::f32::consts::TAU).sin(),
                        )
                    })
                    .collect();
                polys.extend(stroke(&points, stroke_w));
                layout_box.asc = layout_box.asc.max(-y + amp + stroke_w / 2.0 + 0.04 * em);
            }
            _ => {
                let radius = 0.045 * em;
                let points: Vec<(f32, f32)> = (0..12)
                    .map(|i| {
                        let angle = i as f32 / 12.0 * std::f32::consts::TAU;
                        (center_x + radius * angle.cos(), y + radius * angle.sin())
                    })
                    .collect();
                polys.push(points);
            }
        }
        for poly in polys {
            layout_box.items.push(Item::Poly(poly));
        }
        layout_box.asc = layout_box.asc.max(-y + 0.09 * em);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn poly_y_span(points: &[(f32, f32)]) -> f32 {
        let (mut min_y, mut max_y) = (f32::MAX, f32::MIN);
        for &(_, y) in points {
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
        max_y - min_y
    }

    #[test]
    fn mathring_renders_hollow_ring_distinct_from_dot() {
        let dot_math = crate::service::parse_latex_to_mathml(r"\dot{x}").unwrap();
        let ring_math = crate::service::parse_latex_to_mathml(r"\mathring{x}").unwrap();

        let (dot_box, _) = layout_math(&dot_math, 14.0).unwrap();
        let (ring_box, _) = layout_math(&ring_math, 14.0).unwrap();

        let dot_polys = dot_box
            .items
            .iter()
            .filter(|item| matches!(item, Item::Poly(_)))
            .count();
        let ring_polys = ring_box
            .items
            .iter()
            .filter(|item| matches!(item, Item::Poly(_)))
            .count();

        assert_eq!(dot_polys, 1, "overdot should draw a single filled polygon");
        assert!(
            ring_polys > 1,
            "mathring should draw a stroked ring (multiple edge segments), got {ring_polys}"
        );
    }

    #[test]
    fn widetilde_spans_multi_character_base_width() {
        let base_math = crate::service::parse_latex_to_mathml(r"xyz").unwrap();
        let tilde_math = crate::service::parse_latex_to_mathml(r"\widetilde{xyz}").unwrap();
        let hat_math = crate::service::parse_latex_to_mathml(r"\widehat{xyz}").unwrap();

        let (base_box, _) = layout_math(&base_math, 14.0).unwrap();
        let (tilde_box, _) = layout_math(&tilde_math, 14.0).unwrap();
        let (hat_box, _) = layout_math(&hat_math, 14.0).unwrap();

        let accent_x_span = |lbox: &LBox| -> f32 {
            let mut min_x = f32::MAX;
            let mut max_x = f32::MIN;
            for item in &lbox.items {
                if let Item::Poly(points) = item {
                    for &(x, _) in points {
                        min_x = min_x.min(x);
                        max_x = max_x.max(x);
                    }
                }
            }
            max_x - min_x
        };

        let tilde_span = accent_x_span(&tilde_box);
        let hat_span = accent_x_span(&hat_box);
        assert!(
            tilde_span >= 0.85 * base_box.w,
            "widetilde span {tilde_span} must cover base width {}",
            base_box.w
        );
        assert!(
            (tilde_span - hat_span).abs() < 0.25 * base_box.w,
            "widetilde span {tilde_span} should be comparable to widehat span {hat_span}"
        );
    }

    #[test]
    fn multi_char_arrow_accents_extend_with_unscaled_glyphs() {
        let base_math = crate::service::parse_latex_to_mathml(r"xyz").unwrap();
        let (base_box, em) = layout_math(&base_math, 14.0).unwrap();
        let expected_eff_em = 0.65 * em;

        let cases = [
            (r"\overleftarrow{xyz}", '\u{2190}'),
            (r"\overleftrightarrow{xyz}", '\u{2192}'),
            (r"\overleftharpoon{xyz}", '↼'),
            (r"\underrightharpoon{xyz}", '⇀'),
        ];

        for (latex, expected_head) in cases {
            let math = crate::service::parse_latex_to_mathml(latex).unwrap();
            let (accent_box, _) = layout_math(&math, 14.0).unwrap();

            let has_extender = accent_box.items.iter().any(|item| {
                matches!(
                    item,
                    Item::Glyph {
                        ch: '\u{23AF}',
                        em: item_em,
                        ..
                    } if (*item_em - expected_eff_em).abs() < 1e-3
                )
            });
            assert!(
                has_extender,
                "{latex}: must include unscaled '\\u{{23AF}}' extender at em={expected_eff_em}"
            );

            // Check that the arrowhead glyph (or reflected contour for under-harpoon)
            // is present and spans the base width cleanly.
            let has_head = accent_box.items.iter().any(|item| match item {
                Item::Glyph {
                    ch, em: item_em, ..
                } => *ch == expected_head && (*item_em - expected_eff_em).abs() < 1e-3,
                Item::Contours(contours) => {
                    contours
                        .iter()
                        .flatten()
                        .fold(f32::MIN, |acc, &(_, y)| acc.max(y))
                        > base_box.desc
                }
                _ => false,
            });
            assert!(has_head, "{latex}: missing arrowhead piece");

            let (min_x, max_x, _, _) = accent_box.ink_bounds(fonts());
            assert!(
                max_x - min_x >= 0.85 * base_box.w,
                "{latex}: accent width {} must cover base width {}",
                max_x - min_x,
                base_box.w
            );
        }
    }

    #[test]
    fn narrow_arrow_accents_preserve_natural_arrowhead_aspect() {
        let font_set = fonts();

        // Single narrow letter \vec{i}: arrowhead must not be compressed horizontally.
        let vec_math = crate::service::parse_latex_to_mathml(r"\vec{i}").unwrap();
        let (vec_box, em) = layout_math(&vec_math, 14.0).unwrap();
        let eff_em = 0.65 * em;
        let expected_height = (459.0 - 58.0) * (eff_em / 1000.0);
        let arrow_poly = vec_box
            .items
            .iter()
            .find_map(|item| match item {
                Item::Poly(points) => Some(points),
                _ => None,
            })
            .expect(r"\vec{i} should trim shaft into a clipped unscaled Poly");
        let poly_h = poly_y_span(arrow_poly);
        assert!(
            (poly_h - expected_height).abs() < 0.5,
            r"\vec{{i}} arrowhead height {poly_h} should match unscaled {expected_height}"
        );

        // Single narrow letter \overaccent{⇔}{x}: falls back to unscaled centered natural ⇔.
        let double_math = crate::service::parse_latex_to_mathml(r"\overaccent{⇔}{x}").unwrap();
        let (double_box, em_double) = layout_math(&double_math, 14.0).unwrap();
        let double_glyph = double_box.items.iter().find_map(|item| match item {
            Item::Glyph {
                ch: '⇔',
                em: glyph_em,
                ..
            } => Some(*glyph_em),
            _ => None,
        });
        assert!(
            double_glyph.is_some_and(|glyph_em| (glyph_em - em_double).abs() < 1e-3),
            r"\overaccent{{⇔}}{{x}} must emit unscaled natural '⇔' glyph"
        );
        let (natural_x0, natural_x1) = font_set.ink_x(Face::Symbol, '⇔', em_double);
        assert!(natural_x1 > natural_x0);
    }

    #[test]
    fn top_accent_with_horizontal_offset_does_not_double_count_x0_and_ddddot_fits() {
        let layout = Layout {
            font_set: fonts(),
            depth: 0,
        };
        let em = 20.0;
        let base = layout.glyph('x', Face::Variable, em);
        let x0 = 25.0;
        let total_w = base.w + 2.0 * x0;
        let mut boxed = LBox::default();
        boxed.place(&base, x0, 0.0);
        boxed.w = total_w;
        layout.draw_accent(&mut boxed, '\u{0307}', total_w, x0, base.asc, em);

        let expected_cx = x0
            + fonts()
                .top_accent_attachment(Face::Variable, 'x', em)
                .unwrap_or(base.w / 2.0);
        let dot_poly = boxed
            .items
            .iter()
            .find_map(|item| match item {
                Item::Poly(points) => Some(points),
                _ => None,
            })
            .expect("overdot polygon");
        let (min_x, max_x) = dot_poly
            .iter()
            .fold((f32::MAX, f32::MIN), |(lo, hi), &(x, _)| {
                (lo.min(x), hi.max(x))
            });
        let actual_cx = (min_x + max_x) / 2.0;
        assert!(
            (actual_cx - expected_cx).abs() < 0.1,
            "dot center {actual_cx} must equal single-offset {expected_cx}"
        );

        let ddddot = crate::service::parse_latex_to_mathml(r"\ddddot{x}").unwrap();
        let (ddddot_box, _) = layout_math(&ddddot, 14.0).unwrap();
        let (ink_min, ink_max, _, _) = ddddot_box.ink_bounds(fonts());
        assert!(
            ink_min >= 0.0 && ink_max <= ddddot_box.w + 0.5,
            "ddddot ink bounds ({ink_min}..{ink_max}) must stay within box width {}",
            ddddot_box.w
        );
    }
}
