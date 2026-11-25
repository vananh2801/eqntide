use super::*;
use crate::mtef::{char_record, TF_NUMBER, TF_SYMBOL, TF_TEXT, TF_VARIABLE};

fn decoded(math: &MathNode) -> MathNode {
    crate::mtef::mtef_to_mathml(&crate::mtef::build_mtef(
        &mathml_to_mtef_bytes(math).unwrap(),
    ))
    .unwrap()
}

#[test]
fn emits_stable_leaf_character_records() {
    let math = MathNode::parse("<math><mi>x</mi><mo>+</mo><mn>2</mn></math>").unwrap();
    assert_eq!(
        mathml_to_mtef_bytes(&math).unwrap(),
        [
            char_record(TF_VARIABLE, 'x' as u32),
            char_record(TF_SYMBOL, '+' as u32),
            char_record(TF_NUMBER, '2' as u32),
        ]
        .concat()
    );
}

#[test]
fn space_only_text_emits_text_space_records() {
    for text in [" ", "\u{00A0}", " \u{00A0} "] {
        let count = text.chars().count();
        let math = MathNode::with_children("math", vec![MathNode::leaf("mtext", text)]);
        assert_eq!(
            mathml_to_mtef_bytes(&math).unwrap(),
            char_record(TF_TEXT, 0x20).repeat(count),
        );
        let decoded_math = decoded(&math);
        let runs = &decoded_math.children[0].children;
        assert_eq!(runs, &vec![MathNode::leaf("mtext", "\u{00A0}"); count]);
    }
}

#[test]
fn palette_spacing_fraction_and_matrix_variants_survive_mtef() {
    for width in ["0.166667em", "0.222222em", "0.277778em", "1em", "2em"] {
        let math = MathNode::parse(&format!(
            "<math><mi>x</mi><mspace width=\"{width}\"/><mi>y</mi></math>"
        ))
        .unwrap();
        let out = decoded(&math);
        let space = out
            .iter()
            .into_iter()
            .find(|node| node.tag == "mspace")
            .expect("explicit space must not disappear");
        assert!(
            (crate::symbols::misc::space_width_em(space.attr("width").unwrap())
                - crate::symbols::misc::space_width_em(width))
            .abs()
                < 0.00001
        );
    }

    let small = decoded(
        &MathNode::parse("<math><mfrac displaystyle=\"false\"><mi>x</mi><mi>y</mi></mfrac></math>")
            .unwrap(),
    );
    assert!(small
        .iter()
        .iter()
        .any(|node| node.tag == "mfrac" && node.attr("displaystyle") == Some("false")));

    let diag = decoded(
        &MathNode::parse("<math><mfrac bevelled=\"true\"><mi>x</mi><mi>y</mi></mfrac></math>")
            .unwrap(),
    );
    assert!(diag
        .iter()
        .iter()
        .any(|node| node.tag == "mfrac" && node.attr("bevelled") == Some("true")));

    let table = decoded(
        &MathNode::parse(
            "<math><mtable columnalign=\"right\"><mtr><mtd><mi>x</mi></mtd><mtd><mi>y</mi></mtd></mtr><mtr><mtd><mn>1</mn></mtd><mtd><mn>2</mn></mtd></mtr></mtable></math>",
        )
        .unwrap(),
    );
    assert!(table
        .iter()
        .iter()
        .any(|node| node.tag == "mtable" && node.attr("columnalign") == Some("right")));
}

#[test]
fn labeled_arrow_directions_and_both_slots_survive_mtef() {
    for arrow in ["→", "←", "↔", "⇄", "⇌", "⇒", "⇐", "⇔", "⇀", "↼", "⇋", "↦"]
    {
        let math = MathNode::parse(&format!(
            "<math><munderover><mo>{arrow}</mo><mi>a</mi><mi>b</mi></munderover></math>"
        ))
        .unwrap();
        let out = decoded(&math);
        let nodes = out.iter();
        assert!(
            nodes.iter().any(|node| node.text == arrow),
            "arrow changed: {arrow}: {out:?}"
        );
        assert!(
            nodes.iter().any(|node| node.text == "a") && nodes.iter().any(|node| node.text == "b"),
            "lost label: {arrow}"
        );
    }
}
