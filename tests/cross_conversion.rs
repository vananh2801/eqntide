use eqntide::mtef::decode::mtef_to_mathml;
use eqntide::mtef::encode::mathml_to_mtef_bytes;
use eqntide::mtef::{build_mtef, make_equation_native};
use eqntide::omml::{mathml_to_omml_xml, omml_to_mathml, NS_M};
use eqntide::render::render_preview;
use eqntide::service::parse_latex_to_mathml;
use eqntide::xml;
use eqntide::{
    extract_mtef_from_emf, make_ole, mathml_to_latex, ole_to_mathml, render_emf, MathNode,
};

fn count_omml(root: &xml::Element, local: &str) -> usize {
    root.iter()
        .into_iter()
        .filter(|e| e.is(NS_M, local))
        .count()
}

fn collect_text(node: &MathNode) -> String {
    let mut s = node.text.clone();
    for c in &node.children {
        s.push_str(&collect_text(c));
    }
    s
}

fn decode_omml_str(omml_xml: &str) -> MathNode {
    let root = xml::parse(omml_xml.as_bytes()).expect("valid OMML XML");
    omml_to_mathml(&root).expect("decode OMML")
}

fn roundtrip_text_through_mathtype(math: &MathNode, expected: &str) -> MathNode {
    let records = mathml_to_mtef_bytes(math).unwrap();
    let mtef = build_mtef(&records);
    let ole = make_ole(&make_equation_native(&records)).unwrap();
    let decoded = ole_to_mathml(&ole).unwrap();
    assert_eq!(mtef_to_mathml(&mtef).unwrap(), decoded);
    assert_eq!(collect_text(&decoded).replace('\u{00A0}', " "), expected);
    let spaces: usize = decoded
        .find_all("mtext")
        .into_iter()
        .map(|node| node.text.matches('\u{00A0}').count())
        .sum();
    assert_eq!(spaces, expected.matches(' ').count());

    let emf = render_emf(math, 12.0).unwrap();
    assert_eq!(extract_mtef_from_emf(&emf.emf).unwrap(), mtef);
    assert!(!render_preview(math, 12.0).unwrap().png.is_empty());
    decoded
}

#[test]
fn omml_text_and_unit_spaces_survive_mathtype() {
    for (runs, expected) in [
        (
            r#"<m:r><m:t>5</m:t></m:r><m:r><m:t xml:space="preserve"> </m:t></m:r><m:r><m:rPr><m:nor/></m:rPr><m:t>cm</m:t></m:r>"#,
            "5 cm",
        ),
        (
            r#"<m:r><m:rPr><m:nor/></m:rPr><m:t xml:space="preserve">với </m:t></m:r>"#,
            "với ",
        ),
        (
            r#"<m:r><m:rPr><m:sty m:val="p"/></m:rPr><m:t xml:space="preserve">m </m:t></m:r>"#,
            "m ",
        ),
    ] {
        let math = decode_omml_str(&format!(r#"<m:oMath xmlns:m="{NS_M}">{runs}</m:oMath>"#));
        assert_eq!(collect_text(&math), expected);
        roundtrip_text_through_mathtype(&math, expected);
    }
}

#[test]
fn mathtype_text_characters_recombine_into_one_latex_text_span() {
    let source = r"\text{ khi}x\text{ với }";
    let math = parse_latex_to_mathml(source).unwrap();
    let roundtrip = roundtrip_text_through_mathtype(&math, " khix với ");
    assert_eq!(mathml_to_latex(&roundtrip), source);
}

#[test]
fn latex_text_and_spaces_survive_mathtype_and_latex_serialization() {
    for (source, expected) in [
        (r"\text{hello world}", "hello world"),
        (r"\text{5 cm}", "5 cm"),
        (r"\text{m/s}", "m/s"),
        (r"\text{ với  }", " với  "),
        (r"\text{m }", "m "),
        (r"\text{ }", " "),
    ] {
        let math = parse_latex_to_mathml(source).unwrap();
        let roundtrip = roundtrip_text_through_mathtype(&math, expected);
        let latex = mathml_to_latex(&roundtrip);
        let reparsed = parse_latex_to_mathml(&latex).unwrap();
        assert_eq!(collect_text(&reparsed), expected, "{source} -> {latex}");
        assert_eq!(
            reparsed
                .find_all("mtext")
                .iter()
                .map(|span| span.text.matches(' ').count())
                .sum::<usize>(),
            expected.matches(' ').count(),
            "{source} -> {latex}",
        );
    }
}

fn roundtrip_omml_mtef_omml(latex: &str) -> (String, MathNode, String, MathNode) {
    let orig = parse_latex_to_mathml(latex).expect("parse latex");
    let omml1 = mathml_to_omml_xml(&orig).expect("orig -> OMML");
    let from_omml = decode_omml_str(&omml1);
    let mtef_records = mathml_to_mtef_bytes(&from_omml).expect("OMML MathML -> MTEF");
    let mtef = build_mtef(&mtef_records);
    let from_mtef = mtef_to_mathml(&mtef).expect("MTEF -> MathML");
    let omml2 = mathml_to_omml_xml(&from_mtef).expect("MTEF MathML -> OMML");
    let final_mathml = decode_omml_str(&omml2);
    (omml1, from_mtef, omml2, final_mathml)
}

fn roundtrip_mtef_omml_mtef(latex: &str) -> (MathNode, String, MathNode) {
    let orig = parse_latex_to_mathml(latex).expect("parse latex");
    let mtef1 = build_mtef(&mathml_to_mtef_bytes(&orig).expect("orig -> MTEF"));
    let from_mtef1 = mtef_to_mathml(&mtef1).expect("MTEF1 -> MathML");
    let omml = mathml_to_omml_xml(&from_mtef1).expect("MTEF MathML -> OMML");
    let from_omml = decode_omml_str(&omml);
    let mtef2 = build_mtef(&mathml_to_mtef_bytes(&from_omml).expect("OMML MathML -> MTEF"));
    let from_mtef2 = mtef_to_mathml(&mtef2).expect("MTEF2 -> MathML");
    (from_mtef1, omml, from_mtef2)
}

#[test]
fn bug1_under_arrows_and_labeled_arrows_preserve_roles_across_omml_and_mtef() {
    for (latex, arrow_char) in [
        (r"\underleftarrow{AB}", "←"),
        (r"\underrightarrow{AB}", "→"),
        (r"\underleftrightarrow{AB}", "↔"),
        (r"\underleftharpoon{AB}", "↼"),
        (r"\underrightharpoon{AB}", "⇀"),
    ] {
        let (omml1, from_mtef, omml2, final_mathml) = roundtrip_omml_mtef_omml(latex);
        let omml1_el = xml::parse(omml1.as_bytes()).unwrap();
        assert_eq!(
            count_omml(&omml1_el, "groupChr"),
            1,
            "{latex} omml1: {omml1}"
        );

        let omml2_el = xml::parse(omml2.as_bytes()).unwrap();
        assert_eq!(
            count_omml(&omml2_el, "groupChr"),
            1,
            "{latex} omml2: {omml2}"
        );

        assert!(
            from_mtef
                .iter()
                .iter()
                .any(|n| n.tag == "munder" && n.attr("accentunder") == Some("true")),
            "{latex} from_mtef: {}",
            from_mtef.to_xml()
        );
        let under = final_mathml
            .iter()
            .into_iter()
            .find(|n| n.tag == "munder")
            .unwrap_or_else(|| panic!("{latex} missing munder: {}", final_mathml.to_xml()));
        assert_eq!(under.children[1].tag, "mo");
        assert_eq!(under.children[1].text, arrow_char);
        assert!(!final_mathml.iter().iter().any(|n| n.tag == "mover"));
    }

    // Labeled arrows must stay labeled arrows, not under-accents
    for (latex, expect_tag) in [
        (r"\overset{f}{\rightarrow}", "mover"),
        (r"\underset{f}{\rightarrow}", "munder"),
    ] {
        let (_omml1, from_mtef, _omml2, final_mathml) = roundtrip_omml_mtef_omml(latex);
        for tree in [&from_mtef, &final_mathml] {
            let node = tree
                .iter()
                .into_iter()
                .find(|n| n.tag == expect_tag)
                .unwrap_or_else(|| panic!("{latex}: {}", tree.to_xml()));
            assert_eq!(
                collect_text(&node.children[0]),
                "→",
                "{latex}: {}",
                tree.to_xml()
            );
            assert_eq!(
                collect_text(&node.children[1]),
                "f",
                "{latex}: {}",
                tree.to_xml()
            );
        }
    }
}

#[test]
fn bug2_over_and_under_harpoon_directions_survive_both_directions() {
    for (latex, expected) in [
        (r"\overleftharpoon{a}", "↼"),
        (r"\overrightharpoon{a}", "⇀"),
        (r"\underleftharpoon{a}", "↼"),
        (r"\underrightharpoon{a}", "⇀"),
    ] {
        let (_omml1, from_mtef, _omml2, final_mathml) = roundtrip_omml_mtef_omml(latex);
        assert!(
            from_mtef
                .iter()
                .iter()
                .any(|n| n.tag == "mo" && n.text == expected),
            "{latex} from_mtef: {}",
            from_mtef.to_xml()
        );
        assert!(
            final_mathml
                .iter()
                .iter()
                .any(|n| n.tag == "mo" && n.text.contains(expected)
                    || n.text == "\u{20D0}"
                    || n.text == "\u{20D1}"),
            "{latex} final_mathml: {}",
            final_mathml.to_xml()
        );

        let (m1, _omml, m2) = roundtrip_mtef_omml_mtef(latex);
        assert!(
            m1.iter()
                .iter()
                .any(|n| n.tag == "mo" && n.text == expected),
            "{latex} m1: {}",
            m1.to_xml()
        );
        assert!(
            m2.iter()
                .iter()
                .any(|n| n.tag == "mo" && n.text == expected),
            "{latex} m2: {}",
            m2.to_xml()
        );
    }
}

#[test]
fn bug3_prescripts_with_postscripts_survive_both_directions() {
    for (latex, expected_texts) in [
        (r"\prescript{a}{}{b}^{c}", vec!["a", "b", "c"]),
        (r"\prescript{}{b}{a}_{c}", vec!["b", "a", "c"]),
        (
            r"\prescript{a}{b}{c}_{d}^{e}",
            vec!["a", "b", "c", "d", "e"],
        ),
    ] {
        let (_omml1, from_mtef, omml2, final_mathml) = roundtrip_omml_mtef_omml(latex);
        let omml2_el = xml::parse(omml2.as_bytes()).unwrap();
        assert_eq!(
            count_omml(&omml2_el, "sPre"),
            1,
            "{latex} omml2 lost sPre: {omml2}"
        );
        assert!(
            from_mtef.iter().iter().any(|n| n.tag == "mmultiscripts"),
            "{latex} from_mtef: {}",
            from_mtef.to_xml()
        );
        assert!(
            final_mathml.iter().iter().any(|n| n.tag == "mmultiscripts"),
            "{latex} final_mathml: {}",
            final_mathml.to_xml()
        );
        for tok in expected_texts {
            assert!(
                final_mathml.iter().iter().any(|n| n.text == tok),
                "{latex} missing `{tok}` in {}",
                final_mathml.to_xml()
            );
        }

        let (m1, omml, m2) = roundtrip_mtef_omml_mtef(latex);
        assert_eq!(count_omml(&xml::parse(omml.as_bytes()).unwrap(), "sPre"), 1);
        assert!(m1.iter().iter().any(|n| n.tag == "mmultiscripts"));
        assert!(m2.iter().iter().any(|n| n.tag == "mmultiscripts"));
    }
}

#[test]
fn bug4_bigops_nolimits_and_tm_union_inter_roundtrip() {
    let (_omml1, from_mtef, omml2, final_mathml) =
        roundtrip_omml_mtef_omml(r"\bigwedge\nolimits_{a} b");
    assert!(
        from_mtef.iter().iter().any(|n| n.tag == "msub"),
        "expected msub for TM_INTOP, got {}",
        from_mtef.to_xml()
    );
    assert!(
        omml2.contains(r#"m:val="subSup""#),
        "expected subSup in omml2: {omml2}"
    );
    assert!(
        final_mathml
            .iter()
            .iter()
            .any(|n| n.attr("data-limloc") == Some("subSup")),
        "expected data-limloc=\"subSup\" in {}",
        final_mathml.to_xml()
    );

    let (m1, _omml, m2) = roundtrip_mtef_omml_mtef(r"\bigvee\nolimits_{a}^{b} c");
    assert!(m1.iter().iter().any(|n| n.tag == "msubsup"));
    assert!(m2.iter().iter().any(|n| n.tag == "msubsup"));

    for (latex, glyph) in [
        (r"\bigcup\limits_{a}^{b} c", "⋃"),
        (r"\bigcap\limits_{a}^{b} c", "⋂"),
    ] {
        let (_omml1, from_mtef, _omml2, final_mathml) = roundtrip_omml_mtef_omml(latex);
        assert!(
            from_mtef
                .iter()
                .iter()
                .any(|n| n.tag == "mo" && n.text == glyph),
            "{latex} from_mtef: {}",
            from_mtef.to_xml()
        );
        assert!(
            final_mathml
                .iter()
                .iter()
                .any(|n| n.tag == "mo" && n.text == glyph),
            "{latex} final_mathml: {}",
            final_mathml.to_xml()
        );
    }
}

#[test]
fn bug5_combined_over_under_limits_and_leftrightharpoons_roundtrip() {
    for (latex, base_glyph, is_arrow) in [
        (r"\overset{a}{\underset{b}{c}}", "c", false),
        (r"\overset{a}{\underset{b}{\Rightarrow}}", "⇒", true),
        (r"\overset{a}{\underset{b}{\leftrightharpoons}}", "⇋", true),
        (
            r"A \overset{f}{\underset{g}{\leftrightharpoons}} B",
            "⇋",
            true,
        ),
    ] {
        let (_omml1, from_mtef, omml2, final_mathml) = roundtrip_omml_mtef_omml(latex);
        let omml2_el = xml::parse(omml2.as_bytes()).unwrap();
        assert!(
            count_omml(&omml2_el, "sSubSup") == 0,
            "{latex} downgraded to sSubSup: {omml2}"
        );
        if is_arrow {
            assert!(
                count_omml(&omml2_el, "groupChr") >= 1
                    && (count_omml(&omml2_el, "limLow") >= 1
                        || count_omml(&omml2_el, "limUpp") >= 1),
                "{latex} expected stretchy groupChr + limit in {omml2}"
            );
        } else {
            assert!(
                count_omml(&omml2_el, "limLow") >= 1 && count_omml(&omml2_el, "limUpp") >= 1,
                "{latex} expected limLow+limUpp in {omml2}"
            );
        }
        assert!(
            from_mtef.iter().iter().any(|n| n.tag == "munderover"),
            "{latex} from_mtef: {}",
            from_mtef.to_xml()
        );
        assert!(
            final_mathml
                .iter()
                .iter()
                .any(|n| n.tag == "munderover" && n.children[0].text == base_glyph),
            "{latex} final_mathml: {}",
            final_mathml.to_xml()
        );
    }

    // Single-sided leftrightharpoons must remain `<mo>⇋</mo>` (not `<mtext>`)
    let (_omml1, from_mtef, _omml2, final_mathml) =
        roundtrip_omml_mtef_omml(r"\overset{a}{\leftrightharpoons}");
    assert!(
        from_mtef
            .iter()
            .iter()
            .any(|n| n.tag == "mo" && n.text == "⇋"),
        "{}",
        from_mtef.to_xml()
    );
    assert!(
        !from_mtef.iter().iter().any(|n| n.tag == "mtext"),
        "{}",
        from_mtef.to_xml()
    );
    let latex_out = mathml_to_latex(&final_mathml);
    assert!(!latex_out.contains(r"\text{"), "{latex_out}");
}

#[test]
fn bug6_single_column_matrices_preserve_center_default() {
    for latex in [
        r"\begin{matrix} a \\ b \end{matrix}",
        r"\begin{gathered} a \\ b \end{gathered}",
    ] {
        let (_omml1, from_mtef, omml2, final_mathml) = roundtrip_omml_mtef_omml(latex);
        let tbl_mtef = from_mtef
            .iter()
            .into_iter()
            .find(|n| n.tag == "mtable")
            .unwrap();
        assert_ne!(
            tbl_mtef.attr("columnalign"),
            Some("left"),
            "{latex} from_mtef: {}",
            from_mtef.to_xml()
        );
        assert!(
            !omml2.contains(r#"m:val="left""#),
            "{latex} omml2 forced left: {omml2}"
        );
        let tbl_final = final_mathml
            .iter()
            .into_iter()
            .find(|n| n.tag == "mtable")
            .unwrap();
        assert_ne!(tbl_final.attr("columnalign"), Some("left"));
    }
}

#[test]
fn bug7_diacritics_primes_backprime_underparen_and_doublestruck_roundtrip() {
    // \check{a} must not turn into \hat
    {
        let (_omml1, from_mtef, omml2, final_mathml) = roundtrip_omml_mtef_omml(r"\check{a}");
        assert!(
            from_mtef
                .iter()
                .iter()
                .any(|n| n.tag == "mo" && n.text == "ˇ"),
            "from_mtef: {}",
            from_mtef.to_xml()
        );
        assert!(omml2.contains("\u{030C}"), "omml2: {omml2}");
        assert_eq!(mathml_to_latex(&final_mathml), r"\check{a}");
    }

    // \mathring{a} must remain an <m:acc> ring accent
    {
        let (_omml1, from_mtef, omml2, final_mathml) = roundtrip_omml_mtef_omml(r"\mathring{a}");
        assert!(
            from_mtef
                .iter()
                .iter()
                .any(|n| n.tag == "mover" && n.attr("accent") == Some("true")),
            "from_mtef: {}",
            from_mtef.to_xml()
        );
        assert!(
            count_omml(&xml::parse(omml2.as_bytes()).unwrap(), "acc") == 1,
            "{omml2}"
        );
        assert_eq!(mathml_to_latex(&final_mathml), r"\mathring{a}");
    }

    // \underparen{AB} must preserve groupChr instead of limLow + mtext
    {
        let (_omml1, from_mtef, omml2, final_mathml) = roundtrip_omml_mtef_omml(r"\underparen{AB}");
        assert!(
            !from_mtef.iter().iter().any(|n| n.tag == "mtext"),
            "from_mtef: {}",
            from_mtef.to_xml()
        );
        assert!(
            count_omml(&xml::parse(omml2.as_bytes()).unwrap(), "groupChr") == 1,
            "{omml2}"
        );
        assert_eq!(mathml_to_latex(&final_mathml), r"\underparen{AB}");
    }

    // {a}'' and {a}''' must not split trailing primes into <mtext>'</mtext>
    for (latex, expected_prime) in [(r"{a}''", "''"), (r"{a}'''", "'''")] {
        let (_omml1, from_mtef, _omml2, final_mathml) = roundtrip_omml_mtef_omml(latex);
        assert!(
            !from_mtef.iter().iter().any(|n| n.tag == "mtext"),
            "{latex} from_mtef: {}",
            from_mtef.to_xml()
        );
        assert!(
            from_mtef
                .iter()
                .iter()
                .any(|n| n.tag == "mo" && n.text == expected_prime),
            "{latex} from_mtef: {}",
            from_mtef.to_xml()
        );
        assert!(
            final_mathml
                .iter()
                .iter()
                .any(|n| n.tag == "mo" && n.text == expected_prime),
            "{latex} final_mathml: {}",
            final_mathml.to_xml()
        );
    }

    // {a}^{\backprime} must keep ‵ (U+2035) as post-superscript
    {
        let (_omml1, from_mtef, omml2, final_mathml) =
            roundtrip_omml_mtef_omml(r"{a}^{\backprime}");
        assert!(
            !from_mtef.iter().iter().any(|n| n.tag == "mmultiscripts"),
            "backprime jumped to prescript: {}",
            from_mtef.to_xml()
        );
        assert!(
            from_mtef
                .iter()
                .iter()
                .any(|n| n.tag == "mo" && n.text == "‵"),
            "from_mtef: {}",
            from_mtef.to_xml()
        );
        assert!(omml2.contains("‵"), "omml2: {omml2}");
        assert!(
            final_mathml
                .iter()
                .iter()
                .any(|n| n.tag == "msup" && n.children[1].text == "‵"),
            "final_mathml: {}",
            final_mathml.to_xml()
        );
    }

    // Double-struck number sets keep double-struck variant across both directions
    for (latex, letter) in [
        (r"\mathbb{R}", "R"),
        (r"\mathbb{Z}", "Z"),
        (r"\mathbb{C}", "C"),
        (r"\mathbb{Q}", "Q"),
        (r"\mathbb{N}", "N"),
    ] {
        let (_omml1, from_mtef, omml2, final_mathml) = roundtrip_omml_mtef_omml(latex);
        assert!(
            from_mtef.iter().iter().any(|n| n.tag == "mi"
                && n.text == letter
                && n.attr("mathvariant") == Some("double-struck")),
            "{latex} from_mtef: {}",
            from_mtef.to_xml()
        );
        assert!(omml2.contains(r#"double-struck"#), "{latex} omml2: {omml2}");
        assert_eq!(mathml_to_latex(&final_mathml), latex);
    }
}

#[test]
fn fixed_expressions_render_png_without_error() {
    for latex in [
        r"\underrightarrow{AB} + \underleftharpoon{a} + \overleftharpoon{a}",
        r"\prescript{a}{b}{c}_{d}^{e} + \bigwedge\nolimits_{a} b + \bigcup\limits_{a}^{b} c",
        r"\overset{a}{\underset{b}{c}} + \overset{a}{\underset{b}{\Rightarrow}} + \check{a} + \underparen{AB} + {a}'' + {a}^{\backprime} + \mathbb{R}",
    ] {
        let math = parse_latex_to_mathml(latex).unwrap();
        let omml = mathml_to_omml_xml(&math).unwrap();
        let from_omml = decode_omml_str(&omml);
        let mtef = build_mtef(&mathml_to_mtef_bytes(&from_omml).unwrap());
        let from_mtef = mtef_to_mathml(&mtef).unwrap();
        let png = render_preview(&from_mtef, 48.0).expect("render png");
        assert!(png.png.starts_with(b"\x89PNG\r\n\x1a\n"));
    }
}
