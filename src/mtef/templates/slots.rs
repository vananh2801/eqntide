//! Shared MTEF template-slot emission primitives.
//!
//! Keeping the null-line and size-record rules here makes template constructors
//! describe their slot layout without duplicating record framing details.

use super::super::records::{line, END, SUB};

/// Append a required `LINE` slot. Empty content is represented by a null line.
pub(super) fn append_line_slot(out: &mut Vec<u8>, content: &[u8]) {
    if content.is_empty() {
        out.extend_from_slice(&line(true));
    } else {
        out.extend_from_slice(&line(false));
        out.extend_from_slice(content);
        out.extend_from_slice(END);
    }
}

/// Append an optional `LINE` slot, using a null line for an absent value.
pub(super) fn append_opt_line_slot(out: &mut Vec<u8>, content: Option<&[u8]>) {
    match content {
        Some(content) if !content.is_empty() => append_line_slot(out, content),
        _ => out.extend_from_slice(&line(true)),
    }
}

/// Append a fraction slot, applying sub-symbol size when requested.
pub(super) fn append_fraction_slot(out: &mut Vec<u8>, content: &[u8], small: bool) {
    if small {
        out.extend_from_slice(SUB);
    }
    append_line_slot(out, content);
}

/// Encode a `TMPL` header whose variation may use one or two bytes.
pub(super) fn tmpl_header_u16(selector: u8, variation: u16, tmpl_opts: u8) -> Vec<u8> {
    let mut out = vec![0x03, 0x00, selector];
    if variation < 0x80 {
        out.push(variation as u8);
    } else {
        out.push(0x80 | (variation as u8 & 0x7F));
        out.push((variation >> 8) as u8);
    }
    out.push(tmpl_opts);
    out
}
