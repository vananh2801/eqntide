use super::super::records::char_record;
use super::*;
use crate::mtef::typeface::{TF_EXPAND, TF_MTEXTRA, TF_SYMBOL, TF_VARIABLE};

#[test]
fn test_limits_variation_and_three_slots() {
    let x = char_record(TF_VARIABLE, 'x' as u32);
    let a = char_record(TF_VARIABLE, 'a' as u32);
    let b = char_record(TF_VARIABLE, 'b' as u32);

    let over = overscript(&x, &b);
    assert_eq!(&over[0..5], &[0x03, 0x00, TM_LIM, TV_BO_UPPER as u8, 0x00]);

    let under = underscript(&x, &a);
    assert_eq!(&under[0..5], &[0x03, 0x00, TM_LIM, TV_BO_LOWER as u8, 0x00]);

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
    assert_eq!(
        &ket[0..7],
        &[0x03, 0x00, TM_DIRAC, TV_DI_RIGHT, 0x00, 0x01, 0x01]
    );
    assert_eq!(&ket[7..9], &[0x01, 0x00]);
}

#[test]
fn test_hat_box_emits_char_record() {
    let x = char_record(TF_VARIABLE, 'F' as u32);
    let vec_f = vector_arrow(&x, "→");
    assert_eq!(&vec_f[0..5], &[0x03, 0x00, TM_VEC, TV_VE_RIGHT, 0x00]);
    let exp_char = char_record(TF_EXPAND, 0x20D7);
    assert!(vec_f
        .windows(exp_char.len())
        .any(|w| w == exp_char.as_slice()));

    let hat = over_hat(&x);
    let hat_char = char_record(TF_EXPAND, 0x0302);
    assert!(hat
        .windows(hat_char.len())
        .any(|w| w == hat_char.as_slice()));

    let arc = over_arc(&x);
    let arc_char = char_record(TF_EXPAND, 0x2322);
    assert!(arc
        .windows(arc_char.len())
        .any(|w| w == arc_char.as_slice()));
}

#[test]
fn test_script_restores_full_size() {
    let two = char_record(TF_VARIABLE, '2' as u32);
    let sup = superscript(&two);
    assert_eq!(sup.last().copied(), Some(0x0A));
}

#[test]
fn test_integral_operator_characters_and_loop_glyphs() {
    let x = char_record(TF_VARIABLE, 'x' as u32);
    let int_char = char_record(TF_SYMBOL, 0x222B);

    let int1 = integral(&x, None, None, &[], TV_INT_1);
    assert_eq!(
        int1.windows(int_char.len())
            .filter(|w| *w == int_char.as_slice())
            .count(),
        1
    );

    let int2 = integral(&x, None, None, &[], TV_INT_2);
    assert_eq!(
        int2.windows(int_char.len())
            .filter(|w| *w == int_char.as_slice())
            .count(),
        2
    );

    let int3 = integral(&x, None, None, &[], TV_INT_3);
    assert_eq!(
        int3.windows(int_char.len())
            .filter(|w| *w == int_char.as_slice())
            .count(),
        3
    );

    let oint = integral(&x, None, None, &[], TV_INT_1 | TV_INT_LOOP);
    let loop1 = char_record(TF_MTEXTRA, 0xEE11);
    assert!(oint.windows(loop1.len()).any(|w| w == loop1.as_slice()));
    assert_eq!(
        oint.windows(int_char.len())
            .filter(|w| *w == int_char.as_slice())
            .count(),
        1
    );

    let oiint = integral(&x, None, None, &[], TV_INT_2 | TV_INT_LOOP);
    let loop2 = char_record(TF_MTEXTRA, 0xEE12);
    assert!(oiint.windows(loop2.len()).any(|w| w == loop2.as_slice()));
    assert_eq!(
        oiint
            .windows(int_char.len())
            .filter(|w| *w == int_char.as_slice())
            .count(),
        2
    );

    let oiiint = integral(&x, None, None, &[], TV_INT_3 | TV_INT_LOOP);
    let loop3 = char_record(TF_MTEXTRA, 0xEE13);
    assert!(oiiint.windows(loop3.len()).any(|w| w == loop3.as_slice()));
    assert_eq!(
        oiiint
            .windows(int_char.len())
            .filter(|w| *w == int_char.as_slice())
            .count(),
        3
    );

    let a = char_record(TF_VARIABLE, 'a' as u32);
    let b = char_record(TF_VARIABLE, 'b' as u32);
    let oiint_with_both = integral(&x, Some(&b), Some(&a), &[], TV_INT_2 | TV_INT_LOOP);
    let var_code = (oiint_with_both[3] as u16) | ((oiint_with_both[4] as u16) << 8);
    assert_eq!(var_code & TV_BO_UPPER, 0);
    assert_ne!(var_code & TV_BO_LOWER, 0);
}
