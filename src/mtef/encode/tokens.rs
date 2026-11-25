//! Leaf token (`mi`, `mn`, `mo`, `mtext`, `mspace`) encoders for MTEF 5.0.

use crate::mathml::MathNode;
use crate::mtef::typeface::TF_SPACE;
use crate::mtef::{
    char_record, map_char_with_variant, TF_FUNCTION, TF_MTEXTRA, TF_NUMBER, TF_SYMBOL, TF_TEXT,
    TF_VARIABLE, TF_VECTOR,
};
use crate::symbols::operators::ascii_operator_spelling;
use crate::symbols::{has_foreign_letters, is_function};

use super::records::emit_char_with_combining_marks;

/// Encode an `<mi>` identifier or function-name token into MTEF `CHAR` records.
pub(super) fn encode_mi(node: &MathNode) -> Vec<u8> {
    let text = node.text.trim();
    if text.is_empty() {
        return Vec::new();
    }
    let variant = node.attr("mathvariant");
    let is_bold = matches!(variant, Some("bold") | Some("bold-italic"));

    // 1. Known multi-letter function name (`sin`, `cos`, `lim`, `ln`, ...).
    if text.chars().count() > 1 && is_function(text) {
        let typeface = if is_bold { TF_VECTOR } else { TF_FUNCTION };
        let mut out = Vec::new();
        for ch in text.chars() {
            if ch == ' ' {
                out.extend(char_record(TF_SPACE, 0xEB01));
            } else {
                out.extend(char_record(typeface, ch as u32));
            }
        }
        return out;
    }

    // 2. Vietnamese / foreign text run inside `<mi>`.
    if has_foreign_letters(text) {
        let default_tf = if is_bold { TF_VECTOR } else { TF_TEXT };
        let mut out = Vec::new();
        for ch in text.chars() {
            if ch == ' ' {
                out.extend(char_record(TF_TEXT, 0x20));
            } else {
                let (typeface, mt_code) = map_char_with_variant(ch, variant, default_tf);
                out.extend(char_record(typeface, mt_code));
            }
        }
        return out;
    }

    // 3. General `<mi>` run: single variable, combining marks, or compound OMML run.
    let mut out = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut idx = 0;
    while idx < chars.len() {
        let ch = chars[idx];
        if ch.is_whitespace() || ch == '\u{00A0}' {
            idx += 1;
            continue;
        }
        // Check if an ASCII alphabetic sub-run is a known function name (e.g. `"sin"` in `"sinα"`).
        if ch.is_ascii_alphabetic() {
            let start = idx;
            while idx < chars.len() && chars[idx].is_ascii_alphabetic() {
                idx += 1;
            }
            let word: String = chars[start..idx].iter().collect();
            if word.len() > 1 && is_function(&word) {
                let typeface = if is_bold { TF_VECTOR } else { TF_FUNCTION };
                for letter in word.chars() {
                    out.extend(char_record(typeface, letter as u32));
                }
                continue;
            }
            // Otherwise step through each letter so trailing combining marks attach properly.
            idx = start;
        }

        let (typeface, mt_code) = map_char_with_variant(ch, variant, TF_VARIABLE);
        idx += 1;
        out.extend(emit_char_with_combining_marks(
            &chars, &mut idx, typeface, mt_code,
        ));
    }
    out
}

/// Encode an `<mn>` numeric literal into MTEF `CHAR` records.
pub(super) fn encode_mn(node: &MathNode) -> Vec<u8> {
    let variant = node.attr("mathvariant");
    let mut out = Vec::new();
    let chars: Vec<char> = node.text.trim().chars().collect();
    let mut idx = 0;
    while idx < chars.len() {
        let ch = chars[idx];
        if ch.is_whitespace() {
            idx += 1;
            continue;
        }
        let (typeface, mt_code) = if ch == '.' || ch == ',' {
            (TF_SYMBOL, ch as u32)
        } else {
            map_char_with_variant(ch, variant, TF_NUMBER)
        };
        idx += 1;
        out.extend(emit_char_with_combining_marks(
            &chars, &mut idx, typeface, mt_code,
        ));
    }
    out
}

/// Encode an `<mo>` operator token into MTEF `CHAR` records.
pub(super) fn encode_mo(node: &MathNode) -> Vec<u8> {
    let text = node.text.trim();
    if text.is_empty() || text == "\u{2061}" {
        return Vec::new();
    }
    if let Some(codepoint) = ascii_operator_spelling(text) {
        let (typeface, mt_code) =
            map_char_with_variant(char::from_u32(codepoint).unwrap_or('='), None, TF_SYMBOL);
        return char_record(typeface, mt_code);
    }
    let variant = node.attr("mathvariant");
    let chars: Vec<char> = text
        .chars()
        .filter(|ch| !ch.is_whitespace() && *ch != '\u{2061}')
        .collect();
    let mut out = Vec::new();
    let mut idx = 0;
    while idx < chars.len() {
        let ch = chars[idx];
        let (typeface, mt_code) = map_char_with_variant(ch, variant, TF_SYMBOL);
        idx += 1;
        out.extend(emit_char_with_combining_marks(
            &chars, &mut idx, typeface, mt_code,
        ));
    }
    out
}

/// Encode an `<mtext>` prose run into MTEF `CHAR` records.
pub(super) fn encode_mtext(node: &MathNode) -> Vec<u8> {
    let variant = node.attr("mathvariant");
    let is_bold = matches!(variant, Some("bold") | Some("bold-italic"));
    let default_tf = if is_bold { TF_VECTOR } else { TF_TEXT };

    let mut out = Vec::new();
    for ch in node.text.chars() {
        if ch == ' ' || ch == '\u{00A0}' {
            out.extend(char_record(TF_TEXT, 0x20));
        } else if !ch.is_whitespace() {
            let (typeface, mt_code) = map_char_with_variant(ch, variant, default_tf);
            out.extend(char_record(typeface, mt_code));
        }
    }
    out
}

/// Encode an `<mspace>` node using MT Extra space glyph codes.
pub(super) fn encode_mspace(node: &MathNode) -> Vec<u8> {
    let width = crate::symbols::misc::space_width_em(node.attr("width").unwrap_or("0"));
    let code = match width {
        w if (w - 3.0 / 18.0).abs() < 0.00001 => 0xEF02,
        w if (w - 4.0 / 18.0).abs() < 0.00001 => 0xEF03,
        w if (w - 5.0 / 18.0).abs() < 0.00001 => 0xEF04,
        w if (w - 1.0).abs() < 0.00001 => 0xEF05,
        w if (w - 2.0).abs() < 0.00001 => 0xEF06,
        _ => 0xEF08,
    };
    char_record(TF_MTEXTRA, code)
}
