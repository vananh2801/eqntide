//! MTEF (v3 and v5) binary decoder → MathML (`MathNode`).
//!
//! Ports the MTEF record parser, slot/mover logic, character mapping, and
//! template-to-MathML rules from `mathtypejx` into a direct, zero-XSLT Rust
//! decoder producing [`MathNode`].

use crate::mathml::MathNode;
use crate::ole::extract_mtef_from_ole;

use super::typeface::{
    TF_FUNCTION, TF_LCGREEK, TF_MTEXTRA, TF_NUMBER, TF_SYMBOL, TF_TEXT, TF_UCGREEK, TF_USER1,
    TF_USER2, TF_VARIABLE, TF_VECTOR,
};

mod characters;
mod entry;
mod parse;

// ── Record type constants (MTEF v5 / v3) ─────────────────────────────────────

const REC_END: u8 = 0;
const REC_LINE: u8 = 1;
const REC_CHAR: u8 = 2;
const REC_TMPL: u8 = 3;
const REC_PILE: u8 = 4;
const REC_MATRIX: u8 = 5;
const REC_EMBELL: u8 = 6;
const REC_RULER: u8 = 7;
const REC_FONT_STYLE_DEF: u8 = 8; // v5: FONT_STYLE_DEF, v3: FONT
const REC_SIZE: u8 = 9;
const REC_FULL: u8 = 10;
const REC_SUB: u8 = 11;
const REC_SUB2: u8 = 12;
const REC_SYM: u8 = 13;
const REC_SUBSYM: u8 = 14;
const REC_COLOR: u8 = 15;
const REC_COLOR_DEF: u8 = 16;
const REC_FONT_DEF: u8 = 17;
const REC_EQN_PREFS: u8 = 18;
const REC_ENCODING_DEF: u8 = 19;
const REC_MT_COMMENT: u8 = 102;

// ── Option flags ─────────────────────────────────────────────────────────────

const OPT_NUDGE: u8 = 0x08;
const OPT_CHAR_EMBELL: u8 = 0x01;
const OPT_CHAR_ENC_8: u8 = 0x04;
const OPT_CHAR_ENC_16: u8 = 0x10;
const OPT_CHAR_NO_MT: u8 = 0x20;
const OPT_LINE_NULL: u8 = 0x01;
const OPT_LP_RULER: u8 = 0x02;
const OPT_LINE_LSPACE: u8 = 0x04;
const OPT_COLOR_CMYK: u8 = 0x01;
const OPT_COLOR_NAME: u8 = 0x04;

// ── ByteStream ───────────────────────────────────────────────────────────────

#[derive(Clone)]
pub(super) struct ByteStream<'a> {
    data: &'a [u8],
    pos: usize,
    nibble: Option<u8>,
}

impl<'a> ByteStream<'a> {
    pub(super) fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            nibble: None,
        }
    }

    fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    pub(super) fn uint8(&mut self) -> Option<u8> {
        if let Some(low) = self.nibble.take() {
            let high = *self.data.get(self.pos)?;
            self.pos += 1;
            return Some((high << 4) | low);
        }
        let b = *self.data.get(self.pos)?;
        self.pos += 1;
        Some(b)
    }

    fn int8(&mut self) -> Option<i8> {
        self.uint8().map(|b| b as i8)
    }

    fn mtef16(&mut self) -> Option<u16> {
        let lo = self.uint8()? as u16;
        let hi = self.uint8()? as u16;
        Some((hi << 8) | lo)
    }

    fn mt_uint(&mut self) -> Option<usize> {
        let first = self.uint8()?;
        if first < 0xFF {
            Some(first as usize)
        } else {
            self.mtef16().map(|v| v as usize)
        }
    }

    fn nibble(&mut self) -> Option<u8> {
        if let Some(v) = self.nibble.take() {
            return Some(v);
        }
        let byte = *self.data.get(self.pos)?;
        self.pos += 1;
        self.nibble = Some(byte & 0x0F);
        Some((byte >> 4) & 0x0F)
    }

    fn align_byte(&mut self) {
        self.nibble = None;
    }

    fn unread_byte(&mut self) {
        if self.pos > 0 {
            self.pos -= 1;
            self.nibble = None;
        }
    }

    fn skip_bytes(&mut self, n: usize) {
        self.nibble = None;
        self.pos = (self.pos + n).min(self.data.len());
    }
}

// ── Parsed MTEF Record AST ───────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub(super) enum MtefObj {
    Char {
        typeface: u8,
        mt_code: u16,
        is_textmode: bool,
        embells: Vec<u8>,
    },
    Tmpl {
        selector: &'static str,
        variations: Vec<&'static str>,
        subobjects: Vec<MtefObj>,
    },
    Line {
        is_null: bool,
        objects: Vec<MtefObj>,
    },
    Pile {
        halign: &'static str,
        lines: Vec<MtefObj>,
    },
    Matrix {
        rows: usize,
        cols: usize,
        h_just: &'static str,
        cells: Vec<MtefObj>,
    },
    SizeMarker,
    Meta,
}

#[derive(Debug, Clone)]
struct ParsedSlot {
    is_empty_option: bool,
    is_pile: bool,
    pile_halign: &'static str,
    pile_lines: Vec<Vec<MtefObj>>,
    objects: Vec<MtefObj>,
}

// ── Selectors & Variations (v5 and v3) ───────────────────────────────────────

fn v5_selector_name(sel: i8) -> &'static str {
    match sel {
        0 => "tmANGLE",
        1 => "tmPAREN",
        2 => "tmBRACE",
        3 => "tmBRACK",
        4 => "tmBAR",
        5 => "tmDBAR",
        6 => "tmFLOOR",
        7 => "tmCEILING",
        8 => "tmOBRACK",
        9 => "tmINTERVAL",
        10 => "tmROOT",
        11 => "tmFRACT",
        12 => "tmUBAR",
        13 => "tmOBAR",
        14 => "tmARROW",
        15 => "tmINTEG",
        16 => "tmSUM",
        17 => "tmPROD",
        18 => "tmCOPROD",
        19 => "tmUNION",
        20 => "tmINTER",
        21 => "tmINTOP",
        22 => "tmSUMOP",
        23 => "tmLIM",
        24 => "tmHBRACE",
        25 => "tmHBRACK",
        26 => "tmLDIV",
        27 => "tmSUB",
        28 => "tmSUP",
        29 => "tmSUBSUP",
        30 => "tmDIRAC",
        31 => "tmVEC",
        32 => "tmTILDE",
        33 => "tmHAT",
        34 => "tmARC",
        35 => "tmJSTATUS",
        36 => "tmSTRIKE",
        37 => "tmBOX",
        _ => "tmUNKNOWN",
    }
}

fn resolve_v5_variations(sel: i8, var_code: u16) -> Vec<&'static str> {
    if sel == 14 && (var_code & 0x0004) != 0 && (var_code & 0x0008) != 0 {
        let mut res = vec!["tvAR_TOPBOTTOM"];
        if (var_code & 0x0002) == 0x0002 {
            res.push("tvAR_HARPOON");
        }
        if (var_code & 0x0001) == 0x0001 {
            res.push("tvAR_DOUBLE");
        }
        if (var_code & 0x0010) == 0x0010 {
            res.push("tvAR_LEFT");
        }
        if (var_code & 0x0020) == 0x0020 {
            res.push("tvAR_RIGHT");
        }
        if (var_code & 0x0040) == 0x0040 {
            res.push("tvAR_LOS");
        }
        if (var_code & 0x0080) == 0x0080 {
            res.push("tvAR_SOL");
        }
        return res;
    }

    let mut res = Vec::new();
    fn check_flag(res: &mut Vec<&'static str>, var_code: u16, flag: i32, name: &'static str) {
        if flag < 0 {
            if (var_code as i32 & (-flag)) == 0 {
                res.push(name);
            }
        } else if flag == 0 {
            if var_code == 0 {
                res.push(name);
            }
        } else if (var_code as i32 & flag) == flag {
            res.push(name);
        }
    }

    if (0..=8).contains(&sel) {
        check_flag(&mut res, var_code, 0x0001, "tvFENCE_L");
        check_flag(&mut res, var_code, 0x0002, "tvFENCE_R");
    }
    if sel == 9 {
        match var_code & 0x0003 {
            0 => res.push("tvINTV_LEFT_LP"),
            1 => res.push("tvINTV_LEFT_RP"),
            2 => res.push("tvINTV_LEFT_LB"),
            _ => res.push("tvINTV_LEFT_RB"),
        }
        match (var_code >> 4) & 0x0003 {
            0 => res.push("tvINTV_RIGHT_LP"),
            1 => res.push("tvINTV_RIGHT_RP"),
            2 => res.push("tvINTV_RIGHT_LB"),
            _ => res.push("tvINTV_RIGHT_RB"),
        }
    }
    if sel == 10 {
        check_flag(&mut res, var_code, 0, "tvROOT_SQ");
        check_flag(&mut res, var_code, 1, "tvROOT_NTH");
    }
    if sel == 11 {
        check_flag(&mut res, var_code, 0x0001, "tvFR_SMALL");
        check_flag(&mut res, var_code, 0x0002, "tvFR_SLASH");
        check_flag(&mut res, var_code, 0x0004, "tvFR_BASE");
    }
    if sel == 12 || sel == 13 {
        check_flag(&mut res, var_code, 0x0001, "tvBAR_DOUBLE");
    }
    if sel == 14 {
        check_flag(&mut res, var_code, 0x0000, "tvAR_SINGLE");
        check_flag(&mut res, var_code, 0x0001, "tvAR_DOUBLE");
        check_flag(&mut res, var_code, 0x0002, "tvAR_HARPOON");
        check_flag(&mut res, var_code, 0x0004, "tvAR_TOP");
        check_flag(&mut res, var_code, 0x0008, "tvAR_BOTTOM");
        check_flag(&mut res, var_code, 0x0010, "tvAR_LEFT");
        check_flag(&mut res, var_code, 0x0020, "tvAR_RIGHT");
        check_flag(&mut res, var_code, 0x0040, "tvAR_LOS");
        check_flag(&mut res, var_code, 0x0080, "tvAR_SOL");
    }
    if sel == 15 {
        let count_bits = var_code & 0x0003;
        if count_bits == 1 {
            res.push("tvINT_1");
        } else if count_bits == 2 {
            res.push("tvINT_2");
        } else if count_bits == 3 {
            res.push("tvINT_3");
        }
        let loop_bits = var_code & 0x000C;
        if loop_bits == 0x0004 {
            res.push("tvINT_LOOP");
        } else if loop_bits == 0x0008 {
            res.push("tvINT_CW_LOOP");
        } else if loop_bits == 0x000C {
            res.push("tvINT_CCW_LOOP");
        }
        check_flag(&mut res, var_code, 0x0100, "tvINT_EXPAND");
    }
    if (15..=23).contains(&sel) {
        check_flag(&mut res, var_code, 0x0010, "tvBO_LOWER");
        check_flag(&mut res, var_code, 0x0020, "tvBO_UPPER");
        check_flag(&mut res, var_code, 0x0040, "tvBO_SUM");
        check_flag(&mut res, var_code, -0x0040, "tvBO_INT");
    }
    if sel == 23 {
        check_flag(&mut res, var_code, 0, "tvSUBAR");
        check_flag(&mut res, var_code, 1, "tvDUBAR");
    }
    if sel == 24 || sel == 25 {
        check_flag(&mut res, var_code, 0x0001, "tvHB_TOP");
    }
    if sel == 26 {
        check_flag(&mut res, var_code, 0x0001, "tvLD_UPPER");
    }
    if (27..=29).contains(&sel) {
        check_flag(&mut res, var_code, 0x0001, "tvSU_PRECEDES");
    }
    if sel == 30 {
        check_flag(&mut res, var_code, 0x0001, "tvDI_LEFT");
        check_flag(&mut res, var_code, 0x0002, "tvDI_RIGHT");
    }
    if sel == 31 {
        check_flag(&mut res, var_code, 0x0001, "tvVE_LEFT");
        check_flag(&mut res, var_code, 0x0002, "tvVE_RIGHT");
        check_flag(&mut res, var_code, 0x0004, "tvVE_UNDER");
        check_flag(&mut res, var_code, 0x0008, "tvVE_HARPOON");
    }
    if sel == 36 {
        check_flag(&mut res, var_code, 0x0001, "tvST_HORIZ");
        check_flag(&mut res, var_code, 0x0002, "tvST_UP");
        check_flag(&mut res, var_code, 0x0004, "tvST_DOWN");
    }
    if sel == 37 {
        check_flag(&mut res, var_code, 0x0001, "tvBX_ROUND");
        check_flag(&mut res, var_code, 0x0002, "tvBX_LEFT");
        check_flag(&mut res, var_code, 0x0004, "tvBX_RIGHT");
        check_flag(&mut res, var_code, 0x0008, "tvBX_TOP");
        check_flag(&mut res, var_code, 0x0010, "tvBX_BOTTOM");
    }
    res
}

fn v3_selector_name(sel: i8, var_code: u8) -> &'static str {
    if sel == 15 || sel == 44 {
        return match var_code {
            0 => "tmSUP",
            1 => "tmSUB",
            2 => "tmSUBSUP",
            _ => "tmSUB",
        };
    }
    match sel {
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

fn resolve_v3_variations(sel: i8, var_code: u8) -> Vec<&'static str> {
    match (sel, var_code) {
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

fn halign_name(code: i8) -> &'static str {
    match code {
        1 => "left",
        2 => "center",
        3 => "right",
        4 => "al",
        5 => "dec",
        _ => "center",
    }
}

// ── Binary Record Parsers (v5 & v3) ──────────────────────────────────────────

fn parse_nudge(stream: &mut ByteStream<'_>) {
    let sx = stream.int8().unwrap_or(0);
    let sy = stream.int8().unwrap_or(0);
    if sx == -128 && sy == -128 {
        let _ = stream.mtef16();
        let _ = stream.mtef16();
    }
}

fn parse_ruler(stream: &mut ByteStream<'_>) {
    let n_stops = stream.int8().unwrap_or(0).clamp(0, 20);
    for _ in 0..n_stops {
        let _ = stream.int8();
        let _ = stream.mtef16();
    }
}

fn parse_size(stream: &mut ByteStream<'_>) {
    let Some(sel) = stream.int8() else {
        return;
    };
    if sel == 101 {
        let _ = stream.mtef16();
    } else if sel == 100 {
        let _ = stream.uint8();
        let _ = stream.mtef16();
    } else {
        let _ = stream.uint8();
    }
}

fn read_nibble_dim(stream: &mut ByteStream<'_>) {
    if stream.nibble().is_none() {
        return;
    }
    for _ in 0..20 {
        let Some(n) = stream.nibble() else {
            break;
        };
        if n == 0x0F {
            break;
        }
    }
}

fn parse_eqn_prefs(stream: &mut ByteStream<'_>) {
    let _opts = stream.uint8();
    let sizes_count = stream.uint8().unwrap_or(0).min(30);
    for _ in 0..sizes_count {
        read_nibble_dim(stream);
    }
    stream.align_byte();

    let Some(spaces_count) = stream.uint8() else {
        return;
    };
    for _ in 0..spaces_count.min(50) {
        read_nibble_dim(stream);
    }
    stream.align_byte();

    let Some(styles_count) = stream.uint8() else {
        return;
    };
    for _ in 0..styles_count.min(20) {
        let Some(fd) = stream.uint8() else {
            break;
        };
        if fd != 0 {
            let _ = stream.uint8();
        }
    }
}

fn rescan_for_equation_body(stream: &mut ByteStream<'_>) {
    let data = stream.data;
    let end = data.len();
    let pos = stream.pos;
    for i in pos..end.saturating_sub(3).min(pos + 150) {
        if (data[i] & 0x1F) == REC_FULL {
            let next_rt = data.get(i + 1).map(|b| b & 0x1F).unwrap_or(0xFF);
            if next_rt == REC_LINE || next_rt == REC_TMPL {
                stream.pos = i + 1;
                stream.nibble = None;
                return;
            }
        }
    }
    stream.pos = pos.max(end.saturating_sub(50));
    stream.nibble = None;
}

pub(super) fn parse_v5_record(
    stream: &mut ByteStream<'_>,
    rt: u8,
    is_top_line: bool,
    depth: usize,
) -> Option<MtefObj> {
    match rt {
        REC_LINE => {
            let opts = stream.uint8()?;
            if (opts & OPT_NUDGE) != 0 {
                parse_nudge(stream);
            }
            if (opts & OPT_LINE_LSPACE) != 0 {
                let _ = stream.mtef16();
            }
            if (opts & OPT_LP_RULER) != 0 {
                parse_ruler(stream);
            }
            let is_null = (opts & OPT_LINE_NULL) != 0;
            let objects = if !is_null || is_top_line {
                parse::parse_v5_object_list(stream, depth)
            } else {
                Vec::new()
            };
            Some(MtefObj::Line { is_null, objects })
        }
        REC_CHAR => {
            let opts = stream.uint8()?;
            if (opts & OPT_NUDGE) != 0 {
                parse_nudge(stream);
            }
            let tf_signed = stream.int8()?;
            let typeface = (tf_signed as i16 + 128) as u8;
            let mt_code = if (opts & OPT_CHAR_NO_MT) == 0 {
                stream.mtef16()?
            } else {
                0
            };
            if (opts & OPT_CHAR_ENC_8) != 0 {
                let _ = stream.uint8();
            } else if (opts & OPT_CHAR_ENC_16) != 0 {
                let _ = stream.mtef16();
            }
            let is_textmode = matches!(typeface, TF_TEXT | TF_USER1 | TF_USER2);
            let mut embells = Vec::new();
            if (opts & OPT_CHAR_EMBELL) != 0 {
                for obj in parse::parse_v5_object_list(stream, depth) {
                    if let MtefObj::Char {
                        mt_code: e_code, ..
                    } = obj
                    {
                        embells.push(e_code as u8);
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
            let opts = stream.uint8()?;
            if (opts & OPT_NUDGE) != 0 {
                parse_nudge(stream);
            }
            let sel_idx = stream.int8()?;
            let selector = v5_selector_name(sel_idx);
            let var1 = stream.uint8()? as u16;
            let var_code = if (var1 & 0x80) != 0 {
                let var2 = stream.uint8()? as u16;
                (var1 & 0x7F) | (var2 << 8)
            } else {
                var1
            };
            let variations = resolve_v5_variations(sel_idx, var_code);
            let _tmpl_opts = stream.uint8()?;
            let subobjects = parse::parse_v5_object_list(stream, depth);
            Some(MtefObj::Tmpl {
                selector,
                variations,
                subobjects,
            })
        }
        REC_PILE => {
            let opts = stream.uint8()?;
            if (opts & OPT_NUDGE) != 0 {
                parse_nudge(stream);
            }
            let halign = halign_name(stream.int8()?);
            let _valign = stream.int8()?;
            if (opts & OPT_LP_RULER) != 0 {
                parse_ruler(stream);
            }
            let lines = parse::parse_v5_object_list(stream, depth);
            Some(MtefObj::Pile { halign, lines })
        }
        REC_MATRIX => {
            let opts = stream.uint8()?;
            if (opts & OPT_NUDGE) != 0 {
                parse_nudge(stream);
            }
            let _valign = stream.int8()?;
            let h_just = halign_name(stream.int8()?);
            let _v_just = stream.int8()?;
            let n_rows = stream.int8()?.max(0) as usize;
            let n_cols = stream.int8()?.max(0) as usize;
            let n_row_bytes = (n_rows + 4) / 4;
            let n_col_bytes = (n_cols + 4) / 4;
            for _ in 0..n_row_bytes {
                let _ = stream.uint8();
            }
            for _ in 0..n_col_bytes {
                let _ = stream.uint8();
            }
            let cells = parse::parse_v5_object_list(stream, depth);
            Some(MtefObj::Matrix {
                rows: n_rows,
                cols: n_cols,
                h_just,
                cells,
            })
        }
        REC_EMBELL => {
            let _opts = stream.uint8()?;
            let code = stream.uint8()?;
            // Represent embell inside char's embellishment list using mt_code = code
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
            let _ = stream.mt_uint();
            let _ = stream.int8();
            Some(MtefObj::Meta)
        }
        REC_SIZE => {
            parse_size(stream);
            Some(MtefObj::SizeMarker)
        }
        REC_FULL | REC_SUB | REC_SUB2 | REC_SYM | REC_SUBSYM => Some(MtefObj::SizeMarker),
        REC_COLOR => {
            let _ = stream.mt_uint();
            Some(MtefObj::Meta)
        }
        REC_COLOR_DEF => {
            let opts = stream.uint8()?;
            if (opts & OPT_COLOR_CMYK) != 0 {
                for _ in 0..4 {
                    let _ = stream.mtef16();
                }
            } else {
                for _ in 0..3 {
                    let _ = stream.mtef16();
                }
            }
            if (opts & OPT_COLOR_NAME) != 0 {
                while let Some(b) = stream.uint8() {
                    if b == 0 {
                        break;
                    }
                }
            }
            Some(MtefObj::Meta)
        }
        REC_FONT_DEF => {
            let _ = stream.mt_uint();
            while let Some(b) = stream.uint8() {
                if b == 0 {
                    break;
                }
            }
            Some(MtefObj::Meta)
        }
        REC_ENCODING_DEF => {
            while let Some(b) = stream.uint8() {
                if b == 0 {
                    break;
                }
            }
            Some(MtefObj::Meta)
        }
        REC_EQN_PREFS => {
            parse_eqn_prefs(stream);
            Some(MtefObj::Meta)
        }
        REC_MT_COMMENT => {
            let len = stream.mt_uint()?;
            stream.skip_bytes(len);
            None
        }
        r if r >= 20 => {
            let len = stream.mt_uint()?;
            stream.skip_bytes(len);
            None
        }
        _ => None,
    }
}

pub(super) fn parse_v5_equation(stream: &mut ByteStream<'_>) -> Vec<MtefObj> {
    while stream.remaining() > 0 {
        let Some(rt) = stream.uint8() else {
            break;
        };
        if rt == REC_END {
            return Vec::new();
        }
        if rt == REC_MT_COMMENT || rt >= 20 || rt == REC_FONT_DEF || rt == REC_ENCODING_DEF {
            let _ = parse_v5_record(stream, rt, false, 0);
            continue;
        }
        if rt == REC_EQN_PREFS {
            let prefs_pos = stream.pos.saturating_sub(1);
            parse_eqn_prefs(stream);
            if stream.remaining() == 0 {
                stream.pos = prefs_pos + 1;
                rescan_for_equation_body(stream);
            }
            continue;
        }

        stream.unread_byte();
        while stream.remaining() > 0 {
            let Some(rt) = stream.uint8() else {
                break;
            };
            if rt == REC_END {
                continue;
            }
            if rt == REC_LINE {
                if let Some(MtefObj::Line { objects, .. }) =
                    parse_v5_record(stream, REC_LINE, true, 0)
                {
                    if objects.iter().any(parse::is_math_obj) {
                        return objects;
                    }
                }
                continue;
            }
            if matches!(rt, REC_CHAR | REC_TMPL | REC_PILE | REC_MATRIX) {
                if let Some(obj) = parse_v5_record(stream, rt, false, 0) {
                    return vec![obj];
                }
                return Vec::new();
            }
            let _ = parse_v5_record(stream, rt, false, 0);
        }
    }
    Vec::new()
}

pub(super) fn parse_v3_record(
    stream: &mut ByteStream<'_>,
    rt: u8,
    opts: u8,
    depth: usize,
) -> Option<MtefObj> {
    match rt {
        REC_LINE => {
            if (opts & OPT_NUDGE) != 0 {
                parse_nudge(stream);
            }
            if (opts & OPT_LINE_LSPACE) != 0 {
                let _ = stream.mtef16();
            }
            if (opts & OPT_LP_RULER) != 0 {
                parse_ruler(stream);
            }
            let is_null = (opts & OPT_LINE_NULL) != 0;
            let objects = if !is_null {
                parse::parse_v3_object_list(stream, depth)
            } else {
                Vec::new()
            };
            Some(MtefObj::Line { is_null, objects })
        }
        REC_CHAR => {
            if (opts & OPT_NUDGE) != 0 {
                parse_nudge(stream);
            }
            let tf_signed = stream.int8()?;
            let typeface = (tf_signed as i16 + 128) as u8;
            let mt_code = stream.mtef16()?;
            let is_textmode = matches!(typeface, TF_TEXT | TF_USER1 | TF_USER2);
            let mut embells = Vec::new();
            if (opts & OPT_CHAR_EMBELL) != 0 {
                for obj in parse::parse_v3_object_list(stream, depth) {
                    if let MtefObj::Char {
                        mt_code: e_code, ..
                    } = obj
                    {
                        embells.push(e_code as u8);
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
            if (opts & OPT_NUDGE) != 0 {
                parse_nudge(stream);
            }
            let sel_idx = stream.int8()?;
            let var_code = stream.uint8()?;
            let selector = v3_selector_name(sel_idx, var_code);
            let variations = resolve_v3_variations(sel_idx, var_code);
            let _tmpl_opts = stream.uint8()?;
            let subobjects = parse::parse_v3_object_list(stream, depth);
            Some(MtefObj::Tmpl {
                selector,
                variations,
                subobjects,
            })
        }
        REC_PILE => {
            if (opts & OPT_NUDGE) != 0 {
                parse_nudge(stream);
            }
            let halign = halign_name(stream.int8()?);
            let _valign = stream.int8()?;
            if (opts & OPT_LP_RULER) != 0 {
                parse_ruler(stream);
            }
            let lines = parse::parse_v3_object_list(stream, depth);
            Some(MtefObj::Pile { halign, lines })
        }
        REC_MATRIX => {
            if (opts & OPT_NUDGE) != 0 {
                parse_nudge(stream);
            }
            let _valign = stream.int8()?;
            let h_just = halign_name(stream.int8()?);
            let _v_just = stream.int8()?;
            let n_rows = stream.int8()?.max(0) as usize;
            let n_cols = stream.int8()?.max(0) as usize;
            for _ in 0..=n_rows {
                let _ = stream.nibble();
            }
            if !((n_rows + 1) * 2).is_multiple_of(8) {
                stream.align_byte();
            }
            for _ in 0..=n_cols {
                let _ = stream.nibble();
            }
            if !((n_cols + 1) * 2).is_multiple_of(8) {
                stream.align_byte();
            }
            let cells = parse::parse_v3_object_list(stream, depth);
            Some(MtefObj::Matrix {
                rows: n_rows,
                cols: n_cols,
                h_just,
                cells,
            })
        }
        REC_EMBELL => {
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
            while let Some(b) = stream.uint8() {
                if b == 0 {
                    break;
                }
            }
            Some(MtefObj::Meta)
        }
        REC_SIZE => {
            parse_size(stream);
            Some(MtefObj::SizeMarker)
        }
        REC_FULL | REC_SUB | REC_SUB2 | REC_SYM | REC_SUBSYM => Some(MtefObj::SizeMarker),
        r if r >= 15 => {
            let len = stream.mt_uint()?;
            stream.skip_bytes(len);
            None
        }
        _ => None,
    }
}

// ── Slot Grouping & Sub/Sup Mover ────────────────────────────────────────────

fn extract_template_slots(
    selector: &str,
    variations: &[&str],
    subobjects: &[MtefObj],
) -> Vec<ParsedSlot> {
    let mut slots = Vec::new();
    let mut current_slot: Vec<MtefObj> = Vec::new();
    let mut direct_pile_seen = false;

    let flush_slot = |slots: &mut Vec<ParsedSlot>, items: Vec<MtefObj>| {
        if items.is_empty() {
            slots.push(ParsedSlot {
                is_empty_option: true,
                is_pile: false,
                pile_halign: "center",
                pile_lines: Vec::new(),
                objects: Vec::new(),
            });
            return;
        }
        if items.len() == 1 {
            if let MtefObj::Line { is_null, objects } = &items[0] {
                slots.push(ParsedSlot {
                    is_empty_option: *is_null && objects.is_empty(),
                    is_pile: false,
                    pile_halign: "center",
                    pile_lines: Vec::new(),
                    objects: objects.clone(),
                });
                return;
            }
        }
        slots.push(ParsedSlot {
            is_empty_option: false,
            is_pile: false,
            pile_halign: "center",
            pile_lines: Vec::new(),
            objects: items,
        });
    };

    for obj in subobjects {
        match obj {
            MtefObj::SizeMarker => {
                if !current_slot.is_empty() {
                    flush_slot(&mut slots, std::mem::take(&mut current_slot));
                }
            }
            MtefObj::Meta => {}
            MtefObj::Pile { halign, lines } => {
                if !current_slot.is_empty() {
                    flush_slot(&mut slots, std::mem::take(&mut current_slot));
                }
                let mut pile_lines = Vec::new();
                for l in lines {
                    if let MtefObj::Line { objects, .. } = l {
                        pile_lines.push(objects.clone());
                    } else if parse::is_math_obj(l) {
                        pile_lines.push(vec![l.clone()]);
                    }
                }
                slots.push(ParsedSlot {
                    is_empty_option: false,
                    is_pile: true,
                    pile_halign: halign,
                    pile_lines,
                    objects: Vec::new(),
                });
                direct_pile_seen = true;
            }
            MtefObj::Line { .. } => {
                if !current_slot.is_empty() {
                    flush_slot(&mut slots, std::mem::take(&mut current_slot));
                }
                flush_slot(&mut slots, vec![obj.clone()]);
            }
            MtefObj::Char { mt_code, .. } => {
                if is_trailing_single_fence_char(
                    selector,
                    variations,
                    *mt_code,
                    &current_slot,
                    direct_pile_seen,
                ) {
                    continue;
                }
                current_slot.push(obj.clone());
            }
            _ => {
                current_slot.push(obj.clone());
            }
        }
    }
    if !current_slot.is_empty() {
        flush_slot(&mut slots, current_slot);
    }
    slots
}

fn is_trailing_single_fence_char(
    selector: &str,
    variations: &[&str],
    code: u16,
    current_slot: &[MtefObj],
    direct_pile_seen: bool,
) -> bool {
    let has_line_or_pile = direct_pile_seen
        || current_slot
            .iter()
            .any(|o| matches!(o, MtefObj::Line { .. } | MtefObj::Pile { .. }));
    if !has_line_or_pile {
        return false;
    }
    let has_l = variations.contains(&"tvFENCE_L");
    let has_r = variations.contains(&"tvFENCE_R");
    let left_only = has_l && !has_r;
    let right_only = has_r && !has_l;
    let expected_left = match selector {
        "tmPAREN" => Some(0x0028),
        "tmBRACE" => Some(0x007B),
        "tmBRACK" => Some(0x005B),
        _ => None,
    };
    let expected_right = match selector {
        "tmPAREN" => Some(0x0029),
        "tmBRACE" => Some(0x007D),
        "tmBRACK" => Some(0x005D),
        _ => None,
    };
    (left_only && expected_left == Some(code)) || (right_only && expected_right == Some(code))
}

fn is_mt_space_code(code: u16) -> bool {
    matches!(code, 0xEF01..=0xEF06 | 0xEF08)
}

fn is_non_base_code(code: u16) -> bool {
    matches!(code, 0x3001 | 0x3002)
}

fn open_close_pair(open: u16) -> Option<u16> {
    match open {
        0x0028 => Some(0x0029),
        0x005B => Some(0x005D),
        0x007B => Some(0x007D),
        _ => None,
    }
}

fn close_open_pair(close: u16) -> Option<u16> {
    match close {
        0x0029 => Some(0x0028),
        0x005D => Some(0x005B),
        0x007D => Some(0x007B),
        _ => None,
    }
}

fn is_suitable_base(obj: &MtefObj) -> bool {
    match obj {
        MtefObj::Char { mt_code, .. } => !is_mt_space_code(*mt_code) && !is_non_base_code(*mt_code),
        MtefObj::Tmpl {
            selector,
            subobjects,
            ..
        } => {
            if matches!(*selector, "tmSUP" | "tmSUB" | "tmSUBSUP") {
                // If already populated (first Line has non-empty objects), not a candidate
                if let Some(MtefObj::Line {
                    is_null: false,
                    objects,
                }) = subobjects.first()
                {
                    if !objects.is_empty() {
                        return false;
                    }
                }
            }
            true
        }
        MtefObj::Pile { .. } | MtefObj::Matrix { .. } => true,
        _ => false,
    }
}

/// Apply the sub/sup base mover recursively across an object list.
pub(super) fn apply_mover_to_list(objects: &mut Vec<MtefObj>) {
    // 1. Strip 0xEF01 (MT Zero Space) and recurse into children first
    objects.retain(|o| {
        !matches!(
            o,
            MtefObj::Char {
                mt_code: 0xEF01,
                ..
            }
        )
    });

    for obj in objects.iter_mut() {
        match obj {
            MtefObj::Tmpl { subobjects, .. } => apply_mover_to_list(subobjects),
            MtefObj::Line { objects: inner, .. } => apply_mover_to_list(inner),
            MtefObj::Pile { lines, .. } => apply_mover_to_list(lines),
            MtefObj::Matrix { cells, .. } => apply_mover_to_list(cells),
            _ => {}
        }
    }

    // 2. First pass: tmSUP / tmSUB / tmSUBSUP without tvSU_PRECEDES
    let mut idx = 0;
    while idx < objects.len() {
        let is_following_subsup = matches!(
            &objects[idx],
            MtefObj::Tmpl {
                selector: "tmSUP" | "tmSUB" | "tmSUBSUP",
                variations,
                ..
            } if !variations.contains(&"tvSU_PRECEDES")
        );
        if !is_following_subsup {
            idx += 1;
            continue;
        }

        // Find last suitable preceding candidate before `idx`
        let prev_cand = (0..idx).rev().find(|&k| is_suitable_base(&objects[k]));
        if let Some(base_idx) = prev_cand {
            // Check if base_idx is a closing paren `)`, `]`, `}`
            let close_code = match &objects[base_idx] {
                MtefObj::Char { mt_code, .. } => close_open_pair(*mt_code),
                _ => None,
            };
            let base_objs: Vec<MtefObj> = if let Some(open_code) = close_code {
                // Find matching open paren before base_idx
                let open_pos = (0..base_idx).rev().find(|&k| {
                    matches!(&objects[k], MtefObj::Char { mt_code, .. } if *mt_code == open_code)
                });
                if let Some(start) = open_pos {
                    let drained: Vec<MtefObj> = objects.drain(start..=base_idx).collect();
                    idx -= drained.len();
                    drained
                } else {
                    let b = objects.remove(base_idx);
                    idx -= 1;
                    vec![b]
                }
            } else {
                let b = objects.remove(base_idx);
                idx -= 1;
                vec![b]
            };

            if let MtefObj::Tmpl { subobjects, .. } = &mut objects[idx] {
                subobjects.insert(
                    0,
                    MtefObj::Line {
                        is_null: false,
                        objects: base_objs,
                    },
                );
            }
            idx += 1;
        } else {
            // No preceding candidate: mark tvSU_PRECEDES and try following sibling
            let next_cand = ((idx + 1)..objects.len()).find(|&k| is_suitable_base(&objects[k]));
            if let MtefObj::Tmpl { variations, .. } = &mut objects[idx] {
                if !variations.contains(&"tvSU_PRECEDES") {
                    variations.push("tvSU_PRECEDES");
                }
            }
            if let Some(f_idx) = next_cand {
                let b = objects.remove(f_idx);
                if let MtefObj::Tmpl { subobjects, .. } = &mut objects[idx] {
                    subobjects.insert(
                        0,
                        MtefObj::Line {
                            is_null: false,
                            objects: vec![b],
                        },
                    );
                }
            }
            idx += 1;
        }
    }

    // 3. Second pass: tmSUP / tmSUB / tmSUBSUP with tvSU_PRECEDES that don't yet have a base slot
    let mut idx = 0;
    while idx < objects.len() {
        let needs_precedes_base = match &objects[idx] {
            MtefObj::Tmpl {
                selector: "tmSUP" | "tmSUB" | "tmSUBSUP",
                variations,
                subobjects,
            } if variations.contains(&"tvSU_PRECEDES") => {
                // If it already got a non-null Line inserted at 0 in pass 1, skip
                !matches!(
                    subobjects.first(),
                    Some(MtefObj::Line {
                        is_null: false,
                        objects: o
                    }) if !o.is_empty()
                )
            }
            _ => false,
        };
        if !needs_precedes_base {
            idx += 1;
            continue;
        }

        let next_cand = ((idx + 1)..objects.len()).find(|&k| is_suitable_base(&objects[k]));
        if let Some(f_idx) = next_cand {
            let open_code = match &objects[f_idx] {
                MtefObj::Char { mt_code, .. } => open_close_pair(*mt_code),
                _ => None,
            };
            let base_objs: Vec<MtefObj> = if let Some(close_code) = open_code {
                let close_pos = (f_idx + 1..objects.len()).find(|&k| {
                    matches!(&objects[k], MtefObj::Char { mt_code, .. } if *mt_code == close_code)
                });
                if let Some(end) = close_pos {
                    objects.drain(f_idx..=end).collect()
                } else {
                    vec![objects.remove(f_idx)]
                }
            } else {
                vec![objects.remove(f_idx)]
            };
            if let MtefObj::Tmpl { subobjects, .. } = &mut objects[idx] {
                subobjects.insert(
                    0,
                    MtefObj::Line {
                        is_null: false,
                        objects: base_objs,
                    },
                );
            }
        } else if let MtefObj::Tmpl { subobjects, .. } = &mut objects[idx] {
            subobjects.insert(
                0,
                MtefObj::Line {
                    is_null: true,
                    objects: Vec::new(),
                },
            );
        }
        idx += 1;
    }
}

// ── Character & Embellishment Conversion to MathNode ─────────────────────────

fn convert_char_to_mathnode(
    typeface: u8,
    mt_code: u16,
    is_textmode: bool,
    embells: &[u8],
) -> Option<MathNode> {
    let base = map_mt_char_token(typeface, mt_code, is_textmode)?;
    Some(wrap_embellishments(base, embells))
}

fn map_mt_char_token(typeface: u8, mt_code: u16, is_textmode: bool) -> Option<MathNode> {
    // 1. Check MathType Private Use Area (0xE000..=0xF8FF)
    if (0xE000..=0xF8FF).contains(&mt_code) {
        return map_pua_char(mt_code, is_textmode);
    }

    if mt_code == 0 {
        return None;
    }

    let ch = char::from_u32(mt_code as u32)?;
    let s = ch.to_string();

    if is_textmode {
        if mt_code == 0x0020 {
            return Some(MathNode::leaf("mtext", "\u{00A0}"));
        }
        return Some(MathNode::leaf("mtext", &s));
    }

    // Mathmode rules matching chars.py + char.xsl
    match mt_code {
        0x0009 | 0x000A | 0xFEFF | 0x2000..=0x200B => Some(MathNode::leaf("mtext", &s)),
        0x0030..=0x0039 | 0x00BC..=0x00BE | 0x2150..=0x218F => Some(MathNode::leaf("mn", &s)),
        0x0041..=0x005A | 0x0061..=0x007A => {
            let mut node = MathNode::leaf("mi", &s);
            if typeface == TF_VECTOR {
                node.set_attr("mathvariant", "bold");
            } else if typeface == TF_FUNCTION {
                node.set_attr("mathvariant", "normal");
            }
            Some(node)
        }
        0x0028 | 0x0029 | 0x005B..=0x005D => {
            let mut mo = MathNode::leaf("mo", &s);
            mo.set_attr("stretchy", "false");
            Some(mo)
        }
        0x0021..=0x0023
        | 0x0026..=0x0027
        | 0x002A..=0x002F
        | 0x003A..=0x0040
        | 0x005E..=0x0060
        | 0x007B..=0x007E
        | 0x00A0..=0x00BB
        | 0x02C6..=0x036F
        | 0x2010..=0x2027
        | 0x2030..=0x2069
        | 0x2070..=0x209F
        | 0x20D0..=0x214F
        | 0x2190..=0x23FF
        | 0x2500..=0x27EF
        | 0x27F0..=0x2AFF
        | 0x3000..=0x303F => {
            // Note: BMP Letterlike symbols (ℝ, ℤ, ℕ, ℚ, ℂ, ℍ, ℙ, ℒ, ℜ, ℑ) in 0x2100..=0x214F
            // are operators in chars.py except when used as identifiers; in OMML both <mi> and <mo>
            // render as <m:r><m:t>, while <mi mathvariant="normal"> prevents unwanted italicization.
            if matches!(
                mt_code,
                0x2102 | 0x210D | 0x2115 | 0x2119 | 0x211A | 0x211D | 0x2124
            ) {
                return Some(MathNode::leaf("mi", &s));
            }
            Some(MathNode::leaf("mo", &s))
        }
        _ => {
            // Fallback by typeface (char.xsl)
            match typeface {
                TF_FUNCTION | TF_NUMBER => Some(MathNode::leaf("mn", &s)),
                TF_SYMBOL | TF_MTEXTRA => Some(MathNode::leaf("mo", &s)),
                TF_VARIABLE | TF_LCGREEK | TF_UCGREEK | TF_VECTOR => {
                    let mut node = MathNode::leaf("mi", &s);
                    if typeface == TF_VECTOR {
                        node.set_attr("mathvariant", "bold");
                    }
                    Some(node)
                }
                _ => Some(MathNode::leaf("mi", &s)),
            }
        }
    }
}

fn map_pua_char(mt_code: u16, is_textmode: bool) -> Option<MathNode> {
    let text_or_mo = |s: &str| -> MathNode {
        if is_textmode {
            MathNode::leaf("mtext", s)
        } else {
            MathNode::leaf("mo", s)
        }
    };

    match mt_code {
        // Relational / binary operators in MathType PUA
        0xE901 => Some(text_or_mo("\u{2A72}")),
        0xE902 => Some(text_or_mo("\u{2A71}")),
        0xE903 => Some(text_or_mo("\u{2A26}")),
        0xE904 => Some(text_or_mo("\u{2A24}")),
        0xE908 => Some(text_or_mo("\u{2A9C}")),
        0xE909 => Some(text_or_mo("\u{2A9B}")),
        0xE90A => Some(text_or_mo("\u{22FE}")),
        0xE90B => Some(text_or_mo("\u{2287}")),
        0xE90C => Some(text_or_mo("\u{2286}")),
        0xE912 => Some(text_or_mo("\u{22F7}")),
        0xE913 => Some(text_or_mo("\u{2A9A}")),
        0xE914 => Some(text_or_mo("\u{2AC8}")),
        0xE915 => Some(text_or_mo("\u{2AC7}")),
        0xE916 => Some(text_or_mo("\u{2ABE}")),
        0xE917 => Some(text_or_mo("\u{2ABD}")),
        0xE918 => Some(text_or_mo("\u{2A66}")),
        0xE919 => Some(text_or_mo("\u{2A2B}")),
        0xE91A => Some(text_or_mo("\u{2A2C}")),
        0xE91F => Some(text_or_mo("\u{2A6C}")),
        0xE920 => Some(text_or_mo("\u{2A4F}")),
        0xE921 => Some(text_or_mo("\u{2A4E}")),
        0xE922 => Some(text_or_mo("\u{22DA}")),
        0xE924 => Some(text_or_mo("\u{2A6A}")),
        0xE925 => Some(text_or_mo("\u{2A6B}")),
        0xE926 => Some(text_or_mo("\u{2A8B}")),
        0xE927 => Some(text_or_mo("\u{2A8C}")),
        0xE928 | 0xE932 => Some(text_or_mo("\u{2272}")),
        0xE929 | 0xE933 => Some(text_or_mo("\u{2273}")),
        0xE92A => Some(text_or_mo("\u{2ACD}")),
        0xE92B => Some(text_or_mo("\u{2ACE}")),
        0xE92C => Some(text_or_mo("\u{2A67}")),
        0xE92D => Some(text_or_mo("\u{22DB}")),
        0xE93A => Some(text_or_mo("\u{227C}")),
        0xE93B => Some(text_or_mo("\u{227D}")),
        0xE98F => Some(text_or_mo("\u{00B7}")),
        0xEA06 => Some(text_or_mo("\u{2270}")),
        0xEA07 => Some(text_or_mo("\u{2271}")),
        0xEA32 => Some(text_or_mo("\u{2A89}")),
        0xEA33 => Some(text_or_mo("\u{2A8A}")),
        0xEA34 => Some(text_or_mo("\u{2268}")),
        0xEA35 => Some(text_or_mo("\u{2269}")),
        0xEA3A => Some(text_or_mo("\u{22E8}")),
        0xEA3B => Some(text_or_mo("\u{22E9}")),
        0xEA40 => Some(text_or_mo("\u{2AB5}")),
        0xEA41 => Some(text_or_mo("\u{2AB6}")),
        0xEA42 => Some(text_or_mo("\u{2288}")),
        0xEA43 => Some(text_or_mo("\u{2289}")),
        0xEA44 => Some(text_or_mo("\u{228A}")),
        0xEA45 => Some(text_or_mo("\u{228B}")),
        0xEB01 => Some(text_or_mo("\u{2942}")),
        0xEB02 => Some(text_or_mo("\u{2944}")),
        0xEB05 => Some(text_or_mo("\u{2922}")),
        0xEB06 => Some(text_or_mo("\u{2921}")),

        // Differential / Exponential / Imaginary
        0xED01 => Some(MathNode::leaf("mi", "\u{2141}")),
        0xED02 => Some(MathNode::leaf("mi", "\u{0237}")),
        0xED10 => Some(text_or_mo("\u{2146}")),
        0xED11 => Some(text_or_mo("\u{2147}")),
        0xED12 => Some(text_or_mo("\u{2148}")),
        0xED13 => Some(text_or_mo("\u{2149}")),
        0xED16 => Some(text_or_mo("\u{2145}")),

        // Combining wide accents in PUA
        0xEE04..=0xEE06 => {
            let mut mo = MathNode::leaf("mo", "\u{02DC}");
            mo.set_attr("stretchy", "true");
            Some(mo)
        }
        0xEE07..=0xEE09 => {
            let mut mo = MathNode::leaf("mo", "^");
            mo.set_attr("stretchy", "true");
            Some(mo)
        }
        0xEE0A..=0xEE0C => {
            let mut mo = MathNode::leaf("mo", "\u{2322}");
            mo.set_attr("stretchy", "true");
            Some(mo)
        }
        0xEE19 => Some(text_or_mo("\u{2057}")),

        // Spaces (0xEF01..=0xEF08)
        0xEF00 => None, // malignmark
        0xEF01 => None, // zero-width space
        0xEF02..=0xEF08 => {
            let width = match mt_code {
                0xEF02 => "0.166667em",
                0xEF03 => "0.222222em",
                0xEF04 => "0.277778em",
                0xEF05 => "1em",
                0xEF06 => "2em",
                _ => "0.083333em",
            };
            let mut space = MathNode::new("mspace");
            space.set_attr("width", width);
            Some(space)
        }

        // Fraktur Capital A..Z (0xF000..=0xF019)
        0xF000..=0xF019 => {
            let ch = (b'A' + (mt_code - 0xF000) as u8) as char;
            let mut mi = MathNode::leaf("mi", &ch.to_string());
            mi.set_attr("mathvariant", "fraktur");
            Some(mi)
        }
        // Fraktur Small a..z (0xF01A..=0xF033)
        0xF01A..=0xF033 => {
            let ch = (b'a' + (mt_code - 0xF01A) as u8) as char;
            let mut mi = MathNode::leaf("mi", &ch.to_string());
            mi.set_attr("mathvariant", "fraktur");
            Some(mi)
        }
        // Double-Struck Capital A..Z (0xF080..=0xF099)
        0xF080..=0xF099 => {
            let ch = (b'A' + (mt_code - 0xF080) as u8) as char;
            let mut mi = MathNode::leaf("mi", &ch.to_string());
            mi.set_attr("mathvariant", "double-struck");
            Some(mi)
        }
        // Double-Struck Small a..z (0xF09A..=0xF0B3)
        0xF09A..=0xF0B3 => {
            let ch = (b'a' + (mt_code - 0xF09A) as u8) as char;
            let mut mi = MathNode::leaf("mi", &ch.to_string());
            mi.set_attr("mathvariant", "double-struck");
            Some(mi)
        }
        // Double-Struck Digits 0..9 (0xF0C0..=0xF0C9)
        0xF0C0..=0xF0C9 => {
            let ch = (b'0' + (mt_code - 0xF0C0) as u8) as char;
            let mut mn = MathNode::leaf("mn", &ch.to_string());
            mn.set_attr("mathvariant", "double-struck");
            Some(mn)
        }
        // Script Capital A..Z (0xF100..=0xF119)
        0xF100..=0xF119 => {
            let ch = (b'A' + (mt_code - 0xF100) as u8) as char;
            let mut mi = MathNode::leaf("mi", &ch.to_string());
            mi.set_attr("mathvariant", "script");
            Some(mi)
        }
        // Script Small a..z (0xF11A..=0xF133)
        0xF11A..=0xF133 => {
            let ch = (b'a' + (mt_code - 0xF11A) as u8) as char;
            let mut mi = MathNode::leaf("mi", &ch.to_string());
            mi.set_attr("mathvariant", "script");
            Some(mi)
        }

        // Unsupported PUA characters are dropped cleanly
        _ => None,
    }
}

fn wrap_embellishments(mut node: MathNode, embells: &[u8]) -> MathNode {
    for &code in embells {
        node = match code {
            // Over dots / accents
            2 => mover_accent(node, "\u{02D9}"),
            3 => mover_accent(node, "\u{00A8}"),
            4 => mover_accent(node, "\u{20DB}"),
            24 => mover_accent(node, "\u{20DC}"),
            8 => mover_accent(node, "\u{02DC}"),
            9 => mover_accent(node, "\u{02C6}"),
            11 => mover_accent(node, "\u{2192}"),
            12 => mover_accent(node, "\u{2190}"),
            13 => mover_accent(node, "\u{2194}"),
            14 => mover_accent(node, "\u{21C0}"),
            15 => mover_accent(node, "\u{21BC}"),
            17 => mover_accent(node, "\u{00AF}"),
            19 => mover_accent(node, "\u{2322}"),
            20 => mover_accent(node, "\u{2323}"),

            // Primes
            5 => characters::append_prime(node, "'"),
            6 => characters::append_prime(node, "''"),
            18 => characters::append_prime(node, "'''"),
            7 => MathNode::with_children(
                "mmultiscripts",
                vec![
                    node,
                    MathNode::new("mprescripts"),
                    MathNode::new("none"),
                    MathNode::leaf("mo", "\u{2035}"),
                ],
            ),

            // Strikes
            10 | 22 => menclose(node, "updiagonalstrike"),
            16 => menclose(node, "horizontalstrike"),
            21 => menclose(node, "updiagonalstrike downdiagonalstrike"),
            23 => menclose(node, "downdiagonalstrike"),

            // Under dots / accents / arrows
            25 => munder(node, "\u{02D9}", false),
            26 => munder(node, "\u{00A8}", false),
            27 => munder(node, "\u{20DB}", false),
            28 => munder(node, "\u{20DC}", false),
            29 => munder(node, "_", true),
            30 => munder(node, "\u{02DC}", true),
            31 => munder(node, "\u{2322}", true),
            32 => munder(node, "\u{2323}", true),
            33 => munder(node, "\u{2192}", false),
            34 => munder(node, "\u{2190}", false),
            35 => munder(node, "\u{2194}", false),
            36 => munder(node, "\u{21C1}", false),
            37 => munder(node, "\u{21BD}", false),
            _ => node,
        };
    }
    node
}

fn mover_accent(base: MathNode, mo_str: &str) -> MathNode {
    let mut m = MathNode::with_children("mover", vec![base, MathNode::leaf("mo", mo_str)]);
    m.set_attr("accent", "true");
    m
}

fn munder(base: MathNode, mo_str: &str, accentunder: bool) -> MathNode {
    let mut m = MathNode::with_children("munder", vec![base, MathNode::leaf("mo", mo_str)]);
    if accentunder {
        m.set_attr("accentunder", "true");
    }
    m
}

fn menclose(base: MathNode, notation: &str) -> MathNode {
    let mut m = MathNode::with_children("menclose", vec![base]);
    m.set_attr("notation", notation);
    m
}

// ── Template & Record Conversion to MathNode ─────────────────────────────────

fn slot_to_node(slot: Option<&ParsedSlot>) -> MathNode {
    let Some(s) = slot else {
        return MathNode::new("mrow");
    };
    if s.is_pile {
        return build_pile_node(s.pile_halign, &s.pile_lines);
    }
    MathNode::with_children("mrow", convert_objs(&s.objects))
}

fn build_pile_node(halign: &str, lines: &[Vec<MtefObj>]) -> MathNode {
    let mut mtable = MathNode::new("mtable");
    match halign {
        "left" => mtable.set_attr("columnalign", "left"),
        "right" => mtable.set_attr("columnalign", "right"),
        "dec" => mtable.set_attr("groupalign", "decimalpoint"),
        _ => {}
    }
    for line_objs in lines {
        let mrow = MathNode::with_children("mrow", convert_objs(line_objs));
        let mtd = MathNode::with_children("mtd", vec![mrow]);
        let mtr = MathNode::with_children("mtr", vec![mtd]);
        mtable.children.push(mtr);
    }
    mtable
}

pub(super) fn convert_objs(objs: &[MtefObj]) -> Vec<MathNode> {
    let mut out = Vec::new();
    for obj in objs {
        if let Some(n) = convert_obj(obj) {
            out.push(n);
        }
    }
    out
}

fn convert_obj(obj: &MtefObj) -> Option<MathNode> {
    match obj {
        MtefObj::Char {
            typeface,
            mt_code,
            is_textmode,
            embells,
        } => convert_char_to_mathnode(*typeface, *mt_code, *is_textmode, embells),
        MtefObj::Line { objects, .. } => {
            Some(MathNode::with_children("mrow", convert_objs(objects)))
        }
        MtefObj::Pile { halign, lines } => {
            let mut pile_lines = Vec::new();
            for l in lines {
                if let MtefObj::Line { objects, .. } = l {
                    pile_lines.push(objects.clone());
                } else if parse::is_math_obj(l) {
                    pile_lines.push(vec![l.clone()]);
                }
            }
            Some(build_pile_node(halign, &pile_lines))
        }
        MtefObj::Matrix {
            rows,
            cols,
            h_just,
            cells,
        } => Some(convert_matrix(*rows, *cols, h_just, cells)),
        MtefObj::Tmpl {
            selector,
            variations,
            subobjects,
        } => convert_tmpl(selector, variations, subobjects),
        MtefObj::SizeMarker | MtefObj::Meta => None,
    }
}

fn convert_matrix(rows: usize, cols: usize, h_just: &str, cells: &[MtefObj]) -> MathNode {
    let mut cell_nodes = Vec::new();
    for cell in cells {
        match cell {
            MtefObj::Line { objects, .. } => {
                cell_nodes.push(convert_objs(objects));
            }
            MtefObj::Pile { .. } => {
                if let Some(p) = convert_obj(cell) {
                    cell_nodes.push(vec![p]);
                }
            }
            _ => {}
        }
    }

    let mut mtable = MathNode::new("mtable");
    let align = match h_just {
        "left" => Some("left"),
        "right" => Some("right"),
        _ => None,
    };
    if let Some(a) = align {
        mtable.set_attr("columnalign", a);
    }

    let actual_cols = if cols > 0 {
        cols
    } else if rows > 0 && !cell_nodes.is_empty() {
        cell_nodes.len().div_ceil(rows)
    } else {
        1
    };

    for row_chunk in cell_nodes.chunks(actual_cols.max(1)) {
        let mut mtr = MathNode::new("mtr");
        if let Some(a) = align {
            mtr.set_attr("columnalign", a);
        }
        for cell_children in row_chunk {
            let mut mtd = MathNode::with_children("mtd", cell_children.clone());
            if let Some(a) = align {
                mtd.set_attr("columnalign", a);
            }
            mtr.children.push(mtd);
        }
        mtable.children.push(mtr);
    }
    mtable
}

fn convert_tmpl(selector: &str, variations: &[&str], subobjects: &[MtefObj]) -> Option<MathNode> {
    let slots = extract_template_slots(selector, variations, subobjects);
    let has_var = |v: &str| variations.contains(&v);

    match selector {
        // ── Fractions ────────────────────────────────────────────────────────
        "tmFRACT" => {
            let s0 = slot_to_node(slots.first());
            let s1 = slot_to_node(slots.get(1));
            if has_var("tvFR_SLASH") && has_var("tvFR_BASE") {
                return Some(MathNode::with_children(
                    "mrow",
                    vec![s0, MathNode::leaf("mo", "/"), s1],
                ));
            }
            let mut mfrac = MathNode::with_children("mfrac", vec![s0, s1]);
            if has_var("tvFR_SLASH") {
                mfrac.set_attr("bevelled", "true");
            }
            if has_var("tvFR_SMALL") {
                mfrac.set_attr("displaystyle", "false");
                Some(mfrac)
            } else {
                Some(mfrac)
            }
        }

        // ── Roots ────────────────────────────────────────────────────────────
        "tmROOT" => {
            let s0 = slot_to_node(slots.first());
            if has_var("tvROOT_NTH") {
                let s1 = slot_to_node(slots.get(1));
                Some(MathNode::with_children("mroot", vec![s0, s1]))
            } else {
                Some(MathNode::with_children("msqrt", vec![s0]))
            }
        }

        // ── Subscripts / Superscripts ────────────────────────────────────────
        "tmSUP" => {
            let base = slot_to_node(slots.first());
            // Note: builder.py places empty slot[1] for sub and slot[2] for sup,
            // or if only 2 slots exist, slot[1] is sup.
            let sup_idx = if slots.len() >= 3 { 2 } else { 1 };
            let sup = slot_to_node(slots.get(sup_idx));
            if has_var("tvSU_PRECEDES") {
                Some(MathNode::with_children(
                    "mmultiscripts",
                    vec![
                        base,
                        MathNode::new("mprescripts"),
                        MathNode::new("none"),
                        sup,
                    ],
                ))
            } else {
                Some(MathNode::with_children("msup", vec![base, sup]))
            }
        }
        "tmSUB" => {
            let base = slot_to_node(slots.first());
            let sub = slot_to_node(slots.get(1));
            if has_var("tvSU_PRECEDES") {
                Some(MathNode::with_children(
                    "mmultiscripts",
                    vec![
                        base,
                        MathNode::new("mprescripts"),
                        sub,
                        MathNode::new("none"),
                    ],
                ))
            } else {
                Some(MathNode::with_children("msub", vec![base, sub]))
            }
        }
        "tmSUBSUP" => {
            let base = slot_to_node(slots.first());
            let sub = slot_to_node(slots.get(1));
            let sup = slot_to_node(slots.get(2));
            if has_var("tvSU_PRECEDES") {
                Some(MathNode::with_children(
                    "mmultiscripts",
                    vec![base, MathNode::new("mprescripts"), sub, sup],
                ))
            } else {
                Some(MathNode::with_children("msubsup", vec![base, sub, sup]))
            }
        }

        // ── Fences ───────────────────────────────────────────────────────────
        "tmPAREN" | "tmBRACK" | "tmBRACE" | "tmANGLE" | "tmBAR" | "tmDBAR" | "tmFLOOR"
        | "tmCEILING" | "tmOBRACK" => {
            let (open_ch, close_ch) = match selector {
                "tmPAREN" => ("(", ")"),
                "tmBRACK" => ("[", "]"),
                "tmBRACE" => ("{", "}"),
                "tmANGLE" => ("\u{2329}", "\u{232A}"),
                "tmBAR" => ("|", "|"),
                "tmDBAR" => ("\u{2016}", "\u{2016}"),
                "tmFLOOR" => ("\u{230A}", "\u{230B}"),
                "tmCEILING" => ("\u{2308}", "\u{2309}"),
                "tmOBRACK" => ("\u{301A}", "\u{301B}"),
                _ => ("(", ")"),
            };
            let has_l = has_var("tvFENCE_L");
            let has_r = has_var("tvFENCE_R");
            let left_only = has_l && !has_r;
            let right_only = has_r && !has_l;

            let mut children = Vec::new();
            if !right_only {
                children.push(MathNode::leaf("mo", open_ch));
            }
            children.push(slot_to_node(slots.first()));
            if !left_only {
                children.push(MathNode::leaf("mo", close_ch));
            }
            Some(MathNode::with_children("mrow", children))
        }

        "tmINTERVAL" => {
            let left_from_var = if has_var("tvINTV_LEFT_LP") {
                Some("(")
            } else if has_var("tvINTV_LEFT_RP") {
                Some(")")
            } else if has_var("tvINTV_LEFT_LB") {
                Some("[")
            } else if has_var("tvINTV_LEFT_RB") {
                Some("]")
            } else {
                None
            };
            let right_from_var = if has_var("tvINTV_RIGHT_LP") {
                Some("(")
            } else if has_var("tvINTV_RIGHT_RP") {
                Some(")")
            } else if has_var("tvINTV_RIGHT_LB") {
                Some("[")
            } else if has_var("tvINTV_RIGHT_RB") {
                Some("]")
            } else {
                None
            };
            let (open_node, close_node) =
                if let (Some(l), Some(r)) = (left_from_var, right_from_var) {
                    (MathNode::leaf("mo", l), MathNode::leaf("mo", r))
                } else {
                    // Delimiters may be in slot[1] or as trailing chars in subobjects
                    let mut delim_nodes = Vec::new();
                    if let Some(s1) = slots.get(1) {
                        delim_nodes.extend(convert_objs(&s1.objects));
                    }
                    let l = delim_nodes
                        .first()
                        .cloned()
                        .unwrap_or_else(|| MathNode::leaf("mo", "("));
                    let r = delim_nodes
                        .get(1)
                        .cloned()
                        .unwrap_or_else(|| MathNode::leaf("mo", ")"));
                    (l, r)
                };
            let body = slot_to_node(slots.first());
            Some(MathNode::with_children(
                "mrow",
                vec![open_node, body, close_node],
            ))
        }

        "tmDIRAC" => {
            let left = has_var("tvDI_LEFT");
            let right = has_var("tvDI_RIGHT");
            if left && !right {
                Some(MathNode::with_children(
                    "mrow",
                    vec![
                        MathNode::leaf("mo", "\u{2329}"),
                        slot_to_node(slots.first()),
                        MathNode::leaf("mo", "|"),
                    ],
                ))
            } else if right && !left {
                let s = if slots.len() >= 2 {
                    slot_to_node(slots.get(1))
                } else {
                    slot_to_node(slots.first())
                };
                Some(MathNode::with_children(
                    "mrow",
                    vec![
                        MathNode::leaf("mo", "|"),
                        s,
                        MathNode::leaf("mo", "\u{232A}"),
                    ],
                ))
            } else {
                Some(MathNode::with_children(
                    "mrow",
                    vec![
                        MathNode::leaf("mo", "\u{2329}"),
                        slot_to_node(slots.first()),
                        MathNode::leaf("mo", "|"),
                        slot_to_node(slots.get(1)),
                        MathNode::leaf("mo", "\u{232A}"),
                    ],
                ))
            }
        }

        // ── Horizontal braces / brackets ─────────────────────────────────────
        "tmHBRACE" | "tmHBRACK" => {
            let is_top = has_var("tvHB_TOP");
            let s0 = slot_to_node(slots.first());
            let ch = match (selector, is_top) {
                ("tmHBRACE", true) => "\u{FE37}",
                ("tmHBRACE", false) => "\u{FE38}",
                (_, true) => "\u{23B4}",
                (_, false) => "\u{23B5}",
            };
            let mut mo = MathNode::leaf("mo", ch);
            mo.set_attr("stretchy", "true");
            let tag = if is_top { "mover" } else { "munder" };
            let inner = MathNode::with_children(tag, vec![s0, mo]);
            if let Some(s1_slot) = slots.get(1) {
                if !s1_slot.is_empty_option && (!s1_slot.objects.is_empty() || s1_slot.is_pile) {
                    let s1 = slot_to_node(Some(s1_slot));
                    return Some(MathNode::with_children(tag, vec![inner, s1]));
                }
            }
            Some(inner)
        }

        // ── Long embellishments (bars, vec, hat, tilde, arc, strike, box, ldiv)
        "tmTILDE" => Some(characters::mover_stretchy_accent(
            slot_to_node(slots.first()),
            "\u{02DC}",
        )),
        "tmARC" => Some(characters::mover_stretchy_accent(
            slot_to_node(slots.first()),
            "\u{2322}",
        )),
        "tmHAT" => Some(characters::mover_stretchy_accent(
            slot_to_node(slots.first()),
            "^",
        )),
        "tmJSTATUS" => Some(characters::mover_stretchy_accent(
            slot_to_node(slots.first()),
            "\u{FE39}",
        )),
        "tmOBAR" => {
            let base = slot_to_node(slots.first());
            let first = characters::mover_stretchy_accent(base, "\u{00AF}");
            if has_var("tvBAR_DOUBLE") {
                Some(characters::mover_stretchy_accent(first, "\u{00AF}"))
            } else {
                Some(first)
            }
        }
        "tmUBAR" => {
            let base = slot_to_node(slots.first());
            let first = characters::munder_stretchy_accent(base, "_");
            if has_var("tvBAR_DOUBLE") {
                Some(characters::munder_stretchy_accent(first, "_"))
            } else {
                Some(first)
            }
        }
        "tmVEC" => {
            let base = slot_to_node(slots.first());
            let is_under = has_var("tvVE_UNDER");
            let is_left = has_var("tvVE_LEFT");
            let is_right = has_var("tvVE_RIGHT");
            let is_harpoon = has_var("tvVE_HARPOON");
            let arrow = if is_harpoon {
                if is_under {
                    "\u{21C1}"
                } else {
                    "\u{21C0}"
                }
            } else if is_left && is_right {
                "\u{2194}"
            } else if is_left {
                "\u{2190}"
            } else {
                "\u{2192}"
            };
            if is_under {
                Some(characters::munder_stretchy_accent(base, arrow))
            } else {
                Some(characters::mover_stretchy_accent(base, arrow))
            }
        }
        "tmSTRIKE" => {
            let base = slot_to_node(slots.first());
            let h = has_var("tvST_HORIZ");
            let u = has_var("tvST_UP");
            let d = has_var("tvST_DOWN");
            let notation = if h && !u && !d {
                "horizontalstrike"
            } else if u && !h && !d {
                "updiagonalstrike"
            } else if d && !u && !h {
                "downdiagonalstrike"
            } else {
                "updiagonalstrike downdiagonalstrike"
            };
            Some(menclose(base, notation))
        }
        "tmBOX" => {
            let base = slot_to_node(slots.first());
            let round = has_var("tvBX_ROUND");
            let top = has_var("tvBX_TOP");
            let bot = has_var("tvBX_BOTTOM");
            let left = has_var("tvBX_LEFT");
            let right = has_var("tvBX_RIGHT");
            let notation = if round {
                "roundedbox".to_owned()
            } else if (top && bot && left && right) || (!top && !bot && !left && !right) {
                "box".to_owned()
            } else if top && right && !bot && !left {
                "actuarial".to_owned()
            } else {
                let mut sides = Vec::new();
                if top {
                    sides.push("top");
                }
                if bot {
                    sides.push("bottom");
                }
                if left {
                    sides.push("left");
                }
                if right {
                    sides.push("right");
                }
                sides.join(" ")
            };
            Some(menclose(base, &notation))
        }
        "tmLDIV" => {
            let s0 = slot_to_node(slots.first());
            let enc = menclose(s0, "longdiv");
            if has_var("tvLD_UPPER") && slots.len() >= 2 {
                let s1 = slot_to_node(slots.get(1));
                let mut mtd1 = MathNode::with_children("mtd", vec![s1]);
                mtd1.set_attr("columnalign", "right");
                let mut mtd2 = MathNode::with_children("mtd", vec![enc]);
                mtd2.set_attr("columnalign", "right");
                Some(MathNode::with_children(
                    "mtable",
                    vec![
                        MathNode::with_children("mtr", vec![mtd1]),
                        MathNode::with_children("mtr", vec![mtd2]),
                    ],
                ))
            } else {
                Some(enc)
            }
        }

        // ── Limits ───────────────────────────────────────────────────────────
        "tmLIM" => {
            let mut s0 = slot_to_node(slots.first());
            if has_var("tvSUBAR") {
                s0 = characters::munder_stretchy_accent(s0, "\u{00AF}");
            } else if has_var("tvDUBAR") {
                s0 = characters::munder_stretchy_accent(
                    characters::munder_stretchy_accent(s0, "\u{00AF}"),
                    "\u{00AF}",
                );
            }
            let lower = has_var("tvBO_LOWER");
            let upper = has_var("tvBO_UPPER");
            if lower && upper {
                let s1 = slot_to_node(slots.get(1));
                let s2 = slot_to_node(slots.get(2));
                Some(MathNode::with_children("munderover", vec![s0, s1, s2]))
            } else if upper {
                let s2 = slot_to_node(slots.get(2).or_else(|| slots.get(1)));
                let mut accent = MathNode::with_children("mover", vec![s0, s2]);
                if accent.children[1].tag == "mo"
                    && accent.children[1]
                        .text
                        .chars()
                        .next()
                        .is_some_and(crate::symbols::is_accent)
                {
                    accent.set_attr("accent", "true");
                }
                Some(accent)
            } else {
                let s1 = slot_to_node(slots.get(1));
                let mut accent = MathNode::with_children("munder", vec![s0, s1]);
                if accent.children[1].tag == "mo"
                    && accent.children[1]
                        .text
                        .chars()
                        .next()
                        .is_some_and(crate::symbols::is_accent)
                {
                    accent.set_attr("accentunder", "true");
                }
                Some(accent)
            }
        }

        // ── Big Operators (Sum, Prod, Coprod, Union, Inter, SumOp, IntOp) ────
        "tmSUM" | "tmPROD" | "tmCOPROD" | "tmUNION" | "tmINTER" => {
            let op_ch = match selector {
                "tmSUM" => "\u{2211}",
                "tmPROD" => "\u{220F}",
                "tmCOPROD" => "\u{2210}",
                "tmUNION" => "\u{222A}",
                "tmINTER" => "\u{2229}",
                _ => "\u{2211}",
            };
            let op = MathNode::leaf("mo", op_ch);
            let body = slot_to_node(slots.first());
            let lower = has_var("tvBO_LOWER");
            let upper = has_var("tvBO_UPPER");
            let is_sum_style = has_var("tvBO_SUM");

            let op_node = if !lower && !upper {
                op
            } else if lower && !upper {
                let sub = slot_to_node(slots.get(1));
                let tag = if is_sum_style { "munder" } else { "msub" };
                MathNode::with_children(tag, vec![op, sub])
            } else if !lower && upper {
                let sup = slot_to_node(slots.get(2).or_else(|| slots.get(1)));
                let tag = if is_sum_style { "mover" } else { "msup" };
                MathNode::with_children(tag, vec![op, sup])
            } else {
                let sub = slot_to_node(slots.get(1));
                let sup = slot_to_node(slots.get(2));
                let tag = if is_sum_style {
                    "munderover"
                } else {
                    "msubsup"
                };
                MathNode::with_children(tag, vec![op, sub, sup])
            };

            let mut mstyle = MathNode::with_children("mstyle", vec![op_node, body]);
            mstyle.set_attr("displaystyle", "true");
            Some(mstyle)
        }

        "tmSUMOP" | "tmINTOP" => {
            let is_sumop = selector == "tmSUMOP";
            let lower = has_var("tvBO_LOWER");
            let upper = has_var("tvBO_UPPER");
            let op_slot = slot_to_node(slots.get(3).or_else(|| slots.last()));
            let mut op_style = MathNode::with_children("mstyle", vec![op_slot]);
            op_style.set_attr("mathsize", "140%");
            op_style.set_attr("displaystyle", "true");

            let op_wrapped = if lower && !upper {
                let sub = slot_to_node(slots.get(1));
                let tag = if is_sumop { "munder" } else { "msub" };
                MathNode::with_children(tag, vec![op_style, sub])
            } else if upper && !lower {
                let sup = slot_to_node(slots.get(2).or_else(|| slots.get(1)));
                let tag = if is_sumop { "mover" } else { "msup" };
                MathNode::with_children(tag, vec![op_style, sup])
            } else if lower && upper {
                let sub = slot_to_node(slots.get(1));
                let sup = slot_to_node(slots.get(2));
                let tag = if is_sumop { "munderover" } else { "msubsup" };
                MathNode::with_children(tag, vec![op_style, sub, sup])
            } else {
                op_style
            };

            if let Some(first_slot) = slots.first() {
                if !first_slot.is_empty_option && !first_slot.objects.is_empty() {
                    let body = slot_to_node(Some(first_slot));
                    return Some(MathNode::with_children("mrow", vec![op_wrapped, body]));
                }
            }
            Some(op_wrapped)
        }

        // ── Integrals ────────────────────────────────────────────────────────
        "tmINTEG" => {
            let is_2 = has_var("tvINT_2");
            let is_3 = has_var("tvINT_3");
            let is_loop = has_var("tvINT_LOOP");
            let is_cw = has_var("tvINT_CW_LOOP");
            let is_ccw = has_var("tvINT_CCW_LOOP");

            let int_ch = if is_ccw {
                "\u{2233}"
            } else if is_cw {
                "\u{2232}"
            } else if is_loop && is_3 {
                "\u{2230}"
            } else if is_loop && is_2 {
                "\u{222F}"
            } else if is_loop {
                "\u{222E}"
            } else if is_3 {
                "\u{222D}"
            } else if is_2 {
                "\u{222C}"
            } else {
                "\u{222B}"
            };

            let mut op = MathNode::leaf("mo", int_ch);
            if has_var("tvINT_EXPAND") {
                op.set_attr("stretchy", "true");
            }
            let body = slot_to_node(slots.first());
            let lower = has_var("tvBO_LOWER");
            let upper = has_var("tvBO_UPPER");
            let is_sum_style = has_var("tvBO_SUM");

            let op_node = if !lower && !upper {
                op
            } else if lower && !upper {
                let sub = slot_to_node(slots.get(1));
                let tag = if is_sum_style { "munder" } else { "msub" };
                MathNode::with_children(tag, vec![op, sub])
            } else if !lower && upper {
                let sup = slot_to_node(slots.get(2).or_else(|| slots.get(1)));
                let tag = if is_sum_style { "mover" } else { "msup" };
                MathNode::with_children(tag, vec![op, sup])
            } else {
                let sub = slot_to_node(slots.get(1));
                let sup = slot_to_node(slots.get(2));
                let tag = if is_sum_style {
                    "munderover"
                } else {
                    "msubsup"
                };
                MathNode::with_children(tag, vec![op, sub, sup])
            };

            let mrow = MathNode::with_children("mrow", vec![op_node, body]);
            let mut mstyle = MathNode::with_children("mstyle", vec![mrow]);
            mstyle.set_attr("displaystyle", "true");
            Some(mstyle)
        }

        // ── Arrows ───────────────────────────────────────────────────────────
        "tmARROW" => {
            let is_left = has_var("tvAR_LEFT");
            let is_right = has_var("tvAR_RIGHT");
            let is_double = has_var("tvAR_DOUBLE");
            let is_harpoon = has_var("tvAR_HARPOON");
            let is_topbottom = has_var("tvAR_TOPBOTTOM");
            let is_bottom = has_var("tvAR_BOTTOM");

            let arrow_ch = if is_harpoon {
                "\u{21CC}"
            } else if is_double {
                "\u{21C4}"
            } else if is_left && is_right {
                "\u{2194}"
            } else if is_left {
                "\u{2190}"
            } else {
                "\u{2192}"
            };
            let op = MathNode::leaf("mo", arrow_ch);
            if is_topbottom {
                let s0 = slot_to_node(slots.first());
                let s1 = slot_to_node(slots.get(1));
                Some(MathNode::with_children("munderover", vec![op, s1, s0]))
            } else if is_bottom {
                let s1 = slot_to_node(slots.get(1).or_else(|| slots.first()));
                Some(MathNode::with_children("munder", vec![op, s1]))
            } else {
                let s0 = slot_to_node(slots.first());
                Some(MathNode::with_children("mover", vec![op, s0]))
            }
        }

        // ── Fallback ─────────────────────────────────────────────────────────
        _ => {
            let children: Vec<MathNode> = slots.iter().map(|s| slot_to_node(Some(s))).collect();
            Some(MathNode::with_children("mrow", children))
        }
    }
}

/// Convert raw MTEF bytes (after stripping the 28-byte OLE header, or extracted
/// from a WMF comment) into a [`MathNode`] `<math>` tree.
pub fn mtef_to_mathml(mtef_data: &[u8]) -> Option<MathNode> {
    entry::decode_to_mathml(mtef_data)
}

/// Extract MTEF from a MathType OLE binary (`.bin`) and decode it into a [`MathNode`] tree.
pub fn ole_to_mathml(ole_data: &[u8]) -> Option<MathNode> {
    let mtef = extract_mtef_from_ole(ole_data)?;
    mtef_to_mathml(&mtef)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_encoder_output_to_a_math_root() {
        let source = MathNode::parse(
            r#"<math><mrow><mi>x</mi><mo>+</mo><mfrac><mn>1</mn><mi>y</mi></mfrac></mrow></math>"#,
        )
        .unwrap();
        let records = super::super::encode::mathml_to_mtef_bytes(&source).unwrap();
        let bytes = super::super::build_mtef(&records);

        let decoded = mtef_to_mathml(&bytes).unwrap();
        let xml = decoded.to_xml();
        assert!(xml.starts_with("<math"));
        assert!(xml.contains("<mi>x</mi>"));
        assert!(xml.contains("<mfrac>"));
    }
}
