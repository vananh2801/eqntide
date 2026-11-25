use eqntide::MathNode;

use crate::*;

#[test]
fn import_is_lossless_including_unknown_nodes() {
    let math = MathNode::parse("<math display=\"inline\"><mfrac bevelled=\"true\"><mi mathvariant=\"bold\">αβ</mi><unknown foo=\"bar\"><mi>x</mi></unknown></mfrac></math>").unwrap();
    assert_eq!(Editor::new(&math).math(), math);
}

#[test]
fn template_wraps_selection_and_undo_restores_caret() {
    let mut e = Editor::default();
    e.insert_text("x+1");
    e.select_all();
    let before = e.state.clone();
    e.insert_template(&MathNode::with_children(
        "mfrac",
        vec![MathNode::new("mrow"), MathNode::new("mrow")],
    ));
    assert_eq!(e.math().children[0].children[0].children.len(), 3);
    assert!(e.node(e.caret().node).unwrap().children.is_empty());
    e.undo();
    assert_eq!(e.state, before);
    e.redo();
    assert_eq!(e.math().children[0].tag, "mfrac");
}

#[test]
fn unicode_edits_use_char_boundaries() {
    let mut e = Editor::new(&MathNode::with_children(
        "math",
        vec![MathNode::leaf("mtext", "αβγδε ζηθι")],
    ));
    let id = e.root().children[0].id;
    e.set_caret(
        Caret {
            node: id,
            offset: 5,
        },
        false,
    );
    e.insert_text("α");
    e.delete(true);
    assert_eq!(e.math().children[0].text, "αβγδε ζηθι");
}

#[test]
fn insert_structure_splits_token_without_losing_style() {
    let mut m = MathNode::leaf("mi", "ab");
    m.set_attr("mathvariant", "bold");
    let mut e = Editor::new(&MathNode::with_children("math", vec![m]));
    let id = e.root().children[0].id;
    e.set_caret(
        Caret {
            node: id,
            offset: 1,
        },
        false,
    );
    e.insert_template(&MathNode::with_children(
        "msqrt",
        vec![MathNode::new("mrow")],
    ));
    let m = e.math();
    let parts = &m.children[0].children;
    assert_eq!(parts[0].attr("mathvariant"), Some("bold"));
    assert_eq!(parts[2].text, "b");
}

#[test]
fn matrix_grows_and_redo_is_cleared_after_edit() {
    let math = MathNode::parse("<math><mtable><mtr><mtd/></mtr></mtable></math>").unwrap();
    let mut e = Editor::new(&math);
    let id = e.root().children[0].children[0].children[0].id;
    e.set_caret(
        Caret {
            node: id,
            offset: 0,
        },
        false,
    );
    assert!(e.grow_matrix(true));
    assert!(e.grow_matrix(false));
    assert_eq!(e.math().children[0].children.len(), 2);
    e.undo();
    e.insert_text("z");
    let m = e.math();
    e.redo();
    assert_eq!(e.math(), m);
}

#[test]
fn deleting_a_fixed_slot_keeps_valid_arity() {
    let mut e =
        Editor::new(&MathNode::parse("<math><mfrac><mi>a</mi><mi>b</mi></mfrac></math>").unwrap());
    let numerator = e.root().children[0].children[0].id;
    e.select_node(numerator);
    e.delete(true);
    assert_eq!(e.math().children[0].children.len(), 2);
    assert_eq!(e.math().children[0].children[1].text, "b");
}

mod regression_tests {
    use eqntide::MathNode;

    use crate::*;

    #[test]
    fn scripts_take_the_previous_base_and_undo_is_one_transaction() {
        let mut e = Editor::default();
        e.insert_text("x");
        let before = e.math();
        e.insert_template(&MathNode::with_children(
            "msup",
            vec![MathNode::new("mrow"), MathNode::new("mrow")],
        ));
        e.insert_text("2");
        assert_eq!(e.math().children.len(), 1);
        assert_eq!(e.math().children[0].tag, "msup");
        assert_eq!(e.math().children[0].children[0].children[0].text, "x");
        e.undo();
        e.undo();
        assert_eq!(e.math(), before);
    }

    #[test]
    fn operators_pasted_inside_an_identifier_become_operator_nodes() {
        let mut e = Editor::new(&MathNode::with_children(
            "math",
            vec![MathNode::leaf("mi", "xy")],
        ));
        let id = e.root().children[0].id;
        e.set_caret(
            Caret {
                node: id,
                offset: 1,
            },
            false,
        );
        e.insert_text("+2");
        let math = e.math();
        let parts = &math.children[0].children;
        assert_eq!(
            parts.iter().map(|n| n.tag.as_str()).collect::<Vec<_>>(),
            ["mi", "mo", "mn", "mi"]
        );
        e.undo();
        assert_eq!(e.math().children[0].text, "xy");
    }

    #[test]
    fn shrinking_matrix_restores_caret_to_a_surviving_cell() {
        let mut e = Editor::new(
            &MathNode::parse(
                "<math><mtable><mtr><mtd/><mtd/></mtr><mtr><mtd/><mtd/></mtr></mtable></math>",
            )
            .unwrap(),
        );
        let id = e.root().children[0].children[1].children[1].id;
        e.set_caret(
            Caret {
                node: id,
                offset: 0,
            },
            false,
        );
        assert!(e.shrink_matrix(false));
        assert!(e.node(e.caret().node).is_some());
        assert!(e.shrink_matrix(true));
        assert!(!e.shrink_matrix(true));
        assert!(!e.shrink_matrix(false));
        e.undo();
        e.undo();
        assert_eq!(e.math().children[0].children.len(), 2);
    }

    #[test]
    fn untrusted_imports_have_a_depth_limit() {
        let mut math = MathNode::leaf("mi", "x");
        for _ in 0..260 {
            math = MathNode::with_children("mrow", vec![math]);
        }
        assert!(Editor::try_new(&math).is_err());
    }

    #[test]
    fn slash_fraction_takes_previous_expression_into_numerator() {
        let mut e = Editor::default();
        e.insert_text("x");
        e.insert_template_wrapping_previous(&MathNode::with_children(
            "mfrac",
            vec![MathNode::new("mrow"), MathNode::new("mrow")],
        ));
        let math = e.math();
        assert_eq!(math.children.len(), 1);
        assert_eq!(math.children[0].children[0].children[0].text, "x");
        assert!(e.node(e.caret().node).unwrap().children.is_empty());
    }

    #[test]
    fn empty_or_out_of_bounds_selection_never_panics() {
        let mut e = Editor::default();
        let root = e.root().id;
        // Simulating selection on empty root with out-of-bounds offset
        e.state.anchor = Some(Caret {
            node: root,
            offset: 0,
        });
        e.state.caret = Caret {
            node: root,
            offset: 1,
        };
        assert!(e.selected_ids().is_empty());
        assert!(e.selected_math().is_none());
        assert!(e.erase_selection().is_none());
    }

    #[test]
    fn editing_at_fence_boundaries_keeps_delimiters_as_direct_row_siblings() {
        let fence = |left: &str, right: &str| {
            let mut l = MathNode::leaf("mo", left);
            l.set_attr("stretchy", "true");
            l.set_attr("form", "prefix");
            let mut r = MathNode::leaf("mo", right);
            r.set_attr("stretchy", "true");
            r.set_attr("form", "postfix");
            MathNode::with_children(
                "math",
                vec![MathNode::with_children(
                    "mrow",
                    vec![l, MathNode::new("mrow"), r],
                )],
            )
        };

        for (left, right) in [("∥", "∥"), ("(", ")")] {
            let mut e = Editor::new(&fence(left, right));
            let inner_id = e.root().children[0].children[1].id;
            e.set_caret(
                Caret {
                    node: inner_id,
                    offset: 0,
                },
                false,
            );
            e.insert_template(&MathNode::with_children(
                "mfrac",
                vec![MathNode::new("mrow"), MathNode::new("mrow")],
            ));
            e.insert_text("1");
            e.move_caret(Move::NextPlaceholder, false);
            e.insert_text("2");

            // Typing `x` at the left boundary of the closing fence must insert into the fence mrow.
            let right_id = e.root().children[0].children[2].id;
            e.set_caret(
                Caret {
                    node: right_id,
                    offset: 0,
                },
                false,
            );
            e.insert_text("x");
            // Typing `+` at the right boundary of the closing fence must not corrupt the delimiter text.
            e.set_caret(
                Caret {
                    node: right_id,
                    offset: 1,
                },
                false,
            );
            e.insert_text("+");
            // Typing `a` at the left boundary of the opening fence must stay in the same row.
            let left_id = e.root().children[0].children[0].id;
            e.set_caret(
                Caret {
                    node: left_id,
                    offset: 0,
                },
                false,
            );
            e.insert_text("a");

            let math = e.math();
            let row = &math.children[0].children;
            assert_eq!(row[0].tag, "mi");
            assert_eq!(row[0].text, "a");
            assert_eq!(row[1].tag, "mo");
            assert_eq!(row[1].text, left);
            assert_eq!(row[1].attr("form"), Some("prefix"));
            assert_eq!(row[2].tag, "mrow");
            assert_eq!(row[3].tag, "mi");
            assert_eq!(row[3].text, "x");
            assert_eq!(row[4].tag, "mo");
            assert_eq!(row[4].text, right);
            assert_eq!(row[4].attr("form"), Some("postfix"));
            assert_eq!(row[5].tag, "mo");
            assert_eq!(row[5].text, "+");
        }
    }

    #[test]
    fn wrapping_scripts_or_fractions_at_fence_boundaries_does_not_steal_delimiter() {
        let mut l = MathNode::leaf("mo", "∥");
        l.set_attr("stretchy", "true");
        l.set_attr("form", "prefix");
        let mut r = MathNode::leaf("mo", "∥");
        r.set_attr("stretchy", "true");
        r.set_attr("form", "postfix");
        let mut e = Editor::new(&MathNode::with_children(
            "math",
            vec![MathNode::with_children(
                "mrow",
                vec![l, MathNode::leaf("mi", "x"), r],
            )],
        ));
        let row_id = e.root().children[0].id;
        let right_id = e.root().children[0].children[2].id;
        let msup =
            MathNode::with_children("msup", vec![MathNode::new("mrow"), MathNode::new("mrow")]);
        let mfrac =
            MathNode::with_children("mfrac", vec![MathNode::new("mrow"), MathNode::new("mrow")]);

        // Caret on closing delimiter offset 1: pressing `^` must not steal `∥` into the base.
        e.set_caret(
            Caret {
                node: right_id,
                offset: 1,
            },
            false,
        );
        e.insert_template(&msup);
        // Caret right after opening delimiter in parent row: `/` must not steal `∥` into numerator.
        e.set_caret(
            Caret {
                node: row_id,
                offset: 1,
            },
            false,
        );
        e.insert_template_wrapping_previous(&mfrac);

        let math = e.math();
        let row = &math.children[0].children;
        assert_eq!(row[0].tag, "mo");
        assert_eq!(row[0].text, "∥");
        assert_eq!(row[0].attr("form"), Some("prefix"));
        assert_eq!(row[1].tag, "mfrac");
        assert!(row[1].children[0].children.is_empty());
        assert_eq!(row[2].tag, "mi");
        assert_eq!(row[2].text, "x");
        assert_eq!(row[3].tag, "mo");
        assert_eq!(row[3].text, "∥");
        assert_eq!(row[3].attr("form"), Some("postfix"));
        assert_eq!(row[4].tag, "msup");
        assert!(row[4].children[0].children.is_empty());
    }

    #[test]
    fn insert_row_or_promote_fence_adds_row_below_caret_and_undoes_in_one_step() {
        let mut e = Editor::new(
            &MathNode::parse(
                "<math><mtable columnalign=\"left\"><mtr><mtd><mi>a</mi></mtd></mtr><mtr><mtd><mi>b</mi></mtd></mtr></mtable></math>",
            )
            .unwrap(),
        );
        // Place caret inside row 0 cell (`a`).
        let row0_mi = e.root().children[0].children[0].children[0].children[0].id;
        e.set_caret(
            Caret {
                node: row0_mi,
                offset: 1,
            },
            false,
        );
        assert!(e.insert_row_or_promote_fence());

        let math = e.math();
        assert_eq!(math.children[0].children.len(), 3);
        assert_eq!(
            math.children[0].children[0].children[0].children[0].text,
            "a"
        );
        assert!(math.children[0].children[1].children[0].children.is_empty());
        assert_eq!(
            math.children[0].children[2].children[0].children[0].text,
            "b"
        );
        assert_eq!(
            e.caret().node,
            e.root().children[0].children[1].children[0].id
        );

        e.undo();
        assert_eq!(e.math().children[0].children.len(), 2);
        e.redo();
        assert_eq!(e.math().children[0].children.len(), 3);

        // Multi-column table and nested fraction inside a cell preserve column count.
        let mut multi = Editor::new(
            &MathNode::parse(
                "<math><mtable><mtr><mtd><mfrac><mi>x</mi><mi>y</mi></mfrac></mtd><mtd><mi>z</mi></mtd></mtr></mtable></math>",
            )
            .unwrap(),
        );
        let frac_num = multi.root().children[0].children[0].children[0].children[0].children[0].id;
        multi.set_caret(
            Caret {
                node: frac_num,
                offset: 0,
            },
            false,
        );
        assert!(multi.insert_row_or_promote_fence());
        assert_eq!(multi.math().children[0].children.len(), 2);
        assert_eq!(multi.math().children[0].children[1].children.len(), 2);

        // Outside a table or left fence: no-op.
        let mut plain = Editor::default();
        plain.insert_text("x");
        assert!(!plain.insert_row_or_promote_fence());
    }

    #[test]
    fn insert_row_or_promote_fence_promotes_single_sided_brace_and_bracket_only() {
        for open in ["{", "["] {
            let mut l = MathNode::leaf("mo", open);
            l.set_attr("stretchy", "true");
            l.set_attr("form", "prefix");
            let mut r = MathNode::leaf("mo", "");
            r.set_attr("stretchy", "true");
            r.set_attr("form", "postfix");
            let tpl = MathNode::with_children("mrow", vec![l, MathNode::new("mrow"), r]);

            // Non-empty single-slot left fence promotes to 2x1 left-aligned table.
            let mut e = Editor::default();
            e.insert_template(&tpl);
            e.insert_text("x+2=1");
            assert!(e.insert_row_or_promote_fence());
            let math = e.math();
            let fence_row = &math.children[0].children;
            assert_eq!(fence_row[0].text, open);
            assert_eq!(fence_row[1].tag, "mtable");
            assert_eq!(fence_row[1].attr("columnalign"), Some("left"));
            assert_eq!(fence_row[1].children.len(), 2);
            assert_eq!(fence_row[1].children[0].children[0].children.len(), 5);
            assert!(fence_row[1].children[1].children[0].children.is_empty());
            assert_eq!(fence_row[2].text, "");
            assert_eq!(
                e.caret().node,
                e.root().children[0].children[1].children[1].children[0].id
            );

            // Undo restores the single-slot fenced expression in one step.
            e.undo();
            assert_eq!(e.math().children[0].children[1].tag, "mrow");

            // Empty slot also promotes cleanly.
            let mut empty = Editor::default();
            empty.insert_template(&tpl);
            assert!(empty.insert_row_or_promote_fence());
            assert_eq!(empty.math().children[0].children[1].tag, "mtable");
            assert_eq!(empty.math().children[0].children[1].children.len(), 2);
        }

        // Paired fences `( x )` or `{ x }` and one-sided `(` must not promote.
        for (open, close) in [("{", "}"), ("[", "]"), ("(", "")] {
            let mut l = MathNode::leaf("mo", open);
            l.set_attr("stretchy", "true");
            l.set_attr("form", "prefix");
            let mut r = MathNode::leaf("mo", close);
            r.set_attr("stretchy", "true");
            r.set_attr("form", "postfix");
            let mut e = Editor::default();
            e.insert_template(&MathNode::with_children(
                "mrow",
                vec![l, MathNode::new("mrow"), r],
            ));
            e.insert_text("x");
            assert!(!e.insert_row_or_promote_fence());
        }
    }

    #[test]
    fn backspace_and_delete_remove_empty_table_rows_and_guard_nonempty_or_single_row() {
        // 3-row system: row 0 = `a`, row 1 = empty, row 2 = `b`.
        let mut e = Editor::new(
            &MathNode::parse(
                "<math><mtable columnalign=\"left\"><mtr><mtd><mi>a</mi></mtd></mtr><mtr><mtd/></mtr><mtr><mtd><mi>b</mi></mtd></mtr></mtable></math>",
            )
            .unwrap(),
        );
        // Backspace in middle empty row removes row 1 and moves caret to end of row 0 (`a|`).
        let row1_cell = e.root().children[0].children[1].children[0].id;
        e.set_caret(
            Caret {
                node: row1_cell,
                offset: 0,
            },
            false,
        );
        e.delete(true);
        assert_eq!(e.math().children[0].children.len(), 2);
        assert_eq!(
            e.math().children[0].children[0].children[0].children[0].text,
            "a"
        );
        assert_eq!(
            e.math().children[0].children[1].children[0].children[0].text,
            "b"
        );
        let row0_mi = e.root().children[0].children[0].children[0].children[0].id;
        assert_eq!(
            e.caret(),
            Caret {
                node: row0_mi,
                offset: 1
            }
        );

        // Undo restores row 1 in one step; Delete removes row 1 and moves caret to start of row 2 (`|b`).
        e.undo();
        assert_eq!(e.math().children[0].children.len(), 3);
        e.delete(false);
        assert_eq!(e.math().children[0].children.len(), 2);
        let row1_mi_b = e.root().children[0].children[1].children[0].children[0].id;
        assert_eq!(
            e.caret(),
            Caret {
                node: row1_mi_b,
                offset: 0
            }
        );
        e.redo();
        assert_eq!(e.math().children[0].children.len(), 2);

        // Removing the first row when empty moves caret to start of new first row;
        // removing the last row when empty moves caret to end of previous row.
        let mut edges = Editor::new(
            &MathNode::parse(
                "<math><mtable><mtr><mtd/></mtr><mtr><mtd><mi>m</mi></mtd></mtr><mtr><mtd/></mtr></mtable></math>",
            )
            .unwrap(),
        );
        let first_cell = edges.root().children[0].children[0].children[0].id;
        edges.set_caret(
            Caret {
                node: first_cell,
                offset: 0,
            },
            false,
        );
        edges.delete(true);
        assert_eq!(edges.math().children[0].children.len(), 2);
        let new_first_mi = edges.root().children[0].children[0].children[0].children[0].id;
        assert_eq!(
            edges.caret(),
            Caret {
                node: new_first_mi,
                offset: 0
            }
        );
        let last_cell = edges.root().children[0].children[1].children[0].id;
        edges.set_caret(
            Caret {
                node: last_cell,
                offset: 0,
            },
            false,
        );
        edges.delete(false);
        assert_eq!(edges.math().children[0].children.len(), 1);
        assert_eq!(
            edges.caret(),
            Caret {
                node: new_first_mi,
                offset: 1
            }
        );

        // Single-row non-empty table: Backspace at start of cell moves out without removing row;
        // single-row empty table uses A2 (selects table first, then deletes on second press).
        let only_cell = edges.root().children[0].children[0].children[0].id;
        edges.set_caret(
            Caret {
                node: only_cell,
                offset: 0,
            },
            false,
        );
        edges.delete(true);
        assert_eq!(edges.math().children[0].children.len(), 1);

        let mut single_empty = Editor::new(
            &MathNode::parse("<math><mtable><mtr><mtd/></mtr></mtable></math>").unwrap(),
        );
        let table_id = single_empty.root().children[0].id;
        let single_cell = single_empty.root().children[0].children[0].children[0].id;
        single_empty.set_caret(
            Caret {
                node: single_cell,
                offset: 0,
            },
            false,
        );
        single_empty.delete(true);
        assert_eq!(single_empty.selected_ids(), vec![table_id]);
        assert_eq!(single_empty.math().children[0].children.len(), 1);
        single_empty.delete(true);
        assert!(single_empty.math().children.is_empty());

        // Multi-column row where another cell has content: first Backspace/Delete selects the row
        // with caret at row end; second press removes the row.
        let mut multi = Editor::new(
            &MathNode::parse(
                "<math><mtable><mtr><mtd><mi>x</mi></mtd><mtd><mi>y</mi></mtd></mtr><mtr><mtd/><mtd><mi>z</mi></mtd></mtr></mtable></math>",
            )
            .unwrap(),
        );
        let empty_left = multi.root().children[0].children[1].children[0].id;
        let right_cell = multi.root().children[0].children[1].children[1].id;
        let right_mi = multi.root().children[0].children[1].children[1].children[0].id;
        multi.set_caret(
            Caret {
                node: empty_left,
                offset: 0,
            },
            false,
        );
        multi.delete(true);
        assert_eq!(multi.math().children[0].children.len(), 2);
        assert_eq!(multi.selected_ids(), vec![empty_left, right_cell]);
        assert_eq!(
            multi.caret(),
            Caret {
                node: right_mi,
                offset: 1
            }
        );
        multi.delete(false);
        assert_eq!(multi.math().children[0].children.len(), 1);
    }

    #[test]
    fn horizontal_arrow_stops_skip_redundant_wrappers_and_boundaries() {
        // Flat expression `x+1` has 4 stops (1 step per character/operator).
        let mut flat = Editor::new(&MathNode::with_children(
            "math",
            vec![
                MathNode::leaf("mi", "x"),
                MathNode::leaf("mo", "+"),
                MathNode::leaf("mn", "1"),
            ],
        ));
        let mut stops = Vec::new();
        flat.root().stops(&mut stops);
        assert_eq!(stops.len(), 4);
        // Even starting from non-canonical (math, 0), Right moves in 1 step past 'x'.
        flat.move_caret(Move::Right, false);
        assert_eq!(
            flat.caret(),
            Caret {
                node: flat.root().children[0].id,
                offset: 1
            }
        );
        flat.move_caret(Move::Right, false);
        assert_eq!(
            flat.caret(),
            Caret {
                node: flat.root().children[1].id,
                offset: 1
            }
        );
        flat.move_caret(Move::Right, false);
        assert_eq!(
            flat.caret(),
            Caret {
                node: flat.root().children[2].id,
                offset: 1
            }
        );

        // Nested mrow wrappers around `x` still produce only 2 stops.
        let nested = Editor::new(
            &MathNode::parse("<math><mrow><mrow><mi>x</mi></mrow></mrow></math>").unwrap(),
        );
        nested.root().stops(&mut stops);
        assert_eq!(stops.len(), 2);

        // Fraction `\frac{1}{2}` has 6 stops: before frac, |1, 1|, |2, 2|, after frac.
        let mut frac = Editor::new(
            &MathNode::parse(
                "<math><mfrac><mrow><mn>1</mn></mrow><mrow><mn>2</mn></mrow></mfrac></math>",
            )
            .unwrap(),
        );
        frac.root().stops(&mut stops);
        assert_eq!(stops.len(), 6);
        // From end of numerator `1|`, one Right enters start of denominator `|2`.
        let num_mn = frac.root().children[0].children[0].children[0].id;
        let den_mn = frac.root().children[0].children[1].children[0].id;
        frac.set_caret(
            Caret {
                node: num_mn,
                offset: 1,
            },
            false,
        );
        frac.move_caret(Move::Right, false);
        assert_eq!(
            frac.caret(),
            Caret {
                node: den_mn,
                offset: 0
            }
        );
        // From end of denominator `2|`, one Right exits fraction.
        frac.move_caret(Move::Right, false);
        frac.move_caret(Move::Right, false);
        assert_eq!(
            frac.caret(),
            Caret {
                node: frac.root().id,
                offset: 1
            }
        );

        // Fenced `(x)` has 4 stops: |(x), (|x), (x|), (x)|.
        let mut l = MathNode::leaf("mo", "(");
        l.set_attr("stretchy", "true");
        l.set_attr("form", "prefix");
        let mut r = MathNode::leaf("mo", ")");
        r.set_attr("stretchy", "true");
        r.set_attr("form", "postfix");
        let mut fence = Editor::new(&MathNode::with_children(
            "math",
            vec![MathNode::with_children(
                "mrow",
                vec![
                    l,
                    MathNode::with_children("mrow", vec![MathNode::leaf("mi", "x")]),
                    r,
                ],
            )],
        ));
        fence.root().stops(&mut stops);
        assert_eq!(stops.len(), 4);
        let x_id = fence.root().children[0].children[1].children[0].id;
        fence.set_caret(
            Caret {
                node: x_id,
                offset: 1,
            },
            false,
        );
        fence.move_caret(Move::Right, false);
        assert_eq!(
            fence.caret(),
            Caret {
                node: fence.root().id,
                offset: 1
            }
        );
    }

    #[test]
    fn deleting_last_token_char_prunes_empty_token_and_forward_delete_crosses_tokens() {
        let mut e = Editor::new(
            &MathNode::parse(
                "<math><mfrac><mrow><mn>1</mn></mrow><mrow><mn>2</mn></mrow></mfrac></math>",
            )
            .unwrap(),
        );
        let den_mrow = e.root().children[0].children[1].id;
        let den_mn = e.root().children[0].children[1].children[0].id;
        e.set_caret(
            Caret {
                node: den_mn,
                offset: 1,
            },
            false,
        );
        e.delete(true);
        assert!(e.node(den_mrow).unwrap().children.is_empty());
        assert_eq!(
            e.caret(),
            Caret {
                node: den_mrow,
                offset: 0
            }
        );

        // Multi-char token followed by another token: Backspace then Backspace deletes char-by-char.
        let mut seq = Editor::new(&MathNode::with_children(
            "math",
            vec![
                MathNode::leaf("mi", "ab"),
                MathNode::leaf("mo", "+"),
                MathNode::leaf("mn", "1"),
            ],
        ));
        let one_id = seq.root().children[2].id;
        seq.set_caret(
            Caret {
                node: one_id,
                offset: 1,
            },
            false,
        );
        seq.delete(true);
        assert_eq!(seq.math().children.len(), 2);
        seq.delete(true);
        assert_eq!(seq.math().children.len(), 1);
        seq.delete(true);
        assert_eq!(seq.math().children[0].text, "a");

        // Forward Delete at `a|+` deletes `+` in one step.
        let mut fwd = Editor::new(&MathNode::with_children(
            "math",
            vec![
                MathNode::leaf("mi", "a"),
                MathNode::leaf("mo", "+"),
                MathNode::leaf("mn", "1"),
            ],
        ));
        let a_id = fwd.root().children[0].id;
        fwd.set_caret(
            Caret {
                node: a_id,
                offset: 1,
            },
            false,
        );
        fwd.delete(false);
        assert_eq!(fwd.math().children.len(), 2);
        assert_eq!(fwd.math().children[1].text, "1");
    }

    #[test]
    fn empty_slot_backspace_and_delete_use_a2_selection_and_nonempty_boundaries_step() {
        // Empty denominator in `\frac{1}{□}`: 1st Backspace selects fraction, 2nd deletes it.
        let mut frac = Editor::new(
            &MathNode::parse("<math><mfrac><mrow><mn>1</mn></mrow><mrow/></mfrac></math>").unwrap(),
        );
        let frac_id = frac.root().children[0].id;
        let den_id = frac.root().children[0].children[1].id;
        frac.set_caret(
            Caret {
                node: den_id,
                offset: 0,
            },
            false,
        );
        frac.delete(true);
        assert_eq!(frac.selected_ids(), vec![frac_id]);
        assert_eq!(frac.math().children.len(), 1);
        frac.delete(true);
        assert!(frac.math().children.is_empty());
        frac.undo();
        assert_eq!(frac.math().children.len(), 1);

        // Nested `1 + \frac{2}{□}` selects only inner fraction, not outer expression.
        let mut nested = Editor::new(
            &MathNode::parse(
                "<math><mn>1</mn><mo>+</mo><mfrac><mrow><mn>2</mn></mrow><mrow/></mfrac></math>",
            )
            .unwrap(),
        );
        let inner_frac_id = nested.root().children[2].id;
        let inner_den_id = nested.root().children[2].children[1].id;
        nested.set_caret(
            Caret {
                node: inner_den_id,
                offset: 0,
            },
            false,
        );
        nested.delete(false);
        assert_eq!(nested.selected_ids(), vec![inner_frac_id]);
        nested.delete(false);
        assert_eq!(nested.math().children.len(), 2);

        // Non-empty `\frac{1}{|2}` + Backspace moves to `1|` without selecting/deleting.
        let mut nonempty = Editor::new(
            &MathNode::parse(
                "<math><mfrac><mrow><mn>1</mn></mrow><mrow><mn>2</mn></mrow></mfrac></math>",
            )
            .unwrap(),
        );
        let num_mn = nonempty.root().children[0].children[0].children[0].id;
        let den_mn = nonempty.root().children[0].children[1].children[0].id;
        nonempty.set_caret(
            Caret {
                node: den_mn,
                offset: 0,
            },
            false,
        );
        nonempty.delete(true);
        assert!(nonempty.selected_ids().is_empty());
        assert_eq!(
            nonempty.caret(),
            Caret {
                node: num_mn,
                offset: 1
            }
        );
        // Forward Delete from `1|` moves to `|2`.
        nonempty.delete(false);
        assert!(nonempty.selected_ids().is_empty());
        assert_eq!(
            nonempty.caret(),
            Caret {
                node: den_mn,
                offset: 0
            }
        );

        // Empty fenced `(□)` and fenced 1-row empty `{□` select the enclosing fence first.
        let mut l = MathNode::leaf("mo", "(");
        l.set_attr("stretchy", "true");
        l.set_attr("form", "prefix");
        let mut r = MathNode::leaf("mo", ")");
        r.set_attr("stretchy", "true");
        r.set_attr("form", "postfix");
        let mut fence = Editor::new(&MathNode::with_children(
            "math",
            vec![MathNode::with_children(
                "mrow",
                vec![l, MathNode::new("mrow"), r],
            )],
        ));
        let fence_id = fence.root().children[0].id;
        let inner_id = fence.root().children[0].children[1].id;
        fence.set_caret(
            Caret {
                node: inner_id,
                offset: 0,
            },
            false,
        );
        fence.delete(true);
        assert_eq!(fence.selected_ids(), vec![fence_id]);
        fence.delete(true);
        assert!(fence.math().children.is_empty());
    }

    #[test]
    fn erasing_selected_table_row_removes_row_and_partial_selection_preserves_mtd() {
        // Drag-selecting an empty row across 2 cells in a 2x2 matrix and pressing Backspace removes the row.
        let mut mat = Editor::new(
            &MathNode::parse(
                "<math><mtable><mtr><mtd><mi>a</mi></mtd><mtd><mi>b</mi></mtd></mtr><mtr><mtd/><mtd/></mtr></mtable></math>",
            )
            .unwrap(),
        );
        let r1_c0 = mat.root().children[0].children[1].children[0].id;
        let r1_c1 = mat.root().children[0].children[1].children[1].id;
        mat.set_caret(
            Caret {
                node: r1_c0,
                offset: 0,
            },
            false,
        );
        mat.set_caret(
            Caret {
                node: r1_c1,
                offset: 0,
            },
            true,
        );
        mat.delete(true);
        let math = mat.math();
        assert_eq!(math.children[0].children.len(), 1);
        assert_eq!(math.children[0].children[0].tag, "mtr");
        assert_eq!(math.to_latex(), r"\begin{matrix} a & b \end{matrix}");

        // Partial selection inside a row preserves `mtd` tags (never converts to `mrow`).
        let mut partial = Editor::new(
            &MathNode::parse("<math><mtable><mtr><mtd><mi>a</mi></mtd><mtd><mi>b</mi></mtd></mtr></mtable></math>")
                .unwrap(),
        );
        let c0 = partial.root().children[0].children[0].children[0].id;
        partial.select_node(c0);
        partial.delete(true);
        let cell0 = &partial.math().children[0].children[0].children[0];
        assert_eq!(cell0.tag, "mtd");
        assert!(cell0.children.is_empty());
    }

    #[test]
    fn selection_span_respects_leading_and_trailing_token_boundaries() {
        // Dragging from `|x^2` across `-` into the left edge `|2` must exclude `2` in both directions.
        let mut eq = Editor::new(
            &MathNode::parse(
                "<math><mfrac><mn>1</mn><mn>2</mn></mfrac><msup><mi>x</mi><mn>2</mn></msup><mo>-</mo><mn>2</mn><mi>x</mi><mo>+</mo><mn>1</mn><mo>=</mo><mn>0</mn></math>",
            )
            .unwrap(),
        );
        let root_id = eq.root().id;
        let frac_id = eq.root().children[0].id;
        let msup_id = eq.root().children[1].id;
        let base_x_id = eq.root().children[1].children[0].id;
        let minus_id = eq.root().children[2].id;
        let two_id = eq.root().children[3].id;

        // Forward: `|x^2` -> `|2` selects `x^2 -` only.
        eq.set_caret(
            Caret {
                node: base_x_id,
                offset: 0,
            },
            false,
        );
        eq.set_caret(
            Caret {
                node: two_id,
                offset: 0,
            },
            true,
        );
        assert_eq!(eq.selection_span(), Some((root_id, 1, 3)));
        assert_eq!(eq.selected_ids(), vec![msup_id, minus_id]);

        // Backward: `|2` -> `|x^2` selects `x^2 -` only.
        eq.set_caret(
            Caret {
                node: two_id,
                offset: 0,
            },
            false,
        );
        eq.set_caret(
            Caret {
                node: base_x_id,
                offset: 0,
            },
            true,
        );
        assert_eq!(eq.selection_span(), Some((root_id, 1, 3)));
        assert_eq!(eq.selected_ids(), vec![msup_id, minus_id]);

        // Dragging left from `|x^2` into `\frac{1}{2}` excludes `x^2`.
        let num_one_id = eq.root().children[0].children[0].id;
        eq.set_caret(
            Caret {
                node: base_x_id,
                offset: 0,
            },
            false,
        );
        eq.set_caret(
            Caret {
                node: num_one_id,
                offset: 0,
            },
            true,
        );
        assert_eq!(eq.selection_span(), Some((root_id, 0, 1)));
        assert_eq!(eq.selected_ids(), vec![frac_id]);

        // Dragging from trailing edge of `x` (`x|-2`) to leading edge of `2` (`x-|2`) selects `-` only,
        // and shared boundaries return `None`.
        let mut flat = Editor::new(
            &MathNode::parse("<math><mi>x</mi><mo>-</mo><mrow><mn>2</mn></mrow></math>").unwrap(),
        );
        let x_id = flat.root().children[0].id;
        let flat_minus_id = flat.root().children[1].id;
        let wrapped_two_id = flat.root().children[2].children[0].id;
        flat.set_caret(
            Caret {
                node: x_id,
                offset: 1,
            },
            false,
        );
        flat.set_caret(
            Caret {
                node: wrapped_two_id,
                offset: 0,
            },
            true,
        );
        assert_eq!(flat.selected_ids(), vec![flat_minus_id]);
        flat.set_caret(
            Caret {
                node: flat_minus_id,
                offset: 0,
            },
            true,
        );
        assert_eq!(flat.selection_span(), None);

        // `Shift+Arrow` between adjacent tokens selects only the crossed operator.
        flat.set_caret(
            Caret {
                node: x_id,
                offset: 1,
            },
            false,
        );
        flat.move_caret(Move::Right, true);
        assert_eq!(flat.selected_ids(), vec![flat_minus_id]);

        // Empty placeholders are still covered even at offset 0.
        let mut with_empty =
            Editor::new(&MathNode::parse("<math><mrow/><mi>y</mi></math>").unwrap());
        let empty_id = with_empty.root().children[0].id;
        let y_id = with_empty.root().children[1].id;
        with_empty.set_caret(
            Caret {
                node: empty_id,
                offset: 0,
            },
            false,
        );
        with_empty.set_caret(
            Caret {
                node: y_id,
                offset: 0,
            },
            true,
        );
        assert_eq!(with_empty.selected_ids(), vec![empty_id]);
    }

    #[test]
    fn invalid_or_stale_caret_and_boundary_ops_never_panic() {
        let mut editor = Editor::new(&MathNode::new("math"));
        // Stale / nonexistent node IDs in caret must not panic on edits, deletes, or matrix ops.
        editor.set_caret(
            Caret {
                node: 999_999,
                offset: 42,
            },
            false,
        );
        editor.delete(true);
        editor.delete(false);
        editor.insert_text("x");
        assert!(!editor.grow_matrix(false));
        assert!(!editor.shrink_matrix(true));
        assert!(!editor.insert_row_or_promote_fence());
        editor.move_caret(Move::Left, false);
        editor.move_caret(Move::Right, true);
    }
}
