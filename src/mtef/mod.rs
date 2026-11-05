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
//! | CHAR | `0x02` | 5 bytes: type + options + typeface + MTCode lo + hi |
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
//! | `templates`  | fraction, radicals, sub/superscripts             |
//! | `encode`     | MathML → MTEF encoder                            |
//! | `decode`     | MTEF → MathML decoder (planned)                  |

mod records;
mod templates;
mod typeface;

pub mod encode;

pub use records::*;
pub use templates::*;
pub use typeface::*;
pub use crate::symbols::greek::{greek, is_greek_codepoint, GREEK};

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
    let mut out = Vec::with_capacity(header.len() + equation_bytes.len() + 12);
    out.extend_from_slice(&header);
    // main line: FULL + LINE + SIZE + content + END
    out.extend_from_slice(FULL);
    out.extend(line(false));
    out.extend(size_display());
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
