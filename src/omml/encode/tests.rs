use super::font::NS_W;
use super::*;
use crate::omml::decode::omml_to_mathml;

#[test]
fn math_text_runs_have_normal_text_properties() {
    for (text, space) in [
        ("với x", None),
        ("cm", None),
        ("m ", Some("preserve")),
        (" ", Some("preserve")),
    ] {
        let xml = mathml_to_omml_xml(&MathNode::leaf("mtext", text)).unwrap();
        let omml = crate::xml::parse(xml.as_bytes()).unwrap();
        let run = omml.find(NS_M, "r").unwrap();
        let props = run.find(NS_M, "rPr").unwrap();
        assert!(props.find(NS_M, "nor").is_some(), "{xml}");
        let style = props.find(NS_M, "sty").unwrap();
        assert_eq!(style.attr(Some(NS_M), "val"), Some("p"));
        let token = run.find(NS_M, "t").unwrap();
        assert_eq!(token.attr(Some(XML_NS), "space"), space);
        assert_eq!(token.text(), text);
        assert_eq!(
            omml_to_mathml(&omml).unwrap().children[0].children,
            [MathNode::leaf("mtext", text)]
        );
    }
}

#[test]
fn default_token_styles_are_explicit_in_nested_equations() {
    let math = MathNode::parse("<math><mfrac><mn>1</mn><mn>2</mn></mfrac><mi>x</mi><mo>+</mo><mn>3</mn><mo>=</mo><mn>0</mn><mi>sin</mi><mtext>text</mtext><ms>string</ms></math>").unwrap();
    let omml = mathml_to_omml(&math).unwrap();
    for run in omml.iter().into_iter().filter(|e| e.is(NS_M, "r")) {
        let text = run.find(NS_M, "t").unwrap().text();
        let style = run.find(NS_M, "rPr").unwrap().find(NS_M, "sty").unwrap();
        assert_eq!(
            style.attr(Some(NS_M), "val"),
            Some(if text == "x" { "i" } else { "p" }),
            "run {text}"
        );
    }
}

#[test]
fn explicit_token_variants_override_the_default_style() {
    for (tag, variant, expected) in [
        ("mn", "italic", "i"),
        ("mn", "bold", "b"),
        ("mo", "bold-italic", "bi"),
        ("mi", "normal", "p"),
        ("mi", "bold-italic", "bi"),
    ] {
        let mut token = MathNode::leaf(tag, if tag == "mi" { "x" } else { "1" });
        token.set_attr("mathvariant", variant);
        let omml = mathml_to_omml(&token).unwrap();
        let style = omml
            .find(NS_M, "r")
            .unwrap()
            .find(NS_M, "rPr")
            .unwrap()
            .find(NS_M, "sty")
            .unwrap();
        assert_eq!(style.attr(Some(NS_M), "val"), Some(expected));
    }
}

#[test]
fn test_fraction_and_roots_to_omml() {
    let mml = MathNode::parse(
        "<math><mrow><mfrac><mi>a</mi><mi>b</mi></mfrac><mo>+</mo><msqrt><mi>x</mi></msqrt><mo>+</mo><mroot><mi>y</mi><mn>3</mn></mroot></mrow></math>",
    )
    .unwrap();
    let omml = mathml_to_omml(&mml).unwrap();
    assert!(omml.is(NS_M, "oMath"));
    assert!(omml.find(NS_M, "f").is_some());
    let rads = omml.find_all(NS_M, "rad");
    assert_eq!(rads.len(), 2);

    // Round-trip OMML -> MathML -> OMML
    let mml_rt = omml_to_mathml(&omml).unwrap();
    let omml_rt = mathml_to_omml(&mml_rt).unwrap();
    assert_eq!(
        omml.find_all(NS_M, "rad").len(),
        omml_rt.find_all(NS_M, "rad").len()
    );
}

#[test]
fn test_function_name_merging_and_nary() {
    let mml = MathNode::parse(
        "<math><mrow><munderover><mo>∑</mo><mrow><mi>k</mi><mo>=</mo><mn>0</mn></mrow><mi>n</mi></munderover><mi>s</mi><mi>i</mi><msup><mi>n</mi><mn>2</mn></msup><mi>k</mi></mrow></math>",
    )
    .unwrap();
    let omml = mathml_to_omml(&mml).unwrap();
    let nary = omml.find(NS_M, "nary").expect("should have m:nary");
    let e = nary.find(NS_M, "e").expect("nary should have m:e");
    let ssup = e
        .find(NS_M, "sSup")
        .expect("sin^2 should be merged into sSup");
    let ssup_e = ssup.find(NS_M, "e").unwrap();
    let r = ssup_e.find(NS_M, "r").unwrap();
    assert_eq!(r.find(NS_M, "t").unwrap().text(), "sin");
}

#[test]
fn test_single_sided_brace_and_trailing_closer() {
    // Left brace `{` + mtable
    let left_sys = MathNode::parse(
        "<math><mrow><mo>{</mo><mtable><mtr><mtd><mi>x</mi><mo>=</mo><mn>1</mn></mtd></mtr><mtr><mtd><mi>y</mi><mo>=</mo><mn>2</mn></mtd></mtr></mtable></mrow></math>",
    )
    .unwrap();
    let omml_left = mathml_to_omml(&left_sys).unwrap();
    let d_left = omml_left.find(NS_M, "d").expect("should wrap in m:d");
    let d_pr_left = d_left.find(NS_M, "dPr").unwrap();
    assert_eq!(
        d_pr_left
            .find(NS_M, "begChr")
            .unwrap()
            .attr(Some(NS_M), "val"),
        Some("{")
    );
    assert_eq!(
        d_pr_left
            .find(NS_M, "endChr")
            .unwrap()
            .attr(Some(NS_M), "val"),
        Some("")
    );

    // Right brace `mtable` + `}` (even with stretchy="false")
    let right_sys = MathNode::parse(
        "<math><mrow><mtable><mtr><mtd><mi>x</mi><mo>=</mo><mn>1</mn></mtd></mtr></mtable><mo stretchy=\"false\">}</mo></mrow></math>",
    )
    .unwrap();
    let omml_right = mathml_to_omml(&right_sys).unwrap();
    let d_right = omml_right.find(NS_M, "d").expect("should wrap in m:d");
    let d_pr_right = d_right.find(NS_M, "dPr").unwrap();
    assert_eq!(
        d_pr_right
            .find(NS_M, "begChr")
            .unwrap()
            .attr(Some(NS_M), "val"),
        Some("")
    );
    assert_eq!(
        d_pr_right
            .find(NS_M, "endChr")
            .unwrap()
            .attr(Some(NS_M), "val"),
        Some("}")
    );
}

#[test]
fn test_accents_bars_and_prescripts() {
    let mml = MathNode::parse(
        "<math><mrow><mover accent=\"true\"><mi>x</mi><mo>^</mo></mover><mo>+</mo><mover accent=\"true\"><mrow><mi>A</mi><mi>B</mi></mrow><mo>¯</mo></mover><mo>+</mo><mmultiscripts><mi>U</mi><mprescripts/><mn>92</mn><mn>235</mn></mmultiscripts></mrow></math>",
    )
    .unwrap();
    let omml = mathml_to_omml(&mml).unwrap();
    let acc = omml.find(NS_M, "acc").expect("should have m:acc");
    assert_eq!(
        acc.find(NS_M, "accPr")
            .unwrap()
            .find(NS_M, "chr")
            .unwrap()
            .attr(Some(NS_M), "val"),
        Some("\u{0302}")
    );
    assert!(omml.find(NS_M, "bar").is_some());
    assert!(omml.find(NS_M, "sPre").is_some());
}

#[test]
fn test_vector_arrows_embellishments_nary_and_boxes_roundtrip() {
    use crate::mtef::decode::mtef_to_mathml;
    use crate::mtef::encode::mathml_to_mtef_bytes;
    use crate::mtef::{build_mtef, TM_BOX, TM_LDIV, TM_UNION, TM_VEC};

    // 1. Left & bidirectional vector arrows over/under characters -> TM_VEC (31)
    let vec_mml = MathNode::parse(
        "<math><mrow><mover><mrow><mi>A</mi><mi>B</mi></mrow><mo>←</mo></mover><mo>+</mo><mover><mrow><mi>C</mi><mi>D</mi></mrow><mo>↔</mo></mover><mo>+</mo><munder><mrow><mi>E</mi><mi>F</mi></mrow><mo>→</mo></munder></mrow></math>",
    )
    .unwrap();
    let vec_bytes = mathml_to_mtef_bytes(&vec_mml).unwrap();
    assert_eq!(
        vec_bytes
            .windows(3)
            .filter(|w| w[0] == 0x03 && w[2] == TM_VEC)
            .count(),
        3
    );
    let decoded_vec = mtef_to_mathml(&build_mtef(&vec_bytes)).unwrap();
    let omml_vec = mathml_to_omml(&decoded_vec).unwrap();
    assert_eq!(omml_vec.find_all(NS_M, "acc").len(), 2);
    assert_eq!(omml_vec.find_all(NS_M, "groupChr").len(), 1);

    // 2. Combining marks on <mi>/<mo> -> EMB_* record
    let comb_mml =
        MathNode::parse("<math><mrow><mi>x\u{0307}</mi><mo>=\u{0338}</mo><mn>0</mn></mrow></math>")
            .unwrap();
    let comb_bytes = mathml_to_mtef_bytes(&comb_mml).unwrap();
    // Should contain CHAR records with OPT_CHAR_EMBELL (0x01) followed by EMBELL (0x06)
    assert!(comb_bytes
        .windows(2)
        .any(|w| w[0] == 0x02 && (w[1] & 0x01) != 0));
    let decoded_comb = mtef_to_mathml(&build_mtef(&comb_bytes)).unwrap();
    let omml_comb = mathml_to_omml(&decoded_comb).unwrap();
    assert!(omml_comb.find(NS_M, "acc").is_some());
    assert!(omml_comb.find(NS_M, "borderBox").is_some());

    // 3. Union / Intersection big operators -> TM_UNION -> m:nary
    let union_mml = MathNode::parse(
        "<math><mrow><munderover><mo>⋃</mo><mrow><mi>i</mi><mo>=</mo><mn>1</mn></mrow><mi>n</mi></munderover><msub><mi>A</mi><mi>i</mi></msub></mrow></math>",
    )
    .unwrap();
    let union_bytes = mathml_to_mtef_bytes(&union_mml).unwrap();
    assert!(union_bytes
        .windows(3)
        .any(|w| w[0] == 0x03 && w[2] == TM_UNION));
    let decoded_union = mtef_to_mathml(&build_mtef(&union_bytes)).unwrap();
    let omml_union = mathml_to_omml(&decoded_union).unwrap();
    let nary = omml_union
        .find(NS_M, "nary")
        .expect("tmUNION should map to m:nary");
    assert_eq!(
        nary.find(NS_M, "naryPr")
            .unwrap()
            .find(NS_M, "chr")
            .unwrap()
            .attr(Some(NS_M), "val"),
        Some("⋃")
    );

    // 4. Partial border box (top + left) & long division with quotient
    let box_mml = MathNode::parse(
        "<math><mrow><menclose notation=\"top left\"><mi>x</mi></menclose><mo>+</mo><mover><menclose notation=\"longdiv\"><mn>144</mn></menclose><mn>12</mn></mover></mrow></math>",
    )
    .unwrap();
    let box_bytes = mathml_to_mtef_bytes(&box_mml).unwrap();
    assert!(box_bytes.windows(3).any(|w| w[0] == 0x03 && w[2] == TM_BOX));
    assert!(box_bytes
        .windows(4)
        .any(|w| w[0] == 0x03 && w[2] == TM_LDIV && (w[3] & 0x01) != 0));
    let decoded_box = mtef_to_mathml(&build_mtef(&box_bytes)).unwrap();
    let roundtrip_box_bytes = mathml_to_mtef_bytes(&decoded_box).unwrap();
    assert!(roundtrip_box_bytes
        .windows(4)
        .any(|w| w[0] == 0x03 && w[2] == TM_LDIV && (w[3] & 0x01) != 0));
}

#[test]
fn test_derivative_primes_direct_runs_and_mtef_embellishments() {
    use crate::mtef::decode::mtef_to_mathml;
    use crate::mtef::encode::mathml_to_mtef_bytes;
    use crate::mtef::{build_mtef, EMB_1PRIME, EMB_2PRIME, EMB_3PRIME};

    // 1. msup with prime (1st, 2nd, 3rd) encodes directly to OMML without m:sSup
    let primes = [("'", "'"), ("''", "''"), ("'''", "'''"), ("\u{2032}", "'")];
    for (sup_char, expected_prime) in primes {
        let mml = MathNode::parse(&format!(
            "<math><msup><mi>y</mi><mo>{}</mo></msup></math>",
            sup_char
        ))
        .unwrap();
        let omml = mathml_to_omml(&mml).unwrap();
        assert!(
            omml.find(NS_M, "sSup").is_none(),
            "msup with prime '{}' must not emit m:sSup",
            sup_char
        );
        let runs: Vec<String> = omml
            .find_all(NS_M, "r")
            .into_iter()
            .filter_map(|r| r.find(NS_M, "t").map(|t| t.text()))
            .collect();
        assert_eq!(runs, vec!["y".to_string(), expected_prime.to_string()]);
    }

    // 2. Direct sequence [y, '] encodes to MathType EMB_1PRIME, [y, ''] to EMB_2PRIME, [y, '''] to EMB_3PRIME
    let orders = [("'", EMB_1PRIME), ("''", EMB_2PRIME), ("'''", EMB_3PRIME)];
    for (prime_str, expected_emb) in orders {
        let mml = MathNode::parse(&format!(
            "<math><mrow><mi>y</mi><mo>{}</mo></mrow></math>",
            prime_str
        ))
        .unwrap();
        let mtef_bytes = mathml_to_mtef_bytes(&mml).unwrap();
        // Should contain embellishment record: [0x06, 0x00, expected_emb]
        assert!(
            mtef_bytes
                .windows(3)
                .any(|w| w[0] == 0x06 && w[1] == 0x00 && w[2] == expected_emb),
            "Should attach embellishment code {} for prime {}",
            expected_emb,
            prime_str
        );

        // Decode MathType back to OMML: should produce direct runs without m:sSup
        let decoded_mml = mtef_to_mathml(&build_mtef(&mtef_bytes)).unwrap();
        let roundtrip_omml = mathml_to_omml(&decoded_mml).unwrap();
        assert!(
            roundtrip_omml.find(NS_M, "sSup").is_none(),
            "Roundtrip OMML must not contain m:sSup"
        );
        let runs: Vec<String> = roundtrip_omml
            .find_all(NS_M, "r")
            .into_iter()
            .filter_map(|r| r.find(NS_M, "t").map(|t| t.text()))
            .collect();
        assert_eq!(runs, vec!["y".to_string(), prime_str.to_string()]);
    }
}
#[test]
fn test_tuple_with_comma_does_not_become_prime() {
    use crate::mtef::decode::mtef_to_mathml;
    use crate::mtef::encode::mathml_to_mtef_bytes;
    use crate::mtef::{build_mtef, EMB_1PRIME};
    let mml = MathNode::parse("<math><mrow><mo>(</mo><mi>O</mi><mi>u</mi><mo>,</mo><mi>O</mi><mi>v</mi><mo>)</mo></mrow></math>").unwrap();
    let mtef = mathml_to_mtef_bytes(&mml).unwrap();
    assert!(
        !mtef.windows(3).any(|w| w[0] == 0x06 && w[2] == EMB_1PRIME),
        "Comma must not be converted to prime embellishment"
    );
    let decoded = mtef_to_mathml(&build_mtef(&mtef)).unwrap();
    let latex = decoded.to_latex();
    assert_eq!(latex, "(Ou,Ov)");
}
#[test]
fn extended_palette_attributes_survive_omml_roundtrip() {
    for notation in [
        "top left",
        "bottom right",
        "horizontalstrike",
        "verticalstrike",
        "updiagonalstrike downdiagonalstrike",
    ] {
        let math = MathNode::parse(&format!(
            "<math><menclose notation=\"{notation}\"><mi>x</mi></menclose></math>"
        ))
        .unwrap();
        let out = crate::omml::omml_to_mathml(&mathml_to_omml(&math).unwrap()).unwrap();
        let enclosure = out
            .iter()
            .into_iter()
            .find(|n| n.tag == "menclose")
            .unwrap();
        let result = enclosure.attr("notation").unwrap();
        for part in notation.split_whitespace() {
            assert!(result.contains(part), "{notation}: {result}");
        }
    }
    for width in ["0.166667em", "0.222222em", "0.277778em", "1em", "2em"] {
        let math = MathNode::parse(&format!(
            "<math><mi>x</mi><mspace width=\"{width}\"/><mi>y</mi></math>"
        ))
        .unwrap();
        let out = crate::omml::omml_to_mathml(&mathml_to_omml(&math).unwrap()).unwrap();
        let space = out
            .iter()
            .into_iter()
            .find(|n| n.tag == "mspace")
            .expect("explicit spacing must survive");
        assert!(
            (crate::symbols::misc::space_width_em(space.attr("width").unwrap())
                - crate::symbols::misc::space_width_em(width))
            .abs()
                < 0.00001
        );
    }
    let matrix=MathNode::parse("<math><mtable columnalign=\"left right\"><mtr><mtd><mi>x</mi></mtd><mtd><mi>y</mi></mtd></mtr></mtable></math>").unwrap();
    let out = crate::omml::omml_to_mathml(&mathml_to_omml(&matrix).unwrap()).unwrap();
    assert!(out
        .iter()
        .iter()
        .any(|n| n.tag == "mtable" && n.attr("columnalign") == Some("left right")));
}

#[test]
fn apply_math_font_sets_runs_and_ctrl_pr_on_structural_properties() {
    let mml = MathNode::parse(
        "<math><mrow><mfrac><mi>a</mi><mi>b</mi></mfrac><mo>+</mo><msqrt><mi>x</mi></msqrt><mo>+</mo><mrow><mo>(</mo><mi>y</mi><mo>)</mo></mrow><mo>+</mo><mover accent=\"true\"><mi>z</mi><mo>^</mo></mover><mo>+</mo><mover><mi>w</mi><mo>¯</mo></mover><mo>+</mo><munderover><mo>∑</mo><mrow><mi>i</mi><mo>=</mo><mn>1</mn></mrow><mi>n</mi></munderover><msub><mi>x</mi><mi>i</mi></msub></mrow></math>",
    )
    .unwrap();
    let mut omml = mathml_to_omml(&mml).unwrap();
    apply_math_font(&mut omml, "Cambria Math");

    for run in omml.iter().into_iter().filter(|e| e.is(NS_M, "r")) {
        let r_fonts = run
            .find(NS_W, "rPr")
            .and_then(|pr| pr.find(NS_W, "rFonts"))
            .expect("every m:r must have w:rPr/w:rFonts");
        assert_eq!(r_fonts.attr(Some(NS_W), "ascii"), Some("Cambria Math"));
        assert_eq!(r_fonts.attr(Some(NS_W), "hAnsi"), Some("Cambria Math"));
        assert_eq!(r_fonts.attr(Some(NS_W), "cs"), Some("Cambria Math"));
    }

    // Also cover all structural *Pr element names specified for ctrlPr injection.
    let mut synthetic = m_el("oMath");
    for pr_name in [
        "fPr",
        "dPr",
        "sSupPr",
        "sSubPr",
        "sSubSupPr",
        "radPr",
        "accPr",
        "barPr",
        "naryPr",
        "funcPr",
        "limLowPr",
        "limUppPr",
        "mPr",
        "eqArrPr",
        "borderBoxPr",
        "boxPr",
    ] {
        synthetic.push(m_el(pr_name));
    }
    apply_math_font(&mut synthetic, "STIX Two Math");
    for pr_el in synthetic.elements() {
        let r_fonts = pr_el
            .find(NS_M, "ctrlPr")
            .and_then(|c| c.find(NS_W, "rPr"))
            .and_then(|r| r.find(NS_W, "rFonts"))
            .unwrap_or_else(|| panic!("{} missing m:ctrlPr/w:rPr/w:rFonts", pr_el.local));
        assert_eq!(r_fonts.attr(Some(NS_W), "ascii"), Some("STIX Two Math"));
        assert_eq!(r_fonts.attr(Some(NS_W), "hAnsi"), Some("STIX Two Math"));
        assert_eq!(r_fonts.attr(Some(NS_W), "cs"), Some("STIX Two Math"));
    }

    // Calling apply_math_font twice must not duplicate w:rPr or m:ctrlPr.
    apply_math_font(&mut omml, "Cambria Math");
    assert!(omml
        .find_all(NS_M, "fPr")
        .into_iter()
        .all(|pr| pr.find_all(NS_M, "ctrlPr").len() == 1));
}

#[test]
fn set_minus_backslash_normalizes_to_u2216_and_empty_script_base_avoids_empty_e() {
    let set_mml = MathNode::parse(
        "<math><mi>D</mi><mo>=</mo><mi>ℝ</mi><mo>\\</mo><mrow><mo>{</mo><mrow><mo>-</mo><mn>1</mn></mrow><mo>}</mo></mrow><mtext>\\</mtext></math>",
    )
    .unwrap();
    let set_omml = mathml_to_omml(&set_mml).unwrap();
    let texts: Vec<String> = set_omml
        .iter()
        .into_iter()
        .filter(|e| e.is(NS_M, "t"))
        .map(|t| t.text())
        .collect();
    assert!(
        texts.iter().any(|t| t == "∖"),
        "expected U+2216 in {texts:?}"
    );
    // <mtext>\</mtext> remains literal '\', while <mo>\</mo> was converted to U+2216.
    assert_eq!(texts.iter().filter(|t| *t == "\\").count(), 1);

    // Empty script base on y_{CT} (where _{CT} has an empty <mrow/> base)
    let sub_mml = MathNode::parse(
        "<math><mrow><mi>y</mi><msub><mrow/><mrow><mi>C</mi><mi>T</mi></mrow></msub></mrow></math>",
    )
    .unwrap();
    let sub_omml = mathml_to_omml(&sub_mml).unwrap();
    let s_sub = sub_omml.find(NS_M, "sSub").expect("should emit m:sSub");
    let e = s_sub.find(NS_M, "e").expect("m:sSub must have m:e");
    assert!(
        !e.children.is_empty(),
        "empty script base must not produce <m:e/>"
    );
    assert_eq!(
        e.find(NS_M, "r")
            .and_then(|r| r.find(NS_M, "t"))
            .map(|t| t.text()),
        Some("\u{00A0}".to_string())
    );
}

#[test]
fn encode_munderover_with_arrow_base_emits_stretchy_group_chr_and_roundtrips() {
    let mml =
        crate::service::parse_latex_to_mathml(r"A \overset{f}{\underset{g}{\leftrightharpoons}} B")
            .unwrap();
    let omml = mathml_to_omml(&mml).unwrap();
    let group_chr = omml
        .iter()
        .into_iter()
        .find(|e| e.is(NS_M, "groupChr"))
        .expect("arrow munderover should emit m:groupChr");
    let chr = group_chr
        .find(NS_M, "groupChrPr")
        .and_then(|pr| pr.find(NS_M, "chr"))
        .and_then(|c| c.attr(Some(NS_M), "val"));
    assert_eq!(chr, Some("⇋"));

    let decoded = omml_to_mathml(&omml).unwrap();
    assert!(
        decoded
            .iter()
            .into_iter()
            .any(|n| n.tag == "munderover" && n.children[0].text == "⇋"),
        "roundtrip must preserve <munderover>: {}",
        decoded.to_xml()
    );

    // When under-label is longer than over-label, groupChr wraps the under-label inside limUpp
    let wide_under =
        crate::service::parse_latex_to_mathml(r"\overset{f}{\underset{catalyst}{\rightarrow}}")
            .unwrap();
    let omml_wide = mathml_to_omml(&wide_under).unwrap();
    let lim_upp = omml_wide.find(NS_M, "limUpp").expect("should use limUpp");
    assert!(lim_upp
        .find(NS_M, "e")
        .unwrap()
        .find(NS_M, "groupChr")
        .is_some());
    let decoded_wide = omml_to_mathml(&omml_wide).unwrap();
    assert!(decoded_wide
        .iter()
        .into_iter()
        .any(|n| n.tag == "munderover"));
}
