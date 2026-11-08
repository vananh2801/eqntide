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
        Writer { buf: Vec::new(), records: 0 }
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
}

fn u32s(v: &[u32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn i32s(v: &[i32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
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
    let (b, em) = layout::layout_math(math, font_pt)?;
    let asc = b.asc.max(0.70 * em);
    let desc = b.desc.max(0.22 * em);
    let (pad_x, pad_y) = (2.0f32, 2.0f32);
    let (w, h) = (b.w + 2.0 * pad_x, asc + desc + 2.0 * pad_y);
    let (ox, oy) = (pad_x, pad_y + asc);

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
    out.record(EMR_SETMAPMODE, &u32s(&[MM_ANISOTROPIC]));
    out.record(EMR_SETWINDOWEXTEX, &i32s(&[wl, hl]));
    out.record(EMR_SETVIEWPORTEXTEX, &i32s(&[wd, hd]));
    out.record(EMR_SETPOLYFILLMODE, &u32s(&[WINDING]));
    // handle 1 = solid black brush
    out.record(EMR_CREATEBRUSHINDIRECT, &u32s(&[1, 0, 0x0000_0000, 0]));
    out.record(EMR_SELECTOBJECT, &u32s(&[1]));

    let bounds = i32s(&[0, 0, wl, hl]);
    let fill = |out: &mut Writer, build: &dyn Fn(&mut Writer)| {
        out.record(EMR_BEGINPATH, &[]);
        build(out);
        out.record(EMR_ENDPATH, &[]);
        out.record(EMR_FILLPATH, &bounds);
    };

    let f = fonts();
    for it in &b.items {
        match it {
            Item::Glyph { ch, face, em, x, y, ssty } => {
                let Some(segs) = f.glyph_segments_styled(*face, *ch, *em, ox + x, oy + y, *ssty) else {
                    continue;
                };
                fill(&mut out, &|o| write_segs(o, &segs, &l, [0, 0, wl, hl]));
            }
            Item::Poly(pts) => {
                if pts.len() < 3 {
                    continue;
                }
                let mut segs = Vec::with_capacity(pts.len() + 1);
                for (i, (x, y)) in pts.iter().enumerate() {
                    let (x, y) = (ox + x, oy + y);
                    segs.push(if i == 0 { Seg::Move(x, y) } else { Seg::Line(x, y) });
                }
                segs.push(Seg::Close);
                fill(&mut out, &|o| write_segs(o, &segs, &l, [0, 0, wl, hl]));
            }
        }
    }
    out.record(EMR_EOF, &u32s(&[0, 16, 20])); // no palette; nSizeLast = this record's size

    // ---- header ----
    let frame_hm = |pt: f32| (pt * 25.4 / 72.0 * 100.0).round() as i32; // 0.01 mm
    let (fw, fh) = (frame_hm(wpt), frame_hm(hpt));
    let mut hdr = Vec::with_capacity(80);
    hdr.extend(i32s(&[0, 0, wd - 1, hd - 1])); // rclBounds (device units)
    hdr.extend(i32s(&[0, 0, fw, fh])); // rclFrame (0.01 mm)
    hdr.extend(u32s(&[0x464D_4520, 0x0001_0000])); // " EMF", version 1.0
    let total = 88 + out.buf.len() as u32;
    hdr.extend(u32s(&[total, out.records + 1]));
    hdr.extend_from_slice(&2u16.to_le_bytes()); // nHandles (0 is reserved, 1 = brush)
    hdr.extend_from_slice(&0u16.to_le_bytes());
    hdr.extend(u32s(&[0, 0, 0])); // nDescription, offDescription, nPalEntries
    hdr.extend(i32s(&[14400, 8100])); // szlDevice: 10 in x 5.625 in at 1440 per inch
    hdr.extend(i32s(&[254, 143])); // szlMillimeters

    let mut file = Writer::new();
    file.record(EMR_HEADER, &hdr);
    debug_assert_eq!(file.buf.len(), 88);
    let mut emf = file.buf;
    emf.extend_from_slice(&out.buf);
    debug_assert_eq!(emf.len() as u32, total);

    Ok(EmfPreview { emf, width_pt: wpt, height_pt: hpt, depth_pt: (desc + pad_y) / PX_PER_PT })
}

/// Write contour commands as `MOVETOEX` / `LINETO` / `POLYBEZIERTO` / `CLOSEFIGURE`.
fn write_segs(o: &mut Writer, segs: &[Seg], l: &dyn Fn(f32) -> i32, bounds: [i32; 4]) {
    let mut cur = (0.0f32, 0.0f32);
    let bezier = |o: &mut Writer, pts: &[(f32, f32)]| {
        let mut body = i32s(&bounds); // rclBounds: the whole picture (a safe superset)
        body.extend(u32s(&[pts.len() as u32]));
        for (x, y) in pts {
            body.extend(i32s(&[l(*x), l(*y)]));
        }
        o.record(EMR_POLYBEZIERTO, &body);
    };
    for sg in segs {
        match *sg {
            Seg::Move(x, y) => {
                o.record(EMR_MOVETOEX, &i32s(&[l(x), l(y)]));
                cur = (x, y);
            }
            Seg::Line(x, y) => {
                o.record(EMR_LINETO, &i32s(&[l(x), l(y)]));
                cur = (x, y);
            }
            Seg::Quad(cx, cy, x, y) => {
                // quadratic -> cubic: both controls move 2/3 of the way to the quad control
                let c1 = (cur.0 + 2.0 / 3.0 * (cx - cur.0), cur.1 + 2.0 / 3.0 * (cy - cur.1));
                let c2 = (x + 2.0 / 3.0 * (cx - x), y + 2.0 / 3.0 * (cy - y));
                bezier(o, &[c1, c2, (x, y)]);
                cur = (x, y);
            }
            Seg::Cubic(c1x, c1y, c2x, c2y, x, y) => {
                bezier(o, &[(c1x, c1y), (c2x, c2y), (x, y)]);
                cur = (x, y);
            }
            Seg::Close => o.record(EMR_CLOSEFIGURE, &[]),
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
        assert!(extracted.is_some(), "MTEF must be extracted from EMF comment");
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
}
