//! MTEF v5 record parsing, template selectors, and variation flag decoding.

use super::super::typeface::{TF_TEXT, TF_USER1, TF_USER2};
use super::characters::resolve_no_mt_enc;
use super::consts::*;
use super::parse::{
    halign_name, is_math_obj, parse_eqn_prefs, parse_nudge, parse_ruler, parse_size,
    parse_v5_object_list,
};
use super::stream::ByteStream;
use super::MtefObj;

/// Scan past leading MTEF v5 header/definition records and decode the main equation body.
pub(super) fn parse_v5_equation(stream: &mut ByteStream<'_>) -> Vec<MtefObj> {
    while stream.remaining() > 0 {
        let Some(record_type) = stream.uint8() else {
            break;
        };
        if record_type == REC_END {
            return Vec::new();
        }
        if record_type == REC_MT_COMMENT
            || record_type >= 20
            || record_type == REC_FONT_DEF
            || record_type == REC_ENCODING_DEF
        {
            let _ = parse_v5_record(stream, record_type, false, 0);
            continue;
        }
        if record_type == REC_EQN_PREFS {
            let prefs_pos = stream.position().saturating_sub(1);
            parse_eqn_prefs(stream);
            if stream.remaining() == 0 {
                stream.set_position(prefs_pos + 1);
                stream.rescan_for_equation_body();
            }
            continue;
        }

        stream.unread_byte();
        while stream.remaining() > 0 {
            let Some(body_record_type) = stream.uint8() else {
                break;
            };
            if body_record_type == REC_END {
                continue;
            }
            if body_record_type == REC_LINE {
                if let Some(MtefObj::Line { objects, .. }) =
                    parse_v5_record(stream, REC_LINE, true, 0)
                {
                    if objects.iter().any(is_math_obj) {
                        return objects;
                    }
                }
                continue;
            }
            if matches!(
                body_record_type,
                REC_CHAR | REC_TMPL | REC_PILE | REC_MATRIX
            ) {
                if let Some(object) = parse_v5_record(stream, body_record_type, false, 0) {
                    return vec![object];
                }
                return Vec::new();
            }
            let _ = parse_v5_record(stream, body_record_type, false, 0);
        }
    }
    Vec::new()
}

/// Parse a single MTEF v5 record identified by `record_type`.
pub(super) fn parse_v5_record(
    stream: &mut ByteStream<'_>,
    record_type: u8,
    is_top_line: bool,
    depth: usize,
) -> Option<MtefObj> {
    match record_type {
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
                parse_v5_object_list(stream, depth)
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
            let raw_mt_code = if (opts & OPT_CHAR_NO_MT) == 0 {
                stream.mtef16()?
            } else {
                0
            };
            let enc_char = if (opts & OPT_CHAR_ENC_8) != 0 {
                Some((stream.uint8()? as u16, false))
            } else if (opts & OPT_CHAR_ENC_16) != 0 {
                Some((stream.mtef16()?, true))
            } else {
                None
            };
            let mt_code = if raw_mt_code == 0 && (opts & OPT_CHAR_NO_MT) != 0 {
                enc_char
                    .map(|(code, is_16)| resolve_no_mt_enc(typeface, code, is_16))
                    .unwrap_or(0)
            } else {
                raw_mt_code
            };
            let is_textmode = matches!(typeface, TF_TEXT | TF_USER1 | TF_USER2);
            let mut embells = Vec::new();
            if (opts & OPT_CHAR_EMBELL) != 0 {
                for obj in parse_v5_object_list(stream, depth) {
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
            let subobjects = parse_v5_object_list(stream, depth);
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
            let lines = parse_v5_object_list(stream, depth);
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
            let rows = stream.int8()?.max(0) as usize;
            let cols = stream.int8()?.max(0) as usize;
            let row_part_bytes = (rows + 4) / 4;
            let col_part_bytes = (cols + 4) / 4;
            for _ in 0..row_part_bytes {
                let _ = stream.uint8();
            }
            for _ in 0..col_part_bytes {
                let _ = stream.uint8();
            }
            let cells = parse_v5_object_list(stream, depth);
            Some(MtefObj::Matrix {
                rows,
                cols,
                h_just,
                cells,
            })
        }
        REC_EMBELL => {
            let opts = stream.uint8()?;
            if (opts & OPT_NUDGE) != 0 {
                parse_nudge(stream);
            }
            let code = stream.uint8()?;
            // Represent the embellishment inside a character's embell list using `mt_code = code`.
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
                stream.skip_c_string();
            }
            Some(MtefObj::Meta)
        }
        REC_FONT_DEF => {
            let _ = stream.mt_uint();
            stream.skip_c_string();
            Some(MtefObj::Meta)
        }
        REC_ENCODING_DEF => {
            stream.skip_c_string();
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
        tag if tag >= 20 => {
            let len = stream.mt_uint()?;
            stream.skip_bytes(len);
            None
        }
        _ => None,
    }
}

fn v5_selector_name(selector: i8) -> &'static str {
    match selector {
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

fn resolve_v5_variations(selector: i8, var_code: u16) -> Vec<&'static str> {
    if selector == 14 && (var_code & 0x0004) != 0 && (var_code & 0x0008) != 0 {
        let mut flags = vec!["tvAR_TOPBOTTOM"];
        if (var_code & 0x0002) == 0x0002 {
            flags.push("tvAR_HARPOON");
        }
        if (var_code & 0x0001) == 0x0001 {
            flags.push("tvAR_DOUBLE");
        }
        if (var_code & 0x0010) == 0x0010 {
            flags.push("tvAR_LEFT");
        }
        if (var_code & 0x0020) == 0x0020 {
            flags.push("tvAR_RIGHT");
        }
        if (var_code & 0x0040) == 0x0040 {
            flags.push("tvAR_LOS");
        }
        if (var_code & 0x0080) == 0x0080 {
            flags.push("tvAR_SOL");
        }
        return flags;
    }

    let mut flags = Vec::new();
    fn check_flag(flags: &mut Vec<&'static str>, var_code: u16, flag: i32, name: &'static str) {
        if flag < 0 {
            if (var_code as i32 & (-flag)) == 0 {
                flags.push(name);
            }
        } else if flag == 0 {
            if var_code == 0 {
                flags.push(name);
            }
        } else if (var_code as i32 & flag) == flag {
            flags.push(name);
        }
    }

    if (0..=8).contains(&selector) {
        check_flag(&mut flags, var_code, 0x0001, "tvFENCE_L");
        check_flag(&mut flags, var_code, 0x0002, "tvFENCE_R");
    }
    if selector == 9 {
        match var_code & 0x0003 {
            0 => flags.push("tvINTV_LEFT_LP"),
            1 => flags.push("tvINTV_LEFT_RP"),
            2 => flags.push("tvINTV_LEFT_LB"),
            _ => flags.push("tvINTV_LEFT_RB"),
        }
        match (var_code >> 4) & 0x0003 {
            0 => flags.push("tvINTV_RIGHT_LP"),
            1 => flags.push("tvINTV_RIGHT_RP"),
            2 => flags.push("tvINTV_RIGHT_LB"),
            _ => flags.push("tvINTV_RIGHT_RB"),
        }
    }
    if selector == 10 {
        check_flag(&mut flags, var_code, 0, "tvROOT_SQ");
        check_flag(&mut flags, var_code, 1, "tvROOT_NTH");
    }
    if selector == 11 {
        check_flag(&mut flags, var_code, 0x0001, "tvFR_SMALL");
        check_flag(&mut flags, var_code, 0x0002, "tvFR_SLASH");
        check_flag(&mut flags, var_code, 0x0004, "tvFR_BASE");
    }
    if selector == 12 || selector == 13 {
        check_flag(&mut flags, var_code, 0x0001, "tvBAR_DOUBLE");
    }
    if selector == 14 {
        check_flag(&mut flags, var_code, 0x0000, "tvAR_SINGLE");
        check_flag(&mut flags, var_code, 0x0001, "tvAR_DOUBLE");
        check_flag(&mut flags, var_code, 0x0002, "tvAR_HARPOON");
        check_flag(&mut flags, var_code, 0x0004, "tvAR_TOP");
        check_flag(&mut flags, var_code, 0x0008, "tvAR_BOTTOM");
        check_flag(&mut flags, var_code, 0x0010, "tvAR_LEFT");
        check_flag(&mut flags, var_code, 0x0020, "tvAR_RIGHT");
        check_flag(&mut flags, var_code, 0x0040, "tvAR_LOS");
        check_flag(&mut flags, var_code, 0x0080, "tvAR_SOL");
    }
    if selector == 15 {
        let count_bits = var_code & 0x0003;
        if count_bits == 1 {
            flags.push("tvINT_1");
        } else if count_bits == 2 {
            flags.push("tvINT_2");
        } else if count_bits == 3 {
            flags.push("tvINT_3");
        }
        let loop_bits = var_code & 0x000C;
        if loop_bits == 0x0004 {
            flags.push("tvINT_LOOP");
        } else if loop_bits == 0x0008 {
            flags.push("tvINT_CW_LOOP");
        } else if loop_bits == 0x000C {
            flags.push("tvINT_CCW_LOOP");
        }
        check_flag(&mut flags, var_code, 0x0100, "tvINT_EXPAND");
    }
    if (15..=23).contains(&selector) {
        check_flag(&mut flags, var_code, 0x0010, "tvBO_LOWER");
        check_flag(&mut flags, var_code, 0x0020, "tvBO_UPPER");
        check_flag(&mut flags, var_code, 0x0040, "tvBO_SUM");
        check_flag(&mut flags, var_code, -0x0040, "tvBO_INT");
    }
    if selector == 23 {
        check_flag(&mut flags, var_code, 0, "tvSUBAR");
        check_flag(&mut flags, var_code, 1, "tvDUBAR");
    }
    if selector == 24 || selector == 25 {
        check_flag(&mut flags, var_code, 0x0001, "tvHB_TOP");
    }
    if selector == 26 {
        check_flag(&mut flags, var_code, 0x0001, "tvLD_UPPER");
    }
    if (27..=29).contains(&selector) {
        check_flag(&mut flags, var_code, 0x0001, "tvSU_PRECEDES");
    }
    if selector == 30 {
        check_flag(&mut flags, var_code, 0x0001, "tvDI_LEFT");
        check_flag(&mut flags, var_code, 0x0002, "tvDI_RIGHT");
    }
    if selector == 31 {
        check_flag(&mut flags, var_code, 0x0001, "tvVE_LEFT");
        check_flag(&mut flags, var_code, 0x0002, "tvVE_RIGHT");
        check_flag(&mut flags, var_code, 0x0004, "tvVE_UNDER");
        check_flag(&mut flags, var_code, 0x0008, "tvVE_HARPOON");
    }
    if selector == 36 {
        check_flag(&mut flags, var_code, 0x0001, "tvST_HORIZ");
        check_flag(&mut flags, var_code, 0x0002, "tvST_UP");
        check_flag(&mut flags, var_code, 0x0004, "tvST_DOWN");
    }
    if selector == 37 {
        check_flag(&mut flags, var_code, 0x0001, "tvBX_ROUND");
        check_flag(&mut flags, var_code, 0x0002, "tvBX_LEFT");
        check_flag(&mut flags, var_code, 0x0004, "tvBX_RIGHT");
        check_flag(&mut flags, var_code, 0x0008, "tvBX_TOP");
        check_flag(&mut flags, var_code, 0x0010, "tvBX_BOTTOM");
    }
    flags
}
