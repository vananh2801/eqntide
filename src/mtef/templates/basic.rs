//! Fraction, radical, and script template constructors.

use super::slots::{append_fraction_slot, append_line_slot, append_opt_line_slot};
use super::*;

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

pub fn fraction_variant(num: &[u8], den: &[u8], variation: u8) -> Vec<u8> {
    let mut out = tmpl_header(TM_FRACT, variation, 0);
    let small = variation & TV_FR_SMALL != 0;
    append_fraction_slot(&mut out, num, small);
    append_fraction_slot(&mut out, den, small);
    out.extend_from_slice(END);
    out
}

pub fn slash_fraction(num: &[u8], den: &[u8]) -> Vec<u8> {
    fraction_variant(num, den, TV_FR_SLASH)
}
pub fn small_fraction(num: &[u8], den: &[u8]) -> Vec<u8> {
    let mut out = fraction_variant(num, den, TV_FR_SMALL);
    out.extend_from_slice(FULL);
    out
}
pub fn base_slash_fraction(num: &[u8], den: &[u8]) -> Vec<u8> {
    fraction_variant(num, den, TV_FR_SLASH | TV_FR_BASE)
}

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

pub fn presubscript(base: &[u8], sub: &[u8]) -> Vec<u8> {
    let tmpl = script_template(TM_SUB, sub, None, true);
    cat(&[&tmpl, base])
}
pub fn presuperscript(base: &[u8], sup: &[u8]) -> Vec<u8> {
    let tmpl = script_template(TM_SUP, sup, None, true);
    cat(&[&tmpl, base])
}
pub fn presubsuperscript(base: &[u8], sub: &[u8], sup: &[u8]) -> Vec<u8> {
    let tmpl = script_template(TM_SUBSUP, sub, Some(sup), true);
    cat(&[&tmpl, base])
}
