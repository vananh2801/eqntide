//! Template slot grouping, sub/superscript base relocation, and AST-to-MathML conversion.

use crate::mathml::MathNode;

use super::super::typeface::{TF_FUNCTION, TF_SPACE};
use super::characters::{convert_char_to_mathnode, mover_stretchy_accent, munder_stretchy_accent};
use super::embellishment::menclose;
use super::parse::is_math_obj;
use super::MtefObj;

#[derive(Debug, Clone)]
struct ParsedSlot {
    is_empty_option: bool,
    is_pile: bool,
    pile_halign: &'static str,
    pile_lines: Vec<Vec<MtefObj>>,
    objects: Vec<MtefObj>,
}

/// Relocate adjacent base objects into MTEF `tmSUB` / `tmSUP` / `tmSUBSUP` templates
/// recursively across `objects` (and strip MTEF zero-width space `0xEF01` markers).
pub(super) fn apply_mover_to_list(objects: &mut Vec<MtefObj>) {
    // Strip 0xEF01 (MT Zero Space) and recurse into children first.
    objects.retain(|obj| {
        !matches!(
            obj,
            MtefObj::Char {
                mt_code: 0xEF01,
                ..
            }
        )
    });

    for obj in objects.iter_mut() {
        match obj {
            MtefObj::Tmpl { subobjects, .. } => apply_mover_to_list(subobjects),
            MtefObj::Line { objects: inner, .. } => apply_mover_to_list(inner),
            MtefObj::Pile { lines, .. } => apply_mover_to_list(lines),
            MtefObj::Matrix { cells, .. } => apply_mover_to_list(cells),
            _ => {}
        }
    }

    // Pass 1: tmSUP / tmSUB / tmSUBSUP without tvSU_PRECEDES take their preceding base.
    let mut idx = 0;
    while idx < objects.len() {
        let is_following_subsup = matches!(
            &objects[idx],
            MtefObj::Tmpl {
                selector: "tmSUP" | "tmSUB" | "tmSUBSUP",
                variations,
                ..
            } if !variations.contains(&"tvSU_PRECEDES")
        );
        if !is_following_subsup {
            idx += 1;
            continue;
        }

        let prev_cand = (0..idx).rev().find(|&pos| is_suitable_base(&objects[pos]));
        if let Some(base_idx) = prev_cand {
            let close_code = match &objects[base_idx] {
                MtefObj::Char { mt_code, .. } => close_open_pair(*mt_code),
                _ => None,
            };
            let base_objs: Vec<MtefObj> = if let Some(open_code) = close_code {
                let open_pos = (0..base_idx).rev().find(|&pos| {
                    matches!(&objects[pos], MtefObj::Char { mt_code, .. } if *mt_code == open_code)
                });
                if let Some(start) = open_pos {
                    let drained: Vec<MtefObj> = objects.drain(start..=base_idx).collect();
                    idx -= drained.len();
                    drained
                } else {
                    let base = objects.remove(base_idx);
                    idx -= 1;
                    vec![base]
                }
            } else if function_char(&objects[base_idx]).is_some() {
                let mut start = base_idx;
                while start > 0 && function_char(&objects[start - 1]).is_some() {
                    start -= 1;
                }
                let drained: Vec<MtefObj> = objects.drain(start..=base_idx).collect();
                idx -= drained.len();
                drained
            } else {
                let base = objects.remove(base_idx);
                idx -= 1;
                vec![base]
            };

            if let MtefObj::Tmpl { subobjects, .. } = &mut objects[idx] {
                subobjects.insert(
                    0,
                    MtefObj::Line {
                        is_null: false,
                        objects: base_objs,
                    },
                );
            }
            idx += 1;
        } else {
            // No preceding candidate: mark tvSU_PRECEDES and consume the following sibling.
            let next_cand =
                ((idx + 1)..objects.len()).find(|&pos| is_suitable_prescript_base(&objects[pos]));
            if let MtefObj::Tmpl { variations, .. } = &mut objects[idx] {
                if !variations.contains(&"tvSU_PRECEDES") {
                    variations.push("tvSU_PRECEDES");
                }
            }
            if let Some(following_idx) = next_cand {
                let base = objects.remove(following_idx);
                if let MtefObj::Tmpl { subobjects, .. } = &mut objects[idx] {
                    subobjects.insert(
                        0,
                        MtefObj::Line {
                            is_null: false,
                            objects: vec![base],
                        },
                    );
                }
            }
            idx += 1;
        }
    }

    // Pass 2: tmSUP / tmSUB / tmSUBSUP with tvSU_PRECEDES that still lack a base slot.
    let mut idx = 0;
    while idx < objects.len() {
        let needs_precedes_base = match &objects[idx] {
            MtefObj::Tmpl {
                selector: "tmSUP" | "tmSUB" | "tmSUBSUP",
                variations,
                subobjects,
            } if variations.contains(&"tvSU_PRECEDES") => !matches!(
                subobjects.first(),
                Some(MtefObj::Line {
                    is_null: false,
                    objects: base_objs
                }) if !base_objs.is_empty()
            ),
            _ => false,
        };
        if !needs_precedes_base {
            idx += 1;
            continue;
        }

        let next_cand =
            ((idx + 1)..objects.len()).find(|&pos| is_suitable_prescript_base(&objects[pos]));
        if let Some(following_idx) = next_cand {
            let open_code = match &objects[following_idx] {
                MtefObj::Char { mt_code, .. } => open_close_pair(*mt_code),
                _ => None,
            };
            let base_objs: Vec<MtefObj> = if let Some(close_code) = open_code {
                let close_pos = (following_idx + 1..objects.len()).find(|&pos| {
                    matches!(&objects[pos], MtefObj::Char { mt_code, .. } if *mt_code == close_code)
                });
                if let Some(end) = close_pos {
                    objects.drain(following_idx..=end).collect()
                } else {
                    vec![objects.remove(following_idx)]
                }
            } else {
                vec![objects.remove(following_idx)]
            };
            if let MtefObj::Tmpl { subobjects, .. } = &mut objects[idx] {
                subobjects.insert(
                    0,
                    MtefObj::Line {
                        is_null: false,
                        objects: base_objs,
                    },
                );
            }
        } else if let MtefObj::Tmpl { subobjects, .. } = &mut objects[idx] {
            subobjects.insert(
                0,
                MtefObj::Line {
                    is_null: true,
                    objects: Vec::new(),
                },
            );
        }
        idx += 1;
    }
}

/// Convert a slice of sibling `MtefObj` records into MathML nodes, coalescing
/// runs of `TF_FUNCTION` letters into named operator identifiers.
pub(super) fn convert_objs(objs: &[MtefObj]) -> Vec<MathNode> {
    let mut nodes = Vec::new();
    let mut idx = 0;
    while idx < objs.len() {
        if function_char(&objs[idx]).is_some() {
            let mut end = idx;
            let mut word = String::new();
            while end < objs.len() {
                if let Some(ch) = function_char(&objs[end]) {
                    word.push(ch);
                    end += 1;
                } else {
                    break;
                }
            }
            if word == "lim"
                && end + 3 < objs.len()
                && matches!(
                    &objs[end],
                    MtefObj::Char {
                        mt_code: 0xEB01 | 0x0020,
                        is_textmode: false,
                        embells,
                        ..
                    } if embells.is_empty()
                )
            {
                let next_word: String = objs[end + 1..objs.len().min(end + 4)]
                    .iter()
                    .filter_map(function_char)
                    .collect();
                if matches!(next_word.as_str(), "sup" | "inf")
                    && objs.get(end + 4).and_then(function_char).is_none()
                {
                    nodes.push(normal_mi(&format!("lim {next_word}")));
                    idx = end + 4;
                    continue;
                }
            }
            if word.len() >= 2 {
                let mut pos = 0;
                while pos < word.len() {
                    let matched_end = (pos + 2..=word.len())
                        .rev()
                        .find(|&cand| crate::symbols::is_function(&word[pos..cand]));
                    if let Some(next_pos) = matched_end {
                        nodes.push(normal_mi(&word[pos..next_pos]));
                        pos = next_pos;
                    } else if pos == 0 {
                        nodes.push(normal_mi(&word));
                        break;
                    } else {
                        nodes.push(normal_mi(&word[pos..]));
                        break;
                    }
                }
                idx = end;
                continue;
            }
        }
        if let Some(node) = convert_obj(&objs[idx]) {
            nodes.push(node);
        }
        idx += 1;
    }
    nodes
}

fn convert_obj(obj: &MtefObj) -> Option<MathNode> {
    match obj {
        MtefObj::Char {
            typeface,
            mt_code,
            is_textmode,
            embells,
        } => convert_char_to_mathnode(*typeface, *mt_code, *is_textmode, embells),
        MtefObj::Line { objects, .. } => {
            Some(MathNode::with_children("mrow", convert_objs(objects)))
        }
        MtefObj::Pile { halign, lines } => {
            let mut pile_lines = Vec::new();
            for line in lines {
                if let MtefObj::Line { objects, .. } = line {
                    pile_lines.push(objects.clone());
                } else if is_math_obj(line) {
                    pile_lines.push(vec![line.clone()]);
                }
            }
            Some(build_pile_node(halign, &pile_lines))
        }
        MtefObj::Matrix {
            rows,
            cols,
            h_just,
            cells,
        } => Some(convert_matrix(*rows, *cols, h_just, cells)),
        MtefObj::Tmpl {
            selector,
            variations,
            subobjects,
        } => convert_tmpl(selector, variations, subobjects),
        MtefObj::SizeMarker | MtefObj::Meta => None,
    }
}

fn convert_matrix(rows: usize, cols: usize, h_just: &str, cells: &[MtefObj]) -> MathNode {
    let mut cell_nodes = Vec::new();
    for cell in cells {
        match cell {
            MtefObj::Line { objects, .. } => {
                cell_nodes.push(convert_objs(objects));
            }
            MtefObj::Pile { .. } => {
                if let Some(pile_node) = convert_obj(cell) {
                    cell_nodes.push(vec![pile_node]);
                }
            }
            _ => {}
        }
    }

    let mut mtable = MathNode::new("mtable");
    let align = match h_just {
        "left" => Some("left"),
        "right" => Some("right"),
        _ => None,
    };
    if let Some(align_val) = align {
        mtable.set_attr("columnalign", align_val);
    }

    let actual_cols = if cols > 0 {
        cols
    } else if rows > 0 && !cell_nodes.is_empty() {
        cell_nodes.len().div_ceil(rows)
    } else {
        1
    };

    for row_chunk in cell_nodes.chunks(actual_cols.max(1)) {
        let mut mtr = MathNode::new("mtr");
        if let Some(align_val) = align {
            mtr.set_attr("columnalign", align_val);
        }
        for cell_children in row_chunk {
            let mut mtd = MathNode::with_children("mtd", cell_children.clone());
            if let Some(align_val) = align {
                mtd.set_attr("columnalign", align_val);
            }
            mtr.children.push(mtd);
        }
        mtable.children.push(mtr);
    }
    mtable
}

fn convert_tmpl(selector: &str, variations: &[&str], subobjects: &[MtefObj]) -> Option<MathNode> {
    let slots = extract_template_slots(selector, variations, subobjects);
    let has_var = |name: &str| variations.contains(&name);

    match selector {
        "tmFRACT" => {
            let num = slot_to_node(slots.first());
            let den = slot_to_node(slots.get(1));
            if has_var("tvFR_SLASH") && has_var("tvFR_BASE") {
                return Some(MathNode::with_children(
                    "mrow",
                    vec![num, MathNode::leaf("mo", "/"), den],
                ));
            }
            let mut mfrac = MathNode::with_children("mfrac", vec![num, den]);
            if has_var("tvFR_SLASH") {
                mfrac.set_attr("bevelled", "true");
            }
            if has_var("tvFR_SMALL") {
                mfrac.set_attr("displaystyle", "false");
            }
            Some(mfrac)
        }

        "tmROOT" => {
            let radicand = slot_to_node(slots.first());
            if has_var("tvROOT_NTH") {
                let degree = slot_to_node(slots.get(1));
                Some(MathNode::with_children("mroot", vec![radicand, degree]))
            } else {
                Some(MathNode::with_children("msqrt", vec![radicand]))
            }
        }

        "tmSUP" => {
            let base = slot_to_node(slots.first());
            let sup_idx = if slots.len() >= 3 { 2 } else { 1 };
            let sup = slot_to_node(slots.get(sup_idx));
            if has_var("tvSU_PRECEDES") {
                Some(wrap_prescript_node(base, MathNode::new("none"), sup))
            } else {
                Some(MathNode::with_children("msup", vec![base, sup]))
            }
        }
        "tmSUB" => {
            let base = slot_to_node(slots.first());
            let sub = slot_to_node(slots.get(1));
            if has_var("tvSU_PRECEDES") {
                Some(wrap_prescript_node(base, sub, MathNode::new("none")))
            } else {
                Some(MathNode::with_children("msub", vec![base, sub]))
            }
        }
        "tmSUBSUP" => {
            let base = slot_to_node(slots.first());
            let sub = slot_to_node(slots.get(1));
            let sup = slot_to_node(slots.get(2));
            if has_var("tvSU_PRECEDES") {
                Some(wrap_prescript_node(base, sub, sup))
            } else {
                Some(MathNode::with_children("msubsup", vec![base, sub, sup]))
            }
        }

        "tmPAREN" | "tmBRACK" | "tmBRACE" | "tmANGLE" | "tmBAR" | "tmDBAR" | "tmFLOOR"
        | "tmCEILING" | "tmOBRACK" => {
            let (open_ch, close_ch) = match selector {
                "tmPAREN" => ("(", ")"),
                "tmBRACK" => ("[", "]"),
                "tmBRACE" => ("{", "}"),
                "tmANGLE" => ("\u{2329}", "\u{232A}"),
                "tmBAR" => ("|", "|"),
                "tmDBAR" => ("\u{2016}", "\u{2016}"),
                "tmFLOOR" => ("\u{230A}", "\u{230B}"),
                "tmCEILING" => ("\u{2308}", "\u{2309}"),
                "tmOBRACK" => ("\u{301A}", "\u{301B}"),
                _ => ("(", ")"),
            };
            let has_left = has_var("tvFENCE_L");
            let has_right = has_var("tvFENCE_R");
            let left_only = has_left && !has_right;
            let right_only = has_right && !has_left;

            let mut children = Vec::new();
            if !right_only {
                children.push(MathNode::leaf("mo", open_ch));
            }
            children.push(slot_to_node(slots.first()));
            if !left_only {
                children.push(MathNode::leaf("mo", close_ch));
            }
            Some(MathNode::with_children("mrow", children))
        }

        "tmINTERVAL" => {
            let left_from_var = if has_var("tvINTV_LEFT_LP") {
                Some("(")
            } else if has_var("tvINTV_LEFT_RP") {
                Some(")")
            } else if has_var("tvINTV_LEFT_LB") {
                Some("[")
            } else if has_var("tvINTV_LEFT_RB") {
                Some("]")
            } else {
                None
            };
            let right_from_var = if has_var("tvINTV_RIGHT_LP") {
                Some("(")
            } else if has_var("tvINTV_RIGHT_RP") {
                Some(")")
            } else if has_var("tvINTV_RIGHT_LB") {
                Some("[")
            } else if has_var("tvINTV_RIGHT_RB") {
                Some("]")
            } else {
                None
            };
            let (open_node, close_node) =
                if let (Some(left), Some(right)) = (left_from_var, right_from_var) {
                    (MathNode::leaf("mo", left), MathNode::leaf("mo", right))
                } else {
                    let mut delim_nodes = Vec::new();
                    if let Some(delim_slot) = slots.get(1) {
                        delim_nodes.extend(convert_objs(&delim_slot.objects));
                    }
                    let left = delim_nodes
                        .first()
                        .cloned()
                        .unwrap_or_else(|| MathNode::leaf("mo", "("));
                    let right = delim_nodes
                        .get(1)
                        .cloned()
                        .unwrap_or_else(|| MathNode::leaf("mo", ")"));
                    (left, right)
                };
            let body = slot_to_node(slots.first());
            Some(MathNode::with_children(
                "mrow",
                vec![open_node, body, close_node],
            ))
        }

        "tmDIRAC" => {
            let has_left = has_var("tvDI_LEFT");
            let has_right = has_var("tvDI_RIGHT");
            if has_left && !has_right {
                Some(MathNode::with_children(
                    "mrow",
                    vec![
                        MathNode::leaf("mo", "\u{2329}"),
                        slot_to_node(slots.first()),
                        MathNode::leaf("mo", "|"),
                    ],
                ))
            } else if has_right && !has_left {
                let ket = if slots.len() >= 2 {
                    slot_to_node(slots.get(1))
                } else {
                    slot_to_node(slots.first())
                };
                Some(MathNode::with_children(
                    "mrow",
                    vec![
                        MathNode::leaf("mo", "|"),
                        ket,
                        MathNode::leaf("mo", "\u{232A}"),
                    ],
                ))
            } else {
                Some(MathNode::with_children(
                    "mrow",
                    vec![
                        MathNode::leaf("mo", "\u{2329}"),
                        slot_to_node(slots.first()),
                        MathNode::leaf("mo", "|"),
                        slot_to_node(slots.get(1)),
                        MathNode::leaf("mo", "\u{232A}"),
                    ],
                ))
            }
        }

        "tmHBRACE" | "tmHBRACK" => {
            let is_top = has_var("tvHB_TOP");
            let base = slot_to_node(slots.first());
            let glyph = match (selector, is_top) {
                ("tmHBRACE", true) => "\u{23DE}",
                ("tmHBRACE", false) => "\u{23DF}",
                (_, true) => "\u{23B4}",
                (_, false) => "\u{23B5}",
            };
            let mut brace_op = MathNode::leaf("mo", glyph);
            brace_op.set_attr("stretchy", "true");
            let tag = if is_top { "mover" } else { "munder" };
            let inner = MathNode::with_children(tag, vec![base, brace_op]);
            if let Some(label_slot) = slots.get(1) {
                if !label_slot.is_empty_option
                    && (!label_slot.objects.is_empty() || label_slot.is_pile)
                {
                    let label = slot_to_node(Some(label_slot));
                    return Some(MathNode::with_children(tag, vec![inner, label]));
                }
            }
            Some(inner)
        }

        "tmTILDE" => Some(mover_stretchy_accent(
            slot_to_node(slots.first()),
            "\u{02DC}",
        )),
        "tmARC" => Some(mover_stretchy_accent(
            slot_to_node(slots.first()),
            "\u{2322}",
        )),
        "tmHAT" => Some(mover_stretchy_accent(slot_to_node(slots.first()), "^")),
        "tmJSTATUS" => Some(mover_stretchy_accent(
            slot_to_node(slots.first()),
            "\u{FE39}",
        )),
        "tmOBAR" => {
            let base = slot_to_node(slots.first());
            let first = mover_stretchy_accent(base, "\u{00AF}");
            if has_var("tvBAR_DOUBLE") {
                Some(mover_stretchy_accent(first, "\u{00AF}"))
            } else {
                Some(first)
            }
        }
        "tmUBAR" => {
            let base = slot_to_node(slots.first());
            let first = munder_stretchy_accent(base, "_");
            if has_var("tvBAR_DOUBLE") {
                Some(munder_stretchy_accent(first, "_"))
            } else {
                Some(first)
            }
        }
        "tmVEC" => {
            let base = slot_to_node(slots.first());
            let is_under = has_var("tvVE_UNDER");
            let is_left = has_var("tvVE_LEFT");
            let is_right = has_var("tvVE_RIGHT");
            let is_harpoon = has_var("tvVE_HARPOON");
            let arrow = if is_harpoon {
                if is_left {
                    "\u{21BC}"
                } else {
                    "\u{21C0}"
                }
            } else if is_left && is_right {
                "\u{2194}"
            } else if is_left {
                "\u{2190}"
            } else {
                "\u{2192}"
            };
            if is_under {
                Some(munder_stretchy_accent(base, arrow))
            } else {
                Some(mover_stretchy_accent(base, arrow))
            }
        }
        "tmSTRIKE" => {
            let base = slot_to_node(slots.first());
            let horiz = has_var("tvST_HORIZ");
            let up = has_var("tvST_UP");
            let down = has_var("tvST_DOWN");
            let notation = if horiz && !up && !down {
                "horizontalstrike"
            } else if up && !horiz && !down {
                "updiagonalstrike"
            } else if down && !up && !horiz {
                "downdiagonalstrike"
            } else {
                "updiagonalstrike downdiagonalstrike"
            };
            Some(menclose(base, notation))
        }
        "tmBOX" => {
            let base = slot_to_node(slots.first());
            let round = has_var("tvBX_ROUND");
            let top = has_var("tvBX_TOP");
            let bot = has_var("tvBX_BOTTOM");
            let left = has_var("tvBX_LEFT");
            let right = has_var("tvBX_RIGHT");
            let notation = if round {
                "roundedbox".to_owned()
            } else if (top && bot && left && right) || (!top && !bot && !left && !right) {
                "box".to_owned()
            } else if top && right && !bot && !left {
                "actuarial".to_owned()
            } else {
                let mut sides = Vec::new();
                if top {
                    sides.push("top");
                }
                if bot {
                    sides.push("bottom");
                }
                if left {
                    sides.push("left");
                }
                if right {
                    sides.push("right");
                }
                sides.join(" ")
            };
            Some(menclose(base, &notation))
        }
        "tmLDIV" => {
            let divisor = slot_to_node(slots.first());
            let enclosed = menclose(divisor, "longdiv");
            if has_var("tvLD_UPPER") && slots.len() >= 2 {
                let quotient = slot_to_node(slots.get(1));
                let mut mtd_quotient = MathNode::with_children("mtd", vec![quotient]);
                mtd_quotient.set_attr("columnalign", "right");
                let mut mtd_divisor = MathNode::with_children("mtd", vec![enclosed]);
                mtd_divisor.set_attr("columnalign", "right");
                Some(MathNode::with_children(
                    "mtable",
                    vec![
                        MathNode::with_children("mtr", vec![mtd_quotient]),
                        MathNode::with_children("mtr", vec![mtd_divisor]),
                    ],
                ))
            } else {
                Some(enclosed)
            }
        }

        "tmLIM" => {
            let mut base = unwrap_single_mrow_node(slot_to_node(slots.first()));
            if has_var("tvSUBAR") {
                base = munder_stretchy_accent(base, "\u{00AF}");
            } else if has_var("tvDUBAR") {
                base = munder_stretchy_accent(munder_stretchy_accent(base, "\u{00AF}"), "\u{00AF}");
            }
            let lower = has_var("tvBO_LOWER");
            let upper = has_var("tvBO_UPPER");
            if lower && upper {
                let sub = unwrap_single_mrow_node(slot_to_node(slots.get(1)));
                let sup = unwrap_single_mrow_node(slot_to_node(slots.get(2)));
                Some(MathNode::with_children("munderover", vec![base, sub, sup]))
            } else if upper {
                let sup =
                    unwrap_single_mrow_node(slot_to_node(slots.get(2).or_else(|| slots.get(1))));
                let mut mover = MathNode::with_children("mover", vec![base, sup]);
                if mover.children[1].tag == "mo"
                    && mover.children[1]
                        .text
                        .chars()
                        .next()
                        .is_some_and(crate::symbols::is_accent)
                {
                    mover.set_attr("accent", "true");
                }
                Some(mover)
            } else {
                let sub = unwrap_single_mrow_node(slot_to_node(slots.get(1)));
                let mut munder = MathNode::with_children("munder", vec![base, sub]);
                if munder.children[1].tag == "mo"
                    && munder.children[1]
                        .text
                        .chars()
                        .next()
                        .is_some_and(crate::symbols::is_accent)
                {
                    munder.set_attr("accentunder", "true");
                }
                Some(munder)
            }
        }

        "tmSUM" | "tmPROD" | "tmCOPROD" | "tmUNION" | "tmINTER" => {
            let op_ch = match selector {
                "tmSUM" => "\u{2211}",
                "tmPROD" => "\u{220F}",
                "tmCOPROD" => "\u{2210}",
                "tmUNION" => "\u{22C3}",
                "tmINTER" => "\u{22C2}",
                _ => "\u{2211}",
            };
            let op = MathNode::leaf("mo", op_ch);
            let body = slot_to_node(slots.first());
            let lower = has_var("tvBO_LOWER");
            let upper = has_var("tvBO_UPPER");
            let is_sum_style = has_var("tvBO_SUM");

            let op_node = if !lower && !upper {
                op
            } else if lower && !upper {
                let sub = slot_to_node(slots.get(1));
                let tag = if is_sum_style { "munder" } else { "msub" };
                MathNode::with_children(tag, vec![op, sub])
            } else if !lower && upper {
                let sup = slot_to_node(slots.get(2).or_else(|| slots.get(1)));
                let tag = if is_sum_style { "mover" } else { "msup" };
                MathNode::with_children(tag, vec![op, sup])
            } else {
                let sub = slot_to_node(slots.get(1));
                let sup = slot_to_node(slots.get(2));
                let tag = if is_sum_style {
                    "munderover"
                } else {
                    "msubsup"
                };
                MathNode::with_children(tag, vec![op, sub, sup])
            };

            let mut mstyle = MathNode::with_children("mstyle", vec![op_node, body]);
            mstyle.set_attr("displaystyle", "true");
            Some(mstyle)
        }

        "tmSUMOP" | "tmINTOP" => {
            let is_sumop = selector == "tmSUMOP";
            let lower = has_var("tvBO_LOWER");
            let upper = has_var("tvBO_UPPER");
            let op_slot = slot_to_node(slots.get(3).or_else(|| slots.last()));
            let mut op_style = MathNode::with_children("mstyle", vec![op_slot]);
            op_style.set_attr("mathsize", "140%");
            op_style.set_attr("displaystyle", "true");

            let op_wrapped = if lower && !upper {
                let sub = slot_to_node(slots.get(1));
                let tag = if is_sumop { "munder" } else { "msub" };
                MathNode::with_children(tag, vec![op_style, sub])
            } else if upper && !lower {
                let sup = slot_to_node(slots.get(2).or_else(|| slots.get(1)));
                let tag = if is_sumop { "mover" } else { "msup" };
                MathNode::with_children(tag, vec![op_style, sup])
            } else if lower && upper {
                let sub = slot_to_node(slots.get(1));
                let sup = slot_to_node(slots.get(2));
                let tag = if is_sumop { "munderover" } else { "msubsup" };
                MathNode::with_children(tag, vec![op_style, sub, sup])
            } else {
                op_style
            };

            if let Some(first_slot) = slots.first() {
                if !first_slot.is_empty_option && !first_slot.objects.is_empty() {
                    let body = slot_to_node(Some(first_slot));
                    return Some(MathNode::with_children("mrow", vec![op_wrapped, body]));
                }
            }
            Some(op_wrapped)
        }

        "tmINTEG" => {
            let is_double = has_var("tvINT_2");
            let is_triple = has_var("tvINT_3");
            let is_loop = has_var("tvINT_LOOP");
            let is_cw = has_var("tvINT_CW_LOOP");
            let is_ccw = has_var("tvINT_CCW_LOOP");

            let int_ch = if is_ccw {
                "\u{2233}"
            } else if is_cw {
                "\u{2232}"
            } else if is_loop && is_triple {
                "\u{2230}"
            } else if is_loop && is_double {
                "\u{222F}"
            } else if is_loop {
                "\u{222E}"
            } else if is_triple {
                "\u{222D}"
            } else if is_double {
                "\u{222C}"
            } else {
                "\u{222B}"
            };

            let mut op = MathNode::leaf("mo", int_ch);
            if has_var("tvINT_EXPAND") {
                op.set_attr("stretchy", "true");
            }
            let body = slot_to_node(slots.first());
            let lower = has_var("tvBO_LOWER");
            let upper = has_var("tvBO_UPPER");
            let is_sum_style = has_var("tvBO_SUM");

            let op_node = if !lower && !upper {
                op
            } else if lower && !upper {
                let sub = slot_to_node(slots.get(1));
                let tag = if is_sum_style { "munder" } else { "msub" };
                MathNode::with_children(tag, vec![op, sub])
            } else if !lower && upper {
                let sup = slot_to_node(slots.get(2).or_else(|| slots.get(1)));
                let tag = if is_sum_style { "mover" } else { "msup" };
                MathNode::with_children(tag, vec![op, sup])
            } else {
                let sub = slot_to_node(slots.get(1));
                let sup = slot_to_node(slots.get(2));
                let tag = if is_sum_style {
                    "munderover"
                } else {
                    "msubsup"
                };
                MathNode::with_children(tag, vec![op, sub, sup])
            };

            let mrow = MathNode::with_children("mrow", vec![op_node, body]);
            let mut mstyle = MathNode::with_children("mstyle", vec![mrow]);
            mstyle.set_attr("displaystyle", "true");
            Some(mstyle)
        }

        "tmARROW" => {
            let is_left = has_var("tvAR_LEFT");
            let is_right = has_var("tvAR_RIGHT");
            let is_double = has_var("tvAR_DOUBLE");
            let is_harpoon = has_var("tvAR_HARPOON");
            let is_topbottom = has_var("tvAR_TOPBOTTOM");
            let is_bottom = has_var("tvAR_BOTTOM");

            let arrow_ch = if is_harpoon {
                if is_double && is_left && !is_right {
                    "\u{21CB}"
                } else if is_left && !is_right {
                    if is_bottom && !is_topbottom {
                        "\u{21BD}"
                    } else {
                        "\u{21BC}"
                    }
                } else if is_right && !is_left {
                    if is_bottom && !is_topbottom {
                        "\u{21C1}"
                    } else {
                        "\u{21C0}"
                    }
                } else {
                    "\u{21CC}"
                }
            } else if is_double {
                if is_left && !is_right {
                    "\u{21C6}"
                } else {
                    "\u{21C4}"
                }
            } else if is_left && is_right {
                "\u{2194}"
            } else if is_left {
                "\u{2190}"
            } else {
                "\u{2192}"
            };
            let op = MathNode::leaf("mo", arrow_ch);
            if is_topbottom {
                let top_slot = slot_to_node(slots.first());
                let bottom_slot = slot_to_node(slots.get(1));
                Some(MathNode::with_children(
                    "munderover",
                    vec![op, bottom_slot, top_slot],
                ))
            } else if is_bottom {
                let bottom_slot = slot_to_node(slots.get(1).or_else(|| slots.first()));
                Some(MathNode::with_children("munder", vec![op, bottom_slot]))
            } else {
                let top_slot = slot_to_node(slots.first());
                Some(MathNode::with_children("mover", vec![op, top_slot]))
            }
        }

        _ => {
            let children: Vec<MathNode> =
                slots.iter().map(|slot| slot_to_node(Some(slot))).collect();
            Some(MathNode::with_children("mrow", children))
        }
    }
}

fn slot_to_node(slot: Option<&ParsedSlot>) -> MathNode {
    let Some(slot) = slot else {
        return MathNode::new("mrow");
    };
    if slot.is_pile {
        return build_pile_node(slot.pile_halign, &slot.pile_lines);
    }
    MathNode::with_children("mrow", convert_objs(&slot.objects))
}

fn build_pile_node(halign: &str, lines: &[Vec<MtefObj>]) -> MathNode {
    let mut mtable = MathNode::new("mtable");
    match halign {
        "left" => mtable.set_attr("columnalign", "left"),
        "right" => mtable.set_attr("columnalign", "right"),
        "dec" => mtable.set_attr("groupalign", "decimalpoint"),
        _ => {}
    }
    for line_objs in lines {
        let mrow = MathNode::with_children("mrow", convert_objs(line_objs));
        let mtd = MathNode::with_children("mtd", vec![mrow]);
        let mtr = MathNode::with_children("mtr", vec![mtd]);
        mtable.children.push(mtr);
    }
    mtable
}

fn extract_template_slots(
    selector: &str,
    variations: &[&str],
    subobjects: &[MtefObj],
) -> Vec<ParsedSlot> {
    let mut slots = Vec::new();
    let mut current_slot: Vec<MtefObj> = Vec::new();
    let mut direct_pile_seen = false;

    let flush_slot = |slots: &mut Vec<ParsedSlot>, items: Vec<MtefObj>| {
        if items.is_empty() {
            slots.push(ParsedSlot {
                is_empty_option: true,
                is_pile: false,
                pile_halign: "center",
                pile_lines: Vec::new(),
                objects: Vec::new(),
            });
            return;
        }
        if items.len() == 1 {
            if let MtefObj::Line { is_null, objects } = &items[0] {
                slots.push(ParsedSlot {
                    is_empty_option: *is_null && objects.is_empty(),
                    is_pile: false,
                    pile_halign: "center",
                    pile_lines: Vec::new(),
                    objects: objects.clone(),
                });
                return;
            }
        }
        slots.push(ParsedSlot {
            is_empty_option: false,
            is_pile: false,
            pile_halign: "center",
            pile_lines: Vec::new(),
            objects: items,
        });
    };

    for obj in subobjects {
        match obj {
            MtefObj::SizeMarker => {
                if !current_slot.is_empty() {
                    flush_slot(&mut slots, std::mem::take(&mut current_slot));
                }
            }
            MtefObj::Meta => {}
            MtefObj::Pile { halign, lines } => {
                if !current_slot.is_empty() {
                    flush_slot(&mut slots, std::mem::take(&mut current_slot));
                }
                let mut pile_lines = Vec::new();
                for line in lines {
                    if let MtefObj::Line { objects, .. } = line {
                        pile_lines.push(objects.clone());
                    } else if is_math_obj(line) {
                        pile_lines.push(vec![line.clone()]);
                    }
                }
                slots.push(ParsedSlot {
                    is_empty_option: false,
                    is_pile: true,
                    pile_halign: halign,
                    pile_lines,
                    objects: Vec::new(),
                });
                direct_pile_seen = true;
            }
            MtefObj::Line { .. } => {
                if !current_slot.is_empty() {
                    flush_slot(&mut slots, std::mem::take(&mut current_slot));
                }
                flush_slot(&mut slots, vec![obj.clone()]);
            }
            MtefObj::Char { mt_code, .. } => {
                if is_trailing_single_fence_char(
                    selector,
                    variations,
                    *mt_code,
                    &current_slot,
                    direct_pile_seen,
                ) {
                    continue;
                }
                current_slot.push(obj.clone());
            }
            _ => {
                current_slot.push(obj.clone());
            }
        }
    }
    if !current_slot.is_empty() {
        flush_slot(&mut slots, current_slot);
    }
    slots
}

fn unwrap_single_mrow_node(mut node: MathNode) -> MathNode {
    if node.tag == "mrow" && node.children.len() == 1 {
        node.children.pop().unwrap_or_else(|| MathNode::new("mrow"))
    } else {
        node
    }
}

fn wrap_prescript_node(base: MathNode, pre_sub: MathNode, pre_sup: MathNode) -> MathNode {
    let unwrapped = unwrap_single_mrow_node(base);
    if matches!(unwrapped.tag.as_str(), "msup" | "msub" | "msubsup")
        && !unwrapped.children.is_empty()
    {
        let mut post = unwrapped;
        let inner_base = std::mem::replace(&mut post.children[0], MathNode::new("mrow"));
        post.children[0] = MathNode::with_children(
            "mmultiscripts",
            vec![inner_base, MathNode::new("mprescripts"), pre_sub, pre_sup],
        );
        post
    } else {
        MathNode::with_children(
            "mmultiscripts",
            vec![unwrapped, MathNode::new("mprescripts"), pre_sub, pre_sup],
        )
    }
}

fn normal_mi(text: &str) -> MathNode {
    let mut node = MathNode::leaf("mi", text);
    node.set_attr("mathvariant", "normal");
    node
}

fn function_char(obj: &MtefObj) -> Option<char> {
    match obj {
        MtefObj::Char {
            typeface,
            mt_code,
            is_textmode: false,
            embells,
        } if *typeface == TF_FUNCTION && embells.is_empty() => u8::try_from(*mt_code)
            .ok()
            .filter(|byte| byte.is_ascii_alphabetic())
            .map(char::from),
        _ => None,
    }
}

fn is_trailing_single_fence_char(
    selector: &str,
    variations: &[&str],
    code: u16,
    current_slot: &[MtefObj],
    direct_pile_seen: bool,
) -> bool {
    let has_line_or_pile = direct_pile_seen
        || current_slot
            .iter()
            .any(|obj| matches!(obj, MtefObj::Line { .. } | MtefObj::Pile { .. }));
    if !has_line_or_pile {
        return false;
    }
    let has_left = variations.contains(&"tvFENCE_L");
    let has_right = variations.contains(&"tvFENCE_R");
    let left_only = has_left && !has_right;
    let right_only = has_right && !has_left;
    let expected_left = match selector {
        "tmPAREN" => Some(0x0028),
        "tmBRACE" => Some(0x007B),
        "tmBRACK" => Some(0x005B),
        _ => None,
    };
    let expected_right = match selector {
        "tmPAREN" => Some(0x0029),
        "tmBRACE" => Some(0x007D),
        "tmBRACK" => Some(0x005D),
        _ => None,
    };
    (left_only && expected_left == Some(code)) || (right_only && expected_right == Some(code))
}

fn is_mt_space_code(typeface: u8, code: u16) -> bool {
    matches!(code, 0xEF01..=0xEF06 | 0xEF08)
        || (typeface == TF_SPACE && (0xEB01..=0xEB08).contains(&code))
}

fn is_non_base_code(code: u16) -> bool {
    matches!(code, 0x3001 | 0x3002)
}

fn open_close_pair(open: u16) -> Option<u16> {
    match open {
        0x0028 => Some(0x0029),
        0x005B => Some(0x005D),
        0x007B => Some(0x007D),
        _ => None,
    }
}

fn close_open_pair(close: u16) -> Option<u16> {
    match close {
        0x0029 => Some(0x0028),
        0x005D => Some(0x005B),
        0x007D => Some(0x007B),
        _ => None,
    }
}

fn is_suitable_base(obj: &MtefObj) -> bool {
    match obj {
        MtefObj::Char {
            typeface, mt_code, ..
        } => !is_mt_space_code(*typeface, *mt_code) && !is_non_base_code(*mt_code),
        MtefObj::Tmpl {
            selector,
            subobjects,
            ..
        } => {
            if matches!(*selector, "tmSUP" | "tmSUB" | "tmSUBSUP") {
                if let Some(MtefObj::Line {
                    is_null: false,
                    objects,
                }) = subobjects.first()
                {
                    if !objects.is_empty() {
                        return false;
                    }
                }
            }
            true
        }
        MtefObj::Pile { .. } | MtefObj::Matrix { .. } => true,
        _ => false,
    }
}

fn is_suitable_prescript_base(obj: &MtefObj) -> bool {
    if is_suitable_base(obj) {
        return true;
    }
    matches!(
        obj,
        MtefObj::Tmpl {
            selector: "tmSUP" | "tmSUB" | "tmSUBSUP",
            variations,
            ..
        } if !variations.contains(&"tvSU_PRECEDES")
    )
}
