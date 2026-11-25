//! Fraction, radical, and script template constructors.

use super::super::records::{line, tmpl_header, END, FULL, SUB};
use super::consts::{
    TM_FRACT, TM_ROOT, TM_SUB, TM_SUBSUP, TM_SUP, TV_FR_BASE, TV_FR_SLASH, TV_FR_SMALL,
    TV_ROOT_NTH, TV_ROOT_SQ, TV_SU_PRECEDES,
};
use super::slots::{append_fraction_slot, append_line_slot, append_opt_line_slot, concat_bytes};

/// `tmFRACT` (selector `11`): standard fraction with horizontal bar.
pub fn fraction(numerator: &[u8], denominator: &[u8]) -> Vec<u8> {
    concat_bytes(&[
        &tmpl_header(TM_FRACT, 0, 0),
        &line(false),
        numerator,
        END,
        &line(false),
        denominator,
        END,
        END,
    ])
}

/// `tmFRACT` (selector `11`): fraction with explicit `variation` flags.
pub fn fraction_variant(numerator: &[u8], denominator: &[u8], variation: u8) -> Vec<u8> {
    let mut out = tmpl_header(TM_FRACT, variation, 0);
    let is_small = variation & TV_FR_SMALL != 0;
    append_fraction_slot(&mut out, numerator, is_small);
    append_fraction_slot(&mut out, denominator, is_small);
    out.extend_from_slice(END);
    out
}

/// `tmFRACT` with diagonal slash (`TV_FR_SLASH`).
pub fn slash_fraction(numerator: &[u8], denominator: &[u8]) -> Vec<u8> {
    fraction_variant(numerator, denominator, TV_FR_SLASH)
}

/// `tmFRACT` with subscript-size slots (`TV_FR_SMALL`), restoring `FULL` size afterward.
pub fn small_fraction(numerator: &[u8], denominator: &[u8]) -> Vec<u8> {
    let mut out = fraction_variant(numerator, denominator, TV_FR_SMALL);
    out.extend_from_slice(FULL);
    out
}

/// `tmFRACT` with diagonal slash and baseline alignment (`TV_FR_SLASH | TV_FR_BASE`).
pub fn base_slash_fraction(numerator: &[u8], denominator: &[u8]) -> Vec<u8> {
    fraction_variant(numerator, denominator, TV_FR_SLASH | TV_FR_BASE)
}

/// `tmROOT` (`TM_ROOT`, `TV_ROOT_SQ`): square root radical.
pub fn sqrt_radical(radicand: &[u8]) -> Vec<u8> {
    concat_bytes(&[
        &tmpl_header(TM_ROOT, TV_ROOT_SQ, 0),
        &line(false),
        radicand,
        END,
        SUB,
        &line(true),
        END,
        FULL,
    ])
}

/// `tmROOT` (`TM_ROOT`, `TV_ROOT_NTH`): nth root radical with index slot.
pub fn nroot_radical(radicand: &[u8], degree: &[u8]) -> Vec<u8> {
    concat_bytes(&[
        &tmpl_header(TM_ROOT, TV_ROOT_NTH, 0),
        &line(false),
        radicand,
        END,
        SUB,
        &line(false),
        degree,
        END,
        END,
        FULL,
    ])
}

/// `tmSUB` (selector `27`): subscript attached to the preceding base item.
pub fn subscript(sub: &[u8]) -> Vec<u8> {
    concat_bytes(&[
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

/// `tmSUP` (selector `28`): superscript attached to the preceding base item.
pub fn superscript(sup: &[u8]) -> Vec<u8> {
    concat_bytes(&[
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

/// `tmSUBSUP` (selector `29`): combined subscript and superscript on the preceding base item.
pub fn subsuperscript(sub: &[u8], sup: &[u8]) -> Vec<u8> {
    concat_bytes(&[
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

/// Build a `TM_SUB`, `TM_SUP`, or `TM_SUBSUP` template with optional `TV_SU_PRECEDES` variation.
pub fn script_template(
    selector: u8,
    first: &[u8],
    second: Option<&[u8]>,
    precedes: bool,
) -> Vec<u8> {
    let mut out = tmpl_header(selector, if precedes { TV_SU_PRECEDES } else { 0 }, 0);
    out.extend_from_slice(SUB);
    match selector {
        TM_SUP => match second {
            Some(sup) => {
                append_line_slot(&mut out, first);
                append_line_slot(&mut out, sup);
            }
            None => {
                out.extend_from_slice(&line(true));
                append_line_slot(&mut out, first);
            }
        },
        TM_SUB | TM_SUBSUP => {
            append_line_slot(&mut out, first);
            append_opt_line_slot(&mut out, second);
        }
        _ => {
            append_line_slot(&mut out, first);
            append_opt_line_slot(&mut out, second);
        }
    }
    out.extend_from_slice(END);
    out.extend_from_slice(FULL);
    out
}

/// Prescript subscript preceding `base` (`TM_SUB` with `TV_SU_PRECEDES`).
pub fn presubscript(base: &[u8], sub: &[u8]) -> Vec<u8> {
    let tmpl = script_template(TM_SUB, sub, None, true);
    concat_bytes(&[&tmpl, base])
}

/// Prescript superscript preceding `base` (`TM_SUP` with `TV_SU_PRECEDES`).
pub fn presuperscript(base: &[u8], sup: &[u8]) -> Vec<u8> {
    let tmpl = script_template(TM_SUP, sup, None, true);
    concat_bytes(&[&tmpl, base])
}

/// Prescript sub/superscript preceding `base` (`TM_SUBSUP` with `TV_SU_PRECEDES`).
pub fn presubsuperscript(base: &[u8], sub: &[u8], sup: &[u8]) -> Vec<u8> {
    let tmpl = script_template(TM_SUBSUP, sub, Some(sup), true);
    concat_bytes(&[&tmpl, base])
}
