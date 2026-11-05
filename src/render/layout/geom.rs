//! Small geometry helpers: rectangles, strokes and Bézier curves.

pub(super) fn rect(x: f32, y: f32, w: f32, h: f32) -> Vec<(f32, f32)> {
    vec![(x, y), (x + w, y), (x + w, y + h), (x, y + h)]
}

/// Quads along a polyline (square caps), at least one pixel thick.
pub(super) fn stroke(pts: &[(f32, f32)], width: f32) -> Vec<Vec<(f32, f32)>> {
    let w = width.max(1.0);
    let mut out = Vec::new();
    for s in pts.windows(2) {
        let ((ax, ay), (bx, by)) = (s[0], s[1]);
        let (dx, dy) = (bx - ax, by - ay);
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-4 {
            continue;
        }
        let (ux, uy) = (dx / len, dy / len);
        let (nx, ny) = (-uy * w / 2.0, ux * w / 2.0);
        let (ex, ey) = (ux * w / 2.0, uy * w / 2.0);
        let (a0, a1) = (ax - ex, ay - ey);
        let (b0, b1) = (bx + ex, by + ey);
        out.push(vec![(a0 + nx, a1 + ny), (b0 + nx, b1 + ny), (b0 - nx, b1 - ny), (a0 - nx, a1 - ny)]);
    }
    out
}

pub(super) fn quad_bezier(p0: (f32, f32), c: (f32, f32), p1: (f32, f32), n: usize) -> Vec<(f32, f32)> {
    (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let u = 1.0 - t;
            (
                u * u * p0.0 + 2.0 * u * t * c.0 + t * t * p1.0,
                u * u * p0.1 + 2.0 * u * t * c.1 + t * t * p1.1,
            )
        })
        .collect()
}
