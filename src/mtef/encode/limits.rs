//! Big operator, script-limit, overscript (`mover`), and underscript (`munder`) encoders.

use crate::error::Result;
use crate::mathml::MathNode;
use crate::mtef::{
    big_operator, char_record, double_overbar, double_underbar, extensible_arrow, harpoon_arrow,
    integral, joint_status, long_division, over_arc, over_hat, over_tilde, overbar, overbrace,
    overbracket, overscript, subscript, subsuperscript, superscript, under_harpoon_arrow,
    under_overscript, under_vector_arrow, underbar, underbrace, underbracket, underscript,
    vector_arrow, EMB_1DOT, EMB_1PRIME, EMB_2DOT, EMB_2PRIME, EMB_3DOT, EMB_3PRIME, EMB_4DOT,
    EMB_BPRIME, EMB_SMILE, EMB_U_1DOT, EMB_U_2DOT, EMB_U_3DOT, EMB_U_4DOT, EMB_U_FROWN,
    EMB_U_SMILE, EMB_U_TILDE, TF_SYMBOL, TF_TEXT, TM_COPROD, TM_INTER, TM_INTOP, TM_PROD, TM_SUM,
    TM_SUMOP, TM_UNION, TV_BO_SUM, TV_INT_1, TV_INT_2, TV_INT_3, TV_INT_CCW_LOOP, TV_INT_CW_LOOP,
    TV_INT_LOOP,
};
use crate::symbols::prime_order;

use super::dispatch::unwrap_leaf_text;
use super::records::{attach_embell, is_single_char_record};

impl super::MathmlEncoder {
    /// Encode `<msubsup>` or `<munderover>` (extensible arrows, big operators, limits, scripts).
    pub(super) fn encode_msubsup_or_munderover(&mut self, node: &MathNode) -> Result<Vec<u8>> {
        let base_node = node.children.first();
        let sub_node = node.children.get(1);
        let sup_node = node.children.get(2);
        let base_text = base_node.and_then(unwrap_leaf_text).unwrap_or("");
        let sub_text = sub_node.map(|child| child.text.trim()).unwrap_or("");
        let sup_text = sup_node.map(|child| child.text.trim()).unwrap_or("");

        // Case A: Extensible arrow with above and/or below labels.
        if is_extensible_arrow_str(base_text)
            && !is_extensible_arrow_str(sub_text)
            && !is_extensible_arrow_str(sup_text)
        {
            let sub = self.nth(node, 1)?;
            let sup = self.nth(node, 2)?;
            let above = (!sup.is_empty()).then_some(sup.as_slice());
            let below = (!sub.is_empty()).then_some(sub.as_slice());
            return Ok(extensible_arrow(base_text, above, below));
        }

        // Case B: Standalone big operator with limits.
        if is_bigop_str(base_text) {
            let sub = self.nth(node, 1)?;
            let sup = self.nth(node, 2)?;
            let lower_opt = (!sub.is_empty()).then_some(sub.as_slice());
            let upper_opt = (!sup.is_empty()).then_some(sup.as_slice());
            let sum_style = match node.attr("data-limloc") {
                Some("subSup") => false,
                Some("undOvr") => true,
                _ => node.tag == "munderover",
            };
            return Ok(encode_bigop_template(
                base_text,
                &[],
                lower_opt,
                upper_opt,
                sum_style,
            ));
        }

        // Case C: `<munderover>` (not an extensible arrow or big operator) -> `tmLIM`.
        let base = self.nth(node, 0)?;
        let sub = self.nth(node, 1)?;
        let sup = self.nth(node, 2)?;
        if node.tag == "munderover" {
            return Ok(match (!sub.is_empty(), !sup.is_empty()) {
                (true, true) => under_overscript(&base, &sub, &sup),
                (true, false) => underscript(&base, &sub),
                (false, true) => overscript(&base, &sup),
                (false, false) => base,
            });
        }

        // Case D: Standard sub + sup (attaching primes as an embellishment if possible).
        let mut out = base;
        if let Some(order) = prime_order(sup_text) {
            if is_single_char_record(&out) {
                let embell = match order {
                    1 => EMB_1PRIME,
                    2 => EMB_2PRIME,
                    3.. => EMB_3PRIME,
                    0 => unreachable!(),
                };
                out = attach_embell(out, embell);
                if !sub.is_empty() {
                    out.extend(subscript(&sub));
                }
                return Ok(out);
            }
        }
        match (!sub.is_empty(), !sup.is_empty()) {
            (true, true) => out.extend(subsuperscript(&sub, &sup)),
            (true, false) => out.extend(subscript(&sub)),
            (false, true) => out.extend(superscript(&sup)),
            (false, false) => {}
        }
        Ok(out)
    }

    /// Encode `<mover>` (long division quotient, overbrace, extensible arrow, accents, or limit).
    pub(super) fn encode_mover(&mut self, node: &MathNode) -> Result<Vec<u8>> {
        let base_node = node.children.first();
        let over_node = node.children.get(1);
        let base_text = base_node.and_then(unwrap_leaf_text).unwrap_or("");
        let over_text = over_node.map(|child| child.text.trim()).unwrap_or("");

        // Quotient over `<menclose notation="longdiv">` -> `tmLDIV` with `TV_LD_UPPER`.
        if let Some(enclose_base) = base_node.filter(|child| {
            child.tag == "menclose" && child.attr("notation").unwrap_or("").contains("longdiv")
        }) {
            let dividend = self.encode_children(enclose_base)?;
            let quotient = self.nth(node, 1)?;
            return Ok(long_division(
                &dividend,
                (!quotient.is_empty()).then_some(quotient.as_slice()),
            ));
        }

        // Nested `<mover><mover>main <mo>⏞</mo></mover> label</mover>`.
        if let Some(inner_base) = base_node.filter(|child| child.tag == "mover") {
            let inner_over_text = inner_base
                .children
                .get(1)
                .map(|child| child.text.trim())
                .unwrap_or("");
            if is_overbrace_char(inner_over_text) {
                let main = self.nth(inner_base, 0)?;
                let label = self.nth(node, 1)?;
                return Ok(overbrace(&main, &label));
            }
            if is_overbracket_char(inner_over_text) {
                let main = self.nth(inner_base, 0)?;
                let label = self.nth(node, 1)?;
                return Ok(overbracket(&main, &label));
            }
            if is_bar_str(inner_over_text) && is_bar_str(over_text) {
                let main = self.nth(inner_base, 0)?;
                return Ok(double_overbar(&main));
            }
        }

        // Standalone overbrace / overbracket without outer label.
        if is_overbrace_char(over_text) {
            let main = self.nth(node, 0)?;
            return Ok(overbrace(&main, &[]));
        }
        if is_overbracket_char(over_text) {
            let main = self.nth(node, 0)?;
            return Ok(overbracket(&main, &[]));
        }

        // Extensible arrow with label only on top.
        if is_extensible_arrow_str(base_text) && !is_extensible_arrow_str(over_text) {
            let over = self.nth(node, 1)?;
            return Ok(extensible_arrow(
                base_text,
                (!over.is_empty()).then_some(over.as_slice()),
                None,
            ));
        }

        // Big operator with upper limit only.
        if is_bigop_str(base_text) {
            let over = self.nth(node, 1)?;
            let upper_opt = (!over.is_empty()).then_some(over.as_slice());
            let sum_style = node.attr("data-limloc") != Some("subSup");
            return Ok(encode_bigop_template(
                base_text,
                &[],
                None,
                upper_opt,
                sum_style,
            ));
        }

        let base = self.nth(node, 0)?;

        match over_text {
            "\u{21BC}" | "\u{20D0}" | "\u{21BD}" => Ok(harpoon_arrow(&base, true)),
            "\u{21C0}" | "\u{20D1}" | "\u{21C1}" => Ok(harpoon_arrow(&base, false)),
            "\u{2194}" | "\u{20E1}" | "\u{27F7}" => Ok(vector_arrow(&base, "↔")),
            "\u{2190}" | "\u{20D6}" | "\u{27F5}" => Ok(vector_arrow(&base, "←")),
            "\u{2192}" | "\u{20D7}" | "\u{27F6}" => Ok(vector_arrow(&base, "→")),
            "\u{033F}" | "==" | "═" => Ok(double_overbar(&base)),
            "\u{0304}" | "\u{0305}" | "\u{00AF}" | "\u{203E}" | "\u{02C9}" | "―" | "-" | "_" => {
                Ok(overbar(&base))
            }
            "~" | "\u{02DC}" | "\u{0303}" | "\u{223C}" => Ok(over_tilde(&base)),
            "\u{2312}" | "\u{23DC}" | "\u{2322}" | "\u{25DC}" | "\u{23DF}" | "\u{0311}" => {
                Ok(over_arc(&base))
            }
            "^" | "\u{0302}" | "\u{02C6}" => Ok(over_hat(&base)),
            "\u{030C}" | "\u{02C7}" => Ok(overscript(&base, &char_record(TF_TEXT, 0x02C7))),
            "\u{00B0}" | "\u{02DA}" | "\u{030A}" => {
                Ok(overscript(&base, &char_record(TF_SYMBOL, 0x00B0)))
            }
            "\u{FE39}" => Ok(joint_status(&base)),
            "\u{02D9}" | "\u{0307}" | "." | "\u{00B7}" if is_single_char_record(&base) => {
                Ok(attach_embell(base, EMB_1DOT))
            }
            "\u{00A8}" | "\u{0308}" | ".." if is_single_char_record(&base) => {
                Ok(attach_embell(base, EMB_2DOT))
            }
            "\u{20DB}" | "..." | "\u{2026}" if is_single_char_record(&base) => {
                Ok(attach_embell(base, EMB_3DOT))
            }
            "\u{20DC}" if is_single_char_record(&base) => Ok(attach_embell(base, EMB_4DOT)),
            "'" | "\u{2032}" | "\u{00B4}" | "\u{0301}" if is_single_char_record(&base) => {
                Ok(attach_embell(base, EMB_1PRIME))
            }
            "''" | "\u{2033}" if is_single_char_record(&base) => {
                Ok(attach_embell(base, EMB_2PRIME))
            }
            "'''" | "\u{2034}" if is_single_char_record(&base) => {
                Ok(attach_embell(base, EMB_3PRIME))
            }
            "`" | "\u{2035}" if is_single_char_record(&base) => Ok(attach_embell(base, EMB_BPRIME)),
            "\u{2323}" | "\u{23DD}" | "\u{0306}" | "\u{02D8}" if is_single_char_record(&base) => {
                Ok(attach_embell(base, EMB_SMILE))
            }
            _ => {
                let sup = self.nth(node, 1)?;
                if sup.is_empty() {
                    Ok(base)
                } else {
                    Ok(overscript(&base, &sup))
                }
            }
        }
    }

    /// Encode `<munder>` (underbrace, extensible arrow, underbar, under-arrow, embellishments, or limit).
    pub(super) fn encode_munder(&mut self, node: &MathNode) -> Result<Vec<u8>> {
        let base_node = node.children.first();
        let under_node = node.children.get(1);
        let base_text = base_node.and_then(unwrap_leaf_text).unwrap_or("");
        let under_text = under_node.map(|child| child.text.trim()).unwrap_or("");

        // Nested `<munder><munder>main <mo>⏟</mo></munder> label</munder>`.
        if let Some(inner_base) = base_node.filter(|child| child.tag == "munder") {
            let inner_under_text = inner_base
                .children
                .get(1)
                .map(|child| child.text.trim())
                .unwrap_or("");
            if is_underbrace_char(inner_under_text) {
                let main = self.nth(inner_base, 0)?;
                let label = self.nth(node, 1)?;
                return Ok(underbrace(&main, &label));
            }
            if is_underbracket_char(inner_under_text) {
                let main = self.nth(inner_base, 0)?;
                let label = self.nth(node, 1)?;
                return Ok(underbracket(&main, &label));
            }
            if is_underbar_str(inner_under_text) && is_underbar_str(under_text) {
                let main = self.nth(inner_base, 0)?;
                return Ok(double_underbar(&main));
            }
        }

        // Standalone underbrace / underbracket without outer label.
        if is_underbrace_char(under_text) {
            let main = self.nth(node, 0)?;
            return Ok(underbrace(&main, &[]));
        }
        if is_underbracket_char(under_text) {
            let main = self.nth(node, 0)?;
            return Ok(underbracket(&main, &[]));
        }

        // Extensible arrow with label only below.
        if is_extensible_arrow_str(base_text) && !is_extensible_arrow_str(under_text) {
            let under = self.nth(node, 1)?;
            return Ok(extensible_arrow(
                base_text,
                None,
                (!under.is_empty()).then_some(under.as_slice()),
            ));
        }

        // Big operator with lower limit only.
        if is_bigop_str(base_text) {
            let under = self.nth(node, 1)?;
            let lower_opt = (!under.is_empty()).then_some(under.as_slice());
            let sum_style = node.attr("data-limloc") != Some("subSup");
            return Ok(encode_bigop_template(
                base_text,
                &[],
                lower_opt,
                None,
                sum_style,
            ));
        }

        let base = self.nth(node, 0)?;

        // Double or single underbar.
        if matches!(under_text, "\u{0333}" | "==" | "═") {
            return Ok(double_underbar(&base));
        }
        if is_underbar_str(under_text) {
            return Ok(underbar(&base));
        }

        // Harpoons or vector arrows under characters.
        if !is_limit_function(base_text) && !base_text.contains("lim") {
            match under_text {
                "\u{21BC}" | "\u{20D0}" | "\u{21BD}" => {
                    return Ok(under_harpoon_arrow(&base, true))
                }
                "\u{21C0}" | "\u{20D1}" | "\u{21C1}" => {
                    return Ok(under_harpoon_arrow(&base, false));
                }
                "\u{20EC}" | "\u{20ED}" | "\u{20EE}" | "\u{20EF}" | "\u{0362}" | "\u{2192}"
                | "\u{20D7}" | "\u{27F6}" | "\u{2190}" | "\u{20D6}" | "\u{27F5}" | "\u{2194}"
                | "\u{20E1}" | "\u{27F7}" => return Ok(under_vector_arrow(&base, under_text)),
                _ => {}
            }
        }

        // Single-character under-embellishments.
        if is_single_char_record(&base) {
            match under_text {
                "\u{02D9}" | "\u{0307}" | "." | "\u{0323}" => {
                    return Ok(attach_embell(base, EMB_U_1DOT))
                }
                "\u{00A8}" | "\u{0308}" | ".." | "\u{0324}" => {
                    return Ok(attach_embell(base, EMB_U_2DOT))
                }
                "\u{20DB}" | "..." | "\u{20E8}" => return Ok(attach_embell(base, EMB_U_3DOT)),
                "\u{20DC}" => return Ok(attach_embell(base, EMB_U_4DOT)),
                "~" | "\u{02DC}" | "\u{0303}" | "\u{0330}" => {
                    return Ok(attach_embell(base, EMB_U_TILDE))
                }
                "\u{2322}" | "\u{23DC}" | "\u{032F}" => {
                    return Ok(attach_embell(base, EMB_U_FROWN))
                }
                "\u{2323}" | "\u{23DD}" | "\u{032E}" => {
                    return Ok(attach_embell(base, EMB_U_SMILE))
                }
                _ => {}
            }
        }

        // Limit / underscript fallback.
        let sub = self.nth(node, 1)?;
        if sub.is_empty() {
            Ok(base)
        } else {
            Ok(underscript(&base, &sub))
        }
    }
}

/// Inspect `node` as a big operator (with optional limits), returning `(op, lower, upper, sum_style)`.
pub(super) fn extract_bigop_parts(
    node: &MathNode,
) -> Option<(&str, Option<&MathNode>, Option<&MathNode>, bool)> {
    let explicit_sum_style = match node.attr("data-limloc") {
        Some("subSup") => Some(false),
        Some("undOvr") => Some(true),
        _ => None,
    };

    match node.tag.as_str() {
        "mo" | "mstyle" | "mrow" => {
            let text = unwrap_leaf_text(node)?;
            is_bigop_str(text).then_some((text, None, None, explicit_sum_style.unwrap_or(true)))
        }
        "munder" => {
            let base = node.children.first()?;
            let text = unwrap_leaf_text(base)?;
            is_bigop_str(text).then_some((
                text,
                node.children.get(1),
                None,
                explicit_sum_style.unwrap_or(true),
            ))
        }
        "mover" => {
            let base = node.children.first()?;
            let text = unwrap_leaf_text(base)?;
            is_bigop_str(text).then_some((
                text,
                None,
                node.children.get(1),
                explicit_sum_style.unwrap_or(true),
            ))
        }
        "munderover" => {
            let base = node.children.first()?;
            let text = unwrap_leaf_text(base)?;
            is_bigop_str(text).then_some((
                text,
                node.children.get(1),
                node.children.get(2),
                explicit_sum_style.unwrap_or(true),
            ))
        }
        "msub" => {
            let base = node.children.first()?;
            let text = unwrap_leaf_text(base)?;
            is_bigop_str(text).then_some((
                text,
                node.children.get(1),
                None,
                explicit_sum_style.unwrap_or(false),
            ))
        }
        "msup" => {
            let base = node.children.first()?;
            let text = unwrap_leaf_text(base)?;
            is_bigop_str(text).then_some((
                text,
                None,
                node.children.get(1),
                explicit_sum_style.unwrap_or(false),
            ))
        }
        "msubsup" => {
            let base = node.children.first()?;
            let text = unwrap_leaf_text(base)?;
            is_bigop_str(text).then_some((
                text,
                node.children.get(1),
                node.children.get(2),
                explicit_sum_style.unwrap_or(false),
            ))
        }
        _ => None,
    }
}

/// Find the end index of the integrand/summand slice following a big operator in an `<mrow>`.
///
/// Stops before top-level relational operators (`=`, `≤`, `≥`, `≠`, `≈`, `→`, ...).
pub(super) fn find_bigop_operand_end(slice: &[MathNode]) -> usize {
    for (idx, child) in slice.iter().enumerate() {
        if child.tag == "mo" {
            let text = child.text.trim();
            if matches!(
                text,
                "=" | "≠" | "<" | ">" | "≤" | "≥" | "≈" | "≡" | "∼" | "≅" | "→" | "⇒" | "⇔"
            ) {
                return idx;
            }
        }
    }
    slice.len()
}

/// Build the MTEF template bytes for a big operator (`integral` or `big_operator`).
pub(super) fn encode_bigop_template(
    op_str: &str,
    main: &[u8],
    lower: Option<&[u8]>,
    upper: Option<&[u8]>,
    sum_style: bool,
) -> Vec<u8> {
    let ch = op_str.chars().next().unwrap_or('∫');
    let op_char = char_record(TF_SYMBOL, ch as u32);
    let sum_bit = if sum_style { TV_BO_SUM } else { 0 };

    match ch {
        '∫' => integral(main, upper, lower, &[], TV_INT_1 | sum_bit),
        '∬' => integral(main, upper, lower, &[], TV_INT_2 | sum_bit),
        '∭' => integral(main, upper, lower, &[], TV_INT_3 | sum_bit),
        '∮' => integral(main, upper, lower, &[], TV_INT_1 | TV_INT_LOOP | sum_bit),
        '∯' => integral(main, upper, lower, &[], TV_INT_2 | TV_INT_LOOP | sum_bit),
        '∰' => integral(main, upper, lower, &[], TV_INT_3 | TV_INT_LOOP | sum_bit),
        '∲' => integral(main, upper, lower, &[], TV_INT_1 | TV_INT_CW_LOOP | sum_bit),
        '∳' => integral(
            main,
            upper,
            lower,
            &[],
            TV_INT_1 | TV_INT_LOOP | TV_INT_CW_LOOP | TV_INT_CCW_LOOP | sum_bit,
        ),
        '∑' => big_operator(TM_SUM, main, upper, lower, &op_char, sum_bit),
        '∏' => big_operator(TM_PROD, main, upper, lower, &op_char, sum_bit),
        '∐' => big_operator(TM_COPROD, main, upper, lower, &op_char, sum_bit),
        '⋃' | '∪' => {
            let cup = char_record(TF_SYMBOL, 0x222A);
            big_operator(TM_UNION, main, upper, lower, &cup, sum_bit)
        }
        '⋂' | '∩' => {
            let cap = char_record(TF_SYMBOL, 0x2229);
            big_operator(TM_INTER, main, upper, lower, &cap, sum_bit)
        }
        _ => big_operator(
            if sum_style { TM_SUMOP } else { TM_INTOP },
            main,
            upper,
            lower,
            &op_char,
            sum_bit,
        ),
    }
}

fn is_bigop_str(text: &str) -> bool {
    let mut chars = text.chars();
    match (chars.next(), chars.next()) {
        (Some(ch), None) => {
            crate::symbols::operators::is_big_op(ch)
                || crate::symbols::operators::is_integral(ch)
                || matches!(ch, '\u{2232}' | '\u{2233}')
        }
        _ => false,
    }
}

fn is_limit_function(text: &str) -> bool {
    matches!(
        text,
        "lim"
            | "max"
            | "min"
            | "sup"
            | "inf"
            | "det"
            | "gcd"
            | "lcm"
            | "Pr"
            | "limsup"
            | "liminf"
            | "lim sup"
            | "lim inf"
    )
}

fn is_extensible_arrow_str(text: &str) -> bool {
    !text.is_empty()
        && (text.contains('→')
            || text.contains('←')
            || text.contains('↔')
            || text.contains('⇄')
            || text.contains('⇌')
            || text.contains('\u{27F5}')
            || text.contains('\u{27F6}')
            || text.contains('\u{27F7}')
            || text.contains('\u{21C4}')
            || text.contains('\u{21CC}'))
}

fn is_overbrace_char(text: &str) -> bool {
    matches!(text, "\u{23DE}" | "\u{FE37}")
}

fn is_underbrace_char(text: &str) -> bool {
    matches!(text, "\u{23DF}" | "\u{FE38}")
}

fn is_overbracket_char(text: &str) -> bool {
    matches!(text, "\u{23B4}" | "\u{FE47}")
}

fn is_underbracket_char(text: &str) -> bool {
    matches!(text, "\u{23B5}" | "\u{FE48}")
}

fn is_bar_str(text: &str) -> bool {
    matches!(
        text,
        "\u{0304}" | "\u{0305}" | "\u{00AF}" | "\u{203E}" | "\u{02C9}" | "―" | "-" | "_"
    )
}

fn is_underbar_str(text: &str) -> bool {
    matches!(
        text,
        "_" | "-" | "―" | "\u{02C9}" | "\u{0332}" | "\u{0331}" | "\u{203E}" | "\u{00AF}"
    )
}
