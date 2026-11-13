//! Underbar / overbar templates and horizontal braces.

use super::*;

/// Polyline of a horizontal brace of width `w` whose bar is at `y0`.
pub(super) fn brace_h(w: f32, y0: f32, depth: f32, tip_down: bool) -> Vec<(f32, f32)> {
    let s = if tip_down { 1.0 } else { -1.0 };
    let k = depth / 2.0;
    let m = w / 2.0;
    let mut p = vec![(0.0, y0)];
    p.extend(quad_bezier(
        (0.0, y0),
        (0.0, y0 + s * k),
        (k, y0 + s * k),
        6,
    ));
    p.push((m - k, y0 + s * k));
    p.extend(quad_bezier(
        (m - k, y0 + s * k),
        (m, y0 + s * k),
        (m, y0 + s * depth),
        6,
    ));
    p.extend(quad_bezier(
        (m, y0 + s * depth),
        (m, y0 + s * k),
        (m + k, y0 + s * k),
        6,
    ));
    p.push((w - k, y0 + s * k));
    p.extend(quad_bezier(
        (w - k, y0 + s * k),
        (w, y0 + s * k),
        (w, y0),
        6,
    ));
    p
}

impl Layout {
    /// Draw a horizontal brace under a base whose descent is `base_desc`.
    pub(super) fn draw_under_brace(&self, b: &mut LBox, w: f32, base_desc: f32, em: f32) {
        let y0 = base_desc + 0.06 * em;
        let pts = brace_h(w, y0, 0.2 * em, true);
        for q in stroke(&pts, 0.045 * em) {
            b.items.push(Item::Poly(q));
        }
        b.desc = b.desc.max(y0 + 0.2 * em + 0.04 * em);
    }
}
