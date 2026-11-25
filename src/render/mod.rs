//! Draws a MathML tree into raster alpha masks, PNG previews, EMF metafiles, and SVG.
//!
//! Word and WPS show an embedded OLE object through a cached *picture*; without
//! one the object is an empty frame. This module drives a TeX-style box layout
//! engine over embedded STIX Two fonts, following MathType's style conventions:
//!
//! ```text
//! MathNode (&str/XML tree)
//!   └─► layout::layout_math(math, font_pt) -> (LBox, em_px)
//!         ├─► render_preview / rasterize       -> Canvas coverage -> PNG (`png`)
//!         ├─► render_alpha_{fit,at}            -> scaled `AlphaMask` (UI bitmaps)
//!         ├─► render_interactive[_alpha_scaled]-> `InteractivePreview` (`HitRegion` + mask)
//!         ├─► emf::render_emf[_with_mtef]      -> vector EMF (+ optional MTEF record)
//!         └─► svg::render_svg                  -> vector SVG
//! ```
//!
//! * `layout` – box layout, one module per MathType palette group
//! * [`font`]   – font loading and glyph metrics / drawing
//! * [`canvas`] – coverage canvas the glyphs are drawn onto
//! * [`emf`]    – the same picture as vector EMF (what Word/WPS prefer for OLE previews)
//! * [`png`]    – PNG encoder for the alpha mask
//! * [`svg`]    – vector SVG export

pub mod canvas;
pub mod emf;
pub mod font;
pub(crate) mod layout;
pub mod png;
pub mod svg;

use ab_glyph_rasterizer::{point, Rasterizer};

use crate::error::{Error, Result};
use crate::mathml::MathNode;
use canvas::Canvas;
use font::fonts;
use layout::{Item, LBox};

/// Pixels per typographic point in the generated image (288 dpi).
pub const PX_PER_PT: f32 = 8.0;

const MAX_PIXELS: usize = 6_000_000;
const MAX_WIDTH_PX: usize = 20_000;
pub(crate) const MIN_FONT_PT: f32 = 1.0;
pub(crate) const MAX_FONT_PT: f32 = 200.0;

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

/// Coverage alpha mask rasterized directly at an exact pixel budget.
#[derive(Debug, Clone)]
pub struct AlphaMask {
    /// Pixel width of the alpha buffer.
    pub width: u32,
    /// Pixel height of the alpha buffer.
    pub height: u32,
    /// Row-major 8-bit alpha coverage values.
    pub alpha: Vec<u8>,
}

/// Region in typographic points, relative to the preview image's top-left.
#[derive(Clone, Debug)]
pub struct HitRegion {
    /// Candidate token caret positions on the image x axis, in points.
    pub carets: Vec<f32>,
    /// Child-index path from the `<math>` root to the target [`MathNode`].
    pub path: Vec<usize>,
    /// Left edge in points relative to the preview image.
    pub x: f32,
    /// Top edge in points relative to the preview image.
    pub y: f32,
    /// Bounding box width in points.
    pub width: f32,
    /// Bounding box height in points.
    pub height: f32,
    /// Baseline y coordinate in points from the top of the preview image.
    pub baseline: f32,
}

/// Combined rendered equation preview and structural hit-testing geometry.
#[derive(Clone, Debug)]
pub struct InteractivePreview {
    /// Rendered PNG image and point dimensions.
    pub preview: Preview,
    /// Ordered interactive hit regions for AST nodes in the equation.
    pub regions: Vec<HitRegion>,
}

/// Size of the picture [`render_preview`] would produce, in points:
/// `(width, ascent, descent)` of the equation box (without padding).
pub fn measure(math: &MathNode, font_pt: f32) -> Result<(f32, f32, f32)> {
    let (layout_box, _) = layout::layout_math(math, font_pt)?;
    Ok((
        layout_box.w / PX_PER_PT,
        layout_box.asc / PX_PER_PT,
        layout_box.desc / PX_PER_PT,
    ))
}

/// Draw `math` (a `<math>` tree) at `font_pt` points per em.
pub fn render_preview(math: &MathNode, font_pt: f32) -> Result<Preview> {
    let (layout_box, em) = layout::layout_math(math, font_pt)?;
    rasterize(&layout_box, em)
}

/// Lay out `math` at standard proportions and scale its vector geometry directly
/// to fit `(max_w_px, max_h_px)` in exact device pixels (1 texel = 1 pixel).
pub fn render_alpha_fit(math: &MathNode, max_w_px: f32, max_h_px: f32) -> Result<AlphaMask> {
    render_alpha_scaled(math, None, max_w_px, max_h_px)
}

/// Lay out `math` at `target_em_px` pixels per em, scaling down only when its
/// bounds exceed `(max_w_px, max_h_px)`.
pub fn render_alpha_at(
    math: &MathNode,
    target_em_px: f32,
    max_w_px: f32,
    max_h_px: f32,
) -> Result<AlphaMask> {
    render_alpha_scaled(math, Some(target_em_px), max_w_px, max_h_px)
}

/// Raster and hit geometry are generated from the same layout, returning the raw alpha mask
/// directly without PNG Zlib compression overhead. `raster_scale` scales output pixel dimensions
/// (1.0 = full `PX_PER_PT` 576 DPI; smaller values rasterize fewer pixels for software renderers)
/// while keeping `Preview` point dimensions and `HitRegion` coordinates in standard points.
pub fn render_interactive_alpha_scaled(
    math: &MathNode,
    font_pt: f32,
    raster_scale: f32,
) -> Result<(AlphaMask, InteractivePreview)> {
    let (mut layout_box, em) = layout::layout_math(math, font_pt)?;
    let font_set = fonts();
    let (min_x, max_x, min_y, max_y) = layout_box.ink_bounds(font_set);
    let asc = (-min_y).max(0.70 * em);
    let desc = max_y.max(0.22 * em);
    let left_extra = (-min_x).max(0.0);
    let right_extra = (max_x - layout_box.w).max(0.0);
    let (pad_x, pad_y) = (2.0f32, 2.0f32);
    let full_w_px = (layout_box.w + left_extra + right_extra + 2.0 * pad_x)
        .ceil()
        .max(1.0);
    let full_h_px = (asc + desc + 2.0 * pad_y).ceil().max(1.0);
    let origin_x = pad_x + left_extra;
    let origin_y = pad_y + asc;

    let mut paths = std::collections::HashMap::new();
    collect_node_paths(math, &mut vec![], &mut paths);

    let mut regions: Vec<HitRegion> = vec![];
    for hit in &layout_box.hits {
        let Some(path) = paths.get(&hit.key) else {
            continue;
        };
        let x = (origin_x + hit.x) / PX_PER_PT;
        let y = (origin_y + hit.y - hit.asc) / PX_PER_PT;
        let right = x + hit.w / PX_PER_PT;
        let bottom = y + (hit.asc + hit.desc) / PX_PER_PT;
        if let Some(region) = regions.iter_mut().find(|existing| &existing.path == path) {
            let max_right = (region.x + region.width).max(right);
            let max_bottom = (region.y + region.height).max(bottom);
            region.x = region.x.min(x);
            region.y = region.y.min(y);
            region.width = max_right - region.x;
            region.height = max_bottom - region.y;
            region.carets.extend(
                hit.carets
                    .iter()
                    .map(|caret| (origin_x + hit.x + caret) / PX_PER_PT),
            );
        } else {
            regions.push(HitRegion {
                carets: hit
                    .carets
                    .iter()
                    .map(|caret| (origin_x + hit.x + caret) / PX_PER_PT)
                    .collect(),
                path: path.clone(),
                x,
                y,
                width: right - x,
                height: bottom - y,
                baseline: (origin_y + hit.y) / PX_PER_PT,
            });
        }
    }
    regions.push(HitRegion {
        carets: vec![],
        path: vec![],
        x: origin_x / PX_PER_PT,
        y: (origin_y - layout_box.asc) / PX_PER_PT,
        width: layout_box.w / PX_PER_PT,
        height: (layout_box.asc + layout_box.desc) / PX_PER_PT,
        baseline: origin_y / PX_PER_PT,
    });
    for region in &mut regions {
        let is_text_positions = region
            .path
            .iter()
            .try_fold(math, |node, &index| node.children.get(index))
            .is_some_and(|node| {
                node.tag == "mtext" && region.carets.len() == node.text.chars().count() + 1
            });
        // Text offsets index the original characters, including zero-width combining marks.
        if !is_text_positions {
            region.carets.sort_by(f32::total_cmp);
            region
                .carets
                .dedup_by(|left, right| (*left - *right).abs() < 0.001);
        }
    }

    let scale = raster_scale.clamp(0.25, 1.0);
    let (w_px, h_px, alpha) = if (scale - 1.0).abs() > 1e-4 {
        scale_layout_box(&mut layout_box, scale);
        rasterize_alpha(
            &layout_box,
            pad_x * scale,
            pad_y * scale,
            left_extra * scale,
            right_extra * scale,
            asc * scale,
            desc * scale,
        )?
    } else {
        rasterize_alpha(
            &layout_box,
            pad_x,
            pad_y,
            left_extra,
            right_extra,
            asc,
            desc,
        )?
    };

    let preview = Preview {
        png: Vec::new(),
        width_pt: full_w_px / PX_PER_PT,
        height_pt: full_h_px / PX_PER_PT,
        depth_pt: (desc + pad_y) / PX_PER_PT,
    };
    Ok((
        AlphaMask {
            width: w_px as u32,
            height: h_px as u32,
            alpha,
        },
        InteractivePreview { preview, regions },
    ))
}

/// Raster and hit geometry are generated from exactly the same layout.
/// Metadata is separate from drawing primitives and cannot change EMF/PNG output.
pub fn render_interactive(math: &MathNode, font_pt: f32) -> Result<InteractivePreview> {
    let (mask, mut interactive) = render_interactive_alpha_scaled(math, font_pt, 1.0)?;
    let ppm = (PX_PER_PT * 72.0 / 0.0254).round() as u32;
    interactive.preview.png = png::encode_alpha(mask.width, mask.height, &mask.alpha, ppm);
    Ok(interactive)
}

/// Validate that `font_pt` is finite and within `[MIN_FONT_PT, MAX_FONT_PT]`.
pub(crate) fn validate_font_pt(font_pt: f32) -> Result<f32> {
    if font_pt.is_finite() && (MIN_FONT_PT..=MAX_FONT_PT).contains(&font_pt) {
        Ok(font_pt)
    } else {
        Err(Error::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("font_pt must be finite and between {MIN_FONT_PT} and {MAX_FONT_PT}"),
        )))
    }
}

fn render_alpha_scaled(
    math: &MathNode,
    target_em_px: Option<f32>,
    max_w_px: f32,
    max_h_px: f32,
) -> Result<AlphaMask> {
    let (mut layout_box, em) = layout::layout_math(math, 12.0)?;
    let font_set = fonts();
    let (min_x, max_x, min_y, max_y) = layout_box.ink_bounds(font_set);
    let asc = (-min_y).max(0.70 * em);
    let desc = max_y.max(0.22 * em);
    let left_extra = (-min_x).max(0.0);
    let right_extra = (max_x - layout_box.w).max(0.0);
    let box_w = (layout_box.w + left_extra + right_extra).max(1.0);
    let box_h = (asc + desc).max(1.0);
    let pad = 1.0f32;
    let avail_w = (max_w_px - 2.0 * pad - 0.01).max(1.0);
    let avail_h = (max_h_px - 2.0 * pad - 0.01).max(1.0);
    let fit_scale = (avail_w / box_w).min(avail_h / box_h);
    let scale = match target_em_px {
        Some(target) if target.is_finite() && target > 0.0 => fit_scale.min(target / em),
        _ => fit_scale,
    };

    scale_layout_box(&mut layout_box, scale);

    let (w_px, h_px, alpha) = rasterize_alpha(
        &layout_box,
        pad,
        pad,
        left_extra * scale,
        right_extra * scale,
        asc * scale,
        desc * scale,
    )?;
    Ok(AlphaMask {
        width: w_px as u32,
        height: h_px as u32,
        alpha,
    })
}

fn scale_layout_box(layout_box: &mut LBox, scale: f32) {
    layout_box.w *= scale;
    layout_box.asc *= scale;
    layout_box.desc *= scale;
    for item in &mut layout_box.items {
        match item {
            Item::Glyph { em, x, y, .. } => {
                *em *= scale;
                *x *= scale;
                *y *= scale;
            }
            Item::Poly(points) => {
                for (x, y) in points {
                    *x *= scale;
                    *y *= scale;
                }
            }
            Item::Contours(contours) => {
                for points in contours {
                    for (x, y) in points {
                        *x *= scale;
                        *y *= scale;
                    }
                }
            }
        }
    }
}

fn collect_node_paths(
    node: &MathNode,
    path: &mut Vec<usize>,
    paths: &mut std::collections::HashMap<usize, Vec<usize>>,
) {
    paths.insert(node as *const _ as usize, path.clone());
    for (i, child) in node.children.iter().enumerate() {
        path.push(i);
        collect_node_paths(child, path, paths);
        path.pop();
    }
}

fn rasterize(layout_box: &LBox, em: f32) -> Result<Preview> {
    let font_set = fonts();
    let (min_x, max_x, min_y, max_y) = layout_box.ink_bounds(font_set);
    let asc = (-min_y).max(0.70 * em);
    let desc = max_y.max(0.22 * em);
    let left_extra = (-min_x).max(0.0);
    let right_extra = (max_x - layout_box.w).max(0.0);
    let (pad_x, pad_y) = (2.0f32, 2.0f32);
    let (w_px, h_px, alpha) =
        rasterize_alpha(layout_box, pad_x, pad_y, left_extra, right_extra, asc, desc)?;
    let ppm = (PX_PER_PT * 72.0 / 0.0254).round() as u32;
    Ok(Preview {
        png: png::encode_alpha(w_px as u32, h_px as u32, &alpha, ppm),
        width_pt: w_px as f32 / PX_PER_PT,
        height_pt: h_px as f32 / PX_PER_PT,
        depth_pt: (desc + pad_y) / PX_PER_PT,
    })
}

fn rasterize_alpha(
    layout_box: &LBox,
    pad_x: f32,
    pad_y: f32,
    left_extra: f32,
    right_extra: f32,
    asc: f32,
    desc: f32,
) -> Result<(usize, usize, Vec<u8>)> {
    let font_set = fonts();
    let w_px = (layout_box.w + left_extra + right_extra + 2.0 * pad_x)
        .ceil()
        .max(1.0) as usize;
    let h_px = (asc + desc + 2.0 * pad_y).ceil().max(1.0) as usize;
    if w_px > MAX_WIDTH_PX || w_px * h_px > MAX_PIXELS {
        return Err(Error::PreviewTooLarge {
            width: w_px,
            height: h_px,
        });
    }

    let (origin_x, origin_y) = (pad_x + left_extra, pad_y + asc);
    let mut canvas = Canvas::new(w_px, h_px);
    let mut rasterizer: Option<Rasterizer> = None;
    for item in &layout_box.items {
        match item {
            Item::Glyph {
                ch,
                face,
                em,
                x,
                y,
                ssty,
            } => font_set.draw_styled(
                *face,
                *ch,
                *em,
                origin_x + x,
                origin_y + y,
                &mut canvas,
                *ssty,
            ),
            Item::Poly(points) => {
                if points.len() < 3 {
                    continue;
                }
                let mut shifted: Vec<(f32, f32)> = points
                    .iter()
                    .map(|(x, y)| (origin_x + x, origin_y + y))
                    .collect();
                if signed_area(&shifted) < 0.0 {
                    shifted.reverse();
                }
                let target_ras = rasterizer.get_or_insert_with(|| Rasterizer::new(w_px, h_px));
                for i in 0..shifted.len() {
                    let (start, end) = (shifted[i], shifted[(i + 1) % shifted.len()]);
                    target_ras.draw_line(point(start.0, start.1), point(end.0, end.1));
                }
            }
            Item::Contours(contours) => {
                let mut shifted: Vec<Vec<(f32, f32)>> = contours
                    .iter()
                    .filter(|points| points.len() >= 3)
                    .map(|points| {
                        points
                            .iter()
                            .map(|(x, y)| (origin_x + x, origin_y + y))
                            .collect()
                    })
                    .collect();
                if shifted.is_empty() {
                    continue;
                }
                // Orient by the largest-magnitude contour so inner holes keep
                // their opposite winding and subtract cleanly instead of filling.
                let outer_area = shifted
                    .iter()
                    .map(|poly| signed_area(poly))
                    .max_by(|left, right| left.abs().total_cmp(&right.abs()))
                    .unwrap_or(0.0);
                if outer_area < 0.0 {
                    for poly in &mut shifted {
                        poly.reverse();
                    }
                }
                let mut min_x = f32::INFINITY;
                let mut max_x = f32::NEG_INFINITY;
                let mut min_y = f32::INFINITY;
                let mut max_y = f32::NEG_INFINITY;
                for poly in &shifted {
                    for &(px, py) in poly {
                        min_x = min_x.min(px);
                        max_x = max_x.max(px);
                        min_y = min_y.min(py);
                        max_y = max_y.max(py);
                    }
                }
                let bx0 = (min_x.floor().max(0.0) as usize).min(w_px);
                let by0 = (min_y.floor().max(0.0) as usize).min(h_px);
                let bx1 = (max_x.ceil().max(0.0) as usize).min(w_px);
                let by1 = (max_y.ceil().max(0.0) as usize).min(h_px);
                if bx1 <= bx0 || by1 <= by0 {
                    continue;
                }
                let box_w = bx1 - bx0;
                let box_h = by1 - by0;
                let (fx0, fy0) = (bx0 as f32, by0 as f32);
                let mut sub_ras = Rasterizer::new(box_w, box_h);
                for poly in &shifted {
                    for i in 0..poly.len() {
                        let (start, end) = (poly[i], poly[(i + 1) % poly.len()]);
                        sub_ras.draw_line(
                            point(start.0 - fx0, start.1 - fy0),
                            point(end.0 - fx0, end.1 - fy0),
                        );
                    }
                }
                sub_ras.for_each_pixel(|idx, alpha_val| {
                    let local_x = idx % box_w;
                    let local_y = idx / box_w;
                    let dst_idx = (by0 + local_y) * w_px + (bx0 + local_x);
                    if let Some(cov) = canvas.cov.get_mut(dst_idx) {
                        *cov = (*cov + alpha_val).min(1.0);
                    }
                });
            }
        }
    }
    if let Some(rasterizer) = rasterizer {
        rasterizer.for_each_pixel(|idx, alpha_val| {
            if let Some(cov) = canvas.cov.get_mut(idx) {
                *cov = (*cov + alpha_val).min(1.0);
            }
        });
    }

    let alpha: Vec<u8> = canvas
        .cov
        .iter()
        .map(|cov| (cov * 255.0 + 0.5) as u8)
        .collect();
    Ok((w_px, h_px, alpha))
}

fn signed_area(points: &[(f32, f32)]) -> f32 {
    (0..points.len())
        .map(|i| {
            let (curr, next) = (points[i], points[(i + 1) % points.len()]);
            curr.0 * next.1 - next.0 * curr.1
        })
        .sum()
}

#[cfg(test)]
mod interactive_tests {
    use super::*;
    use std::io::Read;

    fn decode_indexed_alpha_png(png: &[u8]) -> (usize, usize, Vec<u8>) {
        let mut pos = 8usize;
        let mut width = 0usize;
        let mut height = 0usize;
        let mut idat = Vec::new();
        while pos + 12 <= png.len() {
            let len = u32::from_be_bytes(png[pos..pos + 4].try_into().unwrap()) as usize;
            let kind = &png[pos + 4..pos + 8];
            let data = &png[pos + 8..pos + 8 + len];
            if kind == b"IHDR" {
                width = u32::from_be_bytes(data[0..4].try_into().unwrap()) as usize;
                height = u32::from_be_bytes(data[4..8].try_into().unwrap()) as usize;
            } else if kind == b"IDAT" {
                idat.extend_from_slice(data);
            }
            pos += 12 + len;
        }
        let mut raw = Vec::new();
        flate2::read::ZlibDecoder::new(&idat[..])
            .read_to_end(&mut raw)
            .unwrap();
        let stride = width + 1;
        let mut out = vec![0u8; width * height];
        for y in 0..height {
            let filter_type = raw[y * stride];
            let row = &raw[y * stride + 1..(y + 1) * stride];
            for x in 0..width {
                let left = if x > 0 { out[y * width + x - 1] } else { 0 };
                let up = if y > 0 { out[(y - 1) * width + x] } else { 0 };
                let upper_left = if x > 0 && y > 0 {
                    out[(y - 1) * width + x - 1]
                } else {
                    0
                };
                let val = match filter_type {
                    0 => row[x],
                    1 => row[x].wrapping_add(left),
                    2 => row[x].wrapping_add(up),
                    3 => row[x].wrapping_add((((left as u16) + (up as u16)) / 2) as u8),
                    4 => {
                        let pred = left as i32 + up as i32 - upper_left as i32;
                        let dist_left = (pred - left as i32).abs();
                        let dist_up = (pred - up as i32).abs();
                        let dist_ul = (pred - upper_left as i32).abs();
                        let paeth = if dist_left <= dist_up && dist_left <= dist_ul {
                            left
                        } else if dist_up <= dist_ul {
                            up
                        } else {
                            upper_left
                        };
                        row[x].wrapping_add(paeth)
                    }
                    _ => unreachable!(),
                };
                out[y * width + x] = val;
            }
        }
        (width, height, out)
    }

    #[test]
    fn fraction_hit_regions_share_the_raster_coordinate_system() {
        let math = MathNode::parse(
            "<math><mfrac><mi>a</mi><msup><mi>b</mi><mn>2</mn></msup></mfrac></math>",
        )
        .unwrap();
        let interactive = render_interactive(&math, 12.0).unwrap();
        assert_eq!(
            interactive.preview.png,
            render_preview(&math, 12.0).unwrap().png
        );
        let numerator = interactive
            .regions
            .iter()
            .find(|region| region.path == [0, 0])
            .unwrap();
        let denominator = interactive
            .regions
            .iter()
            .find(|region| region.path == [0, 1])
            .unwrap();
        assert!(numerator.baseline < denominator.baseline);
        for region in &interactive.regions {
            assert!(region.x.is_finite() && region.y.is_finite());
        }
    }

    #[test]
    fn proportional_text_has_glyph_based_caret_positions() {
        let math = MathNode::parse("<math><mtext>Wi</mtext></math>").unwrap();
        let preview = render_interactive(&math, 12.0).unwrap();
        let region = preview
            .regions
            .iter()
            .find(|region| region.path == [0])
            .unwrap();
        assert_eq!(region.carets.len(), 3);
        assert!(region.carets[1] - region.carets[0] > region.carets[2] - region.carets[1]);
    }

    #[test]
    fn text_caret_positions_include_spaces_and_combining_marks() {
        let math = MathNode::parse("<math><mtext>e\u{0302}\u{0301} x</mtext></math>").unwrap();
        let preview = render_interactive(&math, 12.0).unwrap();
        let region = preview
            .regions
            .iter()
            .find(|region| region.path == [0])
            .unwrap();
        assert_eq!(region.carets.len(), 6);
        assert!(region.carets[4] > region.carets[3]);
    }

    #[test]
    fn standalone_text_spaces_advance_layout_and_emf() {
        use font::Face;

        let empty = MathNode::new("math");
        let blank_emf = emf::render_emf(&empty, 12.0).unwrap();
        for ch in [' ', '\u{00A0}'] {
            let math =
                MathNode::with_children("math", vec![MathNode::leaf("mtext", &ch.to_string())]);
            let (layout_box, em) = layout::layout_math(&math, 12.0).unwrap();
            let advance = fonts().advance(Face::Text, ch, em);
            assert!(advance > 0.0);
            assert_eq!(layout_box.w, advance);
            assert!(emf::render_emf(&math, 12.0).unwrap().width_pt > blank_emf.width_pt);
        }
    }

    #[test]
    fn unit_with_trailing_space_uses_text_glyph_and_advance() {
        use font::Face;

        for (ch, space) in [('m', ' '), ('s', '\u{00A0}')] {
            let math = MathNode::with_children(
                "math",
                vec![MathNode::leaf("mtext", &format!("{ch}{space}"))],
            );
            let (layout_box, em) = layout::layout_math(&math, 12.0).unwrap();
            assert!(matches!(
                layout_box.items.first(),
                Some(Item::Glyph { ch: glyph, face: Face::Text, .. }) if *glyph == ch
            ));
            let expected =
                fonts().advance(Face::Text, ch, em) + fonts().advance(Face::Text, space, em);
            assert!((layout_box.w - expected).abs() < 0.0001);
        }
    }

    fn integral_regions(xml: &str) -> (HitRegion, HitRegion, HitRegion) {
        let preview = render_interactive(&MathNode::parse(xml).unwrap(), 12.0).unwrap();
        let find = |path: &[usize]| {
            preview
                .regions
                .iter()
                .find(|region| region.path == path)
                .unwrap()
                .clone()
        };
        (find(&[0, 0]), find(&[0, 1]), find(&[0, 2]))
    }

    #[test]
    fn integral_under_over_regions_are_above_and_below_the_operator() {
        let (base, lo, hi) = integral_regions(
            "<math><munderover><mo>∫</mo><mi>a</mi><mi>b</mi></munderover></math>",
        );
        assert!(hi.y + hi.height < base.y);
        assert!(lo.y > base.y + base.height);
        assert!(hi.x < base.x + base.width);
        assert!(lo.x < base.x + base.width);
    }

    #[test]
    fn integral_layout_honors_explicit_word_limit_placement() {
        let (base, lo, hi) = integral_regions(
            r#"<math><munderover data-limloc="subSup"><mo>∫</mo><mi>a</mi><mi>b</mi></munderover></math>"#,
        );
        assert!(lo.x > base.x + base.width - 0.5);
        assert!(hi.x > base.x + base.width - 0.5);
        let (base, lo, hi) = integral_regions(
            r#"<math><msubsup data-limloc="undOvr"><mo>∫</mo><mi>a</mi><mi>b</mi></msubsup></math>"#,
        );
        assert!(hi.y + hi.height < base.y);
        assert!(lo.y > base.y + base.height);
    }

    #[test]
    fn hollow_double_arrow_preserves_transparent_interior() {
        for (latex, band_end_ratio) in [
            (r"\overaccent{⇔}{x}", 0.45),
            (r"\overset{a}{\Leftrightarrow}", 1.0),
        ] {
            let math = crate::service::parse_latex_to_mathml(latex).unwrap();
            let preview = render_preview(&math, 14.0).unwrap();
            let (width, height, alpha) = decode_indexed_alpha_png(&preview.png);
            let col = width / 2;
            let y_max = ((height as f32) * band_end_ratio) as usize;
            // Scan down the middle column of the arrow band: expect two ink peaks
            // (upper shaft and lower shaft) separated by transparent pixels (alpha == 0).
            let col_samples: Vec<u8> = (0..y_max).map(|y| alpha[y * width + col]).collect();
            let mut ink_runs = 0usize;
            let mut has_hollow_gap = false;
            let mut in_ink = false;
            let mut seen_first_run = false;
            for &alpha_val in &col_samples {
                if alpha_val >= 200 {
                    if !in_ink {
                        in_ink = true;
                        ink_runs += 1;
                    }
                } else {
                    if in_ink {
                        in_ink = false;
                        seen_first_run = true;
                    }
                    if seen_first_run && ink_runs == 1 && alpha_val == 0 {
                        has_hollow_gap = true;
                    }
                }
            }
            assert!(
                ink_runs >= 2 && has_hollow_gap,
                "{latex}: expected two horizontal shaft walls with a transparent hollow gap at x={col}, ys={col_samples:?}"
            );
        }
    }

    #[test]
    fn arrow_and_extender_share_vertical_axis() {
        let font_set = fonts();
        let axis_of = |ch: char| -> f32 {
            let polys = font_set.glyph_polys(font::Face::Symbol, ch, 100.0, 100.0, 0.0, 0.0);
            let (min_y, max_y) = polys
                .iter()
                .flatten()
                .fold((f32::MAX, f32::MIN), |(lo, hi), &(_, y)| {
                    (lo.min(y), hi.max(y))
                });
            -(min_y + max_y) / 2.0
        };
        let mid_arrow = axis_of('\u{2192}');
        let mid_ext = axis_of('\u{23AF}');
        assert!(
            (mid_arrow - mid_ext).abs() < 1.0 && (mid_arrow - 25.9).abs() < 1.0,
            "arrow axis ({mid_arrow}) and extender axis ({mid_ext}) should match (~25.9px at 100px em)"
        );
    }

    #[test]
    fn render_alpha_at_caps_short_symbols_without_upscaling() {
        let star = MathNode::with_children("math", vec![MathNode::leaf("mo", "★")]);
        let alpha = MathNode::with_children("math", vec![MathNode::leaf("mi", "α")]);
        let frac = crate::service::parse_latex_to_mathml(r"\frac{x}{y}").unwrap();

        let fit_star = render_alpha_fit(&star, 54.0, 32.0).unwrap();
        let at_star = render_alpha_at(&star, 15.0, 54.0, 32.0).unwrap();
        let at_alpha = render_alpha_at(&alpha, 15.0, 54.0, 32.0).unwrap();
        let at_frac = render_alpha_at(&frac, 15.0, 54.0, 32.0).unwrap();

        assert!(at_star.height < fit_star.height);
        assert!(at_star.height <= 20 && at_alpha.height <= 20);
        assert!((at_star.height as i32 - at_alpha.height as i32).abs() <= 2);
        assert!(at_frac.width <= 54 && at_frac.height <= 32);
        assert!(at_star.alpha.iter().any(|&alpha_val| alpha_val > 0));
        assert!(at_alpha.alpha.iter().any(|&alpha_val| alpha_val > 0));
        assert!(at_frac.alpha.iter().any(|&alpha_val| alpha_val > 0));
    }

    #[test]
    fn root_region_y_aligns_with_tallest_child_relative_to_baseline() {
        let math = MathNode::with_children(
            "math",
            vec![
                MathNode::leaf("mi", "x"),
                MathNode::leaf("mo", "+"),
                MathNode::leaf("mn", "1"),
            ],
        );
        let preview = render_interactive(&math, 12.0).unwrap();
        let root = preview
            .regions
            .iter()
            .find(|region| region.path.is_empty())
            .unwrap();
        let plus = preview
            .regions
            .iter()
            .find(|region| region.path == [1])
            .unwrap();
        let num = preview
            .regions
            .iter()
            .find(|region| region.path == [2])
            .unwrap();
        assert!(
            (root.y - num.y).abs() < 0.05,
            "root y ({}) must align with tallest child top ({})",
            root.y,
            num.y
        );
        assert!(
            ((root.y + root.height) - (plus.y + plus.height)).abs() < 0.05,
            "root bottom ({}) must align with lowest child bottom ({})",
            root.y + root.height,
            plus.y + plus.height
        );
    }
}
