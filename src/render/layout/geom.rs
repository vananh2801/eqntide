//! Small geometry helpers: rectangles, strokes, Bézier curves, and polygon clipping.

pub(super) fn rect(x: f32, y: f32, w: f32, h: f32) -> Vec<(f32, f32)> {
    vec![(x, y), (x + w, y), (x + w, y + h), (x, y + h)]
}

/// Quads along a polyline (square caps), at least one pixel thick.
pub(super) fn stroke(points: &[(f32, f32)], width: f32) -> Vec<Vec<(f32, f32)>> {
    let stroke_w = width.max(1.0);
    let mut out = Vec::new();
    for segment in points.windows(2) {
        let ((ax, ay), (bx, by)) = (segment[0], segment[1]);
        let (dx, dy) = (bx - ax, by - ay);
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-4 {
            continue;
        }
        let (ux, uy) = (dx / len, dy / len);
        let (nx, ny) = (-uy * stroke_w / 2.0, ux * stroke_w / 2.0);
        let (ex, ey) = (ux * stroke_w / 2.0, uy * stroke_w / 2.0);
        let (a0, a1) = (ax - ex, ay - ey);
        let (b0, b1) = (bx + ex, by + ey);
        out.push(vec![
            (a0 + nx, a1 + ny),
            (b0 + nx, b1 + ny),
            (b0 - nx, b1 - ny),
            (a0 - nx, a1 - ny),
        ]);
    }
    out
}

pub(super) fn quad_bezier(
    start: (f32, f32),
    control: (f32, f32),
    end: (f32, f32),
    steps: usize,
) -> Vec<(f32, f32)> {
    (0..=steps)
        .map(|i| {
            let t = i as f32 / steps as f32;
            let inv_t = 1.0 - t;
            (
                inv_t * inv_t * start.0 + 2.0 * inv_t * t * control.0 + t * t * end.0,
                inv_t * inv_t * start.1 + 2.0 * inv_t * t * control.1 + t * t * end.1,
            )
        })
        .collect()
}

/// Clip a polygon to the horizontal band `y_min <= y <= y_max` (Sutherland–Hodgman
/// against two half-planes). Works for any simple contour, convex or not; the result
/// may be empty (fewer than 3 points) when the contour lies outside the band.
pub(super) fn clip_poly_y(poly: &[(f32, f32)], y_min: f32, y_max: f32) -> Vec<(f32, f32)> {
    if poly.len() < 3 {
        return Vec::new();
    }
    let clipped = clip_half_plane_y(poly, y_min, |y| y >= y_min);
    if clipped.len() < 3 {
        return Vec::new();
    }
    clip_half_plane_y(&clipped, y_max, |y| y <= y_max)
}

fn clip_half_plane_y(
    points: &[(f32, f32)],
    edge_y: f32,
    keep: impl Fn(f32) -> bool,
) -> Vec<(f32, f32)> {
    let count = points.len();
    let mut out = Vec::with_capacity(count + 2);
    for i in 0..count {
        let (curr, next) = (points[i], points[(i + 1) % count]);
        let (keep_curr, keep_next) = (keep(curr.1), keep(next.1));
        if keep_curr {
            out.push(curr);
        }
        if keep_curr != keep_next {
            let t = (edge_y - curr.1) / (next.1 - curr.1);
            out.push((curr.0 + t * (next.0 - curr.0), edge_y));
        }
    }
    out
}
