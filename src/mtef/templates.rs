//! MTEF 5.0 templates, composite records (pile, matrix, embellishments),
//! selector constants, and variation flags.
//!
//! Verified against `mathtypejx` (`records5.py`, `builder.py`, `mover.py`,
//! and XSLT stylesheets in `src/mathtypejx/mtef/xslt/xsl/*.xsl`) as well as
//! the MTEF 5 specification.

use super::records::*;
use super::typeface::{TF_EXPAND, TF_MTEXTRA, TF_SYMBOL};

// ── Additional size record constants (MTEF 5) ───────────────────────────────

/// `SYM` size record (`0x0D`): symbol size (used for big-operator characters).
pub const SYM: &[u8] = b"\x0d";
/// `SUBSYM` size record (`0x0E`): sub-symbol size.
pub const SUBSYM: &[u8] = b"\x0e";

// ── Template selector constants (0..=37, matching `records5.py` SELECTORS) ──

/// `tmANGLE` (0): angle brackets `< ... >`.
pub const TM_ANGLE: u8 = 0;
/// `tmPAREN` (1): parentheses `( ... )`.
pub const TM_PAREN: u8 = 1;
/// `tmBRACE` (2): braces `{ ... }`.
pub const TM_BRACE: u8 = 2;
/// `tmBRACK` (3): square brackets `[ ... ]`.
pub const TM_BRACK: u8 = 3;
/// `tmBAR` (4): vertical bars `| ... |`.
pub const TM_BAR: u8 = 4;
/// `tmDBAR` (5): double vertical bars `|| ... ||`.
pub const TM_DBAR: u8 = 5;
/// `tmFLOOR` (6): floor brackets.
pub const TM_FLOOR: u8 = 6;
/// `tmCEILING` (7): ceiling brackets.
pub const TM_CEILING: u8 = 7;
/// `tmOBRACK` (8): open (white/tortoise-shell) brackets.
pub const TM_OBRACK: u8 = 8;
/// `tmINTERVAL` (9): unmatched interval fences.
pub const TM_INTERVAL: u8 = 9;
/// `tmROOT` (10): square root / nth root radical.
pub const TM_ROOT: u8 = 10;
/// `tmFRACT` (11): fraction.
pub const TM_FRACT: u8 = 11;
/// `tmUBAR` (12): underbar.
pub const TM_UBAR: u8 = 12;
/// `tmOBAR` (13): overbar.
pub const TM_OBAR: u8 = 13;
/// `tmARROW` (14): extensible arrow with text above/below.
pub const TM_ARROW: u8 = 14;
/// `tmINTEG` (15): integral.
pub const TM_INTEG: u8 = 15;
/// `tmSUM` (16): summation.
pub const TM_SUM: u8 = 16;
/// `tmPROD` (17): product.
pub const TM_PROD: u8 = 17;
/// `tmCOPROD` (18): coproduct.
pub const TM_COPROD: u8 = 18;
/// `tmUNION` (19): union.
pub const TM_UNION: u8 = 19;
/// `tmINTER` (20): intersection.
pub const TM_INTER: u8 = 20;
/// `tmINTOP` (21): integral-style big operator.
pub const TM_INTOP: u8 = 21;
/// `tmSUMOP` (22): summation-style big operator.
pub const TM_SUMOP: u8 = 22;
/// `tmLIM` (23): limits (underscript / overscript / under-overscript).
pub const TM_LIM: u8 = 23;
/// `tmHBRACE` (24): horizontal brace (overbrace / underbrace).
pub const TM_HBRACE: u8 = 24;
/// `tmHBRACK` (25): horizontal bracket (overbracket / underbracket).
pub const TM_HBRACK: u8 = 25;
/// `tmLDIV` (26): long division.
pub const TM_LDIV: u8 = 26;
/// `tmSUB` (27): subscript.
pub const TM_SUB: u8 = 27;
/// `tmSUP` (28): superscript.
pub const TM_SUP: u8 = 28;
/// `tmSUBSUP` (29): subscript and superscript.
pub const TM_SUBSUP: u8 = 29;
/// `tmDIRAC` (30): Dirac bra-ket structure.
pub const TM_DIRAC: u8 = 30;
/// `tmVEC` (31): vector / harpoon arrow over or under a slot.
pub const TM_VEC: u8 = 31;
/// `tmTILDE` (32): tilde over a slot.
pub const TM_TILDE: u8 = 32;
/// `tmHAT` (33): hat / circumflex over a slot.
pub const TM_HAT: u8 = 33;
/// `tmARC` (34): arc / frown over a slot.
pub const TM_ARC: u8 = 34;
/// `tmJSTATUS` (35): joint-status over a slot.
pub const TM_JSTATUS: u8 = 35;
/// `tmSTRIKE` (36): strike-through / cancellation.
pub const TM_STRIKE: u8 = 36;
/// `tmBOX` (37): rectangular or rounded box.
pub const TM_BOX: u8 = 37;

// ── Template variation flags (matching `records5.py` VARIATIONS) ────────────

// Fences (selectors 0..=8)
/// Left fence is present (`tvFENCE_L`).
pub const TV_FENCE_L: u8 = 0x01;
/// Right fence is present (`tvFENCE_R`).
pub const TV_FENCE_R: u8 = 0x02;

// Intervals (selector 9)
/// Left fence is left parenthesis `(` (`tvINTV_LEFT_LP`).
pub const TV_INTV_LEFT_LP: u8 = 0x00;
/// Left fence is right parenthesis `)` (`tvINTV_LEFT_RP`).
pub const TV_INTV_LEFT_RP: u8 = 0x01;
/// Left fence is left bracket `[` (`tvINTV_LEFT_LB`).
pub const TV_INTV_LEFT_LB: u8 = 0x02;
/// Left fence is right bracket `]` (`tvINTV_LEFT_RB`).
pub const TV_INTV_LEFT_RB: u8 = 0x03;
/// Right fence is left parenthesis `(` (`tvINTV_RIGHT_LP`).
pub const TV_INTV_RIGHT_LP: u8 = 0x00;
/// Right fence is right parenthesis `)` (`tvINTV_RIGHT_RP`).
pub const TV_INTV_RIGHT_RP: u8 = 0x10;
/// Right fence is left bracket `[` (`tvINTV_RIGHT_LB`).
pub const TV_INTV_RIGHT_LB: u8 = 0x20;
/// Right fence is right bracket `]` (`tvINTV_RIGHT_RB`).
pub const TV_INTV_RIGHT_RB: u8 = 0x30;

// Radicals (selector 10)
/// Square root (`tvROOT_SQ`).
pub const TV_ROOT_SQ: u8 = 0x00;
/// Nth root (`tvROOT_NTH`).
pub const TV_ROOT_NTH: u8 = 0x01;

// Fractions (selector 11)
/// Subscript-size slots (piece fraction, `tvFR_SMALL`).
pub const TV_FR_SMALL: u8 = 0x01;
/// Diagonal slash fraction (`tvFR_SLASH`).
pub const TV_FR_SLASH: u8 = 0x02;
/// Numerator and denominator on baseline (`tvFR_BASE`).
pub const TV_FR_BASE: u8 = 0x04;

// Underbar / Overbar (selectors 12, 13)
/// Double bar (`tvBAR_DOUBLE`); 0 is single bar.
pub const TV_BAR_DOUBLE: u8 = 0x01;

// Extensible arrows (selector 14)
/// Single arrow (`tvAR_SINGLE`).
pub const TV_AR_SINGLE: u8 = 0x00;
/// Double arrow (`tvAR_DOUBLE`, e.g. `⇄`).
pub const TV_AR_DOUBLE: u8 = 0x01;
/// Harpoon (`tvAR_HARPOON`, e.g. `⇌`).
pub const TV_AR_HARPOON: u8 = 0x02;
/// Top slot is present (`tvAR_TOP`).
pub const TV_AR_TOP: u8 = 0x04;
/// Bottom slot is present (`tvAR_BOTTOM`).
pub const TV_AR_BOTTOM: u8 = 0x08;
/// Both top and bottom slots present (`tvAR_TOP | tvAR_BOTTOM` -> `tvAR_TOPBOTTOM`).
pub const TV_AR_TOPBOTTOM: u8 = TV_AR_TOP | TV_AR_BOTTOM;
/// Arrow points left (`tvAR_LEFT`; with `TV_AR_SINGLE`).
pub const TV_AR_LEFT: u8 = 0x10;
/// Arrow points right (`tvAR_RIGHT`; with `TV_AR_SINGLE`).
pub const TV_AR_RIGHT: u8 = 0x20;
/// Large over small (`tvAR_LOS`; with `TV_AR_DOUBLE` or `TV_AR_HARPOON`).
pub const TV_AR_LOS: u8 = 0x40;
/// Small over large (`tvAR_SOL`; with `TV_AR_DOUBLE` or `TV_AR_HARPOON`).
pub const TV_AR_SOL: u8 = 0x80;

// Integrals (selector 15)
/// Single integral sign (`tvINT_1`).
pub const TV_INT_1: u16 = 0x0001;
/// Double integral sign (`tvINT_2`).
pub const TV_INT_2: u16 = 0x0002;
/// Triple integral sign (`tvINT_3`).
pub const TV_INT_3: u16 = 0x0003;
/// Contour loop integral (`tvINT_LOOP`, no direction arrows).
pub const TV_INT_LOOP: u16 = 0x0004;
/// Clockwise loop integral (`tvINT_CW_LOOP`).
pub const TV_INT_CW_LOOP: u16 = 0x0008;
/// Counter-clockwise loop integral (`tvINT_CCW_LOOP`).
pub const TV_INT_CCW_LOOP: u16 = 0x000C;
/// Expandable integral sign (`tvINT_EXPAND`).
pub const TV_INT_EXPAND: u16 = 0x0100;

// Big operators & Limits (selectors 15..=23)
/// Lower limit is present (`tvBO_LOWER`).
pub const TV_BO_LOWER: u16 = 0x0010;
/// Upper limit is present (`tvBO_UPPER`).
pub const TV_BO_UPPER: u16 = 0x0020;
/// Summation-style limits centered above/below (`tvBO_SUM`); if clear, integral-style (`tvBO_INT`).
pub const TV_BO_SUM: u16 = 0x0040;

// Limits underbar variants (selector 23)
/// Single underbar on limit (`tvSUBAR`).
pub const TV_SUBAR: u8 = 0x00;
/// Double underbar on limit (`tvDUBAR`).
pub const TV_DUBAR: u8 = 0x01;

// Horizontal braces / brackets (selectors 24, 25)
/// Slot is on top (`tvHB_TOP`); otherwise on bottom.
pub const TV_HB_TOP: u8 = 0x01;

// Long division (selector 26)
/// Upper quotient slot is present (`tvLD_UPPER`).
pub const TV_LD_UPPER: u8 = 0x01;

// Subscript / Superscript (selectors 27, 28, 29)
/// Script precedes scripted item (`tvSU_PRECEDES`, e.g. prescripts `<mmultiscripts>`).
pub const TV_SU_PRECEDES: u8 = 0x01;

// Dirac bra-ket (selector 30)
/// Left part (bra) is present (`tvDI_LEFT`).
pub const TV_DI_LEFT: u8 = 0x01;
/// Right part (ket) is present (`tvDI_RIGHT`).
pub const TV_DI_RIGHT: u8 = 0x02;

// Vector / harpoon arrow (selector 31)
/// Arrow points left (`tvVE_LEFT`).
pub const TV_VE_LEFT: u8 = 0x01;
/// Arrow points right (`tvVE_RIGHT`).
pub const TV_VE_RIGHT: u8 = 0x02;
/// Arrow is under slot (`tvVE_UNDER`), else over slot.
pub const TV_VE_UNDER: u8 = 0x04;
/// Harpoon (single barb, `tvVE_HARPOON`), else regular arrow.
pub const TV_VE_HARPOON: u8 = 0x08;

// Strike-through (selector 36)
/// Horizontal strike (`tvST_HORIZ`).
pub const TV_ST_HORIZ: u8 = 0x01;
/// Bottom-left to upper-right strike (`tvST_UP`).
pub const TV_ST_UP: u8 = 0x02;
/// Top-left to bottom-right strike (`tvST_DOWN`).
pub const TV_ST_DOWN: u8 = 0x04;

// Box (selector 37)
/// Rounded corners (`tvBX_ROUND`).
pub const TV_BX_ROUND: u8 = 0x01;
/// Left border present (`tvBX_LEFT`).
pub const TV_BX_LEFT: u8 = 0x02;
/// Right border present (`tvBX_RIGHT`).
pub const TV_BX_RIGHT: u8 = 0x04;
/// Top border present (`tvBX_TOP`).
pub const TV_BX_TOP: u8 = 0x08;
/// Bottom border present (`tvBX_BOTTOM`).
pub const TV_BX_BOTTOM: u8 = 0x10;

// ── Pile / Matrix alignment constants (matching `records5.py`) ──────────────

/// Horizontal alignment: left (`1`).
pub const HALIGN_LEFT: u8 = 1;
/// Horizontal alignment: center (`2`).
pub const HALIGN_CENTER: u8 = 2;
/// Horizontal alignment: right (`3`).
pub const HALIGN_RIGHT: u8 = 3;
/// Horizontal alignment: relational operator (`4`).
pub const HALIGN_AL: u8 = 4;
/// Horizontal alignment: decimal point (`5`).
pub const HALIGN_DEC: u8 = 5;

/// Vertical alignment: top baseline (`0`).
pub const VALIGN_TOP_BASELINE: u8 = 0;
/// Vertical alignment: center baseline (`1`).
pub const VALIGN_CENTER_BASELINE: u8 = 1;
/// Vertical alignment: bottom baseline (`2`).
pub const VALIGN_BOTTOM_BASELINE: u8 = 2;
/// Vertical alignment: center (`3`).
pub const VALIGN_CENTER: u8 = 3;
/// Vertical alignment: math axis (`4`).
pub const VALIGN_AXIS: u8 = 4;

// ── Character embellishment codes (record type 6, matching `records5.py`) ───

pub const EMB_1DOT: u8 = 2;
pub const EMB_2DOT: u8 = 3;
pub const EMB_3DOT: u8 = 4;
pub const EMB_1PRIME: u8 = 5;
pub const EMB_2PRIME: u8 = 6;
pub const EMB_BPRIME: u8 = 7;
pub const EMB_TILDE: u8 = 8;
pub const EMB_HAT: u8 = 9;
pub const EMB_NOT: u8 = 10;
pub const EMB_RARROW: u8 = 11;
pub const EMB_LARROW: u8 = 12;
pub const EMB_BARROW: u8 = 13;
pub const EMB_R1ARROW: u8 = 14;
pub const EMB_L1ARROW: u8 = 15;
pub const EMB_MBAR: u8 = 16;
pub const EMB_OBAR: u8 = 17;
pub const EMB_3PRIME: u8 = 18;
pub const EMB_FROWN: u8 = 19;
pub const EMB_SMILE: u8 = 20;
pub const EMB_X_BARS: u8 = 21;
pub const EMB_UP_BAR: u8 = 22;
pub const EMB_DOWN_BAR: u8 = 23;
pub const EMB_4DOT: u8 = 24;
pub const EMB_U_1DOT: u8 = 25;
pub const EMB_U_2DOT: u8 = 26;
pub const EMB_U_3DOT: u8 = 27;
pub const EMB_U_4DOT: u8 = 28;
pub const EMB_U_BAR: u8 = 29;
pub const EMB_U_TILDE: u8 = 30;
pub const EMB_U_FROWN: u8 = 31;
pub const EMB_U_SMILE: u8 = 32;
pub const EMB_U_RARROW: u8 = 33;
pub const EMB_U_LARROW: u8 = 34;
pub const EMB_U_BARROW: u8 = 35;
pub const EMB_U_R1ARROW: u8 = 36;
pub const EMB_U_L1ARROW: u8 = 37;

// ── Internal helpers ────────────────────────────────────────────────────────

pub(super) fn cat(parts: &[&[u8]]) -> Vec<u8> {
    parts.concat()
}

/// Append a `LINE` slot to `out`. An empty slice is encoded as a null `LINE`
/// (`[0x01, 0x01]`, no trailing `END`), which still produces a `<slot>` element
/// in `mathtypejx/mtef/builder.py` so positional XSLT selectors (`slot[1]`,
/// `slot[2]`, `slot[3]`, `slot[4]`) remain aligned.
fn append_line_slot(out: &mut Vec<u8>, content: &[u8]) {
    if content.is_empty() {
        out.extend_from_slice(&line(true));
    } else {
        out.extend_from_slice(&line(false));
        out.extend_from_slice(content);
        out.extend_from_slice(END);
    }
}

/// Append an optional `LINE` slot to `out`. `None` or `Some(&[])` writes a
/// null `LINE` (`[0x01, 0x01]`).
fn append_opt_line_slot(out: &mut Vec<u8>, content: Option<&[u8]>) {
    match content {
        Some(c) if !c.is_empty() => {
            out.extend_from_slice(&line(false));
            out.extend_from_slice(c);
            out.extend_from_slice(END);
        }
        _ => out.extend_from_slice(&line(true)),
    }
}

/// Encode a `TMPL` header whose variation may use 1 or 2 bytes (MTEF 5).
fn tmpl_header_u16(selector: u8, variation: u16, tmpl_opts: u8) -> Vec<u8> {
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

/// Generic one-slot template helper. Use only with selectors whose class has
/// exactly one line slot (`BarBoxClass`, `HatBoxClass`, `StrikeBoxClass`, `TBoxBoxClass`).
pub fn one_slot_template(selector: u8, variation: u8, content: &[u8]) -> Vec<u8> {
    let mut out = tmpl_header(selector, variation, 0);
    append_line_slot(&mut out, content);
    out.extend_from_slice(END);
    out
}

/// Template with one line slot followed by an expandable CHAR record (`HatBoxClass`:
/// `tmVEC`, `tmTILDE`, `tmHAT`, `tmARC`, `tmJSTATUS`).
pub fn hat_box_template(selector: u8, variation: u8, content: &[u8], mt_code: u32) -> Vec<u8> {
    let mut out = tmpl_header(selector, variation, 0);
    append_line_slot(&mut out, content);
    out.extend_from_slice(&char_record(TF_EXPAND, mt_code));
    out.extend_from_slice(END);
    out
}

// ── Fractions & Radicals (selectors 10, 11) ─────────────────────────────────

/// `tmFRACT` (selector 11, variation 0): standard full-size fraction with two
/// slots (numerator / denominator).
pub fn fraction(num: &[u8], den: &[u8]) -> Vec<u8> {
    cat(&[
        &tmpl_header(TM_FRACT, 0, 0),
        &line(false),
        num,
        END,
        &line(false),
        den,
        END,
        END,
    ])
}

/// `tmFRACT` (selector 11) variants: piece fraction (`TV_FR_SMALL = 0x01`),
/// slash fraction (`TV_FR_SLASH = 0x02`), or baseline-aligned slash fraction
/// (`TV_FR_SLASH | TV_FR_BASE = 0x06`).
pub fn fraction_variant(num: &[u8], den: &[u8], variation: u8) -> Vec<u8> {
    let mut out = tmpl_header(TM_FRACT, variation, 0);
    if variation & TV_FR_SMALL != 0 {
        out.extend_from_slice(SUB);
    }
    append_line_slot(&mut out, num);
    if variation & TV_FR_SMALL != 0 {
        out.extend_from_slice(SUB);
    }
    append_line_slot(&mut out, den);
    out.extend_from_slice(END);
    out
}

/// Diagonal / bevelled fraction (`tmFRACT` with `tvFR_SLASH`).
pub fn slash_fraction(num: &[u8], den: &[u8]) -> Vec<u8> {
    fraction_variant(num, den, TV_FR_SLASH)
}

/// Piece / small fraction (`tmFRACT` with `tvFR_SMALL`).
pub fn small_fraction(num: &[u8], den: &[u8]) -> Vec<u8> {
    let mut out = fraction_variant(num, den, TV_FR_SMALL);
    out.extend_from_slice(FULL);
    out
}

/// Linear / baseline slash fraction (`tmFRACT` with `tvFR_SLASH | tvFR_BASE`).
pub fn base_slash_fraction(num: &[u8], den: &[u8]) -> Vec<u8> {
    fraction_variant(num, den, TV_FR_SLASH | TV_FR_BASE)
}

/// `tmROOT` (selector 10, variation `TV_ROOT_SQ = 0`): square root (radicand
/// slot + null degree slot).
pub fn sqrt_radical(rad: &[u8]) -> Vec<u8> {
    cat(&[
        &tmpl_header(TM_ROOT, TV_ROOT_SQ, 0),
        &line(false),
        rad,
        END,
        SUB,
        &line(true),
        END,
        FULL,
    ])
}

/// `tmROOT` (selector 10, variation `TV_ROOT_NTH = 1`): nth root (two slots:
/// `slot[1]` = radicand, `slot[2]` = degree in `SUB` size).
pub fn nroot_radical(rad: &[u8], deg: &[u8]) -> Vec<u8> {
    cat(&[
        &tmpl_header(TM_ROOT, TV_ROOT_NTH, 0),
        &line(false),
        rad,
        END,
        SUB,
        &line(false),
        deg,
        END,
        END,
        FULL,
    ])
}

// ── Subscripts & Superscripts (selectors 27, 28, 29) ────────────────────────
//
// In MTEF 5 (`ScrBoxClass`) and `mathtypejx` (`mover.py` + `subsup.xsl`):
// - Without `tvSU_PRECEDES` (0x00), the base precedes the TMPL in the parent LINE.
// - With `tvSU_PRECEDES` (0x01), the base follows the TMPL in the parent LINE.
// - Inside the TMPL, there are ALWAYS 2 slots after `SUB`:
//     1. Subscript slot (`line(true)` for `tmSUP`)
//     2. Superscript slot (`line(true)` for `tmSUB`)
//   When `mover.py` runs, it inserts the base at `slot[1]`, shifting the
//   subscript to `slot[2]` and the superscript to `slot[3]`.

/// `tmSUB` (selector 27): the base precedes this TMPL in the parent LINE;
/// slot 1 is the subscript and slot 2 is a null slot.
pub fn subscript(sub: &[u8]) -> Vec<u8> {
    cat(&[
        &tmpl_header(TM_SUB, 0, 0),
        SUB,
        &line(false),
        sub,
        END,
        &line(true),
        END,
        FULL,
    ])
}

/// `tmSUP` (selector 28): the base precedes this TMPL in the parent LINE;
/// slot 1 is a null slot and slot 2 is the superscript.
pub fn superscript(sup: &[u8]) -> Vec<u8> {
    cat(&[
        &tmpl_header(TM_SUP, 0, 0),
        SUB,
        &line(true),
        &line(false),
        sup,
        END,
        END,
        FULL,
    ])
}

/// `tmSUBSUP` (selector 29): the base precedes this TMPL in the parent LINE;
/// slot 1 is the subscript and slot 2 is the superscript.
pub fn subsuperscript(sub: &[u8], sup: &[u8]) -> Vec<u8> {
    cat(&[
        &tmpl_header(TM_SUBSUP, 0, 0),
        SUB,
        &line(false),
        sub,
        END,
        &line(false),
        sup,
        END,
        END,
        FULL,
    ])
}

/// General `tmSUB` / `tmSUP` / `tmSUBSUP` (selectors 27, 28, 29) builder.
///
/// Always emits both the subscript slot and the superscript slot (using a null
/// `LINE` for the unused slot in `tmSUB` or `tmSUP`) so `mathtypejx`'s
/// `mover.py` and `subsup.xsl` (`slot[2]` for sub, `slot[3]` for sup) work
/// identically for postscripts (`precedes = false`) and prescripts
/// (`precedes = true`, `tvSU_PRECEDES`).
pub fn script_template(
    selector: u8,
    first: &[u8],
    second: Option<&[u8]>,
    precedes: bool,
) -> Vec<u8> {
    let variation = if precedes { TV_SU_PRECEDES } else { 0x00 };
    let mut out = tmpl_header(selector, variation, 0);
    out.extend_from_slice(SUB);
    match selector {
        TM_SUP => {
            // For tmSUP, if `second` is None, `first` is the superscript and
            // the subscript slot (slot 1) must be a null line.
            if let Some(sup) = second {
                append_line_slot(&mut out, first);
                append_line_slot(&mut out, sup);
            } else {
                out.extend_from_slice(&line(true));
                append_line_slot(&mut out, first);
            }
        }
        TM_SUB => {
            append_line_slot(&mut out, first);
            append_opt_line_slot(&mut out, second);
        }
        _ => {
            // TM_SUBSUP
            append_line_slot(&mut out, first);
            append_opt_line_slot(&mut out, second);
        }
    }
    out.extend_from_slice(END);
    out.extend_from_slice(FULL);
    out
}

/// Prescript subscript (`tmSUB` with `tvSU_PRECEDES`): the template precedes
/// `base` in the parent `LINE`.
pub fn presubscript(base: &[u8], sub: &[u8]) -> Vec<u8> {
    let tmpl = script_template(TM_SUB, sub, None, true);
    cat(&[&tmpl, base])
}

/// Prescript superscript (`tmSUP` with `tvSU_PRECEDES`): the template precedes
/// `base` in the parent `LINE`.
pub fn presuperscript(base: &[u8], sup: &[u8]) -> Vec<u8> {
    let tmpl = script_template(TM_SUP, sup, None, true);
    cat(&[&tmpl, base])
}

/// Prescript sub+superscript (`tmSUBSUP` with `tvSU_PRECEDES`): the template
/// precedes `base` in the parent `LINE`.
pub fn presubsuperscript(base: &[u8], sub: &[u8], sup: &[u8]) -> Vec<u8> {
    let tmpl = script_template(TM_SUBSUP, sub, Some(sup), true);
    cat(&[&tmpl, base])
}

// ── Over / Under Bars, Vectors, Hats, Arcs & Extensible Arrows ──────────────

/// `tmOBAR` (selector 13, variation 0): single overbar over characters.
pub fn overbar(base: &[u8]) -> Vec<u8> {
    one_slot_template(TM_OBAR, 0, base)
}

/// `tmOBAR` (selector 13, variation `TV_BAR_DOUBLE = 1`): double overbar.
pub fn double_overbar(base: &[u8]) -> Vec<u8> {
    one_slot_template(TM_OBAR, TV_BAR_DOUBLE, base)
}

/// `tmUBAR` (selector 12, variation 0): single underbar under characters.
pub fn underbar(base: &[u8]) -> Vec<u8> {
    one_slot_template(TM_UBAR, 0, base)
}

/// `tmUBAR` (selector 12, variation `TV_BAR_DOUBLE = 1`): double underbar.
pub fn double_underbar(base: &[u8]) -> Vec<u8> {
    one_slot_template(TM_UBAR, TV_BAR_DOUBLE, base)
}

/// `tmUBAR`/`tmOBAR` single or double bar variants (selectors 12 and 13).
pub fn bar_variant(base: &[u8], under: bool, double: bool) -> Vec<u8> {
    one_slot_template(
        if under { TM_UBAR } else { TM_OBAR },
        if double { TV_BAR_DOUBLE } else { 0 },
        base,
    )
}

/// `tmVEC` (selector 31): vector arrow over characters.
///
/// Variation flags:
/// * `0x01`: left arrow (`TV_VE_LEFT`)
/// * `0x02`: right arrow (`TV_VE_RIGHT`)
/// * `0x03`: bidirectional arrow (`TV_VE_LEFT | TV_VE_RIGHT`)
pub fn vector_arrow(base: &[u8], arrow_char: &str) -> Vec<u8> {
    let (var, mt_code) = if arrow_char.contains('↔')
        || arrow_char.contains('\u{2194}')
        || arrow_char.contains('\u{27F7}')
    {
        (TV_VE_LEFT | TV_VE_RIGHT, 0x20E1)
    } else if arrow_char.contains('←')
        || arrow_char.contains('\u{2190}')
        || arrow_char.contains('\u{20D6}')
        || arrow_char.contains('\u{27F5}')
    {
        (TV_VE_LEFT, 0x20D6)
    } else {
        (TV_VE_RIGHT, 0x20D7)
    };

    hat_box_template(TM_VEC, var, base, mt_code)
}

/// `tmVEC` (selector 31): vector arrow under characters.
///
/// Variation flags:
/// * `0x05`: left arrow under slot (`TV_VE_UNDER | TV_VE_LEFT`)
/// * `0x06`: right arrow under slot (`TV_VE_UNDER | TV_VE_RIGHT`)
/// * `0x07`: bidirectional arrow under slot (`TV_VE_UNDER | TV_VE_LEFT | TV_VE_RIGHT`)
pub fn under_vector_arrow(base: &[u8], arrow_char: &str) -> Vec<u8> {
    let (dir, mt_code) = if arrow_char.contains('↔')
        || arrow_char.contains('\u{2194}')
        || arrow_char.contains('\u{27F7}')
    {
        (TV_VE_LEFT | TV_VE_RIGHT, 0x20E1)
    } else if arrow_char.contains('←')
        || arrow_char.contains('\u{2190}')
        || arrow_char.contains('\u{20D6}')
        || arrow_char.contains('\u{27F5}')
    {
        (TV_VE_LEFT, 0x20D6)
    } else {
        (TV_VE_RIGHT, 0x20D7)
    };

    hat_box_template(TM_VEC, TV_VE_UNDER | dir, base, mt_code)
}

/// `tmVEC` (selector 31): harpoon vector arrow over characters (single-barbed arrow).
///
/// Variation flags:
/// * Left harpoon:  `TV_VE_LEFT (0x01) | TV_VE_HARPOON (0x08) = 0x09`
/// * Right harpoon: `TV_VE_RIGHT (0x02) | TV_VE_HARPOON (0x08) = 0x0A`
pub fn harpoon_arrow(base: &[u8], is_left: bool) -> Vec<u8> {
    let (var, mt_code) = if is_left {
        (TV_VE_LEFT | TV_VE_HARPOON, 0x20D0)
    } else {
        (TV_VE_RIGHT | TV_VE_HARPOON, 0x20D1)
    };
    hat_box_template(TM_VEC, var, base, mt_code)
}

/// `tmVEC` (selector 31): harpoon vector arrow under characters.
pub fn under_harpoon_arrow(base: &[u8], is_left: bool) -> Vec<u8> {
    let (var, mt_code) = if is_left {
        (TV_VE_UNDER | TV_VE_LEFT | TV_VE_HARPOON, 0xEB18)
    } else {
        (TV_VE_UNDER | TV_VE_RIGHT | TV_VE_HARPOON, 0xEB19)
    };
    hat_box_template(TM_VEC, var, base, mt_code)
}

/// `tmTILDE` (selector 32): extensible tilde over a slot.
pub fn over_tilde(base: &[u8]) -> Vec<u8> {
    hat_box_template(TM_TILDE, 0, base, 0x0303)
}

/// `tmHAT` (selector 33): hat / circumflex over characters.
pub fn over_hat(base: &[u8]) -> Vec<u8> {
    hat_box_template(TM_HAT, 0, base, 0x0302)
}

/// `tmARC` (selector 34): arc / frown over characters (e.g. `\wideparen{AB}`).
pub fn over_arc(base: &[u8]) -> Vec<u8> {
    hat_box_template(TM_ARC, 0, base, 0x2322)
}

/// `tmJSTATUS` (selector 35): joint-status construct over a slot.
pub fn joint_status(base: &[u8]) -> Vec<u8> {
    hat_box_template(TM_JSTATUS, 0, base, 0xEE0D)
}

/// `tmARROW` (selector 14): extensible arrows with labels above and/or below.
///
/// Matches `records5.py` (`_resolve_variations`) and `arrow.xsl`:
/// * Style/direction bits:
///   * `0x01`: double arrow `⇄` (`TV_AR_DOUBLE`)
///   * `0x02`: harpoon `⇌` (`TV_AR_HARPOON`)
///   * `0x10`: left arrow `←` (`TV_AR_LEFT`)
///   * `0x20`: right arrow `→` (`TV_AR_RIGHT`)
///   * `0x30`: bidirectional arrow `↔` (`TV_AR_LEFT | TV_AR_RIGHT`)
/// * Slot presence bits:
///   * `0x04`: top slot present (`TV_AR_TOP` -> `<mover>`)
///   * `0x08`: bottom slot present (`TV_AR_BOTTOM` -> `<munder>`)
///   * `0x0C`: both top and bottom slots present (`TV_AR_TOPBOTTOM` -> `<munderover>`)
/// * Always emits 2 `LINE` slots (`slot[1]` = top, `slot[2]` = bottom, using
///   `line(true)` when a slot is absent) so `arrow.xsl` (`slot[1]` for top,
///   `slot[2]` for bottom) always indexes the right slot.
pub fn extensible_arrow(
    arrow_char: &str,
    above_content: Option<&[u8]>,
    below_content: Option<&[u8]>,
) -> Vec<u8> {
    let dir_bits = if arrow_char.contains('⇌')
        || arrow_char.contains('\u{21CC}')
        || arrow_char.contains('\u{21CB}')
    {
        TV_AR_HARPOON
    } else if arrow_char.contains('⇄')
        || arrow_char.contains('\u{21C4}')
        || arrow_char.contains('\u{21C6}')
    {
        TV_AR_DOUBLE
    } else if arrow_char.contains('↔')
        || arrow_char.contains('\u{2194}')
        || arrow_char.contains('\u{27F7}')
    {
        TV_AR_LEFT | TV_AR_RIGHT
    } else if arrow_char.contains('←')
        || arrow_char.contains('\u{2190}')
        || arrow_char.contains('\u{27F5}')
    {
        TV_AR_LEFT
    } else {
        TV_AR_RIGHT
    };

    let has_top = above_content.is_some_and(|s| !s.is_empty());
    let has_bottom = below_content.is_some_and(|s| !s.is_empty());
    let slot_bits = match (has_top, has_bottom) {
        (true, true) => TV_AR_TOP | TV_AR_BOTTOM,
        (false, true) => TV_AR_BOTTOM,
        (true, false) | (false, false) => TV_AR_TOP,
    };

    let variation = dir_bits | slot_bits;
    let mut out = tmpl_header(TM_ARROW, variation, 0);
    out.extend_from_slice(SUB);
    append_opt_line_slot(&mut out, above_content);
    append_opt_line_slot(&mut out, below_content);
    out.extend_from_slice(FULL);
    if (variation & TV_AR_HARPOON) != 0 {
        out.extend_from_slice(&char_record(TF_EXPAND, 0x21C0));
        out.extend_from_slice(&char_record(TF_EXPAND, 0x21BD));
    } else if (variation & TV_AR_DOUBLE) != 0 {
        out.extend_from_slice(&char_record(TF_EXPAND, 0x2192));
        out.extend_from_slice(&char_record(TF_EXPAND, 0x2190));
    } else if (variation & (TV_AR_LEFT | TV_AR_RIGHT)) == (TV_AR_LEFT | TV_AR_RIGHT) {
        out.extend_from_slice(&char_record(TF_EXPAND, 0x2194));
    } else if (variation & TV_AR_LEFT) != 0 {
        out.extend_from_slice(&char_record(TF_EXPAND, 0x2190));
    } else {
        out.extend_from_slice(&char_record(TF_EXPAND, 0x2192));
    }
    out.extend_from_slice(END);
    out
}

// ── Limits: Overscript, Underscript & Under-Overscript (selector 23) ────────
//
// In `records5.py` and `lim.xsl`:
// - `tvBO_LOWER = 0x10` (lower limit present)
// - `tvBO_UPPER = 0x20` (upper limit present)
// - `tvBO_SUM   = 0x40` (limits centered above/below)
// - `tmLIM` always has 3 slots:
//     1. `slot[1]`: main base expression
//     2. `slot[2]`: lower limit (`line(true)` if absent)
//     3. `slot[3]`: upper limit (`line(true)` if absent)

/// `tmLIM` (selector 23): general limit template.
///
/// Variation bits: `0x10` (`TV_BO_LOWER`), `0x20` (`TV_BO_UPPER`), `0x40`
/// (`TV_BO_SUM`). Always emits 3 slots (`slot[1]` = base, `slot[2]` = lower,
/// `slot[3]` = upper) so `lim.xsl` (`slot[3]` for `tvBO_UPPER`) resolves
/// correctly.
pub fn limits(
    base: &[u8],
    lower: Option<&[u8]>,
    upper: Option<&[u8]>,
    summation_style: bool,
) -> Vec<u8> {
    let has_lower = lower.is_some_and(|s| !s.is_empty());
    let has_upper = upper.is_some_and(|s| !s.is_empty());
    let mut variation: u8 = 0;
    if has_lower {
        variation |= TV_BO_LOWER as u8;
    }
    if has_upper {
        variation |= TV_BO_UPPER as u8;
    }
    if summation_style {
        variation |= TV_BO_SUM as u8;
    }

    let mut out = tmpl_header(TM_LIM, variation, 0);
    append_line_slot(&mut out, base);
    out.extend_from_slice(SUB);
    append_opt_line_slot(&mut out, lower);
    append_opt_line_slot(&mut out, upper);
    out.extend_from_slice(END);
    out.extend_from_slice(FULL);
    out
}

/// `tmLIM` (selector 23, variation `TV_BO_UPPER = 0x20`): upper limit /
/// overscript (e.g. `\overset{over}{base}`).
pub fn overscript(base: &[u8], over: &[u8]) -> Vec<u8> {
    limits(base, None, Some(over), false)
}

/// `tmLIM` (selector 23, variation `TV_BO_LOWER = 0x10`): lower limit /
/// underscript (e.g. `\underset{under}{base}`).
pub fn underscript(base: &[u8], under: &[u8]) -> Vec<u8> {
    limits(base, Some(under), None, false)
}

/// `tmLIM` (selector 23, variation `TV_BO_LOWER | TV_BO_UPPER | TV_BO_SUM = 0x70`):
/// limits both above and below (`\underset{under}{\overset{over}{base}}`).
pub fn under_overscript(base: &[u8], under: &[u8], over: &[u8]) -> Vec<u8> {
    limits(base, Some(under), Some(over), true)
}

// ── Integrals & Big Operators (selectors 15..=22) ───────────────────────────
//
// In `records5.py`, `builder.py`, `int.xsl`, `sum.xsl`, `product_coproduct.xsl`,
// and `union_intersection.xsl`:
// - Common variation flags:
//     * `TV_BO_LOWER = 0x0010`: lower limit present
//     * `TV_BO_UPPER = 0x0020`: upper limit present
//     * `TV_BO_SUM   = 0x0040`: limits above/below (summation style); when clear,
//       limits are sub/superscripts (integral style).
// - Slot order in `BigOpBoxClass` (selectors 15..=22) is ALWAYS:
//     1. `slot[1]`: main expression (integrand / summand)
//     2. `SUB` + `slot[2]`: lower limit (`line(true)` if absent)
//     3. `slot[3]`: upper limit (`line(true)` if absent)
//     4. `SYM` + operator `CHAR` record (`slot[4]` in `builder.py`)
//     5. `END`

fn default_bigop_char(selector: u8, variation: u16) -> Vec<u8> {
    match selector {
        TM_INTEG => {
            let mut out = Vec::new();
            let n = match variation & 0x0003 {
                2 => 2,
                3 => 3,
                _ => 1,
            };
            // MathType 5 encodes loop integrals as a loop circle glyph from MT Extra (TF_MTEXTRA),
            // followed by n copies of the integral sign (TF_SYMBOL, 0x222B).
            if (variation & TV_INT_CCW_LOOP) == TV_INT_CCW_LOOP {
                out.extend(char_record(TF_MTEXTRA, 0xEE00));
            } else if (variation & TV_INT_CW_LOOP) == TV_INT_CW_LOOP {
                out.extend(char_record(TF_MTEXTRA, 0xEE01));
            } else if (variation & TV_INT_LOOP) != 0 {
                let loop_glyph = match n {
                    2 => 0xEE12,
                    3 => 0xEE13,
                    _ => 0xEE11,
                };
                out.extend(char_record(TF_MTEXTRA, loop_glyph));
            }
            for _ in 0..n {
                out.extend(char_record(TF_SYMBOL, 0x222B));
            }
            out
        }
        TM_SUM | TM_SUMOP => char_record(TF_SYMBOL, 0x2211),
        TM_PROD => char_record(TF_SYMBOL, 0x220F),
        TM_COPROD => char_record(TF_SYMBOL, 0x2210),
        TM_UNION => char_record(TF_SYMBOL, 0x222A),
        TM_INTER => char_record(TF_SYMBOL, 0x2229),
        TM_INTOP => char_record(TF_SYMBOL, 0x222B),
        _ => char_record(TF_SYMBOL, 0x2211),
    }
}

/// Big operators (`tmINTEG`=15, `tmSUM`=16, `tmPROD`=17, `tmCOPROD`=18,
/// `tmUNION`=19, `tmINTER`=20, `tmINTOP`=21, `tmSUMOP`=22).
///
/// Slot order follows `BigOpBoxClass` and `mathtypejx` XSLT (`slot[1]` = main,
/// `slot[2]` = lower limit, `slot[3]` = upper limit, `slot[4]` = operator `CHAR`).
pub fn big_operator(
    selector: u8,
    main: &[u8],
    upper: Option<&[u8]>,
    lower: Option<&[u8]>,
    operator_char: &[u8],
    variation: u16,
) -> Vec<u8> {
    let has_lower = lower.is_some_and(|s| !s.is_empty());
    let has_upper = upper.is_some_and(|s| !s.is_empty());
    let mut var = variation;
    if has_lower {
        var |= TV_BO_LOWER;
    }
    if has_upper {
        var |= TV_BO_UPPER;
    }
    if selector == TM_INTEG && (var & 0x000F) == 0 {
        var |= TV_INT_1;
    }

    let mut out = tmpl_header_u16(selector, var, 0);
    append_line_slot(&mut out, main);
    out.extend_from_slice(SUB);
    append_opt_line_slot(&mut out, lower);
    append_opt_line_slot(&mut out, upper);
    out.extend_from_slice(SYM);
    if selector == TM_INTEG || operator_char.is_empty() {
        out.extend_from_slice(&default_bigop_char(selector, var));
    } else {
        out.extend_from_slice(operator_char);
    }
    out.extend_from_slice(END);
    out.extend_from_slice(FULL);
    out
}

/// `tmINTEG` (selector 15): single/double/triple/loop integrals.
///
/// `variation` uses `TV_INT_*` and `TV_BO_*` bits (`TV_BO_LOWER = 0x0010`,
/// `TV_BO_UPPER = 0x0020`, `TV_BO_SUM = 0x0040`).
pub fn integral(
    main: &[u8],
    upper: Option<&[u8]>,
    lower: Option<&[u8]>,
    operator_char: &[u8],
    variation: u16,
) -> Vec<u8> {
    big_operator(TM_INTEG, main, upper, lower, operator_char, variation)
}

/// `tmSUM` (selector 16): summation with optional lower and upper limits.
/// When `sum_style` is `true`, sets `TV_BO_SUM` (`0x40`) so limits appear
/// above and below `∑`.
pub fn summation(
    main: &[u8],
    lower: Option<&[u8]>,
    upper: Option<&[u8]>,
    sum_style: bool,
) -> Vec<u8> {
    let var = if sum_style { TV_BO_SUM } else { 0 };
    big_operator(TM_SUM, main, upper, lower, &[], var)
}

/// `tmPROD` (selector 17): product `∏` with optional lower and upper limits.
pub fn product(
    main: &[u8],
    lower: Option<&[u8]>,
    upper: Option<&[u8]>,
    sum_style: bool,
) -> Vec<u8> {
    let var = if sum_style { TV_BO_SUM } else { 0 };
    big_operator(TM_PROD, main, upper, lower, &[], var)
}

/// `tmCOPROD` (selector 18): coproduct `∐` with optional lower and upper limits.
pub fn coproduct(
    main: &[u8],
    lower: Option<&[u8]>,
    upper: Option<&[u8]>,
    sum_style: bool,
) -> Vec<u8> {
    let var = if sum_style { TV_BO_SUM } else { 0 };
    big_operator(TM_COPROD, main, upper, lower, &[], var)
}

/// `tmUNION` (selector 19): union `∪` with optional lower and upper limits.
pub fn union_op(
    main: &[u8],
    lower: Option<&[u8]>,
    upper: Option<&[u8]>,
    sum_style: bool,
) -> Vec<u8> {
    let var = if sum_style { TV_BO_SUM } else { 0 };
    big_operator(TM_UNION, main, upper, lower, &[], var)
}

/// `tmINTER` (selector 20): intersection `∩` with optional lower and upper limits.
pub fn intersection_op(
    main: &[u8],
    lower: Option<&[u8]>,
    upper: Option<&[u8]>,
    sum_style: bool,
) -> Vec<u8> {
    let var = if sum_style { TV_BO_SUM } else { 0 };
    big_operator(TM_INTER, main, upper, lower, &[], var)
}

/// `tmINTOP` (selector 21): custom integral-style big operator.
pub fn int_op(
    main: &[u8],
    lower: Option<&[u8]>,
    upper: Option<&[u8]>,
    operator_char: &[u8],
    sum_style: bool,
) -> Vec<u8> {
    let var = if sum_style { TV_BO_SUM } else { 0 };
    big_operator(TM_INTOP, main, upper, lower, operator_char, var)
}

/// `tmSUMOP` (selector 22): custom summation-style big operator.
pub fn sum_op(
    main: &[u8],
    lower: Option<&[u8]>,
    upper: Option<&[u8]>,
    operator_char: &[u8],
    sum_style: bool,
) -> Vec<u8> {
    let var = if sum_style { TV_BO_SUM } else { 0 };
    big_operator(TM_SUMOP, main, upper, lower, operator_char, var)
}

// ── Fences, Intervals, Horizontal Braces & Dirac (selectors 0..=9, 24, 25, 30) ──

fn default_fence_chars(selector: u8) -> (u32, u32) {
    match selector {
        TM_ANGLE => (0x2329, 0x232A),   // ⟨ ⟩
        TM_PAREN => (0x0028, 0x0029),   // ( )
        TM_BRACE => (0x007B, 0x007D),   // { }
        TM_BRACK => (0x005B, 0x005D),   // [ ]
        TM_BAR => (0xEC07, 0xEC08),     // | |
        TM_DBAR => (0xEC09, 0xEC0A),    // ‖ ‖
        TM_FLOOR => (0xF8F0, 0xF8FB),   // ⌊ ⌋
        TM_CEILING => (0xF8EE, 0xF8F9), // ⌈ ⌉
        TM_OBRACK => (0x301A, 0x301B),  // 〚 〛
        _ => (0x0028, 0x0029),
    }
}

/// `tmANGLE`/`tmPAREN`/`tmBRACE`/`tmBRACK`/`tmBAR`/`tmDBAR`/`tmFLOOR`/
/// `tmCEILING`/`tmOBRACK` (selectors 0..=8).
///
/// The main slot is followed by optional left and right fence `CHAR` records.
/// If `left_char` or `right_char` is empty when `has_left` / `has_right` is
/// `true`, the standard delimiter character for `selector` is emitted.
pub fn fence(
    selector: u8,
    has_left: bool,
    has_right: bool,
    main: &[u8],
    left_char: &[u8],
    right_char: &[u8],
) -> Vec<u8> {
    let variation = (if has_left { TV_FENCE_L } else { 0 })
        | (if has_right { TV_FENCE_R } else { 0 });
    let (def_l, def_r) = default_fence_chars(selector);
    let mut out = tmpl_header(selector, variation, 0);
    append_line_slot(&mut out, main);
    if has_left {
        if left_char.is_empty() {
            out.extend_from_slice(&char_record(TF_EXPAND, def_l));
        } else {
            out.extend_from_slice(left_char);
        }
    }
    if has_right {
        if right_char.is_empty() {
            out.extend_from_slice(&char_record(TF_EXPAND, def_r));
        } else {
            out.extend_from_slice(right_char);
        }
    }
    out.extend_from_slice(END);
    out
}

/// Parentheses template `( ... )` (`tmPAREN`, selector 1).
pub fn parens(main: &[u8]) -> Vec<u8> {
    fence(TM_PAREN, true, true, main, &[], &[])
}

/// Square brackets template `[ ... ]` (`tmBRACK`, selector 3).
pub fn brackets(main: &[u8]) -> Vec<u8> {
    fence(TM_BRACK, true, true, main, &[], &[])
}

/// Curly braces template `{ ... }` (`tmBRACE`, selector 2).
pub fn braces(main: &[u8]) -> Vec<u8> {
    fence(TM_BRACE, true, true, main, &[], &[])
}

/// Angle brackets template `⟨ ... ⟩` (`tmANGLE`, selector 0).
pub fn angle_brackets(main: &[u8]) -> Vec<u8> {
    fence(TM_ANGLE, true, true, main, &[], &[])
}

/// Vertical bars template `| ... |` (`tmBAR`, selector 4).
pub fn vertical_bars(main: &[u8]) -> Vec<u8> {
    fence(TM_BAR, true, true, main, &[], &[])
}

/// Double vertical bars template `‖ ... ‖` (`tmDBAR`, selector 5).
pub fn double_vertical_bars(main: &[u8]) -> Vec<u8> {
    fence(TM_DBAR, true, true, main, &[], &[])
}

/// Floor brackets template `⌊ ... ⌋` (`tmFLOOR`, selector 6).
pub fn floor_brackets(main: &[u8]) -> Vec<u8> {
    fence(TM_FLOOR, true, true, main, &[], &[])
}

/// Ceiling brackets template `⌈ ... ⌉` (`tmCEILING`, selector 7).
pub fn ceiling_brackets(main: &[u8]) -> Vec<u8> {
    fence(TM_CEILING, true, true, main, &[], &[])
}

/// Open (white) brackets template `〚 ... 〛` (`tmOBRACK`, selector 8).
pub fn open_brackets(main: &[u8]) -> Vec<u8> {
    fence(TM_OBRACK, true, true, main, &[], &[])
}

fn interval_kind_char(kind: u8) -> u32 {
    match kind & 0x03 {
        0 => 0x0028,
        1 => 0x0029,
        2 => 0x005B,
        _ => 0x005D,
    }
}

/// `tmINTERVAL` (selector 9): unmatched interval fences.
///
/// Variation uses the left-fence code in bits 0..1 (`0`=`(`, `1`=`)`, `2`=`[`,
/// `3`=`]`) and right-fence code in bits 4..5, as specified by MTEF 5.
/// Also always emits the left and right delimiter `CHAR` records after the main
/// slot so `mathtypejx`'s `fence.xsl` (`slot[2]/*[not(self::options)][1..2]`)
/// extracts the exact interval delimiters.
pub fn interval(
    left_kind: u8,
    right_kind: u8,
    main: &[u8],
    left_char: &[u8],
    right_char: &[u8],
) -> Vec<u8> {
    let variation = (left_kind & 0x03) | ((right_kind & 0x03) << 4);
    let mut out = tmpl_header(TM_INTERVAL, variation, 0);
    append_line_slot(&mut out, main);
    if left_char.is_empty() {
        out.extend_from_slice(&char_record(TF_EXPAND, interval_kind_char(left_kind)));
    } else {
        out.extend_from_slice(left_char);
    }
    if right_char.is_empty() {
        out.extend_from_slice(&char_record(TF_EXPAND, interval_kind_char(right_kind)));
    } else {
        out.extend_from_slice(right_char);
    }
    out.extend_from_slice(END);
    out
}

/// `tmHBRACE` (24) / `tmHBRACK` (25): horizontal brace or bracket.
///
/// `top = true` sets `TV_HB_TOP` (`0x01`), placing the small label slot and
/// fence above the main slot; otherwise they are placed below it.
pub fn horizontal_fence(
    selector: u8,
    main: &[u8],
    small: &[u8],
    brace_char: &[u8],
    top: bool,
) -> Vec<u8> {
    let variation = if top { TV_HB_TOP } else { 0x00 };
    let mut out = tmpl_header(selector, variation, 0);
    append_line_slot(&mut out, main);
    out.extend_from_slice(SUB);
    append_line_slot(&mut out, small);
    out.extend_from_slice(FULL);
    if brace_char.is_empty() {
        let default_cp = match (selector, top) {
            (TM_HBRACK, true) => 0xEC0D,
            (TM_HBRACK, false) => 0xEC0C,
            (_, true) => 0xFE37,
            (_, false) => 0xFE38,
        };
        out.extend_from_slice(&char_record(TF_EXPAND, default_cp));
    } else {
        out.extend_from_slice(brace_char);
    }
    out.extend_from_slice(END);
    out
}

/// Overbrace (`tmHBRACE`, selector 24, `TV_HB_TOP`).
pub fn overbrace(main: &[u8], label: &[u8]) -> Vec<u8> {
    horizontal_fence(TM_HBRACE, main, label, &[], true)
}

/// Underbrace (`tmHBRACE`, selector 24, bottom).
pub fn underbrace(main: &[u8], label: &[u8]) -> Vec<u8> {
    horizontal_fence(TM_HBRACE, main, label, &[], false)
}

/// Overbracket (`tmHBRACK`, selector 25, `TV_HB_TOP`).
pub fn overbracket(main: &[u8], label: &[u8]) -> Vec<u8> {
    horizontal_fence(TM_HBRACK, main, label, &[], true)
}

/// Underbracket (`tmHBRACK`, selector 25, bottom).
pub fn underbracket(main: &[u8], label: &[u8]) -> Vec<u8> {
    horizontal_fence(TM_HBRACK, main, label, &[], false)
}

/// `tmLDIV` (selector 26): long division.
///
/// Slot order is dividend (`slot[1]`) then quotient (`slot[2]`); variation
/// `TV_LD_UPPER` (`0x01`) indicates the upper/quotient slot is present.
pub fn long_division(dividend: &[u8], quotient: Option<&[u8]>) -> Vec<u8> {
    let has_quotient = quotient.is_some_and(|q| !q.is_empty());
    let variation = if has_quotient { TV_LD_UPPER } else { 0 };
    let mut out = tmpl_header(TM_LDIV, variation, 0);
    append_line_slot(&mut out, dividend);
    append_opt_line_slot(&mut out, quotient);
    out.extend_from_slice(&line(true)); // slot 3 null
    out.extend_from_slice(&line(true)); // slot 4 null
    out.extend_from_slice(END);
    out
}

/// `tmDIRAC` (selector 30): Dirac bra-ket structure.
///
/// Variation bits `TV_DI_LEFT` (`0x01`) and `TV_DI_RIGHT` (`0x02`) indicate
/// whether the left (bra) and right (ket) parts are present. Both line slots
/// (`slot[1]` = left, `slot[2]` = right) are ALWAYS emitted (using a null
/// `LINE` when a side is absent) so `fence.xsl` (`slot[2]` for ket-only
/// `tvDI_RIGHT`) indexes the right slot accurately.
pub fn dirac(
    left: Option<&[u8]>,
    right: Option<&[u8]>,
    left_angle: &[u8],
    vertical_bar: &[u8],
    right_angle: &[u8],
) -> Vec<u8> {
    let has_left = left.is_some_and(|s| !s.is_empty());
    let has_right = right.is_some_and(|s| !s.is_empty());
    let variation = (if has_left { TV_DI_LEFT } else { 0 })
        | (if has_right { TV_DI_RIGHT } else { 0 });
    let mut out = tmpl_header(TM_DIRAC, variation, 0);
    append_opt_line_slot(&mut out, left);
    append_opt_line_slot(&mut out, right);
    if has_left {
        if left_angle.is_empty() {
            out.extend_from_slice(&char_record(TF_EXPAND, 0x2329));
        } else {
            out.extend_from_slice(left_angle);
        }
    }
    if vertical_bar.is_empty() {
        out.extend_from_slice(&char_record(TF_EXPAND, 0xEC07));
    } else {
        out.extend_from_slice(vertical_bar);
    }
    if has_right {
        if right_angle.is_empty() {
            out.extend_from_slice(&char_record(TF_EXPAND, 0x232A));
        } else {
            out.extend_from_slice(right_angle);
        }
    }
    out.extend_from_slice(END);
    out
}

// ── Strike-through & Box templates (selectors 36, 37) ───────────────────────

/// `tmSTRIKE` (selector 36): strike-through / cancellation.
///
/// Variation flags:
/// * `TV_ST_HORIZ` (`0x01`): horizontal strike
/// * `TV_ST_UP`    (`0x02`): bottom-left to top-right rising slash
/// * `TV_ST_DOWN`  (`0x04`): top-left to bottom-right falling slash
pub fn strike_through(base: &[u8], variation: u8) -> Vec<u8> {
    one_slot_template(TM_STRIKE, variation & 0x07, base)
}

/// `tmBOX` (selector 37): rectangular/rounded box (`menclose`).
///
/// Variation bits:
/// * `TV_BX_ROUND`  (`0x01`): rounded corners
/// * `TV_BX_LEFT`   (`0x02`): left border
/// * `TV_BX_RIGHT`  (`0x04`): right border
/// * `TV_BX_TOP`    (`0x08`): top border
/// * `TV_BX_BOTTOM` (`0x10`): bottom border
pub fn boxed(
    base: &[u8],
    rounded: bool,
    left: bool,
    right: bool,
    top: bool,
    bottom: bool,
) -> Vec<u8> {
    let variation = u8::from(rounded)
        | (u8::from(left) << 1)
        | (u8::from(right) << 2)
        | (u8::from(top) << 3)
        | (u8::from(bottom) << 4);
    one_slot_template(TM_BOX, variation, base)
}

// ── Composite MTEF 5 Records: PILE (4), MATRIX (5), EMBELL (6) ──────────────

/// `PILE` record (type `0x04`): vertical stack of lines (used for equation
/// arrays, piecewise cases, and multi-line slots).
///
/// Format in `records5.py`:
/// `[0x04, options(0), halign, valign, LINE_1, ..., LINE_N, END]`.
pub fn pile(halign: u8, valign: u8, rows: &[&[u8]]) -> Vec<u8> {
    let mut out = vec![0x04, 0x00, halign, valign];
    for row in rows {
        append_line_slot(&mut out, row);
    }
    out.extend_from_slice(END);
    out
}

/// `MATRIX` record (type `0x05`): `rows x cols` grid of cells.
///
/// Format in `records5.py`:
/// `[0x05, options(0), valign, h_just, v_just, rows, cols, row_parts..., col_parts..., cell_1, ..., cell_(rows*cols), END]`.
/// Partition byte counts are `(rows + 4) / 4` and `(cols + 4) / 4`.
pub fn matrix(
    valign: u8,
    h_just: u8,
    v_just: u8,
    rows: u8,
    cols: u8,
    cells: &[&[u8]],
) -> Vec<u8> {
    let n_row_bytes = (usize::from(rows) + 4) / 4;
    let n_col_bytes = (usize::from(cols) + 4) / 4;
    let mut out = vec![0x05, 0x00, valign, h_just, v_just, rows, cols];
    out.resize(out.len() + n_row_bytes + n_col_bytes, 0x00);

    let total_cells = usize::from(rows) * usize::from(cols);
    for idx in 0..total_cells {
        let cell = cells.get(idx).copied().unwrap_or(&[]);
        append_line_slot(&mut out, cell);
    }
    out.extend_from_slice(END);
    out
}

/// Single `EMBELL` record (type `0x06`): `[0x06, options(0), embell_code]`.
pub fn embell_record(embell_code: u8) -> Vec<u8> {
    vec![0x06, 0x00, embell_code]
}

/// `CHAR` record (type `0x02`) with attached character embellishments
/// (`OPT_CHAR_EMBELL = 0x01`, matching `records5.py` and `embellishment.xsl`).
pub fn embellished_char(typeface: u8, unicode_val: u32, embell_codes: &[u8]) -> Vec<u8> {
    if embell_codes.is_empty() {
        return char_record(typeface, unicode_val);
    }
    let mut out = vec![
        0x02,
        0x01, // OPT_CHAR_EMBELL
        typeface.wrapping_add(128),
        (unicode_val & 0xFF) as u8,
        ((unicode_val >> 8) & 0xFF) as u8,
    ];
    for &code in embell_codes {
        out.extend_from_slice(&embell_record(code));
    }
    out.extend_from_slice(END);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mtef::typeface::TF_VARIABLE;

    #[test]
    fn test_limits_variation_and_three_slots() {
        let x = char_record(TF_VARIABLE, 'x' as u32);
        let a = char_record(TF_VARIABLE, 'a' as u32);
        let b = char_record(TF_VARIABLE, 'b' as u32);

        // Overscript only: variation must be TV_BO_UPPER (0x20), with null lower slot (slot 2)
        // followed by upper slot (slot 3).
        let over = overscript(&x, &b);
        assert_eq!(&over[0..5], &[0x03, 0x00, TM_LIM, TV_BO_UPPER as u8, 0x00]);

        // Underscript only: variation must be TV_BO_LOWER (0x10).
        let under = underscript(&x, &a);
        assert_eq!(&under[0..5], &[0x03, 0x00, TM_LIM, TV_BO_LOWER as u8, 0x00]);

        // Both limits centered: variation must be 0x70 (TV_BO_LOWER | TV_BO_UPPER | TV_BO_SUM).
        let both = under_overscript(&x, &a, &b);
        assert_eq!(
            &both[0..5],
            &[
                0x03,
                0x00,
                TM_LIM,
                (TV_BO_LOWER | TV_BO_UPPER | TV_BO_SUM) as u8,
                0x00
            ]
        );
    }

    #[test]
    fn test_extensible_arrow_both_and_single_slots() {
        let a = char_record(TF_VARIABLE, 'a' as u32);
        let b = char_record(TF_VARIABLE, 'b' as u32);

        let top_only = extensible_arrow("→", Some(&a), None);
        assert_eq!(top_only[3], TV_AR_RIGHT | TV_AR_TOP);

        let bottom_only = extensible_arrow("←", None, Some(&b));
        assert_eq!(bottom_only[3], TV_AR_LEFT | TV_AR_BOTTOM);

        let both = extensible_arrow("↔", Some(&a), Some(&b));
        assert_eq!(both[3], TV_AR_LEFT | TV_AR_RIGHT | TV_AR_TOPBOTTOM);
        // Neither slot should be a null line [0x01, 0x01]
        assert!(both.windows(a.len()).any(|w| w == a.as_slice()));
        assert!(both.windows(b.len()).any(|w| w == b.as_slice()));
    }

    #[test]
    fn test_big_operator_variation_and_slot_order() {
        let x = char_record(TF_VARIABLE, 'x' as u32);
        let lo = char_record(TF_VARIABLE, 'a' as u32);
        let hi = char_record(TF_VARIABLE, 'b' as u32);

        let sum_bytes = summation(&x, Some(&lo), Some(&hi), true);
        assert_eq!(
            &sum_bytes[0..5],
            &[
                0x03,
                0x00,
                TM_SUM,
                (TV_BO_LOWER | TV_BO_UPPER | TV_BO_SUM) as u8,
                0x00
            ]
        );
        // Verify lower appears before upper
        let lo_pos = sum_bytes
            .windows(lo.len())
            .position(|w| w == lo.as_slice())
            .unwrap();
        let hi_pos = sum_bytes
            .windows(hi.len())
            .position(|w| w == hi.as_slice())
            .unwrap();
        assert!(lo_pos < hi_pos);
    }

    #[test]
    fn test_dirac_ket_preserves_slot_two() {
        let psi = char_record(TF_VARIABLE, 'x' as u32);
        let ket = dirac(None, Some(&psi), &[], &[], &[]);
        // Variation is TV_DI_RIGHT (0x02), followed by null slot 1 [0x01, 0x01], then slot 2 [0x01, 0x00, ...]
        assert_eq!(&ket[0..7], &[0x03, 0x00, TM_DIRAC, TV_DI_RIGHT, 0x00, 0x01, 0x01]);
        assert_eq!(&ket[7..9], &[0x01, 0x00]);
    }
    #[test]
    fn test_hat_box_emits_char_record() {
        let x = char_record(TF_VARIABLE, 'F' as u32);
        let vec_f = vector_arrow(&x, "→");
        // Must contain TMPL header [0x03, 0x00, TM_VEC, TV_VE_RIGHT (0x02), 0x00]
        assert_eq!(&vec_f[0..5], &[0x03, 0x00, TM_VEC, TV_VE_RIGHT, 0x00]);
        // Must contain expandable CHAR record: [0x02, 0x00, 22 + 128 = 150, 0xD7, 0x20]
        let exp_char = char_record(TF_EXPAND, 0x20D7);
        assert!(vec_f.windows(exp_char.len()).any(|w| w == exp_char.as_slice()));

        let hat = over_hat(&x);
        let hat_char = char_record(TF_EXPAND, 0x0302);
        assert!(hat.windows(hat_char.len()).any(|w| w == hat_char.as_slice()));

        let arc = over_arc(&x);
        let arc_char = char_record(TF_EXPAND, 0x2322);
        assert!(arc.windows(arc_char.len()).any(|w| w == arc_char.as_slice()));
    }

    #[test]
    fn test_script_restores_full_size() {
        let two = char_record(TF_VARIABLE, '2' as u32);
        let sup = superscript(&two);
        // Last byte must be FULL (0x0A)
        assert_eq!(sup.last().copied(), Some(0x0A));
    }

    #[test]
    fn test_integral_operator_characters_and_loop_glyphs() {
        let x = char_record(TF_VARIABLE, 'x' as u32);
        // Single integral: 1 CHAR(TF_SYMBOL, 0x222B)
        let int1 = integral(&x, None, None, &[], TV_INT_1);
        let int_char = char_record(TF_SYMBOL, 0x222B);
        let count = int1.windows(int_char.len()).filter(|w| *w == int_char.as_slice()).count();
        assert_eq!(count, 1);

        // Double integral: 2 CHAR(TF_SYMBOL, 0x222B)
        let int2 = integral(&x, None, None, &[], TV_INT_2);
        let count2 = int2.windows(int_char.len()).filter(|w| *w == int_char.as_slice()).count();
        assert_eq!(count2, 2);

        // Triple integral: 3 CHAR(TF_SYMBOL, 0x222B)
        let int3 = integral(&x, None, None, &[], TV_INT_3);
        let count3 = int3.windows(int_char.len()).filter(|w| *w == int_char.as_slice()).count();
        assert_eq!(count3, 3);

        // Loop integral: 1 CHAR(TF_MTEXTRA, 0xEE11) + 1 CHAR(TF_SYMBOL, 0x222B)
        let oint = integral(&x, None, None, &[], TV_INT_1 | TV_INT_LOOP);
        let loop1 = char_record(TF_MTEXTRA, 0xEE11);
        assert!(oint.windows(loop1.len()).any(|w| w == loop1.as_slice()));
        assert_eq!(oint.windows(int_char.len()).filter(|w| *w == int_char.as_slice()).count(), 1);

        // Double loop integral: 1 CHAR(TF_MTEXTRA, 0xEE12) + 2 CHAR(TF_SYMBOL, 0x222B)
        let oiint = integral(&x, None, None, &[], TV_INT_2 | TV_INT_LOOP);
        let loop2 = char_record(TF_MTEXTRA, 0xEE12);
        assert!(oiint.windows(loop2.len()).any(|w| w == loop2.as_slice()));
        assert_eq!(oiint.windows(int_char.len()).filter(|w| *w == int_char.as_slice()).count(), 2);

        // Triple loop integral: 1 CHAR(TF_MTEXTRA, 0xEE13) + 3 CHAR(TF_SYMBOL, 0x222B)
        let oiiint = integral(&x, None, None, &[], TV_INT_3 | TV_INT_LOOP);
        let loop3 = char_record(TF_MTEXTRA, 0xEE13);
        assert!(oiiint.windows(loop3.len()).any(|w| w == loop3.as_slice()));
        assert_eq!(oiiint.windows(int_char.len()).filter(|w| *w == int_char.as_slice()).count(), 3);
    }
}
