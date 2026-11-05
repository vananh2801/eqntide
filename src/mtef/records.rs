//! Primitive MTEF records: END / LINE / CHAR / SIZE / TMPL headers.

/// END record.
pub const END: &[u8] = b"\x00";
/// FULL size record.
pub const FULL: &[u8] = b"\x0a";
/// SUB size record.
pub const SUB: &[u8] = b"\x0b";
/// SUB2 size record.
pub const SUB2: &[u8] = b"\x0c";

/// LINE record; `null` marks an empty slot.
pub fn line(null: bool) -> Vec<u8> {
    vec![0x01, u8::from(null)]
}

/// CHAR record. Only the low 16 bits of `unicode_val` are stored (MTCode).
pub fn char_record(typeface: u8, unicode_val: u32) -> Vec<u8> {
    vec![
        0x02,
        0x00,
        typeface.wrapping_add(128),
        (unicode_val & 0xFF) as u8,
        ((unicode_val >> 8) & 0xFF) as u8,
    ]
}

/// Explicit display point size (stored value 320).
pub fn size_display() -> Vec<u8> {
    vec![0x09, 0x65, 0x40, 0x01]
}

/// TMPL header.
pub fn tmpl_header(selector: u8, variation: u8, tmpl_opts: u8) -> Vec<u8> {
    vec![0x03, 0x00, selector, variation, tmpl_opts]
}
