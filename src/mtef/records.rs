//! Primitive MTEF records: END / LINE / CHAR / SIZE / TMPL headers.

use super::typeface::{TF_DOUBLESTRUCK, TF_FRAKTUR, TF_FRAKTUR_BOLD, TF_SCRIPT, TF_SCRIPT_BOLD};

/// END record.
pub const END: &[u8] = b"\x00";
/// FULL size record.
pub const FULL: &[u8] = b"\x0a";
/// SUB size record.
pub const SUB: &[u8] = b"\x0b";
/// SUB2 size record.
pub const SUB2: &[u8] = b"\x0c";

/// Return the 8-bit glyph position in MathType's Euclid fonts (`Euclid Fraktur`,
/// `Euclid Math One`, `Euclid Math Two`) when `typeface` is an explicit Euclid
/// font style (`TF_FRAKTUR`, `TF_SCRIPT`, `TF_DOUBLESTRUCK`, etc.).
pub fn explicit_font_position(typeface: u8, mt_code: u32) -> Option<u8> {
    match typeface {
        TF_FRAKTUR | TF_FRAKTUR_BOLD => match mt_code {
            0x212D => Some(b'C'),
            0x210C => Some(b'H'),
            0x2111 => Some(b'I'),
            0x211C => Some(b'R'),
            0x2128 => Some(b'Z'),
            0xF000..=0xF019 => Some(b'A' + (mt_code - 0xF000) as u8),
            0xF01A..=0xF033 => Some(b'a' + (mt_code - 0xF01A) as u8),
            0x41..=0x5A | 0x61..=0x7A => Some(mt_code as u8),
            _ => None,
        },
        TF_SCRIPT | TF_SCRIPT_BOLD => match mt_code {
            0x212C => Some(b'B'),
            0x2130 => Some(b'E'),
            0x2131 => Some(b'F'),
            0x210B => Some(b'H'),
            0x2110 => Some(b'I'),
            0x2112 => Some(b'L'),
            0x2133 => Some(b'M'),
            0x2118 => Some(b'P'),
            0x211B => Some(b'R'),
            0xF100..=0xF119 => Some(b'A' + (mt_code - 0xF100) as u8),
            0x41..=0x5A => Some(mt_code as u8),
            _ => None,
        },
        TF_DOUBLESTRUCK => match mt_code {
            0x2102 => Some(b'C'),
            0x210D => Some(b'H'),
            0x2115 => Some(b'N'),
            0x2119 => Some(b'P'),
            0x211A => Some(b'Q'),
            0x211D => Some(b'R'),
            0x2124 => Some(b'Z'),
            0xF080..=0xF099 => Some(b'A' + (mt_code - 0xF080) as u8),
            0xF0A4 => Some(b'k'),
            0x41..=0x5A | 0x6B => Some(mt_code as u8),
            _ => None,
        },
        _ => None,
    }
}

/// LINE record; `null` marks an empty slot.
pub fn line(null: bool) -> Vec<u8> {
    vec![0x01, u8::from(null)]
}

/// CHAR record. Only the low 16 bits of `unicode_val` are stored (MTCode).
/// When `typeface` refers to an explicit Euclid font (`TF_FRAKTUR`, `TF_SCRIPT`,
/// `TF_DOUBLESTRUCK`), `mtefOPT_CHAR_ENC_CHAR_8` (`0x04`) is set and the 8-bit
/// ASCII glyph position in that font is appended.
pub fn char_record(typeface: u8, unicode_val: u32) -> Vec<u8> {
    if let Some(font_pos) = explicit_font_position(typeface, unicode_val) {
        vec![
            0x02,
            0x04, // mtefOPT_CHAR_ENC_CHAR_8
            typeface.wrapping_add(128),
            (unicode_val & 0xFF) as u8,
            ((unicode_val >> 8) & 0xFF) as u8,
            font_pos,
        ]
    } else {
        vec![
            0x02,
            0x00,
            typeface.wrapping_add(128),
            (unicode_val & 0xFF) as u8,
            ((unicode_val >> 8) & 0xFF) as u8,
        ]
    }
}

/// Explicit display point size (stored value 320).
pub fn size_display() -> Vec<u8> {
    vec![0x09, 0x65, 0x40, 0x01]
}

/// TMPL header.
pub fn tmpl_header(selector: u8, variation: u8, tmpl_opts: u8) -> Vec<u8> {
    vec![0x03, 0x00, selector, variation, tmpl_opts]
}
