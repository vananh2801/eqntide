use super::*;

#[test]
fn latex_text_preserves_plain_text_and_decodes_escapes() {
    for (latex, expected) in [
        (r"\text{hello world}", "hello world"),
        (r"\text{5 cm}", "5 cm"),
        (r"\text{m/s}", "m/s"),
        (r"\text{ }", " "),
        (r"\text{ với  {x} }", " với  {x} "),
        (
            r"\text{a\_b \& \{c\} \% \$ \# \textbackslash{} \\ \textbackslash}",
            "a_b & {c} % $ # \\ \\ \\",
        ),
        (r"\text{a\ b~c}", "a b\u{00A0}c"),
    ] {
        let math = parse_latex_to_mathml(latex).unwrap();
        assert_eq!(
            math.children,
            [MathNode::leaf("mtext", expected)],
            "{latex}"
        );
    }
    assert!(parse_latex_to_mathml(r"\text{unclosed {group}").is_err());
}

#[test]
fn latex_text_works_inside_math_structures() {
    let math = parse_latex_to_mathml(
        r"\begin{cases}\frac{\text{5 cm}}{2} & \text{ với }x \\ \overset{\text{m/s}}{x} & \text{ }_1\end{cases}",
    )
    .unwrap();
    let spans: Vec<_> = math
        .iter()
        .into_iter()
        .filter(|node| node.tag == "mtext")
        .map(|node| node.text.as_str())
        .collect();
    assert_eq!(spans, ["5 cm", " với ", "m/s", " "]);
    assert!(math.iter().iter().any(|node| node.tag == "msub"));
}

#[test]
fn dfrac_converts_through_both_office_toggle_targets() {
    for to in ["flat_opc_omml", "flat_opc_mathtype"] {
        let response = convert_snippet(&ConvertSnippetRequest {
            data: r"\dfrac{1}{2}".into(),
            from: "latex".into(),
            to: to.into(),
            font_pt: 12.0,
        })
        .unwrap();
        let recovered = flat_opc_to_mathml(&response.result).unwrap();
        assert!(
            recovered.iter().iter().any(|node| node.tag == "mfrac"),
            "{to}"
        );
        if let Some(path) = response.emf_path {
            std::fs::remove_file(path).unwrap();
        }
    }
}

#[test]
fn unknown_latex_commands_are_errors_instead_of_equation_text() {
    assert!(parse_latex_to_mathml(r"x+\thiscommanddoesnotexist{a}").is_err());
}

#[test]
fn less_than_operator_parses_and_roundtrips() {
    let math = parse_latex_to_mathml(r"x < 0").unwrap();
    assert!(math
        .iter()
        .into_iter()
        .any(|node| node.tag == "mo" && node.text == "<"));
    let omml = mathml_to_omml_xml(&math).unwrap();
    assert!(omml.contains("&lt;"));
    let back = crate::omml::omml_to_mathml(&crate::xml::parse(omml.as_bytes()).unwrap()).unwrap();
    assert!(back
        .iter()
        .into_iter()
        .any(|node| node.tag == "mo" && node.text == "<"));
}

#[test]
fn aliases_preserve_delimiters_and_longer_control_words() {
    let math = parse_latex_to_mathml(r"\left(x\le0\right)\geq1").unwrap();
    let text: String = math.iter().iter().map(|node| node.text.as_str()).collect();
    assert!(text.contains('≤'));
    assert!(text.contains('≥'));
    assert!(!text.contains("PARSE ERROR"));
    assert!(parse_latex_to_mathml(r"\begin{cases}x & x\le0 \\ x^2 & x>0\end{cases}").is_ok());
}

#[test]
fn latex_integral_limits_use_the_editors_under_over_layout() {
    for (source, tag) in [
        (r"\int_a^b x dx", "munderover"),
        (r"\int_a x dx", "munder"),
        (r"\int^b x dx", "mover"),
        (r"\iint_a^b x dx", "munderover"),
        (r"\iiint_a^b x dx", "munderover"),
        (r"\oint_a^b x dx", "munderover"),
    ] {
        let math = parse_latex_to_mathml(source).unwrap();
        assert!(
            math.iter().iter().any(|node| node.tag == tag
                && node.children.first().is_some_and(|base| base.tag == "mo"
                    && base
                        .text
                        .chars()
                        .next()
                        .is_some_and(crate::symbols::is_integral))),
            "{source}"
        );
        let omml = mathml_to_omml_xml(&math).unwrap();
        assert!(omml.contains("undOvr"), "{source}");
    }
    for source in ["x_a", "x^b"] {
        let original =
            latex2mathml::latex_to_mathml(source, latex2mathml::DisplayStyle::Inline).unwrap();
        assert_eq!(
            parse_latex_to_mathml(source).unwrap(),
            MathNode::parse(&original).unwrap()
        );
    }
    for source in ["x_a^b", "x^b_a"] {
        let math = parse_latex_to_mathml(source).unwrap();
        let subsup = &math.children[0];
        assert_eq!(subsup.tag, "msubsup", "{source}: {}", math.to_xml());
        assert_eq!(subsup.children[0].text, "x");
        assert_eq!(subsup.children[1].text, "a");
        assert_eq!(subsup.children[2].text, "b");
    }
}

#[test]
fn mathml_import_keeps_explicit_side_limits() {
    let xml = r#"<math><munderover data-limloc="subSup"><mo>∫</mo><mi>a</mi><mi>b</mi></munderover></math>"#;
    let math = decode_input_to_mathml(xml, "mathml").unwrap();
    assert_eq!(math, MathNode::parse(xml).unwrap());
    assert!(mathml_to_omml_xml(&math).unwrap().contains("subSup"));
}

#[test]
fn prescript_empty_slot_renders_none_without_placeholder() {
    let math = parse_latex_to_mathml(r"\prescript{\prime}{}{x}").unwrap();
    let mult = math
        .iter()
        .into_iter()
        .find(|node| node.tag == "mmultiscripts")
        .unwrap();
    assert_eq!(mult.children.len(), 4, "{}", math.to_xml());
    assert_eq!(mult.children[1].tag, "mprescripts");
    assert_eq!(mult.children[2].tag, "none", "{}", math.to_xml());
    assert_eq!(mult.children[0].text, "x");
}

#[test]
fn compound_bases_with_script_pairs_parse_to_msubsup_and_roundtrip() {
    for source in [
        r"f'_0^1",
        r"\sqrt{x_a^b}_0^1",
        r"\left. x \right|_{0}^{1}",
        r"\left. \frac{1}{2} \right|_{0}^{1}",
    ] {
        let math = parse_latex_to_mathml(source).unwrap();
        assert!(
            math.iter().into_iter().any(|node| node.tag == "msubsup"),
            "{source}: {}",
            math.to_xml()
        );
        let omml = mathml_to_omml_xml(&math).unwrap();
        assert!(!omml.is_empty(), "{source}");
        let mtef = mathml_to_mtef_bytes(&math).unwrap();
        assert!(!mtef.is_empty(), "{source}");
        let back = parse_latex_to_mathml(&math.to_latex()).unwrap();
        assert!(
            back.iter().into_iter().any(|node| node.tag == "msubsup"),
            "{source} roundtrip: {}",
            back.to_xml()
        );
    }
}

#[test]
fn convert_snippet_rejects_target_unsupported_constructs_with_422() {
    for source in [
        r"\enclose{longdiv}{144}",
        r"\enclose{roundedbox}{x+y}",
        r"\underddddot{x}",
        r"\enclose{verticalstrike}{x}",
        r"\enclose{circle}{x}",
        r"\enclose{phasorangle}{x}",
    ] {
        for to in ["omml", "flat_opc_omml"] {
            let err = convert_snippet(&ConvertSnippetRequest {
                data: source.into(),
                from: "latex".into(),
                to: to.into(),
                font_pt: 12.0,
            })
            .unwrap_err();
            assert_eq!(err.status, 422, "{source} -> {to}: {}", err.message);
        }
    }

    // MathType rejects verticalstrike, circle, phasorangle.
    for source in [
        r"\enclose{verticalstrike}{x}",
        r"\enclose{circle}{x}",
        r"\enclose{phasorangle}{x}",
    ] {
        for to in ["mathtype", "flat_opc_mathtype"] {
            let err = convert_snippet(&ConvertSnippetRequest {
                data: source.into(),
                from: "latex".into(),
                to: to.into(),
                font_pt: 12.0,
            })
            .unwrap_err();
            assert_eq!(err.status, 422, "{source} -> {to}: {}", err.message);
        }
    }

    // MathType accepts longdiv, roundedbox, underddddot.
    for source in [
        r"\enclose{longdiv}{144}",
        r"\enclose{roundedbox}{x+y}",
        r"\underddddot{x}",
    ] {
        let res = convert_snippet(&ConvertSnippetRequest {
            data: source.into(),
            from: "latex".into(),
            to: "flat_opc_mathtype".into(),
            font_pt: 12.0,
        })
        .expect(source);
        assert!(res.result.contains("Equation.DSMT4"));
        if let Some(path) = res.emf_path {
            let _ = std::fs::remove_file(path);
        }
    }

    // Reverse conversion back to LaTeX remains allowed for any construct.
    let rev = convert_snippet(&ConvertSnippetRequest {
        data: "<math><menclose notation=\"longdiv\"><mn>144</mn></menclose></math>".into(),
        from: "mathml".into(),
        to: "latex".into(),
        font_pt: 12.0,
    })
    .unwrap();
    assert!(rev.result.contains("longdiv"));
}

#[test]
fn degree_inputs_unify_across_mathml_omml_and_mtef() {
    let expected = parse_latex_to_mathml(r"90\degree").unwrap();
    for src in [
        r"90^\circ",
        r"90^{\circ}",
        r"90 ^ { \circ }",
        r"90\degree",
        "90°",
    ] {
        let math = parse_latex_to_mathml(src).unwrap();
        assert_eq!(math, expected, "MathML mismatch for {src}");
        assert_eq!(
            mathml_to_omml_xml(&math).unwrap(),
            mathml_to_omml_xml(&expected).unwrap(),
            "OMML mismatch for {src}"
        );
        assert_eq!(
            mathml_to_mtef_bytes(&math).unwrap(),
            mathml_to_mtef_bytes(&expected).unwrap(),
            "MTEF mismatch for {src}"
        );
    }
    let comp = parse_latex_to_mathml(r"f \circ g").unwrap();
    assert!(comp
        .iter()
        .into_iter()
        .any(|node| node.tag == "mo" && node.text == "∘"));
    assert!(!comp.iter().into_iter().any(|node| node.text == "°"));
}

#[test]
fn standard_short_and_explicit_aliases_parse_to_identical_mathml() {
    for (alias, canonical) in [
        (r"x \le y", r"x \leq y"),
        (r"x \ge y", r"x \geq y"),
        (r"x \ne y", r"x \neq y"),
        (r"x \to y", r"x \rightarrow y"),
        (r"x \gets y", r"x \leftarrow y"),
        (r"P \implies Q", r"P \Rightarrow Q"),
        (r"P \iff Q", r"P \Leftrightarrow Q"),
        (r"P \land Q", r"P \wedge Q"),
        (r"P \lor Q", r"P \vee Q"),
        (r"\lnot P", r"\neg P"),
        (r"a, \dots, b", r"a, \ldots, b"),
        (r"A \smallsetminus B", r"A \setminus B"),
        (r"A \backslash B", r"A \setminus B"),
        (r"\varnothing", r"\emptyset"),
        (r"\lvert x \rvert", r"\left| x \right|"),
        (r"\lVert x \rVert", r"\Vert x \Vert"),
    ] {
        let math_alias =
            parse_latex_to_mathml(alias).unwrap_or_else(|err| panic!("{alias}: {err}"));
        let math_canon =
            parse_latex_to_mathml(canonical).unwrap_or_else(|err| panic!("{canonical}: {err}"));
        if !alias.contains(r"\lvert") {
            assert_eq!(math_alias, math_canon, "{alias} vs {canonical}");
        } else {
            assert!(math_alias
                .iter()
                .into_iter()
                .any(|node| node.tag == "mo" && node.text == "|"));
            assert!(math_canon
                .iter()
                .into_iter()
                .any(|node| node.tag == "mo" && node.text == "|"));
        }
    }
}

#[test]
fn standard_structural_syntax_matches_legacy_forms() {
    assert_eq!(
        parse_latex_to_mathml(r"{\displaystyle \frac{1}{2}}").unwrap(),
        parse_latex_to_mathml(r"\displaystyle{\frac{1}{2}}").unwrap()
    );
    assert_eq!(
        parse_latex_to_mathml(r"{\textstyle x + y}").unwrap(),
        parse_latex_to_mathml(r"\textstyle{x + y}").unwrap()
    );
    assert_eq!(
        parse_latex_to_mathml(r"{n \atop k}").unwrap(),
        parse_latex_to_mathml(r"\atopfrac{n}{k}").unwrap()
    );
    assert_eq!(
        parse_latex_to_mathml(r"\genfrac{}{}{0pt}{}{n}{k}").unwrap(),
        parse_latex_to_mathml(r"\atopfrac{n}{k}").unwrap()
    );
    assert_eq!(
        parse_latex_to_mathml(r"\binom{n}{k}").unwrap(),
        parse_latex_to_mathml(r"\genfrac{(}{)}{0pt}{}{n}{k}").unwrap()
    );
    let dbinom = parse_latex_to_mathml(r"\dbinom{n}{k}").unwrap();
    assert!(dbinom
        .iter()
        .into_iter()
        .any(|node| node.tag == "mfrac" && node.attr("displaystyle") == Some("true")));
    let tbinom = parse_latex_to_mathml(r"\tbinom{n}{k}").unwrap();
    assert!(tbinom
        .iter()
        .into_iter()
        .any(|node| node.tag == "mfrac" && node.attr("displaystyle") == Some("false")));
}

#[test]
fn double_and_triple_primes_flatten_to_single_superscript_without_nesting() {
    for (src, glyph) in [(r"f''", "″"), (r"f'''", "‴")] {
        let math = parse_latex_to_mathml(src).unwrap();
        let msups: Vec<_> = math
            .iter()
            .into_iter()
            .filter(|node| node.tag == "msup")
            .collect();
        assert_eq!(msups.len(), 1, "{src}: {}", math.to_xml());
        assert_eq!(msups[0].children[1].text, glyph, "{src}: {}", math.to_xml());
        assert_eq!(math.to_latex(), src);
    }
    let sub_sup = parse_latex_to_mathml(r"f''_0^1").unwrap();
    assert_eq!(sub_sup.to_latex(), r"{f''}_{0}^{1}");
}

#[test]
fn convert_snippet_rejects_non_finite_or_out_of_range_font_pt() {
    for bad in [f32::NAN, f32::INFINITY, 0.0, -1.0, 10_000.0] {
        let err = convert_snippet(&ConvertSnippetRequest {
            data: "x+1".into(),
            from: "latex".into(),
            to: "emf".into(),
            font_pt: bad,
        })
        .unwrap_err();
        assert_eq!(err.status, 400, "font_pt={bad}");
    }
}

#[test]
fn named_functions_roundtrip_across_latex_render_omml_and_mtef() {
    let functions = [
        (r"\sin x", "sin", r"\sin "),
        (r"\cos x", "cos", r"\cos "),
        (r"\tan x", "tan", r"\tan "),
        (r"\cot x", "cot", r"\cot "),
        (r"\sec x", "sec", r"\sec "),
        (r"\csc x", "csc", r"\csc "),
        (r"\arcsin x", "arcsin", r"\arcsin "),
        (r"\arccos x", "arccos", r"\arccos "),
        (r"\arctan x", "arctan", r"\arctan "),
        (
            r"\operatorname{arccot} x",
            "arccot",
            r"\operatorname{arccot}",
        ),
        (
            r"\operatorname{arcsec} x",
            "arcsec",
            r"\operatorname{arcsec}",
        ),
        (
            r"\operatorname{arccsc} x",
            "arccsc",
            r"\operatorname{arccsc}",
        ),
        (r"\sinh x", "sinh", r"\sinh "),
        (r"\cosh x", "cosh", r"\cosh "),
        (r"\tanh x", "tanh", r"\tanh "),
        (r"\coth x", "coth", r"\coth "),
        (r"\operatorname{sech} x", "sech", r"\operatorname{sech}"),
        (r"\operatorname{csch} x", "csch", r"\operatorname{csch}"),
        (
            r"\operatorname{arsinh} x",
            "arsinh",
            r"\operatorname{arsinh}",
        ),
        (
            r"\operatorname{arcosh} x",
            "arcosh",
            r"\operatorname{arcosh}",
        ),
        (
            r"\operatorname{artanh} x",
            "artanh",
            r"\operatorname{artanh}",
        ),
        (
            r"\operatorname{arcoth} x",
            "arcoth",
            r"\operatorname{arcoth}",
        ),
        (r"\operatorname{tg} x", "tg", r"\operatorname{tg}"),
        (r"\operatorname{ctg} x", "ctg", r"\operatorname{ctg}"),
        (r"\operatorname{cotg} x", "cotg", r"\operatorname{cotg}"),
        (r"\operatorname{cosec} x", "cosec", r"\operatorname{cosec}"),
        (r"\log x", "log", r"\log "),
        (r"\ln x", "ln", r"\ln "),
        (r"\lg x", "lg", r"\lg "),
        (r"\exp x", "exp", r"\exp "),
        (r"\limsup_{n} x", "lim sup", r"\limsup"),
        (r"\liminf_{n} x", "lim inf", r"\liminf"),
        (r"\sup S", "sup", r"\sup "),
        (r"\inf S", "inf", r"\inf "),
        (r"\det A", "det", r"\det "),
        (r"\dim V", "dim", r"\dim "),
        (r"\ker f", "ker", r"\ker "),
        (r"\hom(A,B)", "hom", r"\hom "),
        (r"\deg p", "deg", r"\deg "),
        (r"\arg z", "arg", r"\arg "),
        (r"\gcd(a,b)", "gcd", r"\gcd "),
        (r"\operatorname{lcm}(a,b)", "lcm", r"\operatorname{lcm}"),
        (r"\operatorname{sgn} x", "sgn", r"\operatorname{sgn}"),
        (r"\operatorname{mod} m", "mod", r"\operatorname{mod}"),
        (r"\Pr(A)", "Pr", r"\Pr "),
    ];
    for (src, expected_text, expected_latex) in functions {
        let math = parse_latex_to_mathml(src).unwrap_or_else(|err| panic!("{src}: {err}"));
        assert!(
            math.iter()
                .into_iter()
                .any(|node| node.text.trim() == expected_text
                    && crate::symbols::is_function(node.text.trim())),
            "{src}: expected upright function `{expected_text}` in {}",
            math.to_xml()
        );
        assert!(
            math.to_latex().contains(expected_latex),
            "{src}: expected `{expected_latex}` in `{}`",
            math.to_latex()
        );
        let omml = mathml_to_omml_xml(&math).unwrap_or_else(|err| panic!("omml {src}: {err}"));
        assert!(omml.contains("oMath"), "{src}");
        let mtef = mathml_to_mtef_bytes(&math).unwrap_or_else(|err| panic!("mtef {src}: {err}"));
        assert!(!mtef.is_empty(), "{src}");
        let decoded = crate::mtef::decode::mtef_to_mathml(&crate::mtef::build_mtef(&mtef))
            .expect("mtef decode");
        assert!(
            !decoded.to_xml().contains("<mn> </mn>"),
            "space decoded as mn for {src}: {}",
            decoded.to_xml()
        );
        assert!(
            crate::render::render_interactive(&math, 12.0).is_ok(),
            "render {src}"
        );
    }
}
