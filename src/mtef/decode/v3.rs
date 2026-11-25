//! MTEF v3 (Equation Editor 3.0) record parsing, selectors, and variations.

use super::super::typeface::{TF_TEXT, TF_USER1, TF_USER2};
use super::consts::*;
use super::parse::{halign_name, parse_nudge, parse_ruler, parse_size, parse_v3_object_list};
use super::stream::ByteStream;
use super::MtefObj;

/// Parse a single MTEF v3 record identified by `record_type` and inline 4-bit `options`.
pub(super) fn parse_v3_record(
    stream: &mut ByteStream<'_>,
    record_type: u8,
    options: u8,
    depth: usize,
) -> Option<MtefObj> {
    match record_type {
        REC_LINE => {
            if (options & OPT_NUDGE) != 0 {
                parse_nudge(stream);
            }
            if (options & OPT_LINE_LSPACE) != 0 {
                let _ = stream.mtef16();
            }
            if (options & OPT_LP_RULER) != 0 {
                parse_ruler(stream);
            }
            let is_null = (options & OPT_LINE_NULL) != 0;
            let objects = if !is_null {
                parse_v3_object_list(stream, depth)
            } else {
                Vec::new()
            };
            Some(MtefObj::Line { is_null, objects })
        }
        REC_CHAR => {
            if (options & OPT_NUDGE) != 0 {
                parse_nudge(stream);
            }
            let tf_signed = stream.int8()?;
            let typeface = (tf_signed as i16 + 128) as u8;
            let mt_code = stream.mtef16()?;
            let is_textmode = matches!(typeface, TF_TEXT | TF_USER1 | TF_USER2);
            let mut embells = Vec::new();
            if (options & OPT_CHAR_EMBELL) != 0 {
                for obj in parse_v3_object_list(stream, depth) {
                    if let MtefObj::Char {
                        mt_code: embell_code,
                        ..
                    } = obj
                    {
                        embells.push(embell_code as u8);
                    }
                }
            }
            Some(MtefObj::Char {
                typeface,
                mt_code,
                is_textmode,
                embells,
            })
        }
        REC_TMPL => {
            if (options & OPT_NUDGE) != 0 {
                parse_nudge(stream);
            }
            let sel_idx = stream.int8()?;
            let var_code = stream.uint8()?;
            let selector = v3_selector_name(sel_idx, var_code);
            let variations = resolve_v3_variations(sel_idx, var_code);
            let _tmpl_opts = stream.uint8()?;
            let subobjects = parse_v3_object_list(stream, depth);
            Some(MtefObj::Tmpl {
                selector,
                variations,
                subobjects,
            })
        }
        REC_PILE => {
            if (options & OPT_NUDGE) != 0 {
                parse_nudge(stream);
            }
            let halign = halign_name(stream.int8()?);
            let _valign = stream.int8()?;
            if (options & OPT_LP_RULER) != 0 {
                parse_ruler(stream);
            }
            let lines = parse_v3_object_list(stream, depth);
            Some(MtefObj::Pile { halign, lines })
        }
        REC_MATRIX => {
            if (options & OPT_NUDGE) != 0 {
                parse_nudge(stream);
            }
            let _valign = stream.int8()?;
            let h_just = halign_name(stream.int8()?);
            let _v_just = stream.int8()?;
            let rows = stream.int8()?.max(0) as usize;
            let cols = stream.int8()?.max(0) as usize;
            for _ in 0..=rows {
                let _ = stream.nibble();
            }
            if !((rows + 1) * 2).is_multiple_of(8) {
                stream.align_byte();
            }
            for _ in 0..=cols {
                let _ = stream.nibble();
            }
            if !((cols + 1) * 2).is_multiple_of(8) {
                stream.align_byte();
            }
            let cells = parse_v3_object_list(stream, depth);
            Some(MtefObj::Matrix {
                rows,
                cols,
                h_just,
                cells,
            })
        }
        REC_EMBELL => {
            if (options & OPT_NUDGE) != 0 {
                parse_nudge(stream);
            }
            let code = stream.uint8()?;
            Some(MtefObj::Char {
                typeface: 0,
                mt_code: code as u16,
                is_textmode: false,
                embells: Vec::new(),
            })
        }
        REC_RULER => {
            parse_ruler(stream);
            Some(MtefObj::Meta)
        }
        REC_FONT_STYLE_DEF => {
            let _tf = stream.int8();
            let _style = stream.int8();
            stream.skip_c_string();
            Some(MtefObj::Meta)
        }
        REC_SIZE => {
            parse_size(stream);
            Some(MtefObj::SizeMarker)
        }
        REC_FULL | REC_SUB | REC_SUB2 | REC_SYM | REC_SUBSYM => Some(MtefObj::SizeMarker),
        tag if tag >= 15 => {
            let len = stream.mt_uint()?;
            stream.skip_bytes(len);
            None
        }
        _ => None,
    }
}

fn v3_selector_name(selector: i8, var_code: u8) -> &'static str {
    if selector == 15 || selector == 44 {
        return match var_code {
            0 => "tmSUP",
            1 => "tmSUB",
            2 => "tmSUBSUP",
            _ => "tmSUB",
        };
    }
    match selector {
        0 => "tmANGLE",
        1 => "tmPAREN",
        2 => "tmBRACE",
        3 => "tmBRACK",
        4 => "tmBAR",
        5 => "tmDBAR",
        6 => "tmFLOOR",
        7 => "tmCEILING",
        8..=12 => "tmINTERVAL",
        13 => "tmROOT",
        14 => "tmFRACT",
        16 => "tmUBAR",
        17 => "tmOBAR",
        18..=20 => "tmARROW",
        21..=26 => "tmINTEG",
        27..=28 => "tmHBRACE",
        29..=30 => "tmSUM",
        31..=32 => "tmPROD",
        33..=34 => "tmCOPROD",
        35..=36 => "tmUNION",
        37..=38 => "tmINTER",
        39 => "tmLIM",
        40 => "tmLDIV",
        41 => "tmFRACT",
        42 => "tmINTOP",
        43 => "tmSUMOP",
        45 => "tmDIRAC",
        46..=47 => "tmVEC",
        48 => "tmBOX",
        _ => "tmUNKNOWN",
    }
}

fn resolve_v3_variations(selector: i8, var_code: u8) -> Vec<&'static str> {
    match (selector, var_code) {
        (0..=5, 0) => vec!["tvFENCE_L", "tvFENCE_R"],
        (0..=5, 1) => vec!["tvFENCE_L"],
        (0..=5, 2) => vec!["tvFENCE_R"],
        (6..=12, 0) => vec!["tvFENCE_L", "tvFENCE_R"],
        (13, 0) => vec!["tvROOT_SQ"],
        (13, 1) => vec!["tvROOT_NTH"],
        (14, 0) => vec!["tvFR_FULL"],
        (14, 1) => vec!["tvFR_SMALL"],
        (16 | 17, 0) => vec!["tvBAR_SINGLE"],
        (16 | 17, 1) => vec!["tvBAR_DOUBLE"],
        (18, 0) => vec!["tvAR_SINGLE", "tvAR_LEFT", "tvAR_TOP"],
        (18, 1) => vec!["tvAR_SINGLE", "tvAR_LEFT", "tvAR_BOTTOM"],
        (19, 0 | 1) => vec!["tvAR_SINGLE", "tvAR_RIGHT", "tvAR_TOP"],
        (20, 0 | 1) => vec!["tvAR_SINGLE", "tvAR_RIGHT", "tvAR_LEFT", "tvAR_TOP"],
        (21, 0 | 3) => vec!["tvINT_1"],
        (21, 1 | 4) => vec!["tvINT_1", "tvBO_LOWER"],
        (21, 2) => vec!["tvINT_1", "tvBO_LOWER", "tvBO_UPPER"],
        (22, 0 | 3) => vec!["tvINT_2"],
        (22, 1 | 4) => vec!["tvINT_2", "tvBO_LOWER"],
        (22, 2) => vec!["tvINT_2", "tvBO_LOWER", "tvBO_UPPER"],
        (23, 0 | 3) => vec!["tvINT_3"],
        (23, 1 | 4) => vec!["tvINT_3", "tvBO_LOWER"],
        (23, 2) => vec!["tvINT_3", "tvBO_LOWER", "tvBO_UPPER"],
        (24, 0) => vec!["tvINT_1", "tvBO_SUM", "tvBO_LOWER", "tvBO_UPPER"],
        (24, 1 | 2) => vec!["tvINT_1", "tvBO_SUM", "tvBO_LOWER"],
        (25, 0 | 1) => vec!["tvINT_2", "tvBO_SUM", "tvBO_LOWER"],
        (26, 0 | 1) => vec!["tvINT_3", "tvBO_SUM", "tvBO_LOWER"],
        (27, 0) => vec!["tvHB_TOP"],
        (28, 0) => vec!["tvHB_BOT"],
        (29 | 31 | 33 | 35 | 37, 0) => vec!["tvBO_LOWER", "tvBO_SUM"],
        (29 | 31 | 33 | 35 | 37, 1) => vec!["tvBO_LOWER", "tvBO_UPPER", "tvBO_SUM"],
        (29 | 31 | 33 | 35 | 37, 2) => vec!["tvBO_SUM"],
        (30 | 32 | 34 | 36 | 38, 0) => vec!["tvBO_LOWER"],
        (30 | 32 | 34 | 36 | 38, 1) => vec!["tvBO_LOWER", "tvBO_UPPER"],
        (39, 0) => vec!["tvBO_UPPER"],
        (39, 1) => vec!["tvBO_LOWER"],
        (39, 2) => vec!["tvBO_LOWER", "tvBO_UPPER"],
        (40, 0) => vec!["tvLD_UPPER"],
        (41, 0) => vec!["tvFR_SLASH"],
        (41, 1) => vec!["tvFR_SLASH", "tvFR_BASE"],
        (41, 2) => vec!["tvFR_SLASH", "tvFR_SMALL"],
        (42, 0) => vec!["tvBO_LOWER"],
        (42, 1) => vec!["tvBO_UPPER"],
        (42, 2) => vec!["tvBO_LOWER", "tvBO_UPPER"],
        (43, 0) => vec!["tvBO_SUM", "tvBO_LOWER"],
        (43, 1) => vec!["tvBO_SUM", "tvBO_UPPER"],
        (43, 2) => vec!["tvBO_SUM", "tvBO_LOWER", "tvBO_UPPER"],
        (44, _) => vec!["tvSU_PRECEDES"],
        (45, 0) => vec!["tvDI_LEFT", "tvDI_RIGHT"],
        (45, 1) => vec!["tvDI_LEFT"],
        (45, 2) => vec!["tvDI_RIGHT"],
        (46, 0) => vec!["tvVE_UNDER", "tvVE_LEFT"],
        (46, 1) => vec!["tvVE_UNDER", "tvVE_RIGHT"],
        (46, 2) => vec!["tvVE_UNDER", "tvVE_LEFT", "tvVE_RIGHT"],
        (47, 0) => vec!["tvVE_LEFT"],
        (47, 1) => vec!["tvVE_RIGHT"],
        (47, 2) => vec!["tvVE_LEFT", "tvVE_RIGHT"],
        _ => Vec::new(),
    }
}
