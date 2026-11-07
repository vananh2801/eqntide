//! MTEF 5.0 binary format.
//!
//! Binary format verified by reverse-engineering fixture files from
//! <https://github.com/jure/mathtype> (MTEF parser Ruby gem).
//!
//! Record sizes:
//!
//! | record | type | size |
//! | --- | --- | --- |
//! | END  | `0x00` | 1 byte (no options byte) |
//! | LINE | `0x01` | 2 bytes: type + options |
//! | CHAR | `0x02` | 5 bytes: type + options + typeface + MTCode lo + hi (+ optional font_pos) |
//! | TMPL | `0x03` | 5 bytes header: type + options + selector + variation + tmpl_opts |
//! | SIZE | `0x09` | 4 bytes: type + `0x65` + size lo + hi |
//! | FULL | `0x0a` | 1 byte |
//! | SUB  | `0x0b` | 1 byte |
//! | SUB2 | `0x0c` | 1 byte |
//!
//! TMPL selectors: `0x0a` tmROOT, `0x0b` tmFRACT, `0x1b` tmSUB, `0x1c` tmSUP,
//! `0x1d` tmSUBSUP (for the three script templates the base precedes the TMPL
//! in the parent LINE).
//!
//! Typeface byte = typeface + 128.

//! # Layout of this module
//!
//! | Module       | Contents                                          |
//! |--------------|---------------------------------------------------|
//! | `records`    | END/LINE/CHAR/SIZE/TMPL primitives               |
//! | `typeface`   | typeface constants, [`classify_char`]            |
//! | `charmap`    | Unicode ↔ MTCode mapping and typeface rules      |
//! | `templates`  | fraction, radicals, sub/superscripts             |
//! | `encode`     | MathML → MTEF encoder                            |
//! | `decode`     | MTEF → MathML decoder                            |

pub mod charmap;
mod records;
mod templates;
mod typeface;

pub mod decode;
pub mod encode;

pub use charmap::*;
pub use decode::{mtef_to_mathml, ole_to_mathml};
pub use records::*;
pub use templates::*;
pub use typeface::*;
pub use crate::symbols::greek::{greek, is_greek_codepoint, GREEK};

/// MTEF v5 definition records (`ENCODING_DEF`, `FONT_DEF`, `FONT_STYLE_DEF`)
/// for MathType's bundled Euclid fonts (`Euclid Fraktur`, `Euclid Math One`,
/// `Euclid Math Two`).
///
/// Predefined encodings in MTEF v5 occupy indices 1..=4 (`MTCode`, `Unknown`,
/// `Symbol`, `MTExtra`), so the three `ENCODING_DEF` records below define:
/// - Encoding #5: `"EuclidFraktur"`
/// - Encoding #6: `"EuclidMath1"`
/// - Encoding #7: `"EuclidMath2"`
///
/// The three `FONT_DEF` records (`0x11`) define:
/// - Font #1: encoding #5, `"Euclid Fraktur"`
/// - Font #2: encoding #6, `"Euclid Math One"`
/// - Font #3: encoding #7, `"Euclid Math Two"`
///
/// The five `FONT_STYLE_DEF` records (`0x08`) define explicit font styles
/// referenced by negative signed typefaces (`256 - index`, stored biased by +128):
/// - Style #1 (`TF_FRAKTUR = 255`, biased `0x7F`): Font #1 (`Euclid Fraktur`), Plain (`0`)
/// - Style #2 (`TF_SCRIPT = 254`, biased `0x7E`): Font #2 (`Euclid Math One`), Plain (`0`)
/// - Style #3 (`TF_DOUBLESTRUCK = 253`, biased `0x7D`): Font #3 (`Euclid Math Two`), Plain (`0`)
/// - Style #4 (`TF_FRAKTUR_BOLD = 252`, biased `0x7C`): Font #1 (`Euclid Fraktur`), Bold (`1`)
/// - Style #5 (`TF_SCRIPT_BOLD = 251`, biased `0x7B`): Font #2 (`Euclid Math One`), Bold (`1`)
const EUCLID_FONT_DEFS: &[u8] = b"\
\x13EuclidFraktur\x00\
\x13EuclidMath1\x00\
\x13EuclidMath2\x00\
\x11\x05Euclid Fraktur\x00\
\x11\x06Euclid Math One\x00\
\x11\x07Euclid Math Two\x00\
\x08\x01\x00\
\x08\x02\x00\
\x08\x03\x00\
\x08\x01\x01\
\x08\x02\x01";

/// Check whether `equation_bytes` contains any `CHAR` record (`0x02`) that uses
/// one of the explicit Euclid font styles (`0x7B..=0x7F` with `mtefOPT_CHAR_ENC_CHAR_8 = 0x04`).
fn uses_euclid_font_defs(equation_bytes: &[u8]) -> bool {
    equation_bytes
        .windows(3)
        .any(|w| w[0] == 0x02 && (w[1] & 0x04) != 0 && (0x7B..=0x7F).contains(&w[2]))
}

/// Wrap equation content bytes in a complete MTEF 5.0 stream (the part that
/// follows the 28-byte `EQNOLEFILEHDR`).
pub fn build_mtef(equation_bytes: &[u8]) -> Vec<u8> {
    let header: [u8; 12] = [
        0x05, // MTEF version 5
        0x00, // platform: 0 = Windows
        0x00, // product: 0 = MathType
        0x05, // product version 5 (as in a genuine MathType 5 file)
        0x00, // product subversion 0
        0x44, 0x53, 0x4D, 0x54, 0x35, 0x00, // app key "DSMT5\0"
        0x00, // equation options: 0 = display/block
    ];
    let has_euclid = uses_euclid_font_defs(equation_bytes);
    let extra_len = if has_euclid { EUCLID_FONT_DEFS.len() } else { 0 };
    let mut out = Vec::with_capacity(header.len() + extra_len + equation_bytes.len() + 12);
    out.extend_from_slice(&header);
    if has_euclid {
        out.extend_from_slice(EUCLID_FONT_DEFS);
    }
    // main line: FULL + LINE + content + END
    out.extend_from_slice(FULL);
    out.extend(line(false));
    out.extend_from_slice(equation_bytes);
    out.extend_from_slice(END);
    out
}

/// Build the complete `Equation Native` OLE stream:
/// `EQNOLEFILEHDR` (28 bytes) + MTEF data.
pub fn make_equation_native(equation_bytes: &[u8]) -> Vec<u8> {
    let mtef_data = build_mtef(equation_bytes);
    // EQNOLEFILEHDR: cbHdr(WORD) + version(DWORD) + cf(WORD) + cbObject(DWORD) + reserved(16)
    let mut out = Vec::with_capacity(28 + mtef_data.len());
    out.extend_from_slice(&28u16.to_le_bytes());
    out.extend_from_slice(&0x0002_0000u32.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&(mtef_data.len() as u32).to_le_bytes());
    out.extend_from_slice(&[0u8; 16]);
    out.extend(mtef_data);
    out
}
