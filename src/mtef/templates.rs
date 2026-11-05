//! MTEF templates (fraction, radicals, scripts) built from child byte streams.

use super::records::*;

pub(super) fn cat(parts: &[&[u8]]) -> Vec<u8> {
    parts.concat()
}

/// `tmFRACT` template: two slots (numerator / denominator).
pub fn fraction(num: &[u8], den: &[u8]) -> Vec<u8> {
    cat(&[
        &tmpl_header(0x0B, 0, 0), // tmFRACT
        &line(false),
        num,
        END, // slot 1: numerator
        FULL, // reset size between slots
        &line(false),
        den,
        END, // slot 2: denominator
        END, // end TMPL
    ])
}

/// `tmROOT`, variation 0 (square root, one slot).
pub fn sqrt_radical(rad: &[u8]) -> Vec<u8> {
    cat(&[
        &tmpl_header(0x0A, 0, 0),
        &line(false),
        rad,
        END,
        &line(true),
        END,
    ])
}

/// `tmROOT`, variation 1 (nth root, two slots: degree + radicand).
pub fn nroot_radical(rad: &[u8], deg: &[u8]) -> Vec<u8> {
    cat(&[
        &tmpl_header(0x0A, 1, 0),
        &line(false),
        rad,
        END,
        &line(false),
        deg,
        END,
        END,
    ])
}

/// `tmSUB`: the base precedes this TMPL in the parent LINE; one content slot
/// plus a null slot.
pub fn subscript(sub: &[u8]) -> Vec<u8> {
    cat(&[
        FULL,
        &tmpl_header(0x1B, 0, 0),
        SUB,
        &line(false),
        sub,
        END,
        &line(true),
        END,
    ])
}

/// `tmSUP`: the base precedes this TMPL in the parent LINE.
pub fn superscript(sup: &[u8]) -> Vec<u8> {
    cat(&[
        FULL,
        &tmpl_header(0x1C, 0, 0),
        SUB,
        &line(true),
        &line(false),
        sup,
        END,
        END,
    ])
}

/// `tmSUBSUP`: the base precedes this TMPL in the parent LINE.
pub fn subsuperscript(sub: &[u8], sup: &[u8]) -> Vec<u8> {
    cat(&[
        FULL,
        &tmpl_header(0x1D, 0, 0),
        SUB,
        &line(false),
        sub,
        END,
        &line(false),
        sup,
        END,
        END,
    ])
}

// ── Over / Under & Extensible Arrow Templates ───────────────────────────────

/// `tmOBAR` (selector 13): Overbar over characters.
pub fn overbar(base: &[u8]) -> Vec<u8> {
    cat(&[
        &tmpl_header(13, 0, 0),
        &line(false),
        base,
        END,
        END,
    ])
}

/// `tmUBAR` (selector 12): Underbar under characters.
pub fn underbar(base: &[u8]) -> Vec<u8> {
    cat(&[
        &tmpl_header(12, 0, 0),
        &line(false),
        base,
        END,
        END,
    ])
}

/// `tmVEC` (selector 31): Vector arrow over characters.
///
/// Variation flags:
/// * `0x01`: left arrow (`tvVE_LEFT`)
/// * `0x02`: right arrow (`tvVE_RIGHT`)
/// * `0x03`: bidirectional arrow (`tvVE_LEFT | tvVE_RIGHT`)
pub fn vector_arrow(base: &[u8], arrow_char: &str) -> Vec<u8> {
    let var = if arrow_char.contains('↔') || arrow_char.contains('\u{2194}') || arrow_char.contains('\u{27F7}') {
        0x03 // Left + Right (bidirectional)
    } else if arrow_char.contains('←') || arrow_char.contains('\u{2190}') || arrow_char.contains('\u{27F5}') {
        0x01 // Left only
    } else {
        0x02 // Right only (default)
    };

    cat(&[
        &tmpl_header(31, var, 0),
        &line(false),
        base,
        END,
        END,
    ])
}

/// `tmVEC` (selector 31): Vector arrow under characters.
///
/// Variation flags:
/// * `0x04`: arrow under slot (`tvVE_UNDER`)
/// * `0x05`: left arrow under slot (`tvVE_UNDER | tvVE_LEFT`)
/// * `0x06`: right arrow under slot (`tvVE_UNDER | tvVE_RIGHT`)
/// * `0x07`: bidirectional arrow under slot (`tvVE_UNDER | tvVE_LEFT | tvVE_RIGHT`)
pub fn under_vector_arrow(base: &[u8], arrow_char: &str) -> Vec<u8> {
    let var = if arrow_char.contains('↔') || arrow_char.contains('\u{2194}') || arrow_char.contains('\u{27F7}') {
        0x07 // Under + Left + Right (bidirectional)
    } else if arrow_char.contains('←') || arrow_char.contains('\u{2190}') || arrow_char.contains('\u{27F5}') {
        0x05 // Under + Left
    } else {
        0x06 // Under + Right (default)
    };

    cat(&[
        &tmpl_header(31, var, 0),
        &line(false),
        base,
        END,
        END,
    ])
}

/// `tmARC` (selector 34): Arc over characters (e.g. \wideparen{AB}).
pub fn over_arc(base: &[u8]) -> Vec<u8> {
    cat(&[
        &tmpl_header(34, 0, 0),
        &line(false),
        base,
        END,
        END,
    ])
}

/// `tmHAT` (selector 33): Hat / circumflex over characters.
pub fn over_hat(base: &[u8]) -> Vec<u8> {
    cat(&[
        &tmpl_header(33, 0, 0),
        &line(false),
        base,
        END,
        END,
    ])
}

/// `tmARROW` (selector 14): Extensible arrows with labels above and/or below.
///
/// Variation flags:
/// * `0x04`: top slot present (`tvAR_TOP`)
/// * `0x08`: bottom slot present (`tvAR_BOTTOM`)
/// * `0x10`: left arrow (`tvAR_LEFT`)
/// * `0x20`: right arrow (`tvAR_RIGHT`)
/// * `0x30`: bidirectional arrow (`tvAR_LEFT | tvAR_RIGHT`)
/// `tmARROW` (selector 14): Extensible arrows with labels above and/or below.
pub fn extensible_arrow(
    arrow_char: &str,
    above_content: Option<&[u8]>,
    below_content: Option<&[u8]>,
) -> Vec<u8> {
    let dir_bits = if arrow_char.contains('↔') || arrow_char.contains('\u{2194}') || arrow_char.contains('\u{27F7}') {
        0x30 // Left + Right (bidirectional arrow)
    } else if arrow_char.contains('←') || arrow_char.contains('\u{2190}') || arrow_char.contains('\u{27F5}') {
        0x10 // Left arrow
    } else {
        0x20 // Right arrow (default)
    };

    // ALWAYS use 0x0C (Top | Bottom) to force exactly 2 slots.
    // This perfectly bypasses the Chitup/WPS parsing bug.
    let variation = dir_bits | 0x0C;

    if let Some(top) = above_content {
        cat(&[
            &tmpl_header(14, variation, 0),
            &line(false), // Slot 0 (top)
            top,
            END,
            &line(true),  // Slot 1 (bottom): empty
            END,          // end tmARROW
        ])
    } else if let Some(bottom) = below_content {
        cat(&[
            &tmpl_header(14, variation, 0),
            &line(true),  // Slot 0 (top): empty
            &line(false), // Slot 1 (bottom)
            bottom,
            END,
            END,          // end tmARROW
        ])
    } else {
        cat(&[
            &tmpl_header(14, variation, 0),
            &line(true),  // Slot 0: empty
            &line(true),  // Slot 1: empty
            END,          // end tmARROW
        ])
    }
}

// ── Generic Overscript & Underscript (mapped to \overset and \underset) ──────

/// `tmLIM` (selector 23, variation 0x02): Upper limit / overscript (e.g. \overset{over}{base}).
pub fn overscript(base: &[u8], over: &[u8]) -> Vec<u8> {
    cat(&[
        &tmpl_header(23, 0x02, 0),
        &line(false),
        base,
        END, // slot 1: base
        &line(false),
        over,
        END, // slot 2: upper
        END, // end tmLIM
    ])
}

/// `tmLIM` (selector 23, variation 0x01): Lower limit / underscript (e.g. \underset{under}{base}).
pub fn underscript(base: &[u8], under: &[u8]) -> Vec<u8> {
    cat(&[
        &tmpl_header(23, 0x01, 0),
        &line(false),
        base,
        END, // slot 1: base
        &line(false),
        under,
        END, // slot 2: lower
        END, // end tmLIM
    ])
}

/// `tmLIM` (selector 23, variation 0x43): Limits both above and below (summation style).
///
/// Variation flags:
/// * `0x01`: lower limit present (`tvBO_LOWER`)
/// * `0x02`: upper limit present (`tvBO_UPPER`)
/// * `0x40`: center directly above and below (`tvBO_SUM`)
/// * Total variation = `0x01 | 0x02 | 0x40 = 0x43`
///
/// Subobject order for LimBoxClass:
/// 1. Slot 1: base expression
/// 2. Slot 2: lower limit (under)
/// 3. Slot 3: upper limit (over)
/// 4. Final `END` to close template
pub fn under_overscript(base: &[u8], under: &[u8], over: &[u8]) -> Vec<u8> {
    cat(&[
        &tmpl_header(23, 0x43, 0),
        &line(false),
        base,
        END, // slot 1: base
        &line(false),
        under,
        END, // slot 2: lower limit (under)
        &line(false),
        over,
        END, // slot 3: upper limit (over)
        END, // end tmLIM template
    ])
}

/// `tmVEC` (selector 31): Harpoon vector arrow over characters (2-stroke arrow).
///
/// Variation flags:
/// * Left harpoon:  `tvVE_LEFT`  (0x01) | `tvVE_HARPOON` (0x08) = 0x09
/// * Right harpoon: `tvVE_RIGHT` (0x02) | `tvVE_HARPOON` (0x08) = 0x0A
pub fn harpoon_arrow(base: &[u8], is_left: bool) -> Vec<u8> {
    let var = if is_left { 0x09 } else { 0x0A };
    cat(&[
        &tmpl_header(31, var, 0),
        &line(false),
        base,
        END,
        END,
    ])
}
