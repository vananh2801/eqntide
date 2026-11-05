//! MTEF typeface constants and character classification.

use super::records::*;

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
/// Regular text (Arial in the preamble).
pub const TF_TEXT: u8 = 1;

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
    if py_isdigit(ch) {
        return (TF_NUMBER, ch as u32);
    }
    if crate::symbols::operators::MTEF_SYMBOL_OPERATORS.contains(ch) || (ch as u32) > 0x2000 {
        return (TF_SYMBOL, ch as u32);
    }
    if ch.is_ascii_alphabetic() {
        return (TF_VARIABLE, ch as u32);
    }
    (TF_SYMBOL, ch as u32)
}

/// Encode a string as a sequence of CHAR records (spaces are skipped).
pub fn chars_from_text(text: &str) -> Vec<u8> {
    let mut out = Vec::new();
    for ch in text.chars() {
        if ch == ' ' {
            continue;
        }
        let mut buf = [0u8; 4];
        let (tf, val) = classify_char(ch.encode_utf8(&mut buf));
        out.extend(char_record(tf, val));
    }
    out
}
