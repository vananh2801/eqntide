//! Unicode ↔ MathType MTCode character mapping and typeface classification.
//!
//! MathType 5 stores character codes as 16-bit `MTCode` values alongside a
//! typeface slot (`TF_TEXT`, `TF_FUNCTION`, `TF_VARIABLE`, `TF_LCGREEK`,
//! `TF_UCGREEK`, `TF_SYMBOL`, `TF_VECTOR`, `TF_NUMBER`) or an explicit
//! `FONT_STYLE_DEF` reference (`TF_FRAKTUR`, `TF_SCRIPT`, `TF_DOUBLESTRUCK`).
//!
//! Because `MTCode` is 16-bit:
//! - Supplementary Mathematical Alphanumeric Symbols (`U+1D400..=U+1D7FF`) must
//!   be normalized to their 16-bit `MTCode` equivalents (ASCII, Greek, BMP
//!   Letterlike Symbols `U+2100..=U+214F`, or MathType Private Use Area codes
//!   `0xF000..=0xF133`) with the corresponding typeface (`Euclid Fraktur`,
//!   `Euclid Math One`, `Euclid Math Two`, or built-in style fallback for
//!   characters not present in MathType's Euclid fonts).
//! - Certain BMP operators, differentials, and spacing characters map to
//!   MathType's Private Use Area (`0xE900..=0xEF08`) as defined in
//!   `MathType_MTCode.xml` and `char.xsl`.

use super::typeface::{
    py_isdigit, TF_DOUBLESTRUCK, TF_FRAKTUR, TF_FRAKTUR_BOLD, TF_FUNCTION, TF_LCGREEK, TF_MTEXTRA,
    TF_NUMBER, TF_SCRIPT, TF_SCRIPT_BOLD, TF_SYMBOL, TF_TEXT, TF_UCGREEK, TF_VARIABLE, TF_VECTOR,
};

/// Map an uppercase ASCII letter `'A'..='Z'` to its MathType Double-Struck
/// (Blackboard Bold) `MTCode`.
///
/// Letters present in the Unicode BMP Letterlike Symbols block use their
/// standard `U+21xx` code points; the remaining letters use `0xF080..=0xF099`.
pub fn double_struck_mtcode_upper(ch: char) -> u32 {
    match ch {
        'C' => 0x2102, // ℂ
        'H' => 0x210D, // ℍ
        'N' => 0x2115, // ℕ
        'P' => 0x2119, // ℙ
        'Q' => 0x211A, // ℚ
        'R' => 0x211D, // ℝ
        'Z' => 0x2124, // ℤ
        'A'..='Z' => 0xF080 + (ch as u32 - 'A' as u32),
        _ => ch as u32,
    }
}

/// Map a lowercase ASCII letter `'a'..='z'` to its MathType Double-Struck `MTCode`.
pub fn double_struck_mtcode_lower(ch: char) -> u32 {
    match ch {
        'a'..='z' => 0xF09A + (ch as u32 - 'a' as u32),
        _ => ch as u32,
    }
}

/// Map an ASCII digit `'0'..='9'` to its MathType Double-Struck `MTCode`.
pub fn double_struck_mtcode_digit(ch: char) -> u32 {
    match ch {
        '0'..='9' => 0xF0C0 + (ch as u32 - '0' as u32),
        _ => ch as u32,
    }
}

/// Map an uppercase ASCII letter `'A'..='Z'` to its MathType Fraktur `MTCode`.
///
/// Letters present in the Unicode BMP Letterlike Symbols block use their
/// standard `U+21xx` code points; the remaining letters use `0xF000..=0xF019`.
pub fn fraktur_mtcode_upper(ch: char) -> u32 {
    match ch {
        'C' => 0x212D, // ℭ
        'H' => 0x210C, // ℌ
        'I' => 0x2111, // ℑ
        'R' => 0x211C, // ℜ
        'Z' => 0x2128, // ℨ
        'A'..='Z' => 0xF000 + (ch as u32 - 'A' as u32),
        _ => ch as u32,
    }
}

/// Map a lowercase ASCII letter `'a'..='z'` to its MathType Fraktur `MTCode`.
pub fn fraktur_mtcode_lower(ch: char) -> u32 {
    match ch {
        'a'..='z' => 0xF01A + (ch as u32 - 'a' as u32),
        _ => ch as u32,
    }
}

/// Map an uppercase ASCII letter `'A'..='Z'` to its MathType Script/Calligraphic `MTCode`.
///
/// Letters present in the Unicode BMP Letterlike Symbols block use their
/// standard `U+21xx` code points; the remaining letters use `0xF100..=0xF119`.
pub fn script_mtcode_upper(ch: char) -> u32 {
    match ch {
        'B' => 0x212C, // ℬ
        'E' => 0x2130, // ℰ
        'F' => 0x2131, // ℱ
        'H' => 0x210B, // ℋ
        'I' => 0x2110, // ℐ
        'L' => 0x2112, // ℒ
        'M' => 0x2133, // ℳ
        'P' => 0x2118, // ℘
        'R' => 0x211B, // ℛ
        'A'..='Z' => 0xF100 + (ch as u32 - 'A' as u32),
        _ => ch as u32,
    }
}

/// Map a lowercase ASCII letter `'a'..='z'` to its MathType Script/Calligraphic `MTCode`.
pub fn script_mtcode_lower(ch: char) -> u32 {
    match ch {
        'e' => 0x212F, // ℯ
        'g' => 0x210A, // ℊ
        'l' => 0x2113, // ℓ
        'o' => 0x2134, // ℴ
        'a'..='z' => 0xF11A + (ch as u32 - 'a' as u32),
        _ => ch as u32,
    }
}

/// Map a BMP Unicode code point to its MathType `MTCode` if MathType uses a
/// Private Use Area or canonical code point for it.
pub fn unicode_to_mtcode(cp: u32) -> u32 {
    match cp {
        // ASCII hyphen-minus in operator context is handled by caller; keep ASCII by default
        // Differential / Exponential / Imaginary symbols (char.xsl)
        0x2145 => 0xED16, // ⅅ CapitalDifferentialD
        0x2146 => 0xED10, // ⅆ DifferentialD
        0x2147 => 0xED11, // ⅇ ExponentialE
        0x2148 => 0xED12, // ⅈ ImaginaryI
        0x2149 => 0xED13, // ⅉ ImaginaryJ
        0x2141 => 0xED01, // ⅁ Turned sans-serif capital G

        // MathType spaces (char.xsl / MathType_MTCode.xml)
        0x200B => 0xEF01,          // Zero-width space
        0x2009 => 0xEF02,          // Thin space
        0x2005 | 0x205F => 0xEF03, // Medium mathematical space
        0x2004 => 0xEF04,          // Thick space (3-per-em)
        0x2003 => 0xEF05,          // Em space
        0x2002 | 0x200A => 0xEF08, // En / hair / 1-point space

        // Extended relational and binary operators (char.xsl / MathType_MTCode.xml)
        0x2A72 => 0xE901, // ⩲ Equals with plus above
        0x2A71 => 0xE902, // ⩱ Equals with plus below
        0x2A26 => 0xE903, // ⨦ Tilde with plus above
        0x2A24 => 0xE904, // ⨤ Tilde with plus below
        0x2A9C => 0xE908, // ⪜
        0x2A9B => 0xE909, // ⪛
        0x22FE => 0xE90A, // ⋾
        0x22F7 => 0xE912, // ⋷
        0x2A9A => 0xE913, // ⪚
        0x2AC8 => 0xE914, // ⫈
        0x2AC7 => 0xE915, // ⫇
        0x2ABE => 0xE916, // ⪾
        0x2ABD => 0xE917, // ⪽
        0x2A66 => 0xE918, // ⩦
        0x2A2B => 0xE919, // ⨫
        0x2A2C => 0xE91A, // ⨬
        0x2A6C => 0xE91F, // ⩬
        0x2A4F => 0xE920, // ⩏
        0x2A4E => 0xE921, // ⩎
        0x22DA => 0xE922, // ⋚ Less-than equal to or greater-than
        0x2A6A => 0xE924, // ⩪
        0x2A6B => 0xE925, // ⩫
        0x2A8B => 0xE926, // ⪋ Less-than double-line equal to or greater-than
        0x2A8C => 0xE927, // ⪌ Greater-than double-line equal to or less-than
        0x2272 => 0xE928, // ≲ Less-than or equivalent to
        0x2273 => 0xE929, // ≳ Greater-than or equivalent to
        0x2ACD => 0xE92A, // ⫍
        0x2ACE => 0xE92B, // ⫎
        0x2A67 => 0xE92C, // ⩧
        0x22DB => 0xE92D, // ⋛ Greater-than equal to or less-than
        0x2981 => 0xE98F, // ⦁ Medium dot operator
        0x2270 => 0xEA06, // ≰ Neither less-than nor equal to
        0x2271 => 0xEA07, // ≱ Neither greater-than nor equal to
        0x2A89 => 0xEA32, // ⪉ Less-than and not approximate
        0x2A8A => 0xEA33, // ⪊ Greater-than and not approximate
        0x2268 => 0xEA34, // ≨ Less-than but not equal to
        0x2269 => 0xEA35, // ≩ Greater-than but not equal to
        0x22E8 => 0xEA3A, // ⋨ Precedes but not equivalent to
        0x22E9 => 0xEA3B, // ⋩ Succeeds but not equivalent to
        0x2AB5 => 0xEA40, // ⪵ Precedes above not equal to
        0x2AB6 => 0xEA41, // ⪶ Succeeds above not equal to
        0x2288 => 0xEA42, // ⊈ Neither a subset of nor equal to
        0x2289 => 0xEA43, // ⊉ Neither a superset of nor equal to
        0x228A => 0xEA44, // ⊊ Subset of with not equal to
        0x228B => 0xEA45, // ⊋ Superset of with not equal to
        0x2942 => 0xEB01, // ⥂ Rightwards arrow above short leftwards arrow
        0x2944 => 0xEB02, // ⥄ Short rightwards arrow above leftwards arrow
        0x2922 => 0xEB05, // ⤢ North east and south west arrow
        0x2921 => 0xEB06, // ⤡ North west and south east arrow
        0x2057 => 0xEE19, // ⁗ Quadruple prime

        other => other,
    }
}

/// Normalize a Greek offset (0..=31) from the Unicode Mathematical Alphanumeric
/// Symbols block to `(typeface, unicode_cp)`.
fn map_math_greek_offset(offset: u32, is_upper: bool) -> (u8, u32) {
    if is_upper {
        // Offsets 0..=24 are Α..=Ω (with offset 17 = ϴ -> Θ 0x0398); offset 25 is Nabla ∇ (0x2207)
        if offset == 25 {
            (TF_SYMBOL, 0x2207)
        } else if offset == 17 {
            (TF_UCGREEK, 0x0398)
        } else {
            (TF_UCGREEK, 0x0391 + offset)
        }
    } else {
        // Offsets 0..=24 are α..=ω; 25 is ∂ (0x2202); 26..=31 are ϵ, ϑ, ϰ, ϕ, ϱ, ϖ
        match offset {
            0..=24 => (TF_LCGREEK, 0x03B1 + offset),
            25 => (TF_SYMBOL, 0x2202),  // ∂
            26 => (TF_LCGREEK, 0x03F5), // ϵ
            27 => (TF_LCGREEK, 0x03D1), // ϑ
            28 => (TF_LCGREEK, 0x03F0), // ϰ
            29 => (TF_LCGREEK, 0x03D5), // ϕ
            30 => (TF_LCGREEK, 0x03F1), // ϱ
            31 => (TF_LCGREEK, 0x03D6), // ϖ
            _ => (TF_LCGREEK, 0x03B1),
        }
    }
}

/// Decompose a supplementary Mathematical Alphanumeric Symbol (`U+1D400..=U+1D7FF`)
/// or BMP Letterlike Symbol (`U+2100..=U+214F`) into `(typeface, mt_code)`.
pub fn map_supplementary_math_char(cp: u32) -> Option<(u8, u32)> {
    match cp {
        // BMP Letterlike exceptions for Planck constant ℎ (`U+210E`)
        0x210E => Some((TF_VARIABLE, 'h' as u32)),
        0x210F => Some((TF_MTEXTRA, 0x210F)), // ℏ in MT Extra

        // BMP Double-Struck letters (ℂ, ℍ, ℕ, ℙ, ℚ, ℝ, ℤ) -> Euclid Math Two
        0x2102 | 0x210D | 0x2115 | 0x2119 | 0x211A | 0x211D | 0x2124 => Some((TF_DOUBLESTRUCK, cp)),

        // BMP Fraktur letters (ℌ, ℑ, ℜ, ℨ, ℭ) -> Euclid Fraktur
        0x210C | 0x2111 | 0x211C | 0x2128 | 0x212D => Some((TF_FRAKTUR, cp)),

        // BMP Script uppercase letters (ℋ, ℐ, ℒ, ℘, ℛ, ℬ, ℰ, ℱ, ℳ) -> Euclid Math One
        0x210B | 0x2110 | 0x2112 | 0x2118 | 0x211B | 0x212C | 0x2130 | 0x2131 | 0x2133 => {
            Some((TF_SCRIPT, cp))
        }
        // BMP Script small ℓ (U+2113) -> MT Extra
        0x2113 => Some((TF_MTEXTRA, 0x2113)),
        // BMP Script small ℊ, ℯ, ℴ -> Euclid Math One lacks lowercase; fallback to italic variable
        0x210A => Some((TF_VARIABLE, 'g' as u32)),
        0x212F => Some((TF_VARIABLE, 'e' as u32)),
        0x2134 => Some((TF_VARIABLE, 'o' as u32)),

        // Mathematical Bold Capital A..=Z
        0x1D400..=0x1D419 => Some((TF_VECTOR, 'A' as u32 + (cp - 0x1D400))),
        // Mathematical Bold Small a..=z
        0x1D41A..=0x1D433 => Some((TF_VECTOR, 'a' as u32 + (cp - 0x1D41A))),
        // Mathematical Italic Capital A..=Z
        0x1D434..=0x1D44D => Some((TF_VARIABLE, 'A' as u32 + (cp - 0x1D434))),
        // Mathematical Italic Small a..=z
        0x1D44E..=0x1D467 => Some((TF_VARIABLE, 'a' as u32 + (cp - 0x1D44E))),
        // Mathematical Bold Italic Capital A..=Z
        0x1D468..=0x1D481 => Some((TF_VECTOR, 'A' as u32 + (cp - 0x1D468))),
        // Mathematical Bold Italic Small a..=z
        0x1D482..=0x1D49B => Some((TF_VECTOR, 'a' as u32 + (cp - 0x1D482))),

        // Mathematical Script Capital A..=Z -> Euclid Math One
        0x1D49C..=0x1D4B5 => {
            let ch = char::from_u32('A' as u32 + (cp - 0x1D49C)).unwrap_or('A');
            Some((TF_SCRIPT, script_mtcode_upper(ch)))
        }
        // Mathematical Script Small a..=z -> 'l' in MT Extra, others fallback to TF_VARIABLE
        0x1D4B6..=0x1D4CF => {
            let ch = char::from_u32('a' as u32 + (cp - 0x1D4B6)).unwrap_or('a');
            if ch == 'l' {
                Some((TF_MTEXTRA, 0x2113))
            } else {
                Some((TF_VARIABLE, ch as u32))
            }
        }
        // Mathematical Bold Script Capital A..=Z -> Euclid Math One Bold
        0x1D4D0..=0x1D4E9 => {
            let ch = char::from_u32('A' as u32 + (cp - 0x1D4D0)).unwrap_or('A');
            Some((TF_SCRIPT_BOLD, script_mtcode_upper(ch)))
        }
        // Mathematical Bold Script Small a..=z
        0x1D4EA..=0x1D503 => {
            let ch = char::from_u32('a' as u32 + (cp - 0x1D4EA)).unwrap_or('a');
            Some((TF_VECTOR, ch as u32))
        }

        // Mathematical Fraktur Capital A..=Z -> Euclid Fraktur
        0x1D504..=0x1D51D => {
            let ch = char::from_u32('A' as u32 + (cp - 0x1D504)).unwrap_or('A');
            Some((TF_FRAKTUR, fraktur_mtcode_upper(ch)))
        }
        // Mathematical Fraktur Small a..=z -> Euclid Fraktur
        0x1D51E..=0x1D537 => {
            let ch = char::from_u32('a' as u32 + (cp - 0x1D51E)).unwrap_or('a');
            Some((TF_FRAKTUR, fraktur_mtcode_lower(ch)))
        }

        // Mathematical Double-Struck Capital A..=Z -> Euclid Math Two
        0x1D538..=0x1D551 => {
            let ch = char::from_u32('A' as u32 + (cp - 0x1D538)).unwrap_or('A');
            Some((TF_DOUBLESTRUCK, double_struck_mtcode_upper(ch)))
        }
        // Mathematical Double-Struck Small a..=z -> 'k' in Euclid Math Two, others fallback to TF_VARIABLE
        0x1D552..=0x1D56B => {
            let ch = char::from_u32('a' as u32 + (cp - 0x1D552)).unwrap_or('a');
            if ch == 'k' {
                Some((TF_DOUBLESTRUCK, double_struck_mtcode_lower('k')))
            } else {
                Some((TF_VARIABLE, ch as u32))
            }
        }

        // Mathematical Bold Fraktur Capital A..=Z -> Euclid Fraktur Bold
        0x1D56C..=0x1D585 => {
            let ch = char::from_u32('A' as u32 + (cp - 0x1D56C)).unwrap_or('A');
            Some((TF_FRAKTUR_BOLD, fraktur_mtcode_upper(ch)))
        }
        // Mathematical Bold Fraktur Small a..=z -> Euclid Fraktur Bold
        0x1D586..=0x1D59F => {
            let ch = char::from_u32('a' as u32 + (cp - 0x1D586)).unwrap_or('a');
            Some((TF_FRAKTUR_BOLD, fraktur_mtcode_lower(ch)))
        }

        // Mathematical Sans-Serif Capital A..=Z
        0x1D5A0..=0x1D5B9 => Some((TF_TEXT, 'A' as u32 + (cp - 0x1D5A0))),
        // Mathematical Sans-Serif Small a..=z
        0x1D5BA..=0x1D5D3 => Some((TF_TEXT, 'a' as u32 + (cp - 0x1D5BA))),
        // Mathematical Sans-Serif Bold Capital A..=Z
        0x1D5D4..=0x1D5ED => Some((TF_VECTOR, 'A' as u32 + (cp - 0x1D5D4))),
        // Mathematical Sans-Serif Bold Small a..=z
        0x1D5EE..=0x1D607 => Some((TF_VECTOR, 'a' as u32 + (cp - 0x1D5EE))),
        // Mathematical Sans-Serif Italic Capital A..=Z
        0x1D608..=0x1D621 => Some((TF_VARIABLE, 'A' as u32 + (cp - 0x1D608))),
        // Mathematical Sans-Serif Italic Small a..=z
        0x1D622..=0x1D63B => Some((TF_VARIABLE, 'a' as u32 + (cp - 0x1D622))),
        // Mathematical Sans-Serif Bold Italic Capital A..=Z
        0x1D63C..=0x1D655 => Some((TF_VECTOR, 'A' as u32 + (cp - 0x1D63C))),
        // Mathematical Sans-Serif Bold Italic Small a..=z
        0x1D656..=0x1D66F => Some((TF_VECTOR, 'a' as u32 + (cp - 0x1D656))),

        // Mathematical Monospace Capital A..=Z
        0x1D670..=0x1D689 => Some((TF_TEXT, 'A' as u32 + (cp - 0x1D670))),
        // Mathematical Monospace Small a..=z
        0x1D68A..=0x1D6A3 => Some((TF_TEXT, 'a' as u32 + (cp - 0x1D68A))),

        // Mathematical Italic Small Dotless i and j
        0x1D6A4 => Some((TF_VARIABLE, 0x0131)), // ı
        0x1D6A5 => Some((TF_VARIABLE, 0xED02)), // ȷ (MathType dotless j)

        // Mathematical Greek blocks (Bold, Italic, Bold Italic, Sans-Serif Bold, Sans-Serif Bold Italic)
        0x1D6A8..=0x1D6C1 => Some(map_math_greek_offset(cp - 0x1D6A8, true)),
        0x1D6C2..=0x1D6E1 => Some(map_math_greek_offset(cp - 0x1D6C2, false)),
        0x1D6E2..=0x1D6FB => Some(map_math_greek_offset(cp - 0x1D6E2, true)),
        0x1D6FC..=0x1D71B => Some(map_math_greek_offset(cp - 0x1D6FC, false)),
        0x1D71C..=0x1D735 => Some(map_math_greek_offset(cp - 0x1D71C, true)),
        0x1D736..=0x1D755 => Some(map_math_greek_offset(cp - 0x1D736, false)),
        0x1D756..=0x1D76F => Some(map_math_greek_offset(cp - 0x1D756, true)),
        0x1D770..=0x1D78F => Some(map_math_greek_offset(cp - 0x1D770, false)),
        0x1D790..=0x1D7A9 => Some(map_math_greek_offset(cp - 0x1D790, true)),
        0x1D7AA..=0x1D7C9 => Some(map_math_greek_offset(cp - 0x1D7AA, false)),
        0x1D7CA => Some((TF_UCGREEK, 0x03DC)), // 𝟊 Digamma
        0x1D7CB => Some((TF_LCGREEK, 0x03DD)), // 𝟋 small digamma

        // Mathematical Bold Digits 0..=9
        0x1D7CE..=0x1D7D7 => Some((TF_VECTOR, '0' as u32 + (cp - 0x1D7CE))),
        // Mathematical Double-Struck Digits 0..=9 (Euclid Math Two lacks digits; fallback to TF_NUMBER)
        0x1D7D8..=0x1D7E1 => Some((TF_NUMBER, '0' as u32 + (cp - 0x1D7D8))),
        // Mathematical Sans-Serif Digits 0..=9
        0x1D7E2..=0x1D7EB => Some((TF_NUMBER, '0' as u32 + (cp - 0x1D7E2))),
        // Mathematical Sans-Serif Bold Digits 0..=9
        0x1D7EC..=0x1D7F5 => Some((TF_VECTOR, '0' as u32 + (cp - 0x1D7EC))),
        // Mathematical Monospace Digits 0..=9
        0x1D7F6..=0x1D7FF => Some((TF_NUMBER, '0' as u32 + (cp - 0x1D7F6))),

        _ => None,
    }
}

/// `true` if `cp` is an uppercase Greek character (including Greek symbol variants).
pub fn is_greek_upper_cp(cp: u32) -> bool {
    matches!(cp, 0x0391..=0x03A9 | 0x03D2..=0x03D4 | 0x03DC | 0x03F4 | 0x03FE..=0x03FF)
}

/// `true` if `cp` is a lowercase Greek character (including `ϑ`, `ϕ`, `ϖ`, `ϰ`, `ϱ`, `ϵ`, `ς`).
pub fn is_greek_lower_cp(cp: u32) -> bool {
    matches!(
        cp,
        0x03B1..=0x03C9
            | 0x03D0..=0x03D1
            | 0x03D5..=0x03D7
            | 0x03DD
            | 0x03F0..=0x03F3
            | 0x03F5
    )
}

/// `true` if `ch` is a mathematical operator, relation, arrow, fence, or symbol
/// that belongs in `TF_SYMBOL`.
pub fn is_math_symbol_char(ch: char) -> bool {
    let cp = ch as u32;
    if crate::symbols::operators::MTEF_SYMBOL_OPERATORS.contains(ch)
        || crate::symbols::operators::OMML_OPERATOR_CHARS.contains(ch)
        || crate::symbols::relations::contains(ch)
        || crate::symbols::arrows::contains(ch)
        || crate::symbols::logic::contains(ch)
        || crate::symbols::sets::is_relation(ch)
        || crate::symbols::sets::is_operator(ch)
        || crate::symbols::fences::is_open(ch)
        || crate::symbols::fences::is_close(ch)
        || crate::symbols::fences::is_bar(ch)
    {
        return true;
    }
    matches!(
        cp,
        0x00AC // ¬
        | 0x00B0 // °
        | 0x00B1 // ±
        | 0x00B7 // ·
        | 0x00D7 // ×
        | 0x00F7 // ÷
        | 0x2010..=0x205E // General punctuation, primes, dashes, dots
        | 0x2190..=0x2BFF // Arrows, Mathematical Operators, Misc Technical, Geometric Shapes, Supplemental Math
        | 0x3008..=0x301B // CJK angle/white brackets used in math
        | 0xE900..=0xEEFF // MathType Private Use Area operators/symbols
    )
}

/// Check if a character belongs in the MT Extra font (`TF_MTEXTRA`).
pub fn is_mt_extra_char(cp: u32) -> bool {
    matches!(
        cp,
        0x019B // ƛ
        | 0x200A // hair space
        | 0x2026 // …
        | 0x210F // ℏ
        | 0x2113 // ℓ
        | 0x2127 // ℧
        | 0x2195 // ↕
        | 0x21A6 // ↦
        | 0x21BC..=0x21BD // ↼, ↽
        | 0x21C0..=0x21C1 // ⇀, ⇁
        | 0x21C4 // ⇄
        | 0x21C6 // ⇆
        | 0x21CC // ⇌
        | 0x21D5 // ⇕
        | 0x2210 // ∐
        | 0x2213 // ∓
        | 0x2221 // ∡
        | 0x2222 // ∢
        | 0x2225 // ∥
        | 0x2235 // ∵
        | 0x223C // ∼
        | 0x2243 // ≃
        | 0x2250 // ≐
        | 0x2259 // ≙
        | 0x225C // ≜
        | 0x226A // ≪
        | 0x226B // ≫
        | 0x22C2 // ⋂
        | 0x22C3 // ⋃
        | 0x22EE // ⋮
        | 0x22EF // ⋯
        | 0x22F0 // ⋰
        | 0x22F1 // ⋱
        | 0x2310 // ⌐
        | 0x25B3 // △
        | 0x25B7 // ▷
        | 0x25C1 // ◁
        | 0x2A3C // ⨼
        | 0x2A3D // ⨽
        | 0x2B1C // ⬜
        | 0xFFE2 // ￢
        | 0xEE00..=0xEE13 // MathType MT Extra loop integral glyphs
    )
}

/// Check if a character exists in the Windows Symbol font (`TF_SYMBOL`).
pub fn is_symbol_font_char(cp: u32) -> bool {
    matches!(
        cp,
        0x0020..=0x0021 // space, !
        | 0x0023 // #
        | 0x0025..=0x0026 // %, &
        | 0x0028..=0x0029 // (, )
        | 0x002B..=0x002C // +, comma
        | 0x002E..=0x003F // . / 0-9 : ; < = > ?
        | 0x005B | 0x005D | 0x005F // [, ], _
        | 0x007B..=0x007D // {, |, }
        | 0x00A9 | 0x00AC | 0x00AE // ©, ¬, ®
        | 0x00B0..=0x00B1 // °, ±
        | 0x00D7 | 0x00F7 // ×, ÷
        | 0x0192 // ƒ
        | 0x0391..=0x03A1 | 0x03A3..=0x03A9 // Greek capital Α..Ω
        | 0x03B1..=0x03C9 // Greek small α..ω
        | 0x03D1..=0x03D2 | 0x03D5..=0x03D6 // ϑ, ϒ, ϕ, ϖ
        | 0x2022 | 0x2026 // •, …
        | 0x2032..=0x2033 // ′, ″
        | 0x2044 // ⁄
        | 0x20AC // €
        | 0x2111 | 0x2118 | 0x211C | 0x2122 | 0x2135 // ℑ, ℘, ℜ, ™, ℵ
        | 0x2190..=0x2194 | 0x21B5 | 0x21D0..=0x21D4 // arrows
        | 0x2200 | 0x2202..=0x2203 | 0x2205 | 0x2207..=0x2209 | 0x220D // ∀, ∂, ∃, ∅, ∇, ∈, ∉, ∍
        | 0x220F | 0x2211..=0x2212 | 0x2217 | 0x221A | 0x221D..=0x221E // ∏, ∑, −, ∗, √, ∝, ∞
        | 0x2220 | 0x2227..=0x222B // ∠, ∧, ∨, ∩, ∪, ∫
        | 0x2234 | 0x223C // ∴, ∼
        | 0x2245 | 0x2248 // ≅, ≈
        | 0x2260..=0x2261 | 0x2264..=0x2265 // ≠, ≡, ≤, ≥
        | 0x2282..=0x2284 | 0x2286..=0x2287 // ⊂, ⊃, ⊄, ⊆, ⊇
        | 0x2295 | 0x2297 | 0x22A5 | 0x22C4..=0x22C5 // ⊕, ⊗, ⊥, ⋄, ⋅
        | 0x2320..=0x2321 | 0x2329..=0x232A // ⌠, ⌡, 〈, 〉
        | 0x239B..=0x23AF | 0x23D0 // brackets, bar
        | 0x25A1 | 0x2610 | 0x2660 | 0x2663 | 0x2665..=0x2666 // shapes, card suits
    )
}

/// Map a character `ch` and an optional MathML `mathvariant` attribute to
/// `(typeface, mt_code)`.
pub fn map_char_with_variant(ch: char, mathvariant: Option<&str>, default_tf: u8) -> (u8, u32) {
    let cp = ch as u32;

    // 1. Supplementary Mathematical Alphanumeric Symbols (U+1D400..=U+1D7FF)
    //    and BMP Letterlike Symbols (ℝ, ℤ, ℒ, ℜ, …)
    if let Some((tf, mt_code)) = map_supplementary_math_char(cp) {
        if matches!(mathvariant, Some("bold") | Some("bold-italic"))
            && tf == TF_VARIABLE
            && mt_code < 0x80
        {
            return (TF_VECTOR, mt_code);
        }
        return (tf, mt_code);
    }

    // 2. Explicit `mathvariant` attribute on ASCII letters / digits
    if let Some(variant) = mathvariant {
        match variant {
            "double-struck" => {
                if ch.is_ascii_uppercase() {
                    return (TF_DOUBLESTRUCK, double_struck_mtcode_upper(ch));
                }
                if ch == 'k' {
                    return (TF_DOUBLESTRUCK, double_struck_mtcode_lower('k'));
                }
                if ch.is_ascii_lowercase() {
                    // Euclid Math Two lacks lowercase double-struck (except 'k'); fallback to TF_VARIABLE
                    return (TF_VARIABLE, cp);
                }
                if ch.is_ascii_digit() {
                    return (TF_NUMBER, cp);
                }
            }
            "fraktur" => {
                if ch.is_ascii_uppercase() {
                    return (TF_FRAKTUR, fraktur_mtcode_upper(ch));
                }
                if ch.is_ascii_lowercase() {
                    return (TF_FRAKTUR, fraktur_mtcode_lower(ch));
                }
            }
            "bold-fraktur" => {
                if ch.is_ascii_uppercase() {
                    return (TF_FRAKTUR_BOLD, fraktur_mtcode_upper(ch));
                }
                if ch.is_ascii_lowercase() {
                    return (TF_FRAKTUR_BOLD, fraktur_mtcode_lower(ch));
                }
            }
            "script" => {
                if ch.is_ascii_uppercase() {
                    return (TF_SCRIPT, script_mtcode_upper(ch));
                }
                if ch == 'l' {
                    return (TF_MTEXTRA, 0x2113);
                }
                if ch.is_ascii_lowercase() {
                    // Euclid Math One lacks lowercase script; fallback to TF_VARIABLE
                    return (TF_VARIABLE, cp);
                }
            }
            "bold-script" => {
                if ch.is_ascii_uppercase() {
                    return (TF_SCRIPT_BOLD, script_mtcode_upper(ch));
                }
                if ch.is_ascii_lowercase() {
                    return (TF_VECTOR, cp);
                }
            }
            "bold" | "bold-italic" => {
                if ch.is_ascii_alphanumeric() {
                    return (TF_VECTOR, cp);
                }
            }
            "normal" => {
                if ch.is_ascii_alphabetic() {
                    return (TF_FUNCTION, cp);
                }
            }
            "sans-serif" | "monospace" if ch.is_ascii_alphabetic() => {
                return (TF_TEXT, cp);
            }
            _ => {}
        }
    }

    // Explicit overrides for characters where MathType has distinct MTCode conventions:
    // Minus sign in math mode: MathType Symbol font minus is 0x2212
    if ch == '-' || ch == '−' || cp == 0x2212 {
        return (TF_SYMBOL, 0x2212);
    }
    // Tilde in math mode: MathType Symbol font tilde is 0x223C
    if ch == '~' || ch == '∼' || cp == 0x223C {
        return (TF_SYMBOL, 0x223C);
    }
    // Increment / Delta: map to Greek Capital Delta in Symbol font
    if cp == 0x2206 {
        return (TF_UCGREEK, 0x0394);
    }
    // Contains as member: map to 0x220D in Symbol font
    if cp == 0x220B {
        return (TF_SYMBOL, 0x220D);
    }

    // 3. Map special BMP characters to MathType MTCode
    let mt_code = unicode_to_mtcode(cp);

    // 4. MathType spacing codes (0xEF01..=0xEF08)
    if (0xEF01..=0xEF08).contains(&mt_code) {
        return (TF_TEXT, mt_code);
    }

    // 5. Differential / Exponential / Imaginary (0xED01..=0xED16)
    if (0xED01..=0xED16).contains(&mt_code) {
        return (TF_VARIABLE, mt_code);
    }

    // 6. Greek characters (uppercase and lowercase, including symbol variants)
    if is_greek_lower_cp(mt_code) {
        return (TF_LCGREEK, mt_code);
    }
    if is_greek_upper_cp(mt_code) {
        return (TF_UCGREEK, mt_code);
    }

    // 7. Digits (ASCII, superscript/subscript, full-width)
    if py_isdigit(ch) {
        return (TF_NUMBER, mt_code);
    }

    // 8. MT Extra characters
    if is_mt_extra_char(cp) || is_mt_extra_char(mt_code) {
        return (TF_MTEXTRA, mt_code);
    }

    // 9. Windows Symbol font characters or MathType PUA operators
    if is_symbol_font_char(mt_code) || (0xE900..=0xEEFF).contains(&mt_code) {
        return (TF_SYMBOL, mt_code);
    }

    // 10. Other math symbols/relations/arrows not in Symbol or MT Extra -> TF_TEXT
    // This allows Windows font linking / Unicode text font to render them without showing as diamonds.
    if is_math_symbol_char(ch) {
        return (TF_TEXT, mt_code & 0xFFFF);
    }

    // 11. Accented Latin / Vietnamese letters (outside ASCII and Greek) -> upright text
    if ch.is_alphabetic() && !ch.is_ascii() {
        return (TF_TEXT, mt_code & 0xFFFF);
    }

    // 12. ASCII letters or fallback
    if ch.is_ascii_alphabetic() {
        return (default_tf, mt_code);
    }

    // Fallback for any remaining character
    (TF_TEXT, mt_code & 0xFFFF)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supplementary_math_alphanumeric() {
        // Bold A (U+1D400) -> TF_VECTOR, 'A'
        assert_eq!(
            map_char_with_variant('𝐀', None, TF_VARIABLE),
            (TF_VECTOR, 'A' as u32)
        );
        // Italic x (U+1D465) -> TF_VARIABLE, 'x'
        assert_eq!(
            map_char_with_variant('𝑥', None, TF_VARIABLE),
            (TF_VARIABLE, 'x' as u32)
        );
        // Double-struck R (ℝ U+211D) -> TF_DOUBLESTRUCK, 0x211D
        assert_eq!(
            map_char_with_variant('ℝ', None, TF_VARIABLE),
            (TF_DOUBLESTRUCK, 0x211D)
        );
        // Double-struck A (𝔸 U+1D538) -> TF_DOUBLESTRUCK, 0xF080
        assert_eq!(
            map_char_with_variant('𝔸', None, TF_VARIABLE),
            (TF_DOUBLESTRUCK, 0xF080)
        );
        // Double-struck k (𝕜 U+1D55C) -> TF_DOUBLESTRUCK, 0xF0A4
        assert_eq!(
            map_char_with_variant('𝕜', None, TF_VARIABLE),
            (TF_DOUBLESTRUCK, 0xF0A4)
        );
        // Double-struck a (𝕒 U+1D552, not in Euclid Math Two) -> TF_VARIABLE, 'a'
        assert_eq!(
            map_char_with_variant('𝕒', None, TF_VARIABLE),
            (TF_VARIABLE, 'a' as u32)
        );
        // Script L (ℒ U+2112) -> TF_SCRIPT, 0x2112
        assert_eq!(
            map_char_with_variant('ℒ', None, TF_VARIABLE),
            (TF_SCRIPT, 0x2112)
        );
        // Script A (𝒜 U+1D49C) -> TF_SCRIPT, 0xF100
        assert_eq!(
            map_char_with_variant('𝒜', None, TF_VARIABLE),
            (TF_SCRIPT, 0xF100)
        );
        // Fraktur g (𝔤 U+1D524) -> TF_FRAKTUR, 0xF01A + 6 = 0xF020
        assert_eq!(
            map_char_with_variant('𝔤', None, TF_VARIABLE),
            (TF_FRAKTUR, 0xF020)
        );
        // Italic Greek alpha (𝛼 U+1D6FC) -> TF_LCGREEK, 0x03B1
        assert_eq!(
            map_char_with_variant('𝛼', None, TF_VARIABLE),
            (TF_LCGREEK, 0x03B1)
        );
    }

    #[test]
    fn test_mathvariant_attributes() {
        assert_eq!(
            map_char_with_variant('R', Some("double-struck"), TF_VARIABLE),
            (TF_DOUBLESTRUCK, 0x211D)
        );
        assert_eq!(
            map_char_with_variant('A', Some("double-struck"), TF_VARIABLE),
            (TF_DOUBLESTRUCK, 0xF080)
        );
        assert_eq!(
            map_char_with_variant('v', Some("bold"), TF_VARIABLE),
            (TF_VECTOR, 'v' as u32)
        );
        assert_eq!(
            map_char_with_variant('L', Some("script"), TF_VARIABLE),
            (TF_SCRIPT, 0x2112)
        );
        assert_eq!(
            map_char_with_variant('g', Some("fraktur"), TF_VARIABLE),
            (TF_FRAKTUR, 0xF020)
        );
    }

    #[test]
    fn test_greek_and_vietnamese_classification() {
        // Greek variant varphi ϕ (U+03D5) and vartheta ϑ (U+03D1)
        assert_eq!(
            map_char_with_variant('ϕ', None, TF_VARIABLE),
            (TF_LCGREEK, 0x03D5)
        );
        assert_eq!(
            map_char_with_variant('ϑ', None, TF_VARIABLE),
            (TF_LCGREEK, 0x03D1)
        );
        // Capital Omicron Ο (U+039F)
        assert_eq!(
            map_char_with_variant('Ο', None, TF_VARIABLE),
            (TF_UCGREEK, 0x039F)
        );
        // Vietnamese character 'ă' -> TF_TEXT
        assert_eq!(
            map_char_with_variant('ă', None, TF_VARIABLE),
            (TF_TEXT, 'ă' as u32)
        );
        // Math symbol '∞' inside <mi> -> TF_SYMBOL
        assert_eq!(
            map_char_with_variant('∞', None, TF_VARIABLE),
            (TF_SYMBOL, 0x221E)
        );
    }

    #[test]
    fn test_euclid_font_defs_and_char_enc_8() {
        use crate::mtef::{build_mtef, char_record, mtef_to_mathml};

        let mut eq = Vec::new();
        // Script A (Euclid Math One, font_pos = b'A')
        let (tf_scr, mt_scr) = map_char_with_variant('A', Some("script"), TF_VARIABLE);
        let rec_scr = char_record(tf_scr, mt_scr);
        assert_eq!(rec_scr, vec![0x02, 0x04, 0x7E, 0x00, 0xF1, b'A']);
        eq.extend_from_slice(&rec_scr);

        // Fraktur g (Euclid Fraktur, font_pos = b'g')
        let (tf_frk, mt_frk) = map_char_with_variant('g', Some("fraktur"), TF_VARIABLE);
        let rec_frk = char_record(tf_frk, mt_frk);
        assert_eq!(rec_frk, vec![0x02, 0x04, 0x7F, 0x20, 0xF0, b'g']);
        eq.extend_from_slice(&rec_frk);

        // Double-struck R (Euclid Math Two, font_pos = b'R')
        let (tf_ds, mt_ds) = map_char_with_variant('R', Some("double-struck"), TF_VARIABLE);
        let rec_ds = char_record(tf_ds, mt_ds);
        assert_eq!(rec_ds, vec![0x02, 0x04, 0x7D, 0x1D, 0x21, b'R']);
        eq.extend_from_slice(&rec_ds);

        let mtef = build_mtef(&eq);
        // Verify ENCODING_DEF (0x13) and FONT_DEF (0x11) and FONT_STYLE_DEF (0x08) are emitted
        assert!(mtef.windows(14).any(|w| w == b"\x13EuclidFraktur"));
        assert!(mtef.windows(17).any(|w| w == b"\x11\x06Euclid Math One"));

        // Verify MTEF -> MathML decoder round-trips cleanly
        let math = mtef_to_mathml(&mtef).expect("should decode MTEF with Euclid defs");
        assert_eq!(math.children[0].children.len(), 3);
    }
}
