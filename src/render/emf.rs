//! Draws a MathML tree as an Enhanced Metafile (EMF), a vector picture format
//! that Word and WPS accept as the preview of an OLE object.
//!
//! Uses the same layout as the PNG outputs. Glyphs are written as
//! filled paths (outlines), so the picture does not depend on any installed
//! font and stays sharp at any zoom.
//!
//! Coordinates: the picture is described in "logical" units of 1/100 of a layout
//! pixel (`MM_ANISOTROPIC`) and mapped onto a device of 1440 units per inch, so
//! rounding to integers loses almost nothing.

use super::font::{fonts, Seg};
use super::layout::{self, Item};
use super::PX_PER_PT;
use crate::error::Result;
use crate::mathml::MathNode;

/// Logical units per layout pixel.
const LOGICAL_PER_PX: f32 = 100.0;
/// Device units (twips) per point.
const DEVICE_PER_PT: f32 = 20.0;

// Record types (MS-EMF 2.1.1)
const EMR_HEADER: u32 = 1;
const EMR_POLYBEZIERTO: u32 = 5;
const EMR_SETWINDOWEXTEX: u32 = 9;
const EMR_SETVIEWPORTEXTEX: u32 = 11;
const EMR_EOF: u32 = 14;
const EMR_SETPOLYFILLMODE: u32 = 19;
const EMR_SETMAPMODE: u32 = 17;
const EMR_MOVETOEX: u32 = 27;
const EMR_SELECTOBJECT: u32 = 37;
const EMR_CREATEBRUSHINDIRECT: u32 = 39;
const EMR_LINETO: u32 = 54;
const EMR_BEGINPATH: u32 = 59;
const EMR_ENDPATH: u32 = 60;
const EMR_CLOSEFIGURE: u32 = 61;
const EMR_FILLPATH: u32 = 62;
const EMR_COMMENT: u32 = 0x0000_0046;

const MM_ANISOTROPIC: u32 = 8;
const WINDING: u32 = 2;

/// A rendered equation as an EMF file plus its size and baseline in points.
#[derive(Debug, Clone)]
pub struct EmfPreview {
    /// The EMF file.
    pub emf: Vec<u8>,
    /// Width in points.
    pub width_pt: f32,
    /// Height in points.
    pub height_pt: f32,
    /// Distance from the baseline down to the bottom of the image, in points.
    pub depth_pt: f32,
}

/// Little-endian record writer that counts records.
struct Writer {
    buf: Vec<u8>,
    records: u32,
}

impl Writer {
    fn new() -> Self {
        Writer {
            buf: Vec::new(),
            records: 0,
        }
    }

    /// Append one record: `kind`, then `body` (padded to a multiple of 4 bytes).
    fn record(&mut self, kind: u32, body: &[u8]) {
        let padded = (body.len() + 3) & !3;
        let size = 8 + padded as u32;
        self.buf.extend_from_slice(&kind.to_le_bytes());
        self.buf.extend_from_slice(&size.to_le_bytes());
        self.buf.extend_from_slice(body);
        self.buf.resize(self.buf.len() + (padded - body.len()), 0);
        self.records += 1;
    }

    #[inline]
    fn record_empty(&mut self, kind: u32) {
        self.buf.extend_from_slice(&kind.to_le_bytes());
        self.buf.extend_from_slice(&8u32.to_le_bytes());
        self.records += 1;
    }

    #[inline]
    fn record_u32s(&mut self, kind: u32, vals: &[u32]) {
        let size = 8 + (vals.len() * 4) as u32;
        self.buf.extend_from_slice(&kind.to_le_bytes());
        self.buf.extend_from_slice(&size.to_le_bytes());
        for &v in vals {
            self.buf.extend_from_slice(&v.to_le_bytes());
        }
        self.records += 1;
    }

    #[inline]
    fn record_i32s(&mut self, kind: u32, vals: &[i32]) {
        let size = 8 + (vals.len() * 4) as u32;
        self.buf.extend_from_slice(&kind.to_le_bytes());
        self.buf.extend_from_slice(&size.to_le_bytes());
        for &v in vals {
            self.buf.extend_from_slice(&v.to_le_bytes());
        }
        self.records += 1;
    }

    #[inline]
    fn record_polybezierto(&mut self, bounds: [i32; 4], pts: &[(i32, i32)]) {
        let size = (8 + 16 + 4 + pts.len() * 8) as u32;
        self.buf.extend_from_slice(&EMR_POLYBEZIERTO.to_le_bytes());
        self.buf.extend_from_slice(&size.to_le_bytes());
        for b in bounds {
            self.buf.extend_from_slice(&b.to_le_bytes());
        }
        self.buf
            .extend_from_slice(&(pts.len() as u32).to_le_bytes());
        for &(x, y) in pts {
            self.buf.extend_from_slice(&x.to_le_bytes());
            self.buf.extend_from_slice(&y.to_le_bytes());
        }
        self.records += 1;
    }
}

/// Draw `math` (a `<math>` tree) at `font_pt` points per em, as EMF.
/// Automatically encodes `math` to MTEF 5.0 and embeds it inside an `EMR_COMMENT` record.
pub fn render_emf(math: &MathNode, font_pt: f32) -> Result<EmfPreview> {
    let mtef = crate::mtef::encode::mathml_to_mtef_bytes(math)
        .ok()
        .filter(|b| !b.is_empty())
        .map(|b| crate::mtef::build_mtef(&b));
    render_emf_with_mtef(math, font_pt, mtef.as_deref())
}

/// Draw `math` (a `<math>` tree) at `font_pt` points per em, as EMF.
/// If `mtef` binary data is provided, it is embedded inside an `EMR_COMMENT` record
/// with the MathType `AppsMFCC` signature immediately following the header.
/// Both complete MTEF streams (starting with version 3 or 5) and raw inner equation
/// bytes (which will be wrapped via `build_mtef`) are supported.
pub fn render_emf_with_mtef(
    math: &MathNode,
    font_pt: f32,
    mtef: Option<&[u8]>,
) -> Result<EmfPreview> {
    let font_pt = super::validate_font_pt(font_pt)?;
    let (b, em) = layout::layout_math(math, font_pt)?;
    let f = fonts();
    let (min_x, max_x, min_y, max_y) = b.ink_bounds(f);
    let asc = (-min_y).max(0.70 * em);
    let desc = max_y.max(0.22 * em);
    let left_extra = (-min_x).max(0.0);
    let right_extra = (max_x - b.w).max(0.0);
    let (pad_x, pad_y) = (2.0f32, 2.0f32);
    let (w, h) = (
        b.w + left_extra + right_extra + 2.0 * pad_x,
        asc + desc + 2.0 * pad_y,
    );
    let (ox, oy) = (pad_x + left_extra, pad_y + asc);

    let (wpt, hpt) = (w / PX_PER_PT, h / PX_PER_PT);
    let l = |v: f32| (v * LOGICAL_PER_PX).round() as i32; // layout px -> logical units
    let (wl, hl) = (l(w).max(1), l(h).max(1));
    let (wd, hd) = (
        ((wpt * DEVICE_PER_PT).round() as i32).max(1),
        ((hpt * DEVICE_PER_PT).round() as i32).max(1),
    );

    let mut out = Writer::new();

    // Embed MTEF 5.0 binary inside an EMR_COMMENT record if present.
    let full_mtef: Option<std::borrow::Cow<[u8]>> = match mtef {
        Some(bytes) if !bytes.is_empty() => {
            if matches!(bytes.first(), Some(3 | 5)) {
                Some(std::borrow::Cow::Borrowed(bytes))
            } else {
                Some(std::borrow::Cow::Owned(crate::mtef::build_mtef(bytes)))
            }
        }
        _ => None,
    };

    if let Some(mtef_bytes) = full_mtef {
        let mut comment_data = Vec::with_capacity(39 + mtef_bytes.len());
        comment_data.extend_from_slice(b"AppsMFCC");
        comment_data.extend_from_slice(&1u16.to_le_bytes()); // version 1
        comment_data.extend_from_slice(&(mtef_bytes.len() as u32).to_le_bytes()); // size_total
        comment_data.extend_from_slice(&(mtef_bytes.len() as u32).to_le_bytes()); // size_chunk
        comment_data.extend_from_slice(crate::ole::WMF_SIGNATURE); // "Design Science, Inc.\0"
        comment_data.extend_from_slice(&mtef_bytes);

        let mut body = Vec::with_capacity(4 + comment_data.len());
        body.extend_from_slice(&(comment_data.len() as u32).to_le_bytes()); // cbData
        body.extend_from_slice(&comment_data);

        out.record(EMR_COMMENT, &body);
    }

    // The header is written last (it needs the totals); reserve nothing here and
    // prepend it at the end.
    out.record_u32s(EMR_SETMAPMODE, &[MM_ANISOTROPIC]);
    out.record_i32s(EMR_SETWINDOWEXTEX, &[wl, hl]);
    out.record_i32s(EMR_SETVIEWPORTEXTEX, &[wd, hd]);
    out.record_u32s(EMR_SETPOLYFILLMODE, &[WINDING]);
    // handle 1 = solid black brush
    out.record_u32s(EMR_CREATEBRUSHINDIRECT, &[1, 0, 0x0000_0000, 0]);
    out.record_u32s(EMR_SELECTOBJECT, &[1]);

    let bounds = [0, 0, wl, hl];
    let fill = |out: &mut Writer, build: &dyn Fn(&mut Writer)| {
        out.record_empty(EMR_BEGINPATH);
        build(out);
        out.record_empty(EMR_ENDPATH);
        out.record_i32s(EMR_FILLPATH, &bounds);
    };

    let mut poly_segs: Vec<Seg> = Vec::new();
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
                let Some(segs) = f.glyph_segments_styled(*face, *ch, *em, ox + x, oy + y, *ssty)
                else {
                    continue;
                };
                fill(&mut out, &|o| write_segs(o, &segs, &l, bounds));
            }
            Item::Poly(pts) => {
                if pts.len() < 3 {
                    continue;
                }
                poly_segs.clear();
                poly_segs.reserve(pts.len() + 1);
                for (i, (x, y)) in pts.iter().enumerate() {
                    let (x, y) = (ox + x, oy + y);
                    poly_segs.push(if i == 0 {
                        Seg::Move(x, y)
                    } else {
                        Seg::Line(x, y)
                    });
                }
                poly_segs.push(Seg::Close);
                fill(&mut out, &|o| write_segs(o, &poly_segs, &l, bounds));
            }
            Item::Contours(contours) => {
                poly_segs.clear();
                for pts in contours {
                    if pts.len() < 3 {
                        continue;
                    }
                    for (i, (x, y)) in pts.iter().enumerate() {
                        let (x, y) = (ox + x, oy + y);
                        poly_segs.push(if i == 0 {
                            Seg::Move(x, y)
                        } else {
                            Seg::Line(x, y)
                        });
                    }
                    poly_segs.push(Seg::Close);
                }
                if !poly_segs.is_empty() {
                    fill(&mut out, &|o| write_segs(o, &poly_segs, &l, bounds));
                }
            }
        }
    }
    out.record_u32s(EMR_EOF, &[0, 16, 20]); // no palette; nSizeLast = this record's size

    // ---- header ----
    let frame_hm = |pt: f32| (pt * 25.4 / 72.0 * 100.0).round() as i32; // 0.01 mm
    let (fw, fh) = (frame_hm(wpt), frame_hm(hpt));
    let total = 88 + out.buf.len() as u32;
    let mut emf = Vec::with_capacity(total as usize);
    emf.extend_from_slice(&EMR_HEADER.to_le_bytes());
    emf.extend_from_slice(&88u32.to_le_bytes());
    for v in [0, 0, wd - 1, hd - 1, 0, 0, fw, fh] {
        emf.extend_from_slice(&v.to_le_bytes()); // rclBounds, rclFrame
    }
    for v in [0x464D_4520u32, 0x0001_0000, total, out.records + 1] {
        emf.extend_from_slice(&v.to_le_bytes());
    }
    emf.extend_from_slice(&2u16.to_le_bytes()); // nHandles (0 is reserved, 1 = brush)
    emf.extend_from_slice(&0u16.to_le_bytes());
    for v in [0u32, 0, 0] {
        emf.extend_from_slice(&v.to_le_bytes()); // nDescription, offDescription, nPalEntries
    }
    for v in [14400i32, 8100, 254, 143] {
        emf.extend_from_slice(&v.to_le_bytes()); // szlDevice, szlMillimeters
    }
    debug_assert_eq!(emf.len(), 88);
    emf.extend_from_slice(&out.buf);
    debug_assert_eq!(emf.len() as u32, total);

    Ok(EmfPreview {
        emf,
        width_pt: wpt,
        height_pt: hpt,
        depth_pt: (desc + pad_y) / PX_PER_PT,
    })
}

/// Write contour commands as `MOVETOEX` / `LINETO` / `POLYBEZIERTO` / `CLOSEFIGURE`.
fn write_segs(o: &mut Writer, segs: &[Seg], l: &dyn Fn(f32) -> i32, bounds: [i32; 4]) {
    let mut cur = (0.0f32, 0.0f32);
    for sg in segs {
        match *sg {
            Seg::Move(x, y) => {
                o.record_i32s(EMR_MOVETOEX, &[l(x), l(y)]);
                cur = (x, y);
            }
            Seg::Line(x, y) => {
                o.record_i32s(EMR_LINETO, &[l(x), l(y)]);
                cur = (x, y);
            }
            Seg::Quad(cx, cy, x, y) => {
                // quadratic -> cubic: both controls move 2/3 of the way to the quad control
                let c1 = (
                    cur.0 + 2.0 / 3.0 * (cx - cur.0),
                    cur.1 + 2.0 / 3.0 * (cy - cur.1),
                );
                let c2 = (x + 2.0 / 3.0 * (cx - x), y + 2.0 / 3.0 * (cy - y));
                o.record_polybezierto(
                    bounds,
                    &[(l(c1.0), l(c1.1)), (l(c2.0), l(c2.1)), (l(x), l(y))],
                );
                cur = (x, y);
            }
            Seg::Cubic(c1x, c1y, c2x, c2y, x, y) => {
                o.record_polybezierto(bounds, &[(l(c1x), l(c1y)), (l(c2x), l(c2y)), (l(x), l(y))]);
                cur = (x, y);
            }
            Seg::Close => o.record_empty(EMR_CLOSEFIGURE),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mathml::MathNode;

    #[test]
    fn test_render_emf_embeds_mtef_comment() {
        let node = MathNode::with_children("math", vec![MathNode::leaf("mi", "x")]);
        let preview = render_emf(&node, 12.0).expect("renders emf");
        assert!(!preview.emf.is_empty());
        let extracted = crate::ole::extract_mtef_from_emf(&preview.emf);
        assert!(
            extracted.is_some(),
            "MTEF must be extracted from EMF comment"
        );
        let mtef = extracted.unwrap();
        assert_eq!(mtef[0], 5, "Must be MTEF 5.0");
    }

    #[test]
    fn test_render_emf_with_explicit_mtef() {
        let node = MathNode::leaf("mi", "y");
        let custom_mtef = vec![5, 0, 1, 2, 3, 4];
        let preview = render_emf_with_mtef(&node, 12.0, Some(&custom_mtef)).expect("renders emf");
        let extracted = crate::ole::extract_mtef_from_emf(&preview.emf).expect("extracted");
        assert_eq!(extracted, custom_mtef);
    }

    #[test]
    fn test_render_emf_without_mtef() {
        let node = MathNode::leaf("mi", "z");
        let preview = render_emf_with_mtef(&node, 12.0, None).expect("renders emf");
        let extracted = crate::ole::extract_mtef_from_emf(&preview.emf);
        assert!(extracted.is_none());
    }

    #[test]
    fn test_render_emf_italic_f_not_cropped() {
        let node = MathNode::with_children("math", vec![MathNode::leaf("mi", "f")]);
        let preview = render_emf(&node, 12.0).expect("renders emf");
        // Adv is ~3.49pt, ink span is from -1.5pt to +5.25pt (total 6.75pt).
        // With padding and left/right extra, width_pt must be at least 7.0pt.
        assert!(
            preview.width_pt >= 7.0,
            "f glyph width_pt ({}) must accommodate overhang and padding (>= 7.0)",
            preview.width_pt
        );
    }

    #[test]
    fn test_render_emf_contours_single_filled_path() {
        // Double arrow accent `⇔` has outer contour + hollow inner contour and must
        // be emitted inside a single BEGINPATH..ENDPATH..FILLPATH under WINDING fill mode.
        let mut mo = MathNode::leaf("mo", "⇔");
        mo.set_attr("stretchy", "true");
        let mut mover = MathNode::with_children("mover", vec![MathNode::leaf("mi", "x"), mo]);
        mover.set_attr("accent", "true");
        let math = MathNode::with_children("math", vec![mover]);
        let preview = render_emf_with_mtef(&math, 14.0, None).expect("renders emf");

        let mut records: Vec<(u32, Vec<u8>)> = Vec::new();
        let mut pos = 0usize;
        while pos + 8 <= preview.emf.len() {
            let kind = u32::from_le_bytes(preview.emf[pos..pos + 4].try_into().unwrap());
            let size =
                u32::from_le_bytes(preview.emf[pos + 4..pos + 8].try_into().unwrap()) as usize;
            assert!(size >= 8 && pos + size <= preview.emf.len());
            records.push((kind, preview.emf[pos + 8..pos + size].to_vec()));
            pos += size;
        }
        assert_eq!(pos, preview.emf.len());
        assert!(records.iter().any(|(k, b)| {
            *k == EMR_SETPOLYFILLMODE
                && b.len() >= 4
                && u32::from_le_bytes(b[..4].try_into().unwrap()) == WINDING
        }));

        // Collect (EMR_MOVETOEX count, EMR_CLOSEFIGURE count) per FILLPATH group.
        let mut path_figures = Vec::new();
        let mut in_path = false;
        let mut moves = 0usize;
        let mut closes = 0usize;
        for (kind, _) in &records {
            match *kind {
                EMR_BEGINPATH => {
                    in_path = true;
                    moves = 0;
                    closes = 0;
                }
                EMR_MOVETOEX if in_path => moves += 1,
                EMR_CLOSEFIGURE if in_path => closes += 1,
                EMR_ENDPATH => {
                    assert!(in_path);
                    in_path = false;
                }
                EMR_FILLPATH => {
                    path_figures.push((moves, closes));
                }
                _ => {}
            }
        }
        // `x` is 1 figure; `⇔` is a compound Item::Contours with >= 2 figures in 1 path.
        assert!(
            path_figures.iter().any(|&(m, c)| m >= 2 && c >= 2),
            "expected multi-contour ⇔ in a single EMF path, got {path_figures:?}"
        );
    }
}
