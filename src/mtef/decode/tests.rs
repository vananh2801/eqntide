//! Unit tests for MTEF v3/v5 decoding, character/PUA mapping, embellishments, and templates.

use super::super::build_mtef;
use super::super::encode::mathml_to_mtef_bytes;
use super::super::typeface::{TF_LCGREEK, TF_MTEXTRA, TF_SPACE, TF_SYMBOL, TF_VARIABLE};
use super::characters::map_mt_char_token;
use super::consts::{OPT_CHAR_EMBELL, OPT_CHAR_ENC_16, OPT_CHAR_ENC_8, OPT_CHAR_NO_MT, OPT_NUDGE};
use super::embellishment::wrap_embellishments;
use super::*;

#[test]
fn decodes_encoder_output_to_a_math_root() {
    let source = MathNode::parse(
        r#"<math><mrow><mi>x</mi><mo>+</mo><mfrac><mn>1</mn><mi>y</mi></mfrac></mrow></math>"#,
    )
    .unwrap();
    let records = mathml_to_mtef_bytes(&source).unwrap();
    let bytes = build_mtef(&records);

    let decoded = mtef_to_mathml(&bytes).unwrap();
    let xml = decoded.to_xml();
    assert!(xml.starts_with("<math"));
    assert!(xml.contains("<mi>x</mi>"));
    assert!(xml.contains("<mfrac>"));
}

#[test]
fn equation3_fnspace_eb_codes_decode_to_mspace_while_mtextra_keeps_arrows() {
    // Real Equation.3 (MTEF v3) stream for `[0 ; π/2]` using typeface 24 (0x98) + 0xEB02.
    let eq3_interval: &[u8] = &[
        0x03, 0x01, 0x01, 0x03, 0x0a, // v3 header
        0x0a, // REC_FULL
        0x01, // REC_LINE
        0x03, 0x03, 0x00, 0x00, // REC_TMPL tmBRACK
        0x01, // REC_LINE
        0x02, 0x88, 0x30, 0x00, // '0'
        0x02, 0x98, 0x02, 0xeb, // TF_SPACE (24), 0xEB02
        0x02, 0x82, 0x3b, 0x00, // ';'
        0x02, 0x98, 0x02, 0xeb, // TF_SPACE (24), 0xEB02
        0x03, 0x0e, 0x00, 0x00, // REC_TMPL tmFRACT
        0x01, 0x02, 0x84, 0xc0, 0x03, 0x00, // π
        0x01, 0x02, 0x88, 0x32, 0x00, // 2
        0x00, // end tmFRACT
        0x00, // end tmBRACK line
        0x02, 0x96, 0x5b, 0x00, // '['
        0x02, 0x96, 0x5d, 0x00, // ']'
        0x00, // end tmBRACK
        0x00, // end top line
    ];
    let mml = mtef_to_mathml(eq3_interval).expect("Equation.3 interval should decode");
    let xml = mml.to_xml();
    assert!(!xml.contains('\u{2944}'), "unexpected ⥄ in {xml}");
    assert!(
        mml.find_all("mspace").len() == 2
            || mml.iter().into_iter().filter(|n| n.tag == "mspace").count() == 2,
        "expected 2 mspace nodes in {xml}"
    );
    let omml = crate::omml::mathml_to_omml_xml(&mml).unwrap();
    assert!(!omml.contains('\u{2944}'), "unexpected ⥄ in {omml}");

    // Check all 0xEB01..=0xEB08 under TF_SPACE decode to <mspace>, while TF_MTEXTRA keeps arrows.
    for code in 0xEB01u16..=0xEB08 {
        let node = map_mt_char_token(TF_SPACE, code, false)
            .unwrap_or_else(|| panic!("TF_SPACE 0x{code:04X} should decode to spacing"));
        assert_eq!(node.tag, "mspace", "0x{code:04X}");
    }
    assert_eq!(
        map_mt_char_token(TF_MTEXTRA, 0xEB01, false).unwrap().text,
        "\u{2942}"
    );
    assert_eq!(
        map_mt_char_token(TF_MTEXTRA, 0xEB02, false).unwrap().text,
        "\u{2944}"
    );
}

#[test]
fn opt_char_no_mt_falls_back_to_enc_and_rec_embell_consumes_nudge() {
    // 1. OPT_CHAR_NO_MT (0x20) + OPT_CHAR_ENC_8 (0x04) with ASCII 'x' (TF_VARIABLE)
    //    and Symbol 0x61 -> 'α' (TF_SYMBOL), plus OPT_CHAR_ENC_16 (0x10) 'β' (0x03B2).
    let records_no_mt: Vec<u8> = vec![
        0x02,
        OPT_CHAR_NO_MT | OPT_CHAR_ENC_8,
        TF_VARIABLE | 0x80,
        b'x',
        0x02,
        OPT_CHAR_NO_MT | OPT_CHAR_ENC_8,
        TF_SYMBOL | 0x80,
        0x61,
        0x02,
        OPT_CHAR_NO_MT | OPT_CHAR_ENC_16,
        TF_LCGREEK | 0x80,
        0xB2,
        0x03,
    ];
    let mtef_no_mt = build_mtef(&records_no_mt);
    let mml_no_mt = mtef_to_mathml(&mtef_no_mt).expect("OPT_CHAR_NO_MT should decode");
    let xml_no_mt = mml_no_mt.to_xml();
    assert!(xml_no_mt.contains("<mi>x</mi>"), "{xml_no_mt}");
    assert!(xml_no_mt.contains("α"), "{xml_no_mt}");
    assert!(xml_no_mt.contains("β"), "{xml_no_mt}");

    // 2. REC_CHAR with OPT_CHAR_EMBELL followed by REC_EMBELL with OPT_NUDGE (0x08) and trailing 'y'.
    let records_nudge: Vec<u8> = vec![
        0x02,
        OPT_CHAR_EMBELL,
        TF_VARIABLE | 0x80,
        b'x',
        0x00, // CHAR 'x' with embell
        0x06,
        OPT_NUDGE,
        0x02,
        0xFE,
        0x02, // REC_EMBELL, opts=OPT_NUDGE, dx=2, dy=-2, code=EMB_1DOT (2)
        0x00, // end embell list
        0x02,
        0x00,
        TF_VARIABLE | 0x80,
        b'y',
        0x00, // trailing CHAR 'y'
    ];
    let mtef_nudge = build_mtef(&records_nudge);
    let mml_nudge = mtef_to_mathml(&mtef_nudge).expect("nudged REC_EMBELL should decode");
    let xml_nudge = mml_nudge.to_xml();
    assert!(
        xml_nudge.contains("<mover accent=\"true\"><mi>x</mi><mo>\u{02D9}</mo></mover>"),
        "{xml_nudge}"
    );
    assert!(
        xml_nudge.contains("<mi>y</mi>"),
        "trailing sibling 'y' corrupted: {xml_nudge}"
    );
}

#[test]
fn hbrace_under_accents_and_tm_arrow_directionality() {
    // 1. tmHBRACE emits U+23DE / U+23DF.
    let over_bytes = build_mtef(&[
        0x03,
        0x00,
        24,
        0x01,
        0x00, // tmHBRACE, tvHB_TOP (0x01), tmpl_opts=0
        0x01,
        0x00,
        0x02,
        0x00,
        TF_VARIABLE | 0x80,
        b'a',
        0x00,
        0x00, // slot 0: 'a'
        0x01,
        0x00,
        0x02,
        0x00,
        TF_VARIABLE | 0x80,
        b'n',
        0x00,
        0x00, // slot 1: 'n'
        0x00, // end tmHBRACE
    ]);
    let over_mml = mtef_to_mathml(&over_bytes).unwrap();
    assert!(
        over_mml.to_xml().contains('\u{23DE}'),
        "{}",
        over_mml.to_xml()
    );

    let under_bytes = build_mtef(&[
        0x03,
        0x00,
        24,
        0x00,
        0x00, // tmHBRACE, bottom (0x00)
        0x01,
        0x00,
        0x02,
        0x00,
        TF_VARIABLE | 0x80,
        b'a',
        0x00,
        0x00,
        0x01,
        0x00,
        0x02,
        0x00,
        TF_VARIABLE | 0x80,
        b'm',
        0x00,
        0x00,
        0x00,
    ]);
    let under_mml = mtef_to_mathml(&under_bytes).unwrap();
    assert!(
        under_mml.to_xml().contains('\u{23DF}'),
        "{}",
        under_mml.to_xml()
    );

    // 2. Under-accent embellishments 25..=28 have accentunder="true".
    for code in 25u8..=28 {
        let wrapped = wrap_embellishments(MathNode::leaf("mi", "x"), &[code]);
        assert_eq!(wrapped.tag, "munder");
        assert_eq!(wrapped.attr("accentunder"), Some("true"), "embell {code}");
    }

    // 3. tmARROW distinguishes left/right harpoons and double arrows.
    let make_tm_arrow = |var: u8| {
        build_mtef(&[
            0x03,
            0x00,
            14,
            var,
            0x00, // tmARROW
            0x01,
            0x00,
            0x02,
            0x00,
            TF_VARIABLE | 0x80,
            b'f',
            0x00,
            0x00, // top slot
            0x01,
            0x01, // null bottom slot
            0x00,
        ])
    };
    let left_harpoon = mtef_to_mathml(&make_tm_arrow(0x02 | 0x04 | 0x10)).unwrap();
    let right_harpoon = mtef_to_mathml(&make_tm_arrow(0x02 | 0x04 | 0x20)).unwrap();
    let eq_harpoon = mtef_to_mathml(&make_tm_arrow(0x02 | 0x04)).unwrap();
    let left_double = mtef_to_mathml(&make_tm_arrow(0x01 | 0x04 | 0x10)).unwrap();
    let right_double = mtef_to_mathml(&make_tm_arrow(0x01 | 0x04 | 0x20)).unwrap();
    assert!(left_harpoon.to_xml().contains('\u{21BC}'));
    assert!(right_harpoon.to_xml().contains('\u{21C0}'));
    assert!(eq_harpoon.to_xml().contains('\u{21CC}'));
    assert!(left_double.to_xml().contains('\u{21C6}'));
    assert!(right_double.to_xml().contains('\u{21C4}'));
}

#[test]
fn function_char_runs_and_script_bases_coalesce_to_named_functions() {
    for (latex, expected) in [
        (r"y = \cos x", r"y=\cos x"),
        (r"\sin^2 x", r"{\sin }^{2}x"),
        (r"\operatorname{arccot} x", r"\operatorname{arccot} x"),
        (r"\limsup_{n} x", r"\limsup"),
        (r"\liminf_{n} x", r"\liminf"),
    ] {
        let parsed = crate::service::parse_latex_to_mathml(latex).unwrap();
        let rec = crate::mathml_to_mtef_bytes(&parsed).unwrap();
        let decoded = mtef_to_mathml(&build_mtef(&rec)).unwrap();
        let roundtrip_latex = decoded.to_latex();
        assert!(
            !roundtrip_latex.contains(r"\mathrm{c}\mathrm{o}\mathrm{s}"),
            "{latex} -> {roundtrip_latex}"
        );
        assert!(
            roundtrip_latex.contains(expected),
            "{latex} -> expected `{expected}` in `{roundtrip_latex}` (xml: {})",
            decoded.to_xml()
        );
    }
}
