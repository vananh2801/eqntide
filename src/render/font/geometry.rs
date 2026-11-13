//! Conversion between font outlines, drawing segments, and flattened polygons.

use super::Seg;

pub(super) fn outline_to_segments(
    outline: &ab_glyph::Outline,
    upem: f32,
    em_px: f32,
    x: f32,
    y: f32,
) -> Vec<Seg> {
    use ab_glyph::OutlineCurve as Curve;

    let scale = em_px / upem;
    let point = |point: ab_glyph::Point| (x + point.x * scale, y - point.y * scale);
    let mut segments = Vec::with_capacity(outline.curves.len() + 8);
    let mut pen = None;

    for curve in &outline.curves {
        let start = match curve {
            Curve::Line(start, _) | Curve::Quad(start, _, _) | Curve::Cubic(start, _, _, _) => {
                point(*start)
            }
        };
        if pen != Some(start) {
            if pen.is_some() {
                segments.push(Seg::Close);
            }
            segments.push(Seg::Move(start.0, start.1));
        }
        pen = Some(match curve {
            Curve::Line(_, end) => {
                let end = point(*end);
                segments.push(Seg::Line(end.0, end.1));
                end
            }
            Curve::Quad(_, control, end) => {
                let (control, end) = (point(*control), point(*end));
                segments.push(Seg::Quad(control.0, control.1, end.0, end.1));
                end
            }
            Curve::Cubic(_, control1, control2, end) => {
                let (control1, control2, end) = (point(*control1), point(*control2), point(*end));
                segments.push(Seg::Cubic(
                    control1.0, control1.1, control2.0, control2.1, end.0, end.1,
                ));
                end
            }
        });
    }
    if pen.is_some() {
        segments.push(Seg::Close);
    }
    segments
}

pub(super) fn segments_to_polys(
    segments: &[Seg],
    em_x: f32,
    em_y: f32,
    x: f32,
    y: f32,
) -> Vec<Vec<(f32, f32)>> {
    const STEPS: usize = 12;

    let horizontal_scale = if em_y != 0.0 { em_x / em_y } else { 1.0 };
    let map = |px: f32, py: f32| (x + px * horizontal_scale, y + py);
    let mut polygons = Vec::new();
    let mut contour = Vec::new();
    let mut pen = (0.0, 0.0);

    for segment in segments {
        match segment {
            Seg::Move(x, y) => {
                if contour.len() >= 3 {
                    polygons.push(std::mem::take(&mut contour));
                }
                contour.clear();
                pen = (*x, *y);
                contour.push(map(*x, *y));
            }
            Seg::Line(x, y) => {
                pen = (*x, *y);
                contour.push(map(*x, *y));
            }
            Seg::Quad(cx, cy, ex, ey) => {
                let (sx, sy) = pen;
                for step in 1..=STEPS {
                    let t = step as f32 / STEPS as f32;
                    let inverse_t = 1.0 - t;
                    contour.push(map(
                        inverse_t * inverse_t * sx + 2.0 * inverse_t * t * cx + t * t * ex,
                        inverse_t * inverse_t * sy + 2.0 * inverse_t * t * cy + t * t * ey,
                    ));
                }
                pen = (*ex, *ey);
            }
            Seg::Cubic(c1x, c1y, c2x, c2y, ex, ey) => {
                let (sx, sy) = pen;
                for step in 1..=STEPS {
                    let t = step as f32 / STEPS as f32;
                    let inverse_t = 1.0 - t;
                    let (b0, b1, b2, b3) = (
                        inverse_t * inverse_t * inverse_t,
                        3.0 * inverse_t * inverse_t * t,
                        3.0 * inverse_t * t * t,
                        t * t * t,
                    );
                    contour.push(map(
                        b0 * sx + b1 * c1x + b2 * c2x + b3 * ex,
                        b0 * sy + b1 * c1y + b2 * c2y + b3 * ey,
                    ));
                }
                pen = (*ex, *ey);
            }
            Seg::Close => {
                if contour.len() >= 3 {
                    polygons.push(std::mem::take(&mut contour));
                }
                contour.clear();
            }
        }
    }
    if contour.len() >= 3 {
        polygons.push(contour);
    }
    polygons
}
