//! Conversion between font outlines, drawing segments, and flattened polygons.

use ab_glyph_rasterizer::{point, Rasterizer};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::prelude::{LocationRef, Size};
use skrifa::{FontRef, GlyphId, MetadataProvider};

use super::super::canvas::Canvas;
use super::Seg;

struct SegPen {
    scale: f32,
    origin_x: f32,
    origin_y: f32,
    segments: Vec<Seg>,
    open: bool,
}

impl SegPen {
    fn new(scale: f32, origin_x: f32, origin_y: f32) -> Self {
        Self {
            scale,
            origin_x,
            origin_y,
            segments: Vec::new(),
            open: false,
        }
    }

    #[inline]
    fn map(&self, px: f32, py: f32) -> (f32, f32) {
        (
            self.origin_x + px * self.scale,
            self.origin_y - py * self.scale,
        )
    }
}

impl OutlinePen for SegPen {
    fn move_to(&mut self, x: f32, y: f32) {
        if self.open {
            self.segments.push(Seg::Close);
        }
        let (mx, my) = self.map(x, y);
        self.segments.push(Seg::Move(mx, my));
        self.open = true;
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let (lx, ly) = self.map(x, y);
        self.segments.push(Seg::Line(lx, ly));
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        let (cx, cy) = self.map(cx0, cy0);
        let (ex, ey) = self.map(x, y);
        self.segments.push(Seg::Quad(cx, cy, ex, ey));
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        let (c1x, c1y) = self.map(cx0, cy0);
        let (c2x, c2y) = self.map(cx1, cy1);
        let (ex, ey) = self.map(x, y);
        self.segments.push(Seg::Cubic(c1x, c1y, c2x, c2y, ex, ey));
    }

    fn close(&mut self) {
        if self.open {
            self.segments.push(Seg::Close);
            self.open = false;
        }
    }
}

/// Extract pixel-space outline segments for `id` with baseline origin `(x, y)`.
/// Returns `None` when the glyph has no drawable outline (e.g. a space).
pub(super) fn outline_to_segments(
    font: &FontRef<'static>,
    id: GlyphId,
    upem: f32,
    em_px: f32,
    x: f32,
    y: f32,
) -> Option<Vec<Seg>> {
    let outline = font.outline_glyphs().get(id)?;
    let mut pen = SegPen::new(em_px / upem, x, y);
    outline
        .draw(
            DrawSettings::unhinted(Size::unscaled(), LocationRef::default()),
            &mut pen,
        )
        .ok()?;
    if pen.open {
        pen.segments.push(Seg::Close);
    }
    if pen.segments.is_empty() {
        return None;
    }
    Some(pen.segments)
}

/// Compute `(min_x, min_y, max_x, max_y)` in pixel coordinates across all segment endpoints/controls.
pub(super) fn segments_bounds(segments: &[Seg]) -> Option<(f32, f32, f32, f32)> {
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    let mut any = false;

    let mut include = |px: f32, py: f32| {
        any = true;
        min_x = min_x.min(px);
        min_y = min_y.min(py);
        max_x = max_x.max(px);
        max_y = max_y.max(py);
    };

    for seg in segments {
        match *seg {
            Seg::Move(x, y) | Seg::Line(x, y) => include(x, y),
            Seg::Quad(cx, cy, ex, ey) => {
                include(cx, cy);
                include(ex, ey);
            }
            Seg::Cubic(c1x, c1y, c2x, c2y, ex, ey) => {
                include(c1x, c1y);
                include(c2x, c2y);
                include(ex, ey);
            }
            Seg::Close => {}
        }
    }

    any.then_some((min_x, min_y, max_x, max_y))
}

/// Rasterize pixel-space outline `segments` into `canvas` within integer pixel bounds.
pub(super) fn rasterize_segments(
    segments: &[Seg],
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
    canvas: &mut Canvas,
) {
    let bx = min_x.floor() as i64;
    let by = min_y.floor() as i64;
    let width = (max_x.ceil() as i64 - bx).max(0) as usize;
    let height = (max_y.ceil() as i64 - by).max(0) as usize;
    if width == 0 || height == 0 {
        return;
    }

    let mut rasterizer = Rasterizer::new(width, height);
    let ox = bx as f32;
    let oy = by as f32;
    let shift = |px: f32, py: f32| point(px - ox, py - oy);

    let mut start = (0.0f32, 0.0f32);
    let mut cur = (0.0f32, 0.0f32);

    for seg in segments {
        match *seg {
            Seg::Move(mx, my) => {
                start = (mx, my);
                cur = start;
            }
            Seg::Line(lx, ly) => {
                rasterizer.draw_line(shift(cur.0, cur.1), shift(lx, ly));
                cur = (lx, ly);
            }
            Seg::Quad(cx, cy, ex, ey) => {
                rasterizer.draw_quad(shift(cur.0, cur.1), shift(cx, cy), shift(ex, ey));
                cur = (ex, ey);
            }
            Seg::Cubic(c1x, c1y, c2x, c2y, ex, ey) => {
                rasterizer.draw_cubic(
                    shift(cur.0, cur.1),
                    shift(c1x, c1y),
                    shift(c2x, c2y),
                    shift(ex, ey),
                );
                cur = (ex, ey);
            }
            Seg::Close => {
                if cur != start {
                    rasterizer.draw_line(shift(cur.0, cur.1), shift(start.0, start.1));
                    cur = start;
                }
            }
        }
    }

    rasterizer.for_each_pixel_2d(|gx, gy, c| canvas.add(bx + gx as i64, by + gy as i64, c));
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
