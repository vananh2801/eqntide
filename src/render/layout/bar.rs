//! Underbar / overbar templates and horizontal braces.

use super::*;

/// Polyline of a horizontal brace of width `w` whose bar sits at `y0`.
pub(super) fn brace_h(w: f32, y0: f32, depth: f32, tip_down: bool) -> Vec<(f32, f32)> {
    let dir_sign = if tip_down { 1.0 } else { -1.0 };
    let half_depth = depth / 2.0;
    let mid_x = w / 2.0;
    let mut points = vec![(0.0, y0)];
    points.extend(quad_bezier(
        (0.0, y0),
        (0.0, y0 + dir_sign * half_depth),
        (half_depth, y0 + dir_sign * half_depth),
        6,
    ));
    points.push((mid_x - half_depth, y0 + dir_sign * half_depth));
    points.extend(quad_bezier(
        (mid_x - half_depth, y0 + dir_sign * half_depth),
        (mid_x, y0 + dir_sign * half_depth),
        (mid_x, y0 + dir_sign * depth),
        6,
    ));
    points.extend(quad_bezier(
        (mid_x, y0 + dir_sign * depth),
        (mid_x, y0 + dir_sign * half_depth),
        (mid_x + half_depth, y0 + dir_sign * half_depth),
        6,
    ));
    points.push((w - half_depth, y0 + dir_sign * half_depth));
    points.extend(quad_bezier(
        (w - half_depth, y0 + dir_sign * half_depth),
        (w, y0 + dir_sign * half_depth),
        (w, y0),
        6,
    ));
    points
}

impl Layout {
    /// Draw a horizontal brace under a base whose descent is `base_desc`.
    pub(super) fn draw_under_brace(&self, layout_box: &mut LBox, w: f32, base_desc: f32, em: f32) {
        let y0 = base_desc + 0.06 * em;
        let span = w.max(0.55 * em);
        let brace_x0 = (w - span) / 2.0;
        let depth = (span * 0.28).clamp(0.12 * em, 0.20 * em);
        let points: Vec<(f32, f32)> = brace_h(span, y0, depth, true)
            .into_iter()
            .map(|(x, y)| (x + brace_x0, y))
            .collect();
        for quad in stroke(&points, 0.045 * em) {
            layout_box.items.push(Item::Poly(quad));
        }
        layout_box.desc = layout_box.desc.max(y0 + depth + 0.04 * em);
    }
}
