//! Vector SVG renderer for MathML equation previews.

use std::fmt::Write;

use super::font::{fonts, Seg};
use super::layout::{self, Item};
use super::PX_PER_PT;
use crate::error::Result;
use crate::mathml::MathNode;

/// A rendered equation as a standalone vector SVG string and its size/baseline in points.
#[derive(Debug, Clone)]
pub struct SvgPreview {
    /// Standalone SVG XML using `fill="currentColor"` so callers or themes can style it.
    pub svg: String,
    /// Image width in points.
    pub width_pt: f32,
    /// Image height in points.
    pub height_pt: f32,
    /// Distance from the baseline down to the bottom of the image, in points.
    pub depth_pt: f32,
}

/// Render `math` (a `<math>` tree) at `font_pt` points per em as vector SVG.
pub fn render_svg(math: &MathNode, font_pt: f32) -> Result<SvgPreview> {
    let (b, em) = layout::layout_math(math, font_pt)?;
    let f = fonts();
    let (min_x, max_x, min_y, max_y) = b.ink_bounds(f);
    let asc = (-min_y).max(0.70 * em);
    let desc = max_y.max(0.22 * em);
    let left_extra = (-min_x).max(0.0);
    let right_extra = (max_x - b.w).max(0.0);
    let (pad_x, pad_y) = (2.0f32, 2.0f32);
    let w_px = (b.w + left_extra + right_extra + 2.0 * pad_x).max(1.0);
    let h_px = (asc + desc + 2.0 * pad_y).max(1.0);
    let w_pt = w_px / PX_PER_PT;
    let h_pt = h_px / PX_PER_PT;
    let depth_pt = (desc + pad_y) / PX_PER_PT;
    let (ox, oy) = (pad_x + left_extra, pad_y + asc);

    let mut paths = String::new();
    for it in &b.items {
        match it {
            Item::Glyph {
                ch,
                face,
                em,
                x,
                y,
                ssty,
            } => {
                if let Some(segs) = f.glyph_segments_styled(*face, *ch, *em, ox + x, oy + y, *ssty)
                {
                    write_segs_path(&mut paths, &segs);
                }
            }
            Item::Poly(pts) => {
                if pts.len() >= 3 {
                    paths.push_str("  <path d=\"");
                    append_poly_subpath(&mut paths, pts, ox, oy);
                    paths.push_str("\"/>\n");
                }
            }
            Item::Contours(contours) => {
                let valid: Vec<_> = contours.iter().filter(|pts| pts.len() >= 3).collect();
                if !valid.is_empty() {
                    paths.push_str("  <path d=\"");
                    for pts in valid {
                        append_poly_subpath(&mut paths, pts, ox, oy);
                    }
                    paths.push_str("\"/>\n");
                }
            }
        }
    }

    let svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w_pt:.3}pt\" height=\"{h_pt:.3}pt\" viewBox=\"0 0 {w_px:.2} {h_px:.2}\">\n<g fill=\"currentColor\">\n{paths}</g>\n</svg>\n"
    );

    Ok(SvgPreview {
        svg,
        width_pt: w_pt,
        height_pt: h_pt,
        depth_pt,
    })
}

fn write_segs_path(out: &mut String, segs: &[Seg]) {
    if segs.is_empty() {
        return;
    }
    out.push_str("  <path d=\"");
    for (idx, seg) in segs.iter().enumerate() {
        if idx > 0 {
            out.push(' ');
        }
        match *seg {
            Seg::Move(x, y) => {
                let _ = write!(out, "M{x:.2} {y:.2}");
            }
            Seg::Line(x, y) => {
                let _ = write!(out, "L{x:.2} {y:.2}");
            }
            Seg::Quad(cx, cy, x, y) => {
                let _ = write!(out, "Q{cx:.2} {cy:.2} {x:.2} {y:.2}");
            }
            Seg::Cubic(c1x, c1y, c2x, c2y, x, y) => {
                let _ = write!(out, "C{c1x:.2} {c1y:.2} {c2x:.2} {c2y:.2} {x:.2} {y:.2}");
            }
            Seg::Close => out.push('Z'),
        }
    }
    out.push_str("\"/>\n");
}

fn append_poly_subpath(out: &mut String, pts: &[(f32, f32)], ox: f32, oy: f32) {
    for (idx, (x, y)) in pts.iter().enumerate() {
        let px = ox + x;
        let py = oy + y;
        if idx == 0 {
            let _ = write!(out, "M{px:.2} {py:.2}");
        } else {
            let _ = write!(out, " L{px:.2} {py:.2}");
        }
    }
    out.push('Z');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_fraction_and_integral_to_vector_svg() {
        let math = MathNode::parse(
            "<math><mrow><mo>∫</mo><mfrac><mi>x</mi><mn>2</mn></mfrac></mrow></math>",
        )
        .unwrap();
        let preview = render_svg(&math, 12.0).unwrap();
        assert!(preview.svg.starts_with("<svg "));
        assert!(preview.svg.contains("fill=\"currentColor\""));
        assert!(preview.svg.contains("<path d=\""));
        assert!(preview.width_pt > 0.0 && preview.height_pt > 0.0);
    }
}
