//! Composite MTEF 5.0 template and structural record constructors.
//!
//! Covers bars, vector/harpoon/extensible arrows, hats/tildes/arcs, limits,
//! integrals and big operators, fences/intervals/horizontal braces, Dirac
//! bra-kets, long division, strike-throughs, boxes, `PILE`, `MATRIX`, and
//! `EMBELL` records.

use super::super::records::{char_record, line, tmpl_header, END, FULL, SUB};
use super::super::typeface::{TF_EXPAND, TF_MTEXTRA, TF_SYMBOL};
use super::consts::{
    SYM, TM_ANGLE, TM_ARC, TM_ARROW, TM_BAR, TM_BOX, TM_BRACE, TM_BRACK, TM_CEILING, TM_COPROD,
    TM_DBAR, TM_DIRAC, TM_FLOOR, TM_HAT, TM_HBRACE, TM_HBRACK, TM_INTEG, TM_INTER, TM_INTERVAL,
    TM_INTOP, TM_JSTATUS, TM_LDIV, TM_LIM, TM_OBAR, TM_OBRACK, TM_PAREN, TM_PROD, TM_STRIKE,
    TM_SUM, TM_SUMOP, TM_TILDE, TM_UBAR, TM_UNION, TM_VEC, TV_AR_BOTTOM, TV_AR_DOUBLE,
    TV_AR_HARPOON, TV_AR_LEFT, TV_AR_RIGHT, TV_AR_TOP, TV_BAR_DOUBLE, TV_BO_LOWER, TV_BO_SUM,
    TV_BO_UPPER, TV_DI_LEFT, TV_DI_RIGHT, TV_FENCE_L, TV_FENCE_R, TV_HB_TOP, TV_INT_1,
    TV_INT_CCW_LOOP, TV_INT_CW_LOOP, TV_INT_LOOP, TV_LD_UPPER, TV_VE_HARPOON, TV_VE_LEFT,
    TV_VE_RIGHT, TV_VE_UNDER,
};
use super::slots::{append_line_slot, append_opt_line_slot, tmpl_header_u16};

// ── Generic Single-Slot & Hat-Box Builders ──────────────────────────────────

/// Generic one-slot template helper for `BarBoxClass`, `HatBoxClass`,
/// `StrikeBoxClass`, and `TBoxBoxClass`.
pub fn one_slot_template(selector: u8, variation: u8, content: &[u8]) -> Vec<u8> {
    let mut out = tmpl_header(selector, variation, 0);
    append_line_slot(&mut out, content);
    out.extend_from_slice(END);
    out
}

/// Template with one line slot followed by an expandable `CHAR` record
/// (`HatBoxClass`: `tmVEC`, `tmTILDE`, `tmHAT`, `tmARC`, `tmJSTATUS`).
pub fn hat_box_template(selector: u8, variation: u8, content: &[u8], mt_code: u32) -> Vec<u8> {
    let mut out = tmpl_header(selector, variation, 0);
    append_line_slot(&mut out, content);
    out.extend_from_slice(&char_record(TF_EXPAND, mt_code));
    out.extend_from_slice(END);
    out
}

// ── Over / Under Bars, Vectors, Hats, Arcs & Extensible Arrows ──────────────

/// `tmOBAR` (selector `13`, variation `0`): single overbar over characters.
pub fn overbar(base: &[u8]) -> Vec<u8> {
    one_slot_template(TM_OBAR, 0, base)
}

/// `tmOBAR` (selector `13`, variation `TV_BAR_DOUBLE`): double overbar.
pub fn double_overbar(base: &[u8]) -> Vec<u8> {
    one_slot_template(TM_OBAR, TV_BAR_DOUBLE, base)
}

/// `tmUBAR` (selector `12`, variation `0`): single underbar under characters.
pub fn underbar(base: &[u8]) -> Vec<u8> {
    one_slot_template(TM_UBAR, 0, base)
}

/// `tmUBAR` (selector `12`, variation `TV_BAR_DOUBLE`): double underbar.
pub fn double_underbar(base: &[u8]) -> Vec<u8> {
    one_slot_template(TM_UBAR, TV_BAR_DOUBLE, base)
}

/// `tmUBAR` / `tmOBAR` single or double bar variants (selectors `12` and `13`).
pub fn bar_variant(base: &[u8], is_under: bool, is_double: bool) -> Vec<u8> {
    one_slot_template(
        if is_under { TM_UBAR } else { TM_OBAR },
        if is_double { TV_BAR_DOUBLE } else { 0 },
        base,
    )
}

/// `tmVEC` (selector `31`): vector arrow over characters.
///
/// Variation flags:
/// * `0x01`: left arrow (`TV_VE_LEFT`)
/// * `0x02`: right arrow (`TV_VE_RIGHT`)
/// * `0x03`: bidirectional arrow (`TV_VE_LEFT | TV_VE_RIGHT`)
pub fn vector_arrow(base: &[u8], arrow_char: &str) -> Vec<u8> {
    let (variation, mt_code) = vector_direction_and_code(arrow_char);
    hat_box_template(TM_VEC, variation, base, mt_code)
}

/// `tmVEC` (selector `31`): vector arrow under characters.
///
/// Variation flags combine `TV_VE_UNDER` (`0x04`) with direction bits.
pub fn under_vector_arrow(base: &[u8], arrow_char: &str) -> Vec<u8> {
    let (direction, mt_code) = vector_direction_and_code(arrow_char);
    hat_box_template(TM_VEC, TV_VE_UNDER | direction, base, mt_code)
}

/// `tmVEC` (selector `31`): single-barbed harpoon vector arrow over characters.
pub fn harpoon_arrow(base: &[u8], is_left: bool) -> Vec<u8> {
    let (variation, mt_code) = if is_left {
        (TV_VE_LEFT | TV_VE_HARPOON, 0x20D0)
    } else {
        (TV_VE_RIGHT | TV_VE_HARPOON, 0x20D1)
    };
    hat_box_template(TM_VEC, variation, base, mt_code)
}

/// `tmVEC` (selector `31`): single-barbed harpoon vector arrow under characters.
pub fn under_harpoon_arrow(base: &[u8], is_left: bool) -> Vec<u8> {
    let (variation, mt_code) = if is_left {
        (TV_VE_UNDER | TV_VE_LEFT | TV_VE_HARPOON, 0xEB18)
    } else {
        (TV_VE_UNDER | TV_VE_RIGHT | TV_VE_HARPOON, 0xEB19)
    };
    hat_box_template(TM_VEC, variation, base, mt_code)
}

/// `tmTILDE` (selector `32`): extensible tilde over a slot.
pub fn over_tilde(base: &[u8]) -> Vec<u8> {
    hat_box_template(TM_TILDE, 0, base, 0x0303)
}

/// `tmHAT` (selector `33`): hat / circumflex over characters.
pub fn over_hat(base: &[u8]) -> Vec<u8> {
    hat_box_template(TM_HAT, 0, base, 0x0302)
}

/// `tmARC` (selector `34`): arc / frown over characters (e.g. `\wideparen{AB}`).
pub fn over_arc(base: &[u8]) -> Vec<u8> {
    hat_box_template(TM_ARC, 0, base, 0x2322)
}

/// `tmJSTATUS` (selector `35`): joint-status construct over a slot.
pub fn joint_status(base: &[u8]) -> Vec<u8> {
    hat_box_template(TM_JSTATUS, 0, base, 0xEE0D)
}

/// `tmARROW` (selector `14`): extensible arrows with labels above and/or below.
///
/// Always emits 2 `LINE` slots (`slot[1]` = top, `slot[2]` = bottom, using a
/// null line when absent) followed by the arrow glyph(s).
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

    let has_top = above_content.is_some_and(|bytes| !bytes.is_empty());
    let has_bottom = below_content.is_some_and(|bytes| !bytes.is_empty());
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

/// `tmLIM` (selector `23`): general limit template.
///
/// Always emits 3 slots (`slot[1]` = base, `slot[2]` = lower, `slot[3]` = upper).
pub fn limits(
    base: &[u8],
    lower: Option<&[u8]>,
    upper: Option<&[u8]>,
    summation_style: bool,
) -> Vec<u8> {
    let has_lower = lower.is_some_and(|bytes| !bytes.is_empty());
    let has_upper = upper.is_some_and(|bytes| !bytes.is_empty());
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

/// `tmLIM` (selector `23`, `TV_BO_UPPER`): upper limit / overscript.
pub fn overscript(base: &[u8], over: &[u8]) -> Vec<u8> {
    limits(base, None, Some(over), false)
}

/// `tmLIM` (selector `23`, `TV_BO_LOWER`): lower limit / underscript.
pub fn underscript(base: &[u8], under: &[u8]) -> Vec<u8> {
    limits(base, Some(under), None, false)
}

/// `tmLIM` (selector `23`, `TV_BO_LOWER | TV_BO_UPPER | TV_BO_SUM`): limits both above and below.
pub fn under_overscript(base: &[u8], under: &[u8], over: &[u8]) -> Vec<u8> {
    limits(base, Some(under), Some(over), true)
}

// ── Integrals & Big Operators (selectors 15..=22) ───────────────────────────

/// Big operators (`tmINTEG`=15, `tmSUM`=16, `tmPROD`=17, `tmCOPROD`=18,
/// `tmUNION`=19, `tmINTER`=20, `tmINTOP`=21, `tmSUMOP`=22).
///
/// Slot order is `slot[1]` = main, `slot[2]` = lower limit, `slot[3]` = upper limit,
/// followed by `SYM` and the operator `CHAR` record.
pub fn big_operator(
    selector: u8,
    main: &[u8],
    upper: Option<&[u8]>,
    lower: Option<&[u8]>,
    operator_char: &[u8],
    variation: u16,
) -> Vec<u8> {
    let has_lower = lower.is_some_and(|bytes| !bytes.is_empty());
    let mut has_upper = upper.is_some_and(|bytes| !bytes.is_empty());
    let mut flags = variation;

    // In MathType 5, loop integrals and multiple integrals only support lower limits;
    // setting TV_BO_UPPER on them causes MathType to crash/hang.
    if selector == TM_INTEG {
        let is_loop = (flags & (TV_INT_LOOP | TV_INT_CW_LOOP | TV_INT_CCW_LOOP)) != 0;
        let is_multi = (flags & 0x0003) > 1;
        if is_loop || is_multi {
            has_upper = false;
        }
    }

    if has_lower {
        flags |= TV_BO_LOWER;
    }
    if has_upper {
        flags |= TV_BO_UPPER;
    }
    if selector == TM_INTEG && (flags & 0x000F) == 0 {
        flags |= TV_INT_1;
    }

    let upper_slot = if has_upper { upper } else { None };

    let mut out = tmpl_header_u16(selector, flags, 0);
    append_line_slot(&mut out, main);
    out.extend_from_slice(SUB);
    append_opt_line_slot(&mut out, lower);
    append_opt_line_slot(&mut out, upper_slot);
    out.extend_from_slice(SYM);
    if selector == TM_INTEG || operator_char.is_empty() {
        out.extend_from_slice(&default_bigop_char(selector, flags));
    } else {
        out.extend_from_slice(operator_char);
    }
    out.extend_from_slice(END);
    out.extend_from_slice(FULL);
    out
}

/// `tmINTEG` (selector `15`): single/double/triple/loop integrals.
pub fn integral(
    main: &[u8],
    upper: Option<&[u8]>,
    lower: Option<&[u8]>,
    operator_char: &[u8],
    variation: u16,
) -> Vec<u8> {
    big_operator(TM_INTEG, main, upper, lower, operator_char, variation)
}

/// `tmSUM` (selector `16`): summation `∑` with optional lower and upper limits.
pub fn summation(
    main: &[u8],
    lower: Option<&[u8]>,
    upper: Option<&[u8]>,
    sum_style: bool,
) -> Vec<u8> {
    let variation = if sum_style { TV_BO_SUM } else { 0 };
    big_operator(TM_SUM, main, upper, lower, &[], variation)
}

/// `tmPROD` (selector `17`): product `∏` with optional lower and upper limits.
pub fn product(
    main: &[u8],
    lower: Option<&[u8]>,
    upper: Option<&[u8]>,
    sum_style: bool,
) -> Vec<u8> {
    let variation = if sum_style { TV_BO_SUM } else { 0 };
    big_operator(TM_PROD, main, upper, lower, &[], variation)
}

/// `tmCOPROD` (selector `18`): coproduct `∐` with optional lower and upper limits.
pub fn coproduct(
    main: &[u8],
    lower: Option<&[u8]>,
    upper: Option<&[u8]>,
    sum_style: bool,
) -> Vec<u8> {
    let variation = if sum_style { TV_BO_SUM } else { 0 };
    big_operator(TM_COPROD, main, upper, lower, &[], variation)
}

/// `tmUNION` (selector `19`): union `∪` with optional lower and upper limits.
pub fn union_op(
    main: &[u8],
    lower: Option<&[u8]>,
    upper: Option<&[u8]>,
    sum_style: bool,
) -> Vec<u8> {
    let variation = if sum_style { TV_BO_SUM } else { 0 };
    big_operator(TM_UNION, main, upper, lower, &[], variation)
}

/// `tmINTER` (selector `20`): intersection `∩` with optional lower and upper limits.
pub fn intersection_op(
    main: &[u8],
    lower: Option<&[u8]>,
    upper: Option<&[u8]>,
    sum_style: bool,
) -> Vec<u8> {
    let variation = if sum_style { TV_BO_SUM } else { 0 };
    big_operator(TM_INTER, main, upper, lower, &[], variation)
}

/// `tmINTOP` (selector `21`): custom integral-style big operator.
pub fn int_op(
    main: &[u8],
    lower: Option<&[u8]>,
    upper: Option<&[u8]>,
    operator_char: &[u8],
    sum_style: bool,
) -> Vec<u8> {
    let variation = if sum_style { TV_BO_SUM } else { 0 };
    big_operator(TM_INTOP, main, upper, lower, operator_char, variation)
}

/// `tmSUMOP` (selector `22`): custom summation-style big operator.
pub fn sum_op(
    main: &[u8],
    lower: Option<&[u8]>,
    upper: Option<&[u8]>,
    operator_char: &[u8],
    sum_style: bool,
) -> Vec<u8> {
    let variation = if sum_style { TV_BO_SUM } else { 0 };
    big_operator(TM_SUMOP, main, upper, lower, operator_char, variation)
}

// ── Fences, Intervals, Horizontal Braces & Dirac (selectors 0..=9, 24, 25, 30) ──

/// Matched or single-sided fence (`selectors 0..=8`).
///
/// The main slot is followed by optional left and right fence `CHAR` records;
/// empty overrides fall back to standard delimiter glyphs for `selector`.
pub fn fence(
    selector: u8,
    has_left: bool,
    has_right: bool,
    main: &[u8],
    left_char: &[u8],
    right_char: &[u8],
) -> Vec<u8> {
    let variation =
        (if has_left { TV_FENCE_L } else { 0 }) | (if has_right { TV_FENCE_R } else { 0 });
    let (default_left, default_right) = default_fence_chars(selector);
    let mut out = tmpl_header(selector, variation, 0);
    append_line_slot(&mut out, main);
    if has_left {
        if left_char.is_empty() {
            out.extend_from_slice(&char_record(TF_EXPAND, default_left));
        } else {
            out.extend_from_slice(left_char);
        }
    }
    if has_right {
        if right_char.is_empty() {
            out.extend_from_slice(&char_record(TF_EXPAND, default_right));
        } else {
            out.extend_from_slice(right_char);
        }
    }
    out.extend_from_slice(END);
    out
}

/// Parentheses template `( ... )` (`tmPAREN`, selector `1`).
pub fn parens(main: &[u8]) -> Vec<u8> {
    fence(TM_PAREN, true, true, main, &[], &[])
}

/// Square brackets template `[ ... ]` (`tmBRACK`, selector `3`).
pub fn brackets(main: &[u8]) -> Vec<u8> {
    fence(TM_BRACK, true, true, main, &[], &[])
}

/// Curly braces template `{ ... }` (`tmBRACE`, selector `2`).
pub fn braces(main: &[u8]) -> Vec<u8> {
    fence(TM_BRACE, true, true, main, &[], &[])
}

/// Angle brackets template `⟨ ... ⟩` (`tmANGLE`, selector `0`).
pub fn angle_brackets(main: &[u8]) -> Vec<u8> {
    fence(TM_ANGLE, true, true, main, &[], &[])
}

/// Vertical bars template `| ... |` (`tmBAR`, selector `4`).
pub fn vertical_bars(main: &[u8]) -> Vec<u8> {
    fence(TM_BAR, true, true, main, &[], &[])
}

/// Double vertical bars template `‖ ... ‖` (`tmDBAR`, selector `5`).
pub fn double_vertical_bars(main: &[u8]) -> Vec<u8> {
    fence(TM_DBAR, true, true, main, &[], &[])
}

/// Floor brackets template `⌊ ... ⌋` (`tmFLOOR`, selector `6`).
pub fn floor_brackets(main: &[u8]) -> Vec<u8> {
    fence(TM_FLOOR, true, true, main, &[], &[])
}

/// Ceiling brackets template `⌈ ... ⌉` (`tmCEILING`, selector `7`).
pub fn ceiling_brackets(main: &[u8]) -> Vec<u8> {
    fence(TM_CEILING, true, true, main, &[], &[])
}

/// Open (white) brackets template `〚 ... 〛` (`tmOBRACK`, selector `8`).
pub fn open_brackets(main: &[u8]) -> Vec<u8> {
    fence(TM_OBRACK, true, true, main, &[], &[])
}

/// `tmINTERVAL` (selector `9`): unmatched interval fences.
///
/// Variation encodes the left-fence code in bits `0..1` and right-fence code in
/// bits `4..5` (`0`=`(`, `1`=`)`, `2`=`[`, `3`=`]`), followed by delimiter `CHAR` records.
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

/// `tmHBRACE` (`24`) / `tmHBRACK` (`25`): horizontal brace or bracket.
///
/// When `is_top` is `true`, sets `TV_HB_TOP` so the label and fence sit above `main`.
pub fn horizontal_fence(
    selector: u8,
    main: &[u8],
    small: &[u8],
    brace_char: &[u8],
    is_top: bool,
) -> Vec<u8> {
    let variation = if is_top { TV_HB_TOP } else { 0x00 };
    let mut out = tmpl_header(selector, variation, 0);
    append_line_slot(&mut out, main);
    out.extend_from_slice(SUB);
    append_line_slot(&mut out, small);
    out.extend_from_slice(FULL);
    if brace_char.is_empty() {
        let default_cp = match (selector, is_top) {
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

/// Overbrace (`tmHBRACE`, selector `24`, `TV_HB_TOP`).
pub fn overbrace(main: &[u8], label: &[u8]) -> Vec<u8> {
    horizontal_fence(TM_HBRACE, main, label, &[], true)
}

/// Underbrace (`tmHBRACE`, selector `24`, bottom).
pub fn underbrace(main: &[u8], label: &[u8]) -> Vec<u8> {
    horizontal_fence(TM_HBRACE, main, label, &[], false)
}

/// Overbracket (`tmHBRACK`, selector `25`, `TV_HB_TOP`).
pub fn overbracket(main: &[u8], label: &[u8]) -> Vec<u8> {
    horizontal_fence(TM_HBRACK, main, label, &[], true)
}

/// Underbracket (`tmHBRACK`, selector `25`, bottom).
pub fn underbracket(main: &[u8], label: &[u8]) -> Vec<u8> {
    horizontal_fence(TM_HBRACK, main, label, &[], false)
}

/// `tmLDIV` (selector `26`): long division with dividend and optional upper quotient slot.
pub fn long_division(dividend: &[u8], quotient: Option<&[u8]>) -> Vec<u8> {
    let has_quotient = quotient.is_some_and(|bytes| !bytes.is_empty());
    let variation = if has_quotient { TV_LD_UPPER } else { 0 };
    let mut out = tmpl_header(TM_LDIV, variation, 0);
    append_line_slot(&mut out, dividend);
    append_opt_line_slot(&mut out, quotient);
    out.extend_from_slice(&line(true));
    out.extend_from_slice(&line(true));
    out.extend_from_slice(END);
    out
}

/// `tmDIRAC` (selector `30`): Dirac bra-ket structure.
///
/// Both line slots (`slot[1]` = left, `slot[2]` = right) are always emitted
/// (using a null `LINE` when a side is absent) so slot indices stay consistent.
pub fn dirac(
    left: Option<&[u8]>,
    right: Option<&[u8]>,
    left_angle: &[u8],
    vertical_bar: &[u8],
    right_angle: &[u8],
) -> Vec<u8> {
    let has_left = left.is_some_and(|bytes| !bytes.is_empty());
    let has_right = right.is_some_and(|bytes| !bytes.is_empty());
    let variation =
        (if has_left { TV_DI_LEFT } else { 0 }) | (if has_right { TV_DI_RIGHT } else { 0 });
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

/// `tmSTRIKE` (selector `36`): strike-through / cancellation (`TV_ST_*`).
pub fn strike_through(base: &[u8], variation: u8) -> Vec<u8> {
    one_slot_template(TM_STRIKE, variation & 0x07, base)
}

/// `tmBOX` (selector `37`): rectangular or rounded enclosure box.
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

/// `PILE` record (type `0x04`): vertical stack of `LINE` records.
pub fn pile(halign: u8, valign: u8, rows: &[&[u8]]) -> Vec<u8> {
    let mut out = vec![0x04, 0x00, halign, valign];
    for row in rows {
        append_line_slot(&mut out, row);
    }
    out.extend_from_slice(END);
    out
}

/// `MATRIX` record (type `0x05`): `rows x cols` grid of cell `LINE` records.
pub fn matrix(valign: u8, h_just: u8, v_just: u8, rows: u8, cols: u8, cells: &[&[u8]]) -> Vec<u8> {
    let row_partition_bytes = (usize::from(rows) + 4) / 4;
    let col_partition_bytes = (usize::from(cols) + 4) / 4;
    let mut out = vec![0x05, 0x00, valign, h_just, v_just, rows, cols];
    out.resize(out.len() + row_partition_bytes + col_partition_bytes, 0x00);

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

/// `CHAR` record (type `0x02`) with attached `EMBELL` records (`OPT_CHAR_EMBELL`).
pub fn embellished_char(typeface: u8, unicode_val: u32, embell_codes: &[u8]) -> Vec<u8> {
    let mut out = char_record(typeface, unicode_val);
    if embell_codes.is_empty() {
        return out;
    }
    out[1] |= 0x01; // OPT_CHAR_EMBELL
    for &code in embell_codes {
        out.extend_from_slice(&embell_record(code));
    }
    out.extend_from_slice(END);
    out
}

// ── Private Helpers ─────────────────────────────────────────────────────────

fn vector_direction_and_code(arrow_char: &str) -> (u8, u32) {
    if arrow_char.contains('↔')
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
    }
}

fn default_bigop_char(selector: u8, variation: u16) -> Vec<u8> {
    match selector {
        TM_INTEG => {
            let mut out = Vec::new();
            let integral_count = match variation & 0x0003 {
                2 => 2,
                3 => 3,
                _ => 1,
            };
            // MathType 5 encodes loop integrals as a loop circle glyph from MT Extra
            // followed by `integral_count` copies of the integral sign.
            if (variation & TV_INT_CCW_LOOP) == TV_INT_CCW_LOOP {
                out.extend(char_record(TF_MTEXTRA, 0xEE00));
            } else if (variation & TV_INT_CW_LOOP) == TV_INT_CW_LOOP {
                out.extend(char_record(TF_MTEXTRA, 0xEE01));
            } else if (variation & TV_INT_LOOP) != 0 {
                let loop_glyph = match integral_count {
                    2 => 0xEE12,
                    3 => 0xEE13,
                    _ => 0xEE11,
                };
                out.extend(char_record(TF_MTEXTRA, loop_glyph));
            }
            for _ in 0..integral_count {
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

fn interval_kind_char(kind: u8) -> u32 {
    match kind & 0x03 {
        0 => 0x0028,
        1 => 0x0029,
        2 => 0x005B,
        _ => 0x005D,
    }
}
