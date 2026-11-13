//! Draws a MathML tree as a transparent PNG.
//!
//! Word and WPS show an embedded OLE object through a cached *picture*; without
//! one the object is an empty frame. This module produces that picture with a
//! small TeX-style box layout engine (fractions, radicals, scripts, stretchy
//! delimiters, big operators, matrices, accents) on top of the embedded STIX Two
//! fonts, following MathType's style conventions.
//!
//! * `layout` – box layout, one module per MathType palette group
//! * [`font`]   – font loading and glyph metrics / drawing
//! * [`canvas`] – coverage canvas the glyphs are drawn onto
//! * [`emf`]    – the same picture as vector EMF (what Word/WPS prefer for OLE previews)
//! * [`png`]    – PNG encoder for the alpha mask

pub mod canvas;
pub mod emf;
pub mod font;
pub(crate) mod layout;
pub mod png;

use ab_glyph_rasterizer::{point, Rasterizer};

use crate::error::{Error, Result};
use crate::mathml::MathNode;
use canvas::Canvas;
use font::fonts;
use layout::Item;

/// Pixels per typographic point in the generated image (288 dpi).
pub const PX_PER_PT: f32 = 8.0;

const MAX_PIXELS: usize = 6_000_000;
const MAX_WIDTH_PX: usize = 20_000;

/// A rendered equation picture and its size/baseline in points.
#[derive(Debug, Clone)]
pub struct Preview {
    /// PNG bytes (black on transparent).
    pub png: Vec<u8>,
    /// Image width in points.
    pub width_pt: f32,
    /// Image height in points.
    pub height_pt: f32,
    /// Distance from the baseline down to the bottom of the image, in points.
    pub depth_pt: f32,
}

/// Size of the picture [`render_preview`] would produce, in points:
/// `(width, ascent, descent)` of the equation box (without padding).
pub fn measure(math: &MathNode, font_pt: f32) -> Result<(f32, f32, f32)> {
    let (b, _) = layout::layout_math(math, font_pt)?;
    Ok((b.w / PX_PER_PT, b.asc / PX_PER_PT, b.desc / PX_PER_PT))
}

/// Draw `math` (a `<math>` tree) at `font_pt` points per em.
pub fn render_preview(math: &MathNode, font_pt: f32) -> Result<Preview> {
    let (b, em) = layout::layout_math(math, font_pt)?;
    let f = fonts();
    let (min_x, max_x, min_y, max_y) = b.ink_bounds(f);
    let asc = (-min_y).max(0.70 * em);
    let desc = max_y.max(0.22 * em);
    let left_extra = (-min_x).max(0.0);
    let right_extra = (max_x - b.w).max(0.0);
    let (pad_x, pad_y) = (2.0f32, 2.0f32);
    let w_px = (b.w + left_extra + right_extra + 2.0 * pad_x)
        .ceil()
        .max(1.0) as usize;
    let h_px = (asc + desc + 2.0 * pad_y).ceil().max(1.0) as usize;
    if w_px > MAX_WIDTH_PX || w_px * h_px > MAX_PIXELS {
        return Err(Error::PreviewTooLarge {
            width: w_px,
            height: h_px,
        });
    }

    let (ox, oy) = (pad_x + left_extra, pad_y + asc);
    let mut canvas = Canvas::new(w_px, h_px);
    let mut ras = Rasterizer::new(w_px, h_px);
    for it in &b.items {
        match it {
            Item::Glyph {
                ch,
                face,
                em,
                x,
                y,
                ssty,
            } => f.draw_styled(*face, *ch, *em, ox + x, oy + y, &mut canvas, *ssty),
            Item::Poly(pts) => {
                let mut p: Vec<(f32, f32)> = pts.iter().map(|(x, y)| (ox + x, oy + y)).collect();
                let area: f32 = (0..p.len())
                    .map(|i| {
                        let (a, c) = (p[i], p[(i + 1) % p.len()]);
                        a.0 * c.1 - c.0 * a.1
                    })
                    .sum();
                if area < 0.0 {
                    p.reverse();
                }
                for i in 0..p.len() {
                    let (a, c) = (p[i], p[(i + 1) % p.len()]);
                    ras.draw_line(point(a.0, a.1), point(c.0, c.1));
                }
            }
        }
    }
    ras.for_each_pixel(|i, v| {
        if let Some(c) = canvas.cov.get_mut(i) {
            *c = (*c + v).min(1.0);
        }
    });

    let alpha: Vec<u8> = canvas.cov.iter().map(|c| (c * 255.0 + 0.5) as u8).collect();
    let ppm = (PX_PER_PT * 72.0 / 0.0254).round() as u32;
    Ok(Preview {
        png: png::encode_alpha(w_px as u32, h_px as u32, &alpha, ppm),
        width_pt: w_px as f32 / PX_PER_PT,
        height_pt: h_px as f32 / PX_PER_PT,
        depth_pt: (desc + pad_y) / PX_PER_PT,
    })
}
