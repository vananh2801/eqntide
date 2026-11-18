//! MTEF typeface constants and character classification.

use super::charmap::map_char_with_variant;
use super::records::*;

/// Regular text (Times New Roman / Arial in the preamble).
pub const TF_TEXT: u8 = 1;
/// `sin`, `cos`, `log` …
pub const TF_FUNCTION: u8 = 2;
/// Italic math variables.
pub const TF_VARIABLE: u8 = 3;
/// α β γ …
pub const TF_LCGREEK: u8 = 4;
/// Α Β Γ …
pub const TF_UCGREEK: u8 = 5;
/// `+ − = ×` …
pub const TF_SYMBOL: u8 = 6;
/// Bold.
pub const TF_VECTOR: u8 = 7;
/// `0–9`.
pub const TF_NUMBER: u8 = 8;
/// User 1 font.
pub const TF_USER1: u8 = 9;
/// User 2 font.
pub const TF_USER2: u8 = 10;
/// MT Extra font (`fnMTEXTRA`).
pub const TF_MTEXTRA: u8 = 11;
/// Expandable characters / fences / arrows (`fnEXPAND`).
pub const TF_EXPAND: u8 = 22;
/// Equation Editor 3.0 space font (`fnSPACE`).
pub const TF_SPACE: u8 = 24;

// ── Explicit Euclid font styles (negative signed typeface: `256 - index`) ────

/// Explicit font style #1 (`-1i8 as u8 = 255`): `Euclid Fraktur` (Plain).
pub const TF_FRAKTUR: u8 = 255;
/// Explicit font style #2 (`-2i8 as u8 = 254`): `Euclid Math One` (Script/Calligraphic Plain).
pub const TF_SCRIPT: u8 = 254;
/// Explicit font style #3 (`-3i8 as u8 = 253`): `Euclid Math Two` (Double-Struck Plain).
pub const TF_DOUBLESTRUCK: u8 = 253;
/// Explicit font style #4 (`-4i8 as u8 = 252`): `Euclid Fraktur` (Bold).
pub const TF_FRAKTUR_BOLD: u8 = 252;
/// Explicit font style #5 (`-5i8 as u8 = 251`): `Euclid Math One` (Script/Calligraphic Bold).
pub const TF_SCRIPT_BOLD: u8 = 251;

/// Approximation of Python's `str.isdigit()` for the characters that matter in
/// math: ASCII digits, super/subscript digits and full-width digits.
pub(crate) fn py_isdigit(c: char) -> bool {
    matches!(c,
        '0'..='9'
        | '\u{00B2}' | '\u{00B3}' | '\u{00B9}'
        | '\u{2070}' | '\u{2074}'..='\u{2079}' | '\u{2080}'..='\u{2089}'
        | '\u{FF10}'..='\u{FF19}')
}

/// Return `(typeface, unicode_val)` for the first character of `text`.
pub fn classify_char(text: &str) -> (u8, u32) {
    let Some(ch) = text.chars().next() else {
        return (TF_VARIABLE, 0x20);
    };
    map_char_with_variant(ch, None, TF_VARIABLE)
}

/// Encode a string as a sequence of CHAR records (spaces are skipped).
pub fn chars_from_text(text: &str) -> Vec<u8> {
    let mut out = Vec::new();
    for ch in text.chars() {
        if ch == ' ' {
            continue;
        }
        let (tf, val) = map_char_with_variant(ch, None, TF_VARIABLE);
        out.extend(char_record(tf, val));
    }
    out
}
