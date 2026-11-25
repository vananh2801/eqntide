//! A grey-scale coverage canvas that glyphs and polygons are drawn onto.

/// How incoming coverage combines with existing pixel coverage on a [`Canvas`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlendMode {
    /// Accumulate clamped sum (`(dst + src).min(1.0)`), used at 1× resolution.
    Add,
    /// Take per-pixel maximum (`dst.max(src)`), used on supersampled union grids so
    /// overlapping glyph pieces do not double-count edge anti-aliasing.
    Union,
}

/// A grey-scale coverage canvas (0.0 – 1.0 per pixel).
pub struct Canvas {
    /// Width in pixels.
    pub w: usize,
    /// Height in pixels.
    pub h: usize,
    /// Row-major coverage.
    pub cov: Vec<f32>,
    mode: BlendMode,
}

impl Canvas {
    /// A blank additive canvas (`Add` blend mode).
    pub fn new(w: usize, h: usize) -> Self {
        Canvas {
            w,
            h,
            cov: vec![0.0; w * h],
            mode: BlendMode::Add,
        }
    }

    /// A blank union canvas (`Union` blend mode) for supersampled rasterization.
    pub fn new_union(w: usize, h: usize) -> Self {
        Canvas {
            w,
            h,
            cov: vec![0.0; w * h],
            mode: BlendMode::Union,
        }
    }

    /// Combine coverage at a pixel according to this canvas's blend mode; out-of-range
    /// pixels are ignored.
    pub fn add(&mut self, x: i64, y: i64, c: f32) {
        if x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h {
            let slot = &mut self.cov[y as usize * self.w + x as usize];
            *slot = match self.mode {
                BlendMode::Add => (*slot + c).min(1.0),
                BlendMode::Union => slot.max(c.clamp(0.0, 1.0)),
            };
        }
    }
}
