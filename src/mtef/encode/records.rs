//! MTEF character-record framing helpers.

use crate::mtef::{embell_record, END};

pub(super) fn is_single_char_record(bytes: &[u8]) -> bool {
    bytes.len() == 5 && bytes[0] == 0x02 && bytes[1] == 0x00
}

pub(super) fn attach_embell(mut char_bytes: Vec<u8>, embell_code: u8) -> Vec<u8> {
    if is_single_char_record(&char_bytes) {
        char_bytes[1] |= 0x01; // OPT_CHAR_EMBELL
        char_bytes.extend_from_slice(&embell_record(embell_code));
        char_bytes.extend_from_slice(END);
    }
    char_bytes
}
