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
        // Retrieve top accent attachment position from OpenType MATH table if available;
        // fall back to italic correction compensation if slanted.
        let center_x = if let Some(att) = self.top_accent_attachment_of_box(layout_box) {
            x0 + att
        } else {
            let ic = self.italic_correction_of_box(layout_box);
            if ic > 0.0 {
                w / 2.0 + 0.35 * ic
            } else {
                w / 2.0
            }
        };

        let y = -(base_asc + 0.10 * em);
        let stroke_w = 0.045 * em;
        let mut polys: Vec<Vec<(f32, f32)>> = Vec::new();
        match ch {
            '¨' | '⃛' | '⃜' => {
                let count = match ch {
                    '¨' => 2,
                    '⃛' => 3,
                    _ => 4,
                };
                let step = 0.18 * em;
                for j in 0..count {
                    let dot_cx = center_x + (j as f32 - (count - 1) as f32 / 2.0) * step;
                    polys.push(
                        (0..12)
                            .map(|i| {
                                let angle = i as f32 / 12.0 * std::f32::consts::TAU;
                                (
                                    dot_cx + 0.045 * em * angle.cos(),
                                    y + 0.045 * em * angle.sin(),
                                )
                            })
                            .collect(),
                    );
                }
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
                layout_box.asc = layout_box.asc.max(base_asc + depth + 0.15 * em);
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
                layout_box.asc = layout_box.asc.max(base_asc + 0.32 * em);
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
                layout_box.asc = layout_box.asc.max(base_asc + 0.30 * em);
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
                layout_box.asc = layout_box.asc.max(base_asc + 0.28 * em);
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
                layout_box.asc = layout_box.asc.max(base_asc + 0.28 * em);
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
                layout_box.asc = layout_box.asc.max(base_asc + 0.25 * em);
            }
            '_' | '═' | '̿' | '̳' => {
                polys.push(rect(
                    x0,
                    y - stroke_w / 2.0,
                    (w - 2.0 * x0).max(0.3 * em),
                    stroke_w.max(1.0),
                ));
                if ch != '_' {
                    polys.push(rect(
                        x0,
                        y - 0.12 * em - stroke_w / 2.0,
                        (w - 2.0 * x0).max(0.3 * em),
                        stroke_w.max(1.0),
                    ));
                    layout_box.asc = layout_box.asc.max(base_asc + 0.29 * em);
                }
            }
            '←' | '↔' | '⇀' | '↼' | '⇌' | '⇋' | '⇒' | '⇐' | '⇔' => {
                let natural = self
                    .font_set
                    .glyph_polys(Face::Symbol, ch, em, em, 0.0, 0.0);
                let (min_x, max_x, min_y, max_y) = natural.iter().flatten().fold(
                    (f32::MAX, f32::MIN, f32::MAX, f32::MIN),
                    |(x_lo, x_hi, y_lo, y_hi), point| {
                        (
                            x_lo.min(point.0),
                            x_hi.max(point.0),
                            y_lo.min(point.1),
                            y_hi.max(point.1),
                        )
                    },
                );
                if max_x > min_x {
                    let content_w = w - 2.0 * x0;
                    let span = content_w.max(0.45 * em);
                    let arrow_x0 = x0 - (span - content_w) / 2.0;
                    let contours: Vec<Vec<(f32, f32)>> = natural
                        .into_iter()
                        .map(|poly| {
                            poly.into_iter()
                                .map(|(x, py)| {
                                    (
                                        arrow_x0 + (x - min_x) * span / (max_x - min_x),
                                        py + y - max_y,
                                    )
                                })
                                .collect()
                        })
                        .collect();
                    layout_box.items.push(Item::Contours(contours));
                    layout_box.asc = layout_box.asc.max(-(min_y + y - max_y) + 0.04 * em);
                }
            }
            '\u{20D7}' | '\u{2192}' | '\u{27F6}' => {
                let ic = self.italic_correction_of_box(layout_box);
                let eff_em = 0.65 * em;
                let unit = eff_em / 1000.0;
                let x_left = x0 + 0.35 * ic;
                let x_right = (w - x0 + ic).max(x_left + 0.40 * em);
                let span = x_right - x_left;
                let y_shaft = -(base_asc + 0.18 * em);
                let y_glyph = y_shaft + 259.0 * unit;
                let min_w = 805.0 * unit;

                if span < min_w {
                    // Short arrow (single letter): scale '\u{2192}' horizontally to span the letter.
                    let em_x = eff_em * span / min_w;
                    let origin_x = x_left - 71.0 * (em_x / 1000.0);
                    let polys = self.font_set.glyph_polys(
                        Face::Symbol,
                        '\u{2192}',
                        em_x,
                        eff_em,
                        origin_x,
                        y_glyph,
                    );
                    layout_box.items.push(Item::Contours(polys));
                } else {
                    // Delimiter-style piece assembly:
                    // 1. Arrowhead piece at right.
                    let arrow_x = x_right - 876.0 * unit;
                    layout_box.items.push(Item::Glyph {
                        ch: '\u{2192}',
                        face: Face::Symbol,
                        em: eff_em,
                        x: arrow_x,
                        y: y_glyph,
                        ssty: 0,
                    });
                    // 2. Extenders tiled to the left.
                    let from_x = x_left;
                    let to_x = arrow_x + 150.0 * unit;
                    let gap = to_x - from_x;
                    if gap > 0.0 {
                        let ext_w = 322.0 * unit;
                        let count = (gap / ext_w).ceil().max(1.0) as usize;
                        let step = if count > 1 {
                            (gap - ext_w) / (count - 1) as f32
                        } else {
                            0.0
                        };
                        let first_x = if count > 1 {
                            from_x
                        } else {
                            from_x + (gap - ext_w) / 2.0
                        };
                        for i in 0..count {
                            layout_box.items.push(Item::Glyph {
                                ch: '\u{23AF}',
                                face: Face::Symbol,
                                em: eff_em,
                                x: first_x + i as f32 * step,
                                y: y_glyph,
                                ssty: 0,
                            });
                        }
                    }
                }
                layout_box.asc = layout_box.asc.max(-y_shaft + 200.0 * unit + 0.04 * em);
            }
            '^' | '\u{0302}' | '\u{02C6}' => {
                // Determine content width (such as single letter 'B' vs multi-letter 'ABC').
                let content_w = w - 2.0 * x0;
                let (half_w, height, y_base) = if content_w > 0.65 * em {
                    // Wide angle hat spanning multiple letters (such as \widehat{ABC}).
                    let span = content_w.max(0.60 * em);
                    let half_w = span / 2.0;
                    let height = (0.08 * em + 0.04 * span).min(0.22 * em);
                    let base_y = -(base_asc + 0.06 * em);
                    (half_w, height, base_y)
                } else {
                    // Standard compact hat for a single letter (such as \hat{x}, \hat{B}).
                    let half_w = (content_w * 0.65).clamp(0.13 * em, 0.22 * em);
                    let height = 0.10 * em;
                    let base_y = -(base_asc + 0.03 * em);
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
                    let y_target = -(base_asc + 0.04 * em);
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
                        let t = i as f32 / steps as f32;
                        (
                            center_x - span / 2.0 + span * t,
                            y - amp * (t * std::f32::consts::TAU).sin(),
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

    /// Reflect a vector accent below the baseline, preserving its natural width.
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
        self.draw_accent(&mut accent, ch, w, x0, desc, em);
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
                } if ch == '\u{2192}' || ch == '\u{23AF}' => {
                    // Both '\u{2192}' and '\u{23AF}' are vertically symmetric around their shaft
                    // (+259 font units above baseline), so keeping Item::Glyph avoids seam notches.
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
