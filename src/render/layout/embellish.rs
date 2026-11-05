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
        let y = -(base_asc + 0.13 * em);
        let st = 0.045 * em;
        let mut polys: Vec<Vec<(f32, f32)>> = Vec::new();
        match c {
            '\u{20D7}' | '\u{2192}' | '\u{27F6}' => {
                let len = (w - 2.0 * x0).max(0.5 * em);
                let (l, r) = (cx - len / 2.0, cx + len / 2.0);
                polys.extend(stroke(&[(l, y), (r, y)], st));
                polys.extend(stroke(&[(r - 0.11 * em, y - 0.07 * em), (r, y), (r - 0.11 * em, y + 0.07 * em)], st));
            }
            '^' | '\u{0302}' | '\u{02C6}' => {
                // Determine the actual content width (e.g. single letter 'B' vs multi-letter 'ABC')
                let content_w = w - 2.0 * x0;
                let (hw, h, y_base, t_apex) = if content_w > 0.65 * em {
                    // Wide angle hat spanning multiple letters (e.g. \widehat{ABC})
                    let span = content_w.max(0.60 * em);
                    let half_w = span / 2.0;
                    let height = (0.08 * em + 0.04 * span).min(0.22 * em);
                    let base_y = -(base_asc + 0.06 * em);
                    (half_w, height, base_y, 0.045 * em)
                } else {
                    // Standard compact hat for a single letter (e.g. \hat{x}, \hat{B})
                    let half_w = (content_w * 0.65).clamp(0.13 * em, 0.22 * em);
                    let height = 0.10 * em;
                    let base_y = -(base_asc + 0.03 * em);
                    (half_w, height, base_y, 0.035 * em)
                };
                let y_apex = y_base - h;
                let y_inner = y_apex + t_apex;
                let dx_inner = 0.035 * em;
                // 6-point hollow chevron that dynamically scales to the base width
                polys.push(vec![
                    (cx, y_apex),                      // Top center peak
                    (cx + hw, y_base),                 // Right outer wingtip
                    (cx + hw - dx_inner, y_base),      // Right inner wingtip
                    (cx, y_inner),                     // Underside hollow apex
                    (cx - hw + dx_inner, y_base),      // Left inner wingtip
                    (cx - hw, y_base),                 // Left outer wingtip
                ]);
                // Ensure the bounding box covers the peak of the hat
                b.asc = b.asc.max(-y_apex + 0.04 * em);
            }
            '\u{2312}' | '\u{23DC}' | '\u{2322}' => {
                // Determine span of the arc based on the base width
                let margin = 0.04 * em;
                let x_left = x0 + margin;
                let x_right = (w - x0 - margin).max(x_left + 0.40 * em);
                let mid_x = (x_left + x_right) / 2.0;
                // Arc heights
                let y_ends = -(base_asc + 0.08 * em);       // Two ends close to letters
                let y_peak = y_ends - 0.14 * em;            // Crown of the arc
                // Generate quadratic Bezier curve points for smooth circular arc
                let arc_pts = quad_bezier(
                    (x_left, y_ends),
                    (mid_x, y_peak - 0.04 * em), // Control point pulls peak upward
                    (x_right, y_ends),
                    16,
                );
                // Stroke with uniform calligraphic line
                polys.extend(stroke(&arc_pts, 0.045 * em));
            }
            '\u{00AF}' | '\u{0304}' | '\u{203E}' | '\u{2015}' | '\u{02C9}' => {
                polys.push(rect(x0, y - st / 2.0, (w - 2.0 * x0).max(0.3 * em), st.max(1.0)));
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
