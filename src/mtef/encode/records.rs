//! MTEF character-record framing and combining-mark embellishment helpers.

use crate::mtef::{
    char_record, embell_record, EMB_1DOT, EMB_1PRIME, EMB_2DOT, EMB_2PRIME, EMB_3DOT, EMB_3PRIME,
    EMB_4DOT, EMB_BARROW, EMB_BPRIME, EMB_FROWN, EMB_HAT, EMB_L1ARROW, EMB_LARROW, EMB_MBAR,
    EMB_NOT, EMB_OBAR, EMB_R1ARROW, EMB_RARROW, EMB_SMILE, EMB_TILDE, EMB_U_1DOT, EMB_U_2DOT,
    EMB_U_3DOT, EMB_U_BAR, EMB_U_BARROW, EMB_U_FROWN, EMB_U_L1ARROW, EMB_U_LARROW, EMB_U_R1ARROW,
    EMB_U_RARROW, EMB_U_SMILE, EMB_U_TILDE, END,
};
use crate::symbols::is_prime_char;

/// Return whether `bytes` is a single un-embellished 5-byte `CHAR` record.
pub(super) fn is_single_char_record(bytes: &[u8]) -> bool {
    bytes.len() == 5 && bytes[0] == 0x02 && bytes[1] == 0x00
}

/// Attach `embell_code` to a plain 5-byte `CHAR` record (`OPT_CHAR_EMBELL`).
pub(super) fn attach_embell(mut char_bytes: Vec<u8>, embell_code: u8) -> Vec<u8> {
    if is_single_char_record(&char_bytes) {
        char_bytes[1] |= 0x01; // OPT_CHAR_EMBELL
        char_bytes.extend_from_slice(&embell_record(embell_code));
        char_bytes.extend_from_slice(END);
    }
    char_bytes
}

/// Emit a base `CHAR` record together with any trailing prime or combining-mark embellishments.
pub(super) fn emit_char_with_combining_marks(
    chars: &[char],
    idx: &mut usize,
    typeface: u8,
    mt_code: u32,
) -> Vec<u8> {
    let mut rec = char_record(typeface, mt_code);
    let mut embells = Vec::new();

    // Consume any adjacent prime characters (e.g. y', y'', y''').
    let mut prime_count = 0u8;
    while *idx < chars.len() && is_prime_char(chars[*idx]) {
        match chars[*idx] {
            '\'' | '\u{2032}' | '\u{00B4}' => prime_count = prime_count.saturating_add(1),
            '\u{2033}' => prime_count = prime_count.saturating_add(2),
            '\u{2034}' => prime_count = prime_count.saturating_add(3),
            '`' | '\u{2035}' => embells.push(EMB_BPRIME),
            _ => break,
        }
        *idx += 1;
    }
    match prime_count {
        1 => embells.push(EMB_1PRIME),
        2 => embells.push(EMB_2PRIME),
        3.. => embells.push(EMB_3PRIME),
        0 => {}
    }

    while *idx < chars.len() {
        if let Some(emb) = combining_mark_to_embell(chars[*idx]) {
            embells.push(emb);
            *idx += 1;
        } else {
            break;
        }
    }
    if !embells.is_empty() && is_single_char_record(&rec) {
        rec[1] |= 0x01; // OPT_CHAR_EMBELL
        for emb in embells {
            rec.extend_from_slice(&embell_record(emb));
        }
        rec.extend_from_slice(END);
    }
    rec
}

/// Map a Unicode combining diacritic to its MTEF `EMB_*` embellishment code.
fn combining_mark_to_embell(ch: char) -> Option<u8> {
    match ch {
        '\u{0307}' => Some(EMB_1DOT),
        '\u{0308}' => Some(EMB_2DOT),
        '\u{20DB}' => Some(EMB_3DOT),
        '\u{20DC}' => Some(EMB_4DOT),
        '\u{0302}' => Some(EMB_HAT),
        '\u{0303}' => Some(EMB_TILDE),
        '\u{0304}' | '\u{0305}' => Some(EMB_OBAR),
        '\u{0311}' => Some(EMB_FROWN),
        '\u{0306}' => Some(EMB_SMILE),
        '\u{0338}' | '\u{20D2}' => Some(EMB_NOT),
        '\u{0335}' | '\u{0336}' => Some(EMB_MBAR),
        '\u{20D7}' => Some(EMB_RARROW),
        '\u{20D6}' => Some(EMB_LARROW),
        '\u{20E1}' => Some(EMB_BARROW),
        '\u{20D1}' => Some(EMB_R1ARROW),
        '\u{20D0}' => Some(EMB_L1ARROW),
        '\u{0323}' => Some(EMB_U_1DOT),
        '\u{0324}' => Some(EMB_U_2DOT),
        '\u{20E8}' => Some(EMB_U_3DOT),
        '\u{0331}' | '\u{0332}' => Some(EMB_U_BAR),
        '\u{0330}' => Some(EMB_U_TILDE),
        '\u{032F}' => Some(EMB_U_FROWN),
        '\u{032E}' => Some(EMB_U_SMILE),
        '\u{20EF}' => Some(EMB_U_RARROW),
        '\u{20EE}' => Some(EMB_U_LARROW),
        '\u{034D}' => Some(EMB_U_BARROW),
        '\u{20EC}' => Some(EMB_U_R1ARROW),
        '\u{20ED}' => Some(EMB_U_L1ARROW),
        _ => None,
    }
}
