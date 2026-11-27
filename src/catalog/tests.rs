use super::*;

fn fill_slots(n: &mut MathNode) {
    if matches!(n.tag.as_str(), "mrow" | "mtd") && n.children.is_empty() && n.text.is_empty() {
        n.children.push(MathNode::leaf("mi", "x"));
    }
    for child in &mut n.children {
        fill_slots(child);
    }
}

fn find_node<'a>(node: &'a MathNode, pred: &dyn Fn(&MathNode) -> bool) -> Option<&'a MathNode> {
    if pred(node) {
        return Some(node);
    }
    for child in &node.children {
        if let Some(found) = find_node(child, pred) {
            return Some(found);
        }
    }
    None
}

fn roundtrip_mtef(math: &MathNode) -> MathNode {
    let rec = crate::mathml_to_mtef_bytes(math).expect("mtef encode");
    crate::mtef::mtef_to_mathml(&crate::mtef::build_mtef(&rec))
        .expect("mtef decode should return Some")
}

fn roundtrip_omml(math: &MathNode) -> MathNode {
    let omml = crate::omml::mathml_to_omml(math).expect("omml encode");
    crate::omml::omml_to_mathml(&omml).expect("omml decode")
}

#[test]
fn catalog_has_unique_ids_and_twenty_one_nonempty_groups() {
    let ids: std::collections::HashSet<_> = ENTRIES.iter().map(|e| e.id).collect();
    assert_eq!(ids.len(), ENTRIES.len());
    assert_eq!(GROUPS.len(), 21);
    for (group, _) in GROUPS {
        assert!(ENTRIES.iter().any(|e| e.group == *group), "{group}");
    }
    for e in ENTRIES {
        assert!(GROUPS.iter().any(|(g, _)| *g == e.group), "{}", e.id);
    }
    let function_count = ENTRIES.iter().filter(|e| e.group == "Functions").count();
    assert_eq!(function_count, 39);
    assert!(!ENTRIES
        .iter()
        .any(|e| e.group == "Scripts"
            && matches!(e.id, "Scripts-lim" | "Scripts-min" | "Scripts-max")));
}

#[test]
fn every_catalog_recipe_is_insertable() {
    let errors: Vec<_> = ENTRIES
        .iter()
        .filter_map(|e| {
            template(e.source)
                .err()
                .map(|err| format!("{}: {}: {err}", e.id, e.source))
        })
        .collect();
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

#[test]
fn commands_catalog_includes_style_and_operator_commands() {
    for expected in [
        "mathscr",
        "mathrm",
        "mathbf",
        "mathit",
        "boldsymbol",
        "operatorname",
        "textstyle",
        "displaystyle",
        "vec",
        "lg",
        "tg",
        "ctg",
        "cotg",
        "cosec",
        "arsinh",
        "arcosh",
        "artanh",
        "arcoth",
    ] {
        let entry = COMMANDS
            .iter()
            .find(|(name, _, _)| *name == expected)
            .unwrap_or_else(|| panic!("missing command \\{expected} from catalog::COMMANDS"));
        assert!(
            template(entry.2).is_ok(),
            "template for \\{expected} (`{}`) failed",
            entry.2
        );
    }
    assert!(attaches_to_previous(r"\vec{#?}"));
    let vec_math = crate::service::parse_latex_to_mathml(r"\vec{x}").unwrap();
    assert_eq!(vec_math.to_latex(), r"\vec{x}");
    assert!(!roundtrip_mtef(&vec_math).children.is_empty());
    assert!(!roundtrip_omml(&vec_math).children.is_empty());
}

#[test]
fn literal_square_is_not_an_insertion_slot() {
    assert_eq!(template(r"\square").unwrap().text, "□");
    assert!(template(r"\frac{#?}{#?}")
        .unwrap()
        .children
        .iter()
        .all(|n| n.tag == "mrow" && n.children.is_empty()));
}

#[test]
fn custom_matrix_bounds_alignment_and_fences() {
    let node = matrix(0, 30, "right", "brackets");
    assert_eq!(node.children[0].text, "[");
    let table = &node.children[1];
    assert_eq!(table.children.len(), 1);
    assert_eq!(table.children[0].children.len(), 20);
    assert_eq!(table.attr("columnalign"), Some("right"));
    let table = matrix(2, 3, "center", "none");
    assert_eq!(table.tag, "mtable");
    assert_eq!(table.children.len(), 2);
    assert_eq!(table.children[0].children.len(), 3);

    for (fence_key, expected_open) in [("left-brace", "{"), ("left-bracket", "[")] {
        let mut sys = matrix(3, 1, "left", fence_key);
        assert_eq!(sys.children[0].text, expected_open);
        assert_eq!(sys.children[0].attr("form"), Some("prefix"));
        assert_eq!(sys.children[2].text, "");
        assert_eq!(sys.children[2].attr("form"), Some("postfix"));
        assert_eq!(sys.children[1].tag, "mtable");
        assert_eq!(sys.children[1].children.len(), 3);
        assert_eq!(sys.children[1].children[0].children.len(), 1);
        assert_eq!(sys.children[1].attr("columnalign"), Some("left"));

        fill_slots(&mut sys);
        let math = MathNode::with_children("math", vec![sys]);
        let latex = math.to_latex();
        assert!(
            latex.contains(r"\begin{array}{l}") && latex.ends_with(r"\right."),
            "unexpected latex for {fence_key}: {latex}"
        );
        let re_latex = crate::service::parse_latex_to_mathml(&latex).unwrap();
        assert!(find_node(&re_latex, &|n| n.tag == "mtable").is_some());

        let omml_back = roundtrip_omml(&math);
        assert!(
            find_node(&omml_back, &|n| n.tag == "mo"
                && n.text.trim() == expected_open)
            .is_some(),
            "omml lost {expected_open}: {}",
            omml_back.to_xml()
        );
        assert!(find_node(&omml_back, &|n| n.tag == "mtable").is_some());

        let mtef_back = roundtrip_mtef(&math);
        assert!(
            find_node(&mtef_back, &|n| n.tag == "mo"
                && n.text.trim() == expected_open)
            .is_some(),
            "mtef lost {expected_open}: {}",
            mtef_back.to_xml()
        );
        assert!(find_node(&mtef_back, &|n| n.tag == "mtable").is_some());

        let preview = crate::render::render_interactive(&math, 12.0).unwrap();
        assert!(!preview.preview.png.is_empty());
    }
}

#[test]
fn every_catalog_entry_roundtrips_through_latex_mtef_omml_and_render() {
    for ch in "⟦⟧⊄≙ƛ⤢⤡↵∍◊∖".chars() {
        assert!(
            crate::render::font::fonts().has_glyph(ch),
            "missing font glyph for {ch} (U+{:04X})",
            ch as u32
        );
    }
    for e in ENTRIES {
        let mut node = template(e.source).expect("parse template");
        fill_slots(&mut node);
        let math = MathNode::with_children("math", vec![node]);

        let latex = math.to_latex();
        crate::service::parse_latex_to_mathml(&latex)
            .unwrap_or_else(|err| panic!("{}: latex roundtrip `{latex}` failed: {err}", e.id));

        let mtef_back = roundtrip_mtef(&math);
        assert!(
            !mtef_back.children.is_empty(),
            "{}: mtef decode produced empty math",
            e.id
        );

        let omml_back = roundtrip_omml(&math);
        assert!(
            !omml_back.children.is_empty(),
            "{}: omml decode produced empty math",
            e.id
        );

        let preview = crate::render::render_interactive(&math, 12.0)
            .unwrap_or_else(|err| panic!("{}: render_interactive failed: {err}", e.id));
        assert!(!preview.preview.png.is_empty(), "{}: empty PNG", e.id);
    }
}

#[test]
fn catalog_semantic_attributes_survive_roundtrips() {
    // 1. Integral / sum / big-operator limit placement (\limits vs \nolimits).
    for (stack_src, side_src, op_glyph) in [
        (r"\int\limits_{a}^{b} x", r"\int\nolimits_{a}^{b} x", "∫"),
        (r"\sum\limits_{a}^{b} x", r"\sum\nolimits_{a}^{b} x", "∑"),
        (r"\prod\limits_{a}^{b} x", r"\prod\nolimits_{a}^{b} x", "∏"),
    ] {
        let stack_math = crate::service::parse_latex_to_mathml(stack_src).unwrap();
        let side_math = crate::service::parse_latex_to_mathml(side_src).unwrap();
        assert!(
            find_node(&stack_math, &|n| n.tag == "munderover").is_some(),
            "{stack_src}"
        );
        assert!(
            find_node(&side_math, &|n| n.tag == "msubsup").is_some(),
            "{side_src}"
        );

        for decoded in [
            crate::service::parse_latex_to_mathml(&stack_math.to_latex()).unwrap(),
            roundtrip_mtef(&stack_math),
            roundtrip_omml(&stack_math),
        ] {
            assert!(
                find_node(&decoded, &|n| {
                    n.tag == "munderover"
                        && n.attr("data-limloc") != Some("subSup")
                        && find_node(n, &|c| c.text.trim() == op_glyph).is_some()
                })
                .is_some(),
                "stacked limits lost for {stack_src}: {}",
                decoded.to_xml()
            );
        }
        for decoded in [
            crate::service::parse_latex_to_mathml(&side_math.to_latex()).unwrap(),
            roundtrip_mtef(&side_math),
            roundtrip_omml(&side_math),
        ] {
            assert!(
                find_node(&decoded, &|n| {
                    (n.tag == "msubsup"
                        || (n.tag == "munderover" && n.attr("data-limloc") == Some("subSup")))
                        && find_node(n, &|c| c.text.trim() == op_glyph).is_some()
                })
                .is_some(),
                "side limits lost for {side_src}: {}",
                decoded.to_xml()
            );
        }
    }

    // 2. Fraction variants (\dfrac, \tfrac, \sfrac, \tsfrac, \atopfrac).
    for (src, expected_cmd, attr_k, attr_v, preserved_in_omml, is_mtef_pile) in [
        (
            r"\dfrac{a}{b}",
            r"\dfrac",
            "displaystyle",
            "true",
            false,
            false,
        ),
        (
            r"\tfrac{a}{b}",
            r"\tfrac",
            "displaystyle",
            "false",
            false,
            false,
        ),
        (r"\sfrac{a}{b}", r"\sfrac", "bevelled", "true", true, false),
        (
            r"\tsfrac{a}{b}",
            r"\tsfrac",
            "bevelled",
            "true",
            true,
            false,
        ),
        (
            r"\atopfrac{a}{b}",
            r"\atop",
            "linethickness",
            "0",
            true,
            true,
        ),
    ] {
        let math = crate::service::parse_latex_to_mathml(src).unwrap();
        assert!(
            find_node(&math, &|n| n.tag == "mfrac"
                && n.attr(attr_k) == Some(attr_v))
            .is_some(),
            "{src}: {}",
            math.to_xml()
        );
        let latex_out = math.to_latex();
        assert!(latex_out.contains(expected_cmd), "{src} -> {latex_out}");
        let re_latex = crate::service::parse_latex_to_mathml(&latex_out).unwrap();
        assert!(
            find_node(&re_latex, &|n| n.tag == "mfrac"
                && n.attr(attr_k) == Some(attr_v))
            .is_some(),
            "latex roundtrip lost {attr_k}={attr_v} for {src}: {}",
            re_latex.to_xml()
        );
        let omml_back = roundtrip_omml(&math);
        assert!(
            find_node(&omml_back, &|n| n.tag == "mfrac").is_some(),
            "omml lost mfrac for {src}: {}",
            omml_back.to_xml()
        );
        if preserved_in_omml {
            assert!(
                find_node(&omml_back, &|n| n.tag == "mfrac"
                    && n.attr(attr_k) == Some(attr_v))
                .is_some(),
                "omml lost {attr_k}={attr_v} for {src}: {}",
                omml_back.to_xml()
            );
        }
        let mtef_back = roundtrip_mtef(&math);
        if is_mtef_pile {
            assert!(
                find_node(&mtef_back, &|n| n.tag == "mtable" && n.children.len() == 2).is_some(),
                "mtef lost 2-row pile for {src}: {}",
                mtef_back.to_xml()
            );
        } else {
            assert!(
                find_node(&mtef_back, &|n| n.tag == "mfrac").is_some(),
                "mtef lost mfrac for {src}: {}",
                mtef_back.to_xml()
            );
            if expected_cmd != r"\dfrac" {
                assert!(
                    find_node(&mtef_back, &|n| n.tag == "mfrac"
                        && n.attr(attr_k) == Some(attr_v))
                    .is_some(),
                    "mtef lost {attr_k}={attr_v} for {src}: {}",
                    mtef_back.to_xml()
                );
            }
        }
    }

    // 3. Prescripts (\prescript{a}{b}{c}).
    let pre = crate::service::parse_latex_to_mathml(r"\prescript{a}{b}{c}").unwrap();
    assert!(pre.to_latex().contains(r"\prescript"));
    for decoded in [roundtrip_mtef(&pre), roundtrip_omml(&pre)] {
        assert!(
            find_node(&decoded, &|n| n.tag == "mprescripts").is_some(),
            "mprescripts lost: {}",
            decoded.to_xml()
        );
    }

    // 4. Font styles (mathvariant).
    for (src, variant) in [
        (r"\mathbb{R}", "double-struck"),
        (r"\mathfrak{A}", "fraktur"),
        (r"\mathcal{A}", "script"),
        (r"\mathbf{x}", "bold"),
        (r"\boldsymbol{\alpha}", "bold-italic"),
        (r"\mathrm{d}", "normal"),
    ] {
        let math = crate::service::parse_latex_to_mathml(src).unwrap();
        assert!(
            find_node(&math, &|n| n.attr("mathvariant") == Some(variant)).is_some(),
            "{src}"
        );
        let re_latex = crate::service::parse_latex_to_mathml(&math.to_latex()).unwrap();
        assert!(
            find_node(&re_latex, &|n| n.attr("mathvariant") == Some(variant)).is_some(),
            "latex roundtrip lost {variant} for {src}: {}",
            math.to_latex()
        );
        let omml_back = roundtrip_omml(&math);
        assert!(
            find_node(&omml_back, &|n| n.attr("mathvariant") == Some(variant)).is_some(),
            "omml roundtrip lost {variant} for {src}: {}",
            omml_back.to_xml()
        );
        let mtef_back = roundtrip_mtef(&math);
        assert!(
            !mtef_back.children.is_empty(),
            "mtef roundtrip failed for {src}"
        );
    }

    // 5. Enclosures, strikes, double bars, accents.
    for (src, notation_token) in [
        (r"\boxed{x}", "box"),
        (r"\cancel{x}", "updiagonalstrike"),
        (r"\bcancel{x}", "downdiagonalstrike"),
        (r"\xcancel{x}", "updiagonalstrike"),
        (r"\enclose{top left}{x}", "top"),
        (r"\enclose{top bottom left}{x}", "bottom"),
        (r"\enclose{horizontalstrike}{x}", "horizontalstrike"),
    ] {
        let math = crate::service::parse_latex_to_mathml(src).unwrap();
        for decoded in [
            crate::service::parse_latex_to_mathml(&math.to_latex()).unwrap(),
            roundtrip_mtef(&math),
            roundtrip_omml(&math),
        ] {
            assert!(
                find_node(&decoded, &|n| {
                    n.tag == "menclose"
                        && n.attr("notation")
                            .is_some_and(|v| v.split_whitespace().any(|t| t == notation_token))
                })
                .is_some(),
                "menclose({notation_token}) lost for {src}: {}",
                decoded.to_xml()
            );
        }
    }
    for src in [
        r"\doubleoverline{x}",
        r"\doubleunderline{x}",
        r"\overaccent{↔}{x}",
        r"\underaccent{↔}{x}",
    ] {
        let math = crate::service::parse_latex_to_mathml(src).unwrap();
        assert!(!math.to_latex().is_empty());
        let _ = roundtrip_mtef(&math);
        let _ = roundtrip_omml(&math);
    }

    // 6. Matrices (array alignment, Bmatrix, Vmatrix, 4x4).
    let arr =
        crate::service::parse_latex_to_mathml(r"\begin{array}{lcr} a & b & c \end{array}").unwrap();
    assert_eq!(
        find_node(&arr, &|n| n.tag == "mtable").and_then(|t| t.attr("columnalign")),
        Some("left center right")
    );
    for decoded in [
        crate::service::parse_latex_to_mathml(&arr.to_latex()).unwrap(),
        roundtrip_omml(&arr),
    ] {
        assert_eq!(
            find_node(&decoded, &|n| n.tag == "mtable").and_then(|t| t.attr("columnalign")),
            Some("left center right"),
            "columnalign lost: {}",
            decoded.to_xml()
        );
    }
    for (src, delim) in [
        (r"\begin{Bmatrix} a & b \\ c & d \end{Bmatrix}", "{"),
        (r"\begin{Vmatrix} a & b \\ c & d \end{Vmatrix}", "‖"),
    ] {
        let math = crate::service::parse_latex_to_mathml(src).unwrap();
        assert!(find_node(&math, &|n| n.tag == "mo" && n.text.trim() == delim).is_some());
        assert!(find_node(&roundtrip_mtef(&math), &|n| n.tag == "mtable").is_some());
        assert!(find_node(&roundtrip_omml(&math), &|n| n.tag == "mtable").is_some());
    }
    let mut custom = matrix(4, 4, "right", "brackets");
    fill_slots(&mut custom);
    let custom_math = MathNode::with_children("math", vec![custom]);
    assert_eq!(
        find_node(&roundtrip_omml(&custom_math), &|n| n.tag == "mtable").map(|t| t.children.len()),
        Some(4)
    );

    // 7. Labeled arrows.
    for (src, arrow_glyph) in [
        (r"\overset{a}{\rightarrow}", "→"),
        (r"\underset{b}{\leftarrow}", "←"),
        (r"\overset{a}{\underset{b}{\leftrightarrow}}", "↔"),
        (r"\overset{a}{\rightleftharpoons}", "⇌"),
    ] {
        let math = crate::service::parse_latex_to_mathml(src).unwrap();
        for decoded in [
            crate::service::parse_latex_to_mathml(&math.to_latex()).unwrap(),
            roundtrip_mtef(&math),
            roundtrip_omml(&math),
        ] {
            assert!(
                find_node(&decoded, &|n| n.tag == "mo" && n.text.trim() == arrow_glyph).is_some(),
                "arrow `{arrow_glyph}` lost for {src}: {}",
                decoded.to_xml()
            );
        }
    }

    // 8. Explicit spaces.
    for width in [
        "0.083333em",
        "0.166667em",
        "0.222222em",
        "0.277778em",
        "1em",
        "2em",
    ] {
        let src = format!(r"x\hspace{{{width}}}y");
        let math = crate::service::parse_latex_to_mathml(&src).unwrap();
        assert!(
            find_node(&math, &|n| n.tag == "mspace"
                && n.attr("width") == Some(width))
            .is_some(),
            "{src}"
        );
        assert!(find_node(&roundtrip_mtef(&math), &|n| n.tag == "mspace").is_some());
        assert!(find_node(&roundtrip_omml(&math), &|n| n.tag == "mspace").is_some());
    }

    // 9. Half-open, Dirac, and double-bracket fences.
    for (src, open_ch, close_ch) in [
        (r"\left[ x \right)", Some("["), Some(")")),
        (r"\left( x \right]", Some("("), Some("]")),
        (r"\left| x \right\rangle", Some("|"), Some("⟩")),
        (r"\left\langle x \right|", Some("⟨"), Some("|")),
        (r"\left[ x \right[", Some("["), Some("[")),
        (r"\left] x \right]", Some("]"), Some("]")),
        (r"\left] x \right[", Some("]"), Some("[")),
        (r"\left\llbracket x \right\rrbracket", Some("⟦"), Some("⟧")),
        (r"\left\llbracket x \right.", Some("⟦"), None),
        (r"\left. x \right\rrbracket", None, Some("⟧")),
    ] {
        let math = crate::service::parse_latex_to_mathml(src).unwrap();
        for decoded in [
            crate::service::parse_latex_to_mathml(&math.to_latex()).unwrap(),
            roundtrip_mtef(&math),
            roundtrip_omml(&math),
        ] {
            if let Some(open) = open_ch {
                assert!(
                    find_node(&decoded, &|n| {
                        n.tag == "mo"
                            && (n.text.trim() == open
                                || (open == "⟦" && n.text.trim() == "〚")
                                || (open == "⟨" && n.text.trim() == "〈"))
                    })
                    .is_some(),
                    "open `{open}` lost for {src}: {}",
                    decoded.to_xml()
                );
            }
            if let Some(close) = close_ch {
                assert!(
                    find_node(&decoded, &|n| {
                        n.tag == "mo"
                            && (n.text.trim() == close
                                || (close == "⟧" && n.text.trim() == "〛")
                                || (close == "⟩" && n.text.trim() == "〉"))
                    })
                    .is_some(),
                    "close `{close}` lost for {src}: {}",
                    decoded.to_xml()
                );
            }
        }
    }
}
