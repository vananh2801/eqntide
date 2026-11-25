//! MTEF (v3 and v5) binary decoder → MathML (`MathNode`).
//!
//! ```text
//! OLE (.bin) ─[ole_to_mathml]─▶ MTEF byte slice
//!                                     │
//!                             [mtef_to_mathml]
//!                                     ▼
//!                       entry::decode_to_mathml (v3 / v5)
//!                                     │
//!                        v3 / v5 binary record parsers
//!                                     ▼
//!                             Vec<MtefObj> AST
//!                                     │
//!                    templates::apply_mover_to_list (sub/sup bases)
//!                                     ▼
//!               templates / characters / embellishment ─▶ MathNode tree
//! ```
//!
//! Ports the MTEF record parser, slot/mover logic, character mapping, and
//! template-to-MathML rules into a direct, zero-XSLT Rust decoder producing
//! [`MathNode`].

use crate::mathml::MathNode;
use crate::ole::extract_mtef_from_ole;

mod characters;
mod consts;
mod embellishment;
mod entry;
mod parse;
mod stream;
mod templates;
mod v3;
mod v5;

#[cfg(test)]
mod tests;

/// Intermediate AST node decoded from MTEF v3/v5 binary records before
/// sub/superscript base relocation and MathML conversion.
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
