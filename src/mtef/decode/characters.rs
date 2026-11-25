//! MTEF character, PUA codepoint, spacing, and stretchy accent mapping to [`MathNode`].

use crate::mathml::MathNode;

use super::super::typeface::{
    TF_FUNCTION, TF_LCGREEK, TF_MTEXTRA, TF_NUMBER, TF_SPACE, TF_SYMBOL, TF_UCGREEK, TF_VARIABLE,
    TF_VECTOR,
};
use super::embellishment::wrap_embellishments;

/// Decode a single MTEF character record (with optional attachments) into a [`MathNode`].
pub(super) fn convert_char_to_mathnode(
    typeface: u8,
    mt_code: u16,
    is_textmode: bool,
    embells: &[u8],
) -> Option<MathNode> {
    let base = map_mt_char_token(typeface, mt_code, is_textmode)?;
    Some(wrap_embellishments(base, embells))
}

/// Resolve an 8-bit/16-bit font character when `OPT_CHAR_NO_MT` is set and `mt_code == 0`.
pub(super) fn resolve_no_mt_enc(typeface: u8, code: u16, is_16: bool) -> u16 {
    if is_16 {
        return code;
    }
    let byte = code as u8;
    if matches!(typeface, TF_SYMBOL | TF_LCGREEK | TF_UCGREEK) {
        return match byte {
            0x22 => 0x2200, // ∀
            0x24 => 0x2203, // ∃
            0x27 => 0x220D, // ∍
            0x2A => 0x2217, // ∗
            0x2D => 0x2212, // −
            0x40 => 0x2245, // ≅
            0x41 => 0x0391, // Α
            0x42 => 0x0392, // Β
            0x43 => 0x03A7, // Χ
            0x44 => 0x0394, // Δ
            0x45 => 0x0395, // Ε
            0x46 => 0x03A6, // Φ
            0x47 => 0x0393, // Γ
            0x48 => 0x0397, // Η
            0x49 => 0x0399, // Ι
            0x4A => 0x03D1, // ϑ
            0x4B => 0x039A, // Κ
            0x4C => 0x039B, // Λ
            0x4D => 0x039C, // Μ
            0x4E => 0x039D, // Ν
            0x4F => 0x039F, // Ο
            0x50 => 0x03A0, // Π
            0x51 => 0x0398, // Θ
            0x52 => 0x03A1, // Ρ
            0x53 => 0x03A3, // Σ
            0x54 => 0x03A4, // Τ
            0x55 => 0x03A5, // Υ
            0x56 => 0x03C2, // ς
            0x57 => 0x03A9, // Ω
            0x58 => 0x039E, // Ξ
            0x59 => 0x03A8, // Ψ
            0x5A => 0x0396, // Ζ
            0x5C => 0x2234, // ∴
            0x5E => 0x22A5, // ⊥
            0x61 => 0x03B1, // α
            0x62 => 0x03B2, // β
            0x63 => 0x03C7, // χ
            0x64 => 0x03B4, // δ
            0x65 => 0x03B5, // ε
            0x66 => 0x03C6, // φ
            0x67 => 0x03B3, // γ
            0x68 => 0x03B7, // η
            0x69 => 0x03B9, // ι
            0x6A => 0x03D5, // ϕ
            0x6B => 0x03BA, // κ
            0x6C => 0x03BB, // λ
            0x6D => 0x03BC, // μ
            0x6E => 0x03BD, // ν
            0x6F => 0x03BF, // ο
            0x70 => 0x03C0, // π
            0x71 => 0x03B8, // θ
            0x72 => 0x03C1, // ρ
            0x73 => 0x03C3, // σ
            0x74 => 0x03C4, // τ
            0x75 => 0x03C5, // υ
            0x76 => 0x03D6, // ϖ
            0x77 => 0x03C9, // ω
            0x78 => 0x03BE, // ξ
            0x79 => 0x03C8, // ψ
            0x7A => 0x03B6, // ζ
            0x7E => 0x223C, // ∼
            0xA0 => 0x20AC, // €
            0xA1 => 0x03D2, // ϒ
            0xA2 => 0x2032, // ′
            0xA3 => 0x2264, // ≤
            0xA4 => 0x2044, // ⁄
            0xA5 => 0x221E, // ∞
            0xA6 => 0x0192, // ƒ
            0xAB => 0x2194, // ↔
            0xAC => 0x2190, // ←
            0xAD => 0x2191, // ↑
            0xAE => 0x2192, // →
            0xAF => 0x2193, // ↓
            0xB0 => 0x00B0, // °
            0xB1 => 0x00B1, // ±
            0xB2 => 0x2033, // ″
            0xB3 => 0x2265, // ≥
            0xB4 => 0x00D7, // ×
            0xB5 => 0x221D, // ∝
            0xB6 => 0x2202, // ∂
            0xB7 => 0x2022, // •
            0xB8 => 0x00F7, // ÷
            0xB9 => 0x2260, // ≠
            0xBA => 0x2261, // ≡
            0xBB => 0x2248, // ≈
            0xBC => 0x2026, // …
            0xC0 => 0x2135, // ℵ
            0xC1 => 0x2111, // ℑ
            0xC2 => 0x211C, // ℜ
            0xC3 => 0x2118, // ℘
            0xC4 => 0x2297, // ⊗
            0xC5 => 0x2295, // ⊕
            0xC6 => 0x2205, // ∅
            0xC7 => 0x2229, // ∩
            0xC8 => 0x222A, // ∪
            0xC9 => 0x2283, // ⊃
            0xCA => 0x2287, // ⊇
            0xCB => 0x2284, // ⊄
            0xCC => 0x2282, // ⊂
            0xCD => 0x2286, // ⊆
            0xCE => 0x2208, // ∈
            0xCF => 0x2209, // ∉
            0xD0 => 0x2220, // ∠
            0xD1 => 0x2207, // ∇
            0xD5 => 0x220F, // ∏
            0xD6 => 0x221A, // √
            0xD7 => 0x22C5, // ⋅
            0xD8 => 0x00AC, // ¬
            0xD9 => 0x2227, // ∧
            0xDA => 0x2228, // ∨
            0xDB => 0x21D4, // ⇔
            0xDC => 0x21D0, // ⇐
            0xDD => 0x21D1, // ⇑
            0xDE => 0x21D2, // ⇒
            0xDF => 0x21D3, // ⇓
            0xE0 => 0x25CA, // ◊
            0xE1 => 0x2329, // ⟨
            0xE5 => 0x2211, // ∑
            0xF1 => 0x232A, // ⟩
            0xF2 => 0x222B, // ∫
            _ => code,
        };
    }
    match byte {
        0x80 => 0x20AC,
        0x82 => 0x201A,
        0x83 => 0x0192,
        0x84 => 0x201E,
        0x85 => 0x2026,
        0x86 => 0x2020,
        0x87 => 0x2021,
        0x88 => 0x02C6,
        0x89 => 0x2030,
        0x8A => 0x0160,
        0x8B => 0x2039,
        0x8C => 0x0152,
        0x8E => 0x017D,
        0x91 => 0x2018,
        0x92 => 0x2019,
        0x93 => 0x201C,
        0x94 => 0x201D,
        0x95 => 0x2022,
        0x96 => 0x2013,
        0x97 => 0x2014,
        0x98 => 0x02DC,
        0x99 => 0x2122,
        0x9A => 0x0161,
        0x9B => 0x203A,
        0x9C => 0x0153,
        0x9E => 0x017E,
        0x9F => 0x0178,
        _ => code,
    }
}

/// Map an MTCode spacing value or PUA codepoint into an untyped MathML node.
pub(super) fn map_mt_char_token(typeface: u8, mt_code: u16, is_textmode: bool) -> Option<MathNode> {
    // Equation Editor 3.0 `fnSPACE` (typeface 24) uses 0xEB01..=0xEB08 for spacing.
    if typeface == TF_SPACE && (0xEB00..=0xEB08).contains(&mt_code) {
        return map_space_code(mt_code);
    }

    // Check MathType Private Use Area (0xE000..=0xF8FF).
    if (0xE000..=0xF8FF).contains(&mt_code) {
        return map_pua_char(mt_code, is_textmode);
    }

    if mt_code == 0 {
        return None;
    }

    let ch = char::from_u32(mt_code as u32)?;
    let text = ch.to_string();

    if is_textmode {
        if mt_code == 0x0020 {
            return Some(MathNode::leaf("mtext", "\u{00A0}"));
        }
        // Math symbols, modifier accents, arrows, and delimiters encoded under TF_TEXT
        // for Windows font-linking should decode as `<mo>` rather than literal text.
        if matches!(
            mt_code,
            0x005C
                | 0x02C6..=0x02DC
                | 0x2032..=0x2037
                | 0x2057
                | 0x20D0..=0x20FF
                | 0x2190..=0x2BFF
                | 0x3008..=0x301B
        ) {
            return Some(MathNode::leaf("mo", &text));
        }
        return Some(MathNode::leaf("mtext", &text));
    }

    match mt_code {
        0x0009 | 0x000A | 0xFEFF | 0x2000..=0x200B => Some(MathNode::leaf("mtext", &text)),
        0x0030..=0x0039 | 0x00BC..=0x00BE | 0x2150..=0x218F => Some(MathNode::leaf("mn", &text)),
        0x0041..=0x005A | 0x0061..=0x007A => {
            let mut node = MathNode::leaf("mi", &text);
            if typeface == TF_VECTOR {
                node.set_attr("mathvariant", "bold");
            } else if typeface == TF_FUNCTION {
                node.set_attr("mathvariant", "normal");
            }
            Some(node)
        }
        0x0028 | 0x0029 | 0x005B..=0x005D => {
            let mut mo = MathNode::leaf("mo", &text);
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
            if let Some(ascii) = match mt_code {
                0x2102 => Some("C"),
                0x210D => Some("H"),
                0x2115 => Some("N"),
                0x2119 => Some("P"),
                0x211A => Some("Q"),
                0x211D => Some("R"),
                0x2124 => Some("Z"),
                _ => None,
            } {
                let mut mi = MathNode::leaf("mi", ascii);
                mi.set_attr("mathvariant", "double-struck");
                return Some(mi);
            }
            Some(MathNode::leaf("mo", &text))
        }
        _ => match typeface {
            TF_FUNCTION | TF_NUMBER => Some(MathNode::leaf("mn", &text)),
            TF_SYMBOL | TF_MTEXTRA => Some(MathNode::leaf("mo", &text)),
            TF_VARIABLE | TF_LCGREEK | TF_UCGREEK | TF_VECTOR => {
                let mut node = MathNode::leaf("mi", &text);
                if typeface == TF_VECTOR {
                    node.set_attr("mathvariant", "bold");
                }
                Some(node)
            }
            _ => Some(MathNode::leaf("mi", &text)),
        },
    }
}

/// Append a prime operator to `node`, extending an existing `<mrow>` when present.
pub(super) fn append_prime(mut node: MathNode, prime_str: &str) -> MathNode {
    if node.tag == "mrow" {
        node.children.push(MathNode::leaf("mo", prime_str));
        node
    } else {
        MathNode::with_children("mrow", vec![node, MathNode::leaf("mo", prime_str)])
    }
}

/// Wrap `base` in `<mover accent="true">` with a stretchy operator glyph.
pub(super) fn mover_stretchy_accent(base: MathNode, mo_str: &str) -> MathNode {
    let mut mo = MathNode::leaf("mo", mo_str);
    mo.set_attr("stretchy", "true");
    let mut mover = MathNode::with_children("mover", vec![base, mo]);
    mover.set_attr("accent", "true");
    mover
}

/// Wrap `base` in `<munder accentunder="true">` with a stretchy operator glyph.
pub(super) fn munder_stretchy_accent(base: MathNode, mo_str: &str) -> MathNode {
    let mut mo = MathNode::leaf("mo", mo_str);
    mo.set_attr("stretchy", "true");
    let mut munder = MathNode::with_children("munder", vec![base, mo]);
    munder.set_attr("accentunder", "true");
    munder
}

fn map_space_code(mt_code: u16) -> Option<MathNode> {
    match mt_code {
        0xEF00 | 0xEF01 | 0xEB00 => None,
        0xEB01..=0xEB08 | 0xEF02..=0xEF08 => {
            let width = match mt_code {
                0xEB02 | 0xEF02 => "0.166667em",
                0xEB03 | 0xEF03 => "0.222222em",
                0xEB04 | 0xEF04 => "0.277778em",
                0xEB05 | 0xEF05 => "1em",
                0xEB06 | 0xEF06 => "2em",
                _ => "0.083333em",
            };
            let mut space = MathNode::new("mspace");
            space.set_attr("width", width);
            Some(space)
        }
        _ => None,
    }
}

fn map_pua_char(mt_code: u16, is_textmode: bool) -> Option<MathNode> {
    let text_or_mo = |text: &str| -> MathNode {
        if is_textmode {
            MathNode::leaf("mtext", text)
        } else {
            MathNode::leaf("mo", text)
        }
    };

    match mt_code {
        // Relational and binary operators in MathType PUA
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

        // Differential, exponential, and imaginary constants
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
        0xEF00 | 0xEF01 => None,
        0xEF02..=0xEF08 => map_space_code(mt_code),

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

        // Unsupported PUA characters are dropped cleanly.
        _ => None,
    }
}
