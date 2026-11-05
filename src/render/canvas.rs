//! A grey-scale coverage canvas that glyphs and polygons are drawn onto.

/// A grey-scale coverage canvas (0.0 – 1.0 per pixel).
pub struct Canvas {
    /// Width in pixels.
    pub w: usize,
    /// Height in pixels.
    pub h: usize,
    /// Row-major coverage.
    pub cov: Vec<f32>,
}

impl Canvas {
    /// A blank canvas.
    pub fn new(w: usize, h: usize) -> Self {
        Canvas { w, h, cov: vec![0.0; w * h] }
    }

    /// Add coverage (clamped to 1) at a pixel; out-of-range pixels are ignored.
    pub fn add(&mut self, x: i64, y: i64, c: f32) {
        if x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h {
            let p = &mut self.cov[y as usize * self.w + x as usize];
            *p = (*p + c).min(1.0);
        }
    }
}
