//! Embellishments: accents drawn over a base (hat, tilde, dot, arrow, bar, …).

use super::*;


impl Layout {
    pub(super) fn draw_accent(&self, b: &mut LBox, c: char, w: f32, x0: f32, base_asc: f32, em: f32) {
        // Compensate for rightward slant if the base letter is an italic variable (like italic 'S')
        let is_italic = b.items.first().map_or(false, |it| match it {
            Item::Glyph { face: Face::Variable, .. } => true,
            _ => false,
        });
        let cx = w / 2.0 + if is_italic { 0.04 * em } else { 0.0 };
        let y = -(base_asc + 0.10 * em);
        let st = 0.045 * em;
        let mut polys: Vec<Vec<(f32, f32)>> = Vec::new();
        match c {
            '\u{20D7}' | '\u{2192}' | '\u{27F6}' => {
                let eff_em = 0.65 * em;
                let k = eff_em / 1000.0;
                let slant = if is_italic { 0.04 * em } else { 0.0 };
                let x_left = x0 + slant;
                let x_right = (w - x0 + slant + 0.04 * em).max(x_left + 0.40 * em);
                let span = x_right - x_left;
                let y_shaft = -(base_asc + 0.10 * em);
                let y_glyph = y_shaft + 259.0 * k;
                let min_w = 805.0 * k;

                if span < min_w {
                    // Short arrow (single letter): scale uni2192 horizontally to span the letter
                    let em_x = eff_em * span / min_w;
                    let ox = x_left - 71.0 * (em_x / 1000.0);
                    let oy = y_glyph;
                    let pts = self.f.glyph_polys(Face::Symbol, '\u{2192}', em_x, eff_em, ox, oy);
                    polys.extend(pts);
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
                    });
                    // 2. Extenders tiled to the left
                    let from_x = x_left;
                    let to_x = arrow_x + 150.0 * k;
                    let gap = to_x - from_x;
                    if gap > 0.0 {
                        let e_w = 322.0 * k;
                        let count = (gap / e_w).ceil().max(1.0) as usize;
                        let step = if count > 1 { (gap - e_w) / (count - 1) as f32 } else { 0.0 };
                        let first = if count > 1 { from_x } else { from_x + (gap - e_w) / 2.0 };
                        for i in 0..count {
                            b.items.push(Item::Glyph {
                                ch: '\u{23AF}',
                                face: Face::Symbol,
                                em: eff_em,
                                x: first + i as f32 * step,
                                y: y_glyph,
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
                let natural = self.f.glyph_polys(Face::Symbol, '\u{2322}', em, em, 0.0, 0.0);
                if !natural.is_empty() {
                    let (x_lo, x_hi) = natural.iter().flatten().fold((f32::MAX, f32::MIN), |(lo, hi), p| (lo.min(p.0), hi.max(p.0)));
                    let (y_lo, y_hi) = natural.iter().flatten().fold((f32::MAX, f32::MIN), |(lo, hi), p| (lo.min(p.1), hi.max(p.1)));
                    let w0 = (x_hi - x_lo).max(1.0);
                    let content_w = w - 2.0 * x0;
                    let span = content_w.max(0.40 * em);
                    let em_x = em * span / w0;
                    let em_y = em;
                    let y_target = -(base_asc + 0.04 * em);
                    let oy = y_target - y_hi;
                    let ox = cx - (x_lo + x_hi) / 2.0 * (em_x / em);
                    let pts = self.f.glyph_polys(Face::Symbol, '\u{2322}', em_x, em_y, ox, oy);
                    polys.extend(pts);
                    b.asc = b.asc.max(-(oy + y_lo) + 0.04 * em);
                }
            }
            '\u{00AF}' | '\u{0304}' | '\u{203E}' | '\u{2015}' | '\u{02C9}' => {
                let y_bar = -(base_asc + 0.08 * em);
                polys.push(rect(x0, y_bar - st / 2.0, (w - 2.0 * x0).max(0.3 * em), st.max(1.0)));
            }
            '~' | '\u{0303}' | '\u{02DC}' => {
                let pts: Vec<(f32, f32)> = (0..=12)
                    .map(|i| {
                        let t = i as f32 / 12.0;
                        (cx - 0.17 * em + 0.34 * em * t, y - 0.04 * em * (t * std::f32::consts::TAU).sin())
                    })
                    .collect();
                polys.extend(stroke(&pts, st));
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
}
