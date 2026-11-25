//! Small geometry helpers: rectangles, strokes, Bézier curves, and polygon clipping.

/// Axis-aligned rectangle polygon with top-left corner `(x, y)` and size `(w, h)`.
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

/// Sample a quadratic Bézier curve from `start` through `control` to `end`.
pub(super) fn quad_bezier(
    start: (f32, f32),
    control: (f32, f32),
    end: (f32, f32),
    steps: usize,
) -> Vec<(f32, f32)> {
    (0..=steps)
        .map(|i| {
            let ratio = i as f32 / steps as f32;
            let inv_ratio = 1.0 - ratio;
            (
                inv_ratio * inv_ratio * start.0
                    + 2.0 * inv_ratio * ratio * control.0
                    + ratio * ratio * end.0,
                inv_ratio * inv_ratio * start.1
                    + 2.0 * inv_ratio * ratio * control.1
                    + ratio * ratio * end.1,
            )
        })
        .collect()
}

/// Clip a polygon to the vertical strip `x_min <= x <= x_max` (Sutherland–Hodgman
/// against two half-planes). Works for any simple contour, convex or not; the result
/// may be empty (fewer than 3 points) when the contour lies outside the strip.
pub(super) fn clip_poly_x(poly: &[(f32, f32)], x_min: f32, x_max: f32) -> Vec<(f32, f32)> {
    if poly.len() < 3 {
        return Vec::new();
    }
    let clipped = clip_half_plane_x(poly, x_min, |x| x >= x_min);
    if clipped.len() < 3 {
        return Vec::new();
    }
    clip_half_plane_x(&clipped, x_max, |x| x <= x_max)
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

fn clip_half_plane_x(
    points: &[(f32, f32)],
    edge_x: f32,
    keep: impl Fn(f32) -> bool,
) -> Vec<(f32, f32)> {
    let count = points.len();
    let mut out = Vec::with_capacity(count + 2);
    for i in 0..count {
        let (curr, next) = (points[i], points[(i + 1) % count]);
        let (keep_curr, keep_next) = (keep(curr.0), keep(next.0));
        if keep_curr {
            out.push(curr);
        }
        if keep_curr != keep_next {
            let ratio = (edge_x - curr.0) / (next.0 - curr.0);
            out.push((edge_x, curr.1 + ratio * (next.1 - curr.1)));
        }
    }
    out
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
            let ratio = (edge_y - curr.1) / (next.1 - curr.1);
            out.push((curr.0 + ratio * (next.0 - curr.0), edge_y));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_poly_x_handles_inside_crossing_and_outside_contours() {
        let inside = rect(2.0, 0.0, 4.0, 3.0);
        let clipped_inside = clip_poly_x(&inside, 1.0, 7.0);
        assert_eq!(clipped_inside, inside);

        let crossing = vec![(0.0, 0.0), (10.0, 0.0), (10.0, 4.0), (0.0, 4.0)];
        let clipped_crossing = clip_poly_x(&crossing, 2.5, 7.5);
        assert_eq!(clipped_crossing.len(), 4);
        let (mut min_x, mut max_x) = (f32::MAX, f32::MIN);
        for &(x, _) in &clipped_crossing {
            min_x = min_x.min(x);
            max_x = max_x.max(x);
        }
        assert!((min_x - 2.5).abs() < 1e-4);
        assert!((max_x - 7.5).abs() < 1e-4);

        let outside_left = rect(-10.0, 0.0, 4.0, 2.0);
        assert!(clip_poly_x(&outside_left, 0.0, 5.0).is_empty());

        let outside_right = rect(8.0, 0.0, 4.0, 2.0);
        assert!(clip_poly_x(&outside_right, 0.0, 5.0).is_empty());
    }
}
