//! Embellishments: accents drawn over a base (hat, tilde, dot, arrow, bar, …).

use super::*;

impl Layout {
    pub(super) fn draw_accent(
        &self,
        b: &mut LBox,
        c: char,
        w: f32,
        x0: f32,
        base_asc: f32,
        em: f32,
    ) {
        let c = match c {
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
            _ => c,
        };
        // Retrieve top accent attachment position from OpenType MATH table if available;
        // fallback to italic correction compensation if slanted.
        let cx = if let Some(att) = self.top_accent_attachment_of_box(b) {
            x0 + att
        } else {
            let ic = self.italic_correction_of_box(b);
            if ic > 0.0 {
                w / 2.0 + 0.35 * ic
            } else {
                w / 2.0
            }
        };

        let y = -(base_asc + 0.10 * em);
        let st = 0.045 * em;
        let mut polys: Vec<Vec<(f32, f32)>> = Vec::new();
        match c {
            '¨' | '⃛' | '⃜' => {
                let count = match c {
                    '¨' => 2,
                    '⃛' => 3,
                    _ => 4,
                };
                let step = 0.18 * em;
                for j in 0..count {
                    let dcx = cx + (j as f32 - (count - 1) as f32 / 2.0) * step;
                    polys.push(
                        (0..12)
                            .map(|i| {
                                let a = i as f32 / 12.0 * std::f32::consts::TAU;
                                (dcx + 0.045 * em * a.cos(), y + 0.045 * em * a.sin())
                            })
                            .collect(),
                    );
                }
            }
            '⏞' | '⏟' => {
                let content_w = w - 2.0 * x0;
                let span = content_w.max(0.55 * em);
                let bx0 = x0 - (span - content_w) / 2.0;
                let depth = (span * 0.28).clamp(0.12 * em, 0.20 * em);
                polys.extend(stroke(
                    &bar::brace_h(span, y, depth, c == '⏟')
                        .into_iter()
                        .map(|(x, y)| (x + bx0, y))
                        .collect::<Vec<_>>(),
                    st,
                ));
                b.asc = b.asc.max(base_asc + depth + 0.15 * em);
            }
            '⎴' | '⎵' => {
                let sign = if c == '⎴' { -1.0 } else { 1.0 };
                let h = 0.16 * em;
                polys.extend(stroke(
                    &[
                        (x0, y),
                        (x0, y + sign * h),
                        (w - x0, y + sign * h),
                        (w - x0, y),
                    ],
                    st,
                ));
                b.asc = b.asc.max(base_asc + 0.32 * em);
            }
            'ˊ' | 'ˋ' => {
                let (left, right) = if c == 'ˊ' {
                    (y, y - 0.14 * em)
                } else {
                    (y - 0.14 * em, y)
                };
                polys.extend(stroke(
                    &[(cx - 0.07 * em, left), (cx + 0.07 * em, right)],
                    st,
                ));
                b.asc = b.asc.max(base_asc + 0.30 * em);
            }
            'ˇ' => {
                polys.extend(stroke(
                    &[
                        (cx - 0.20 * em, y - 0.12 * em),
                        (cx, y),
                        (cx + 0.20 * em, y - 0.12 * em),
                    ],
                    st,
                ));
                b.asc = b.asc.max(base_asc + 0.28 * em);
            }
            '˘' | '⌣' | '⏝' => {
                polys.extend(stroke(
                    &quad_bezier(
                        (x0, y - 0.12 * em),
                        (w / 2.0, y + 0.08 * em),
                        (w - x0, y - 0.12 * em),
                        16,
                    ),
                    st,
                ));
                b.asc = b.asc.max(base_asc + 0.28 * em);
            }
            '˚' => {
                polys.extend(stroke(
                    &(0..=20)
                        .map(|i| {
                            let a = i as f32 / 20.0 * std::f32::consts::TAU;
                            (
                                cx + 0.07 * em * a.cos(),
                                y - 0.05 * em + 0.07 * em * a.sin(),
                            )
                        })
                        .collect::<Vec<_>>(),
                    st * 0.65,
                ));
                b.asc = b.asc.max(base_asc + 0.25 * em);
            }
            '_' | '═' | '̿' | '̳' => {
                polys.push(rect(
                    x0,
                    y - st / 2.0,
                    (w - 2.0 * x0).max(0.3 * em),
                    st.max(1.0),
                ));
                if c != '_' {
                    polys.push(rect(
                        x0,
                        y - 0.12 * em - st / 2.0,
                        (w - 2.0 * x0).max(0.3 * em),
                        st.max(1.0),
                    ));
                    b.asc = b.asc.max(base_asc + 0.29 * em);
                }
            }
            '←' | '↔' | '⇀' | '↼' | '⇌' | '⇋' | '⇒' | '⇐' | '⇔' => {
                let natural = self.f.glyph_polys(Face::Symbol, c, em, em, 0.0, 0.0);
                let (xl, xh, yl, yh) = natural.iter().flatten().fold(
                    (f32::MAX, f32::MIN, f32::MAX, f32::MIN),
                    |(xl, xh, yl, yh), p| (xl.min(p.0), xh.max(p.0), yl.min(p.1), yh.max(p.1)),
                );
                if xh > xl {
                    let content_w = w - 2.0 * x0;
                    let span = content_w.max(0.45 * em);
                    let ax0 = x0 - (span - content_w) / 2.0;
                    let contours: Vec<Vec<(f32, f32)>> = natural
                        .into_iter()
                        .map(|p| {
                            p.into_iter()
                                .map(|(x, py)| (ax0 + (x - xl) * span / (xh - xl), py + y - yh))
                                .collect()
                        })
                        .collect();
                    b.items.push(Item::Contours(contours));
                    b.asc = b.asc.max(-(yl + y - yh) + 0.04 * em);
                }
            }
            '\u{20D7}' | '\u{2192}' | '\u{27F6}' => {
                let ic = self.italic_correction_of_box(b);
                let eff_em = 0.65 * em;
                let k = eff_em / 1000.0;
                let x_left = x0 + 0.35 * ic;
                let x_right = (w - x0 + ic).max(x_left + 0.40 * em);
                let span = x_right - x_left;
                let y_shaft = -(base_asc + 0.18 * em);
                let y_glyph = y_shaft + 259.0 * k;
                let min_w = 805.0 * k;

                if span < min_w {
                    // Short arrow (single letter): scale uni2192 horizontally to span the letter
                    let em_x = eff_em * span / min_w;
                    let ox = x_left - 71.0 * (em_x / 1000.0);
                    let oy = y_glyph;
                    let pts = self
                        .f
                        .glyph_polys(Face::Symbol, '\u{2192}', em_x, eff_em, ox, oy);
                    b.items.push(Item::Contours(pts));
                } else {
                    // Delimiter-style piece assembly:
                    // 1. Arrowhead piece at right
                    let arrow_x = x_right - 876.0 * k;
                    b.items.push(Item::Glyph {
                        ch: '\u{2192}',
                        face: Face::Symbol,
                        em: eff_em,
                        x: arrow_x,
                        y: y_glyph,
                        ssty: 0,
                    });
                    // 2. Extenders tiled to the left
                    let from_x = x_left;
                    let to_x = arrow_x + 150.0 * k;
                    let gap = to_x - from_x;
                    if gap > 0.0 {
                        let e_w = 322.0 * k;
                        let count = (gap / e_w).ceil().max(1.0) as usize;
                        let step = if count > 1 {
                            (gap - e_w) / (count - 1) as f32
                        } else {
                            0.0
                        };
                        let first = if count > 1 {
                            from_x
                        } else {
                            from_x + (gap - e_w) / 2.0
                        };
                        for i in 0..count {
                            b.items.push(Item::Glyph {
                                ch: '\u{23AF}',
                                face: Face::Symbol,
                                em: eff_em,
                                x: first + i as f32 * step,
                                y: y_glyph,
                                ssty: 0,
                            });
                        }
                    }
                }
                b.asc = b.asc.max(-y_shaft + 200.0 * k + 0.04 * em);
            }
            '^' | '\u{0302}' | '\u{02C6}' => {
                // Determine the actual content width (e.g. single letter 'B' vs multi-letter 'ABC')
                let content_w = w - 2.0 * x0;
                let (hw, h, y_base) = if content_w > 0.65 * em {
                    // Wide angle hat spanning multiple letters (e.g. \widehat{ABC})
                    let span = content_w.max(0.60 * em);
                    let half_w = span / 2.0;
                    let height = (0.08 * em + 0.04 * span).min(0.22 * em);
                    let base_y = -(base_asc + 0.06 * em);
                    (half_w, height, base_y)
                } else {
                    // Standard compact hat for a single letter (e.g. \hat{x}, \hat{B})
                    let half_w = (content_w * 0.65).clamp(0.13 * em, 0.22 * em);
                    let height = 0.10 * em;
                    let base_y = -(base_asc + 0.03 * em);
                    (half_w, height, base_y)
                };
                // Stroke thickness measured perpendicular to the arms:
                // thick at the apex, thin at the two wingtips.
                let stroke_apex = 0.08 * em;
                let stroke_end = 0.015 * em;
                let slope_len = (h * h + hw * hw).sqrt();
                // Vertical thickness at the apex that gives `stroke_apex` perpendicular thickness.
                let t_apex = (stroke_apex * slope_len / hw).min(h * 0.8);
                // Horizontal thickness at the wingtips that gives `stroke_end` perpendicular thickness.
                let dx_inner = (stroke_end * slope_len / h).clamp(0.006 * em, hw * 0.5);
                let y_apex = y_base - h;
                let y_inner = y_apex + t_apex;
                // 6-point hollow chevron: thick at the top, tapering to thin wingtips
                polys.push(vec![
                    (cx, y_apex),                 // Top center peak
                    (cx + hw, y_base),            // Right outer wingtip
                    (cx + hw - dx_inner, y_base), // Right inner wingtip
                    (cx, y_inner),                // Underside hollow apex
                    (cx - hw + dx_inner, y_base), // Left inner wingtip
                    (cx - hw, y_base),            // Left outer wingtip
                ]);
                // Ensure the bounding box covers the peak of the hat
                b.asc = b.asc.max(-y_apex + 0.04 * em);
            }
            '\u{2312}' | '\u{23DC}' | '\u{2322}' => {
                let natural = self
                    .f
                    .glyph_polys(Face::Symbol, '\u{2322}', em, em, 0.0, 0.0);
                if !natural.is_empty() {
                    let (x_lo, x_hi) = natural
                        .iter()
                        .flatten()
                        .fold((f32::MAX, f32::MIN), |(lo, hi), p| {
                            (lo.min(p.0), hi.max(p.0))
                        });
                    let (y_lo, y_hi) = natural
                        .iter()
                        .flatten()
                        .fold((f32::MAX, f32::MIN), |(lo, hi), p| {
                            (lo.min(p.1), hi.max(p.1))
                        });
                    let w0 = (x_hi - x_lo).max(1.0);
                    let content_w = w - 2.0 * x0;
                    let span = content_w.max(0.58 * em);
                    let em_x = em * span / w0;
                    let em_y = if content_w < 0.7 * em { 0.78 * em } else { em };
                    let y_target = -(base_asc + 0.04 * em);
                    let oy = y_target - y_hi * (em_y / em);
                    let ox = cx - (x_lo + x_hi) / 2.0 * (em_x / em);
                    let pts = self
                        .f
                        .glyph_polys(Face::Symbol, '\u{2322}', em_x, em_y, ox, oy);
                    b.items.push(Item::Contours(pts));
                    b.asc = b.asc.max(-(oy + y_lo * (em_y / em)) + 0.04 * em);
                }
            }
            '\u{00AF}' | '\u{0304}' | '\u{203E}' | '\u{2015}' | '\u{02C9}' => {
                let y_bar = -(base_asc + 0.08 * em);
                polys.push(rect(
                    x0,
                    y_bar - st / 2.0,
                    (w - 2.0 * x0).max(0.3 * em),
                    st.max(1.0),
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
                let pts: Vec<(f32, f32)> = (0..=steps)
                    .map(|i| {
                        let t = i as f32 / steps as f32;
                        (
                            cx - span / 2.0 + span * t,
                            y - amp * (t * std::f32::consts::TAU).sin(),
                        )
                    })
                    .collect();
                polys.extend(stroke(&pts, st));
                b.asc = b.asc.max(-y + amp + st / 2.0 + 0.04 * em);
            }
            _ => {
                let r = 0.045 * em;
                let pts: Vec<(f32, f32)> = (0..12)
                    .map(|i| {
                        let a = i as f32 / 12.0 * std::f32::consts::TAU;
                        (cx + r * a.cos(), y + r * a.sin())
                    })
                    .collect();
                polys.push(pts);
            }
        }
        for p in polys {
            b.items.push(Item::Poly(p));
        }
        b.asc = b.asc.max(-y + 0.09 * em);
    }
    /// Reflect a vector accent below the baseline, preserving its natural width.
    pub(super) fn draw_under_accent(
        &self,
        b: &mut LBox,
        c: char,
        w: f32,
        x0: f32,
        desc: f32,
        em: f32,
    ) {
        let mut accent = LBox::default();
        let c = match c {
            '⏟' => '⏞',
            '⎵' => '⎴',
            '⏝' => '⏜',
            _ => c,
        };
        self.draw_accent(&mut accent, c, w, x0, desc, em);
        for item in accent.items {
            match item {
                Item::Poly(p) => b
                    .items
                    .push(Item::Poly(p.into_iter().map(|(x, y)| (x, -y)).collect())),
                Item::Contours(cs) => b.items.push(Item::Contours(
                    cs.into_iter()
                        .map(|p| p.into_iter().map(|(x, y)| (x, -y)).collect())
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
                    // (+259 font units above baseline), so keeping Item::Glyph avoids extender seam notches.
                    b.items.push(Item::Glyph {
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
                        .f
                        .glyph_polys(face, ch, glyph_em, glyph_em, x, y)
                        .into_iter()
                        .map(|p| p.into_iter().map(|(px, py)| (px, -py)).collect())
                        .collect();
                    if !contours.is_empty() {
                        b.items.push(Item::Contours(contours));
                    }
                }
            }
        }
        b.desc = b.desc.max(accent.asc);
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
                if let Item::Poly(pts) = item {
                    for &(x, _) in pts {
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
