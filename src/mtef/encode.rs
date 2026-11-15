//! Walk a MathML tree and produce MTEF binary equation content.
//!
//! These bytes are the "inner" equation records (CHAR, TMPL, PILE, MATRIX, …)
//! that [`mtef::build_mtef`](crate::mtef::build_mtef) wraps in the full MTEF stream.

use crate::error::Result;
use crate::mathml::MathNode;
use crate::mtef::{
    big_operator, char_record, dirac, double_overbar, double_underbar, embell_record,
    extensible_arrow, fence, fraction, harpoon_arrow, integral, interval, joint_status,
    long_division, map_char_with_variant, nroot_radical, over_arc, over_hat, over_tilde, overbar,
    overbrace, overbracket, overscript, slash_fraction, sqrt_radical, subscript, subsuperscript,
    superscript, under_harpoon_arrow, under_overscript, under_vector_arrow, underbar, underbrace,
    underbracket, underscript, vector_arrow, EMB_1DOT, EMB_1PRIME, EMB_2DOT, EMB_2PRIME, EMB_3DOT,
    EMB_3PRIME, EMB_4DOT, EMB_BARROW, EMB_BPRIME, EMB_FROWN, EMB_HAT, EMB_L1ARROW, EMB_LARROW,
    EMB_MBAR, EMB_NOT, EMB_OBAR, EMB_R1ARROW, EMB_RARROW, EMB_SMILE, EMB_TILDE, EMB_U_1DOT,
    EMB_U_2DOT, EMB_U_3DOT, EMB_U_4DOT, EMB_U_BAR, EMB_U_BARROW, EMB_U_FROWN, EMB_U_L1ARROW,
    EMB_U_LARROW, EMB_U_R1ARROW, EMB_U_RARROW, EMB_U_SMILE, EMB_U_TILDE, END, TF_FUNCTION,
    TF_NUMBER, TF_SYMBOL, TF_TEXT, TF_VARIABLE, TF_VECTOR, TM_ANGLE, TM_BAR, TM_BRACE, TM_BRACK,
    TM_CEILING, TM_COPROD, TM_DBAR, TM_FLOOR, TM_INTER, TM_INTOP, TM_OBRACK, TM_PAREN, TM_PROD, TM_SUM,
    TM_SUMOP, TM_UNION, TV_BO_SUM, TV_INT_1, TV_INT_2, TV_INT_3, TV_INT_CCW_LOOP, TV_INT_CW_LOOP,
    TV_INT_LOOP,
};
use crate::symbols::{has_foreign_letters, is_function, is_prime_char, prime_order};

const MAX_DEPTH: usize = 256;

use crate::symbols::operators::ascii_operator_spelling as op_map;

mod dispatch;
mod records;
mod structural;

use records::{attach_embell, is_single_char_record};

type ScriptedCloseFence = (usize, Vec<u8>, Vec<u8>);

pub(super) struct MathmlEncoder {
    depth: usize,
}

impl MathmlEncoder {
    fn new() -> Self {
        Self { depth: 0 }
    }

    pub(super) fn encode_children(&mut self, node: &MathNode) -> Result<Vec<u8>> {
        if node.tag == "mrow"
            && (node.attr("data-omml-d") == Some("true") || is_stretchy_fence_mrow(node))
        {
            if let Some(delim_bytes) = self.try_encode_omml_delimiter(node)? {
                return Ok(delim_bytes);
            }
        }
        self.encode_children_slice(&node.children)
    }

    fn encode_children_slice(&mut self, children: &[MathNode]) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < children.len() {
            // 1. Check for fence `<mo>` wrapping an `<mtable>` (e.g. matrix `( ... )`
            //    or piecewise system `{ ...` produced by `omml::decode::handle_d`).
            if i + 1 < children.len() && children[i].tag == "mo" && children[i + 1].tag == "mtable"
            {
                if let Some(sel) = open_fence_selector(children[i].text.trim()) {
                    let table_bytes = self.dispatch(&children[i + 1])?;
                    let has_right = i + 2 < children.len()
                        && children[i + 2].tag == "mo"
                        && close_fence_matches(sel, children[i + 2].text.trim());
                    out.extend(fence(sel, true, has_right, &table_bytes, &[], &[]));
                    i += if has_right { 3 } else { 2 };
                    continue;
                }
            }
            // 2. Also check for `<mtable>` followed by a lone closing fence `<mo>`.
            if i + 1 < children.len() && children[i].tag == "mtable" && children[i + 1].tag == "mo"
            {
                if let Some(sel) = close_fence_selector(children[i + 1].text.trim()) {
                    let table_bytes = self.dispatch(&children[i])?;
                    out.extend(fence(sel, false, true, &table_bytes, &[], &[]));
                    i += 2;
                    continue;
                }
            }

            // 3. Check for linear `<mo>(</mo> ... <msup><mo>)</mo> sup</msup>` (or `msub` / `msubsup`)
            //    where a user typed `(` ... `)^2` without `<m:d>`, attaching the script only to `)`.
            if children[i].tag == "mo" {
                if let Some(sel) = open_fence_selector(children[i].text.trim()) {
                    if let Some((close_idx, sub_bytes, sup_bytes)) =
                        self.find_scripted_close_fence(children, i, sel)?
                    {
                        let inner_bytes =
                            self.encode_children_slice(&children[i + 1..close_idx])?;
                        out.extend(fence(sel, true, true, &inner_bytes, &[], &[]));
                        match (!sub_bytes.is_empty(), !sup_bytes.is_empty()) {
                            (true, true) => out.extend(subsuperscript(&sub_bytes, &sup_bytes)),
                            (true, false) => out.extend(subscript(&sub_bytes)),
                            (false, true) => out.extend(superscript(&sup_bytes)),
                            (false, false) => {}
                        }
                        i = close_idx + 1;
                        continue;
                    }
                }
            }

            // 4. Check if `children[i]` is a big operator (integral, sum, prod, union, …)
            //    with following operand siblings in the same row (e.g. from `handle_nary`).
            if i + 1 < children.len() {
                if let Some((op_str, lo_node, hi_node, sum_style)) =
                    extract_bigop_parts(&children[i])
                {
                    let split_end = find_bigop_operand_end(&children[i + 1..]) + (i + 1);
                    if split_end > i + 1 {
                        let main_bytes = self.encode_children_slice(&children[i + 1..split_end])?;
                        let lo_bytes = match lo_node {
                            Some(n) => self.dispatch(n)?,
                            None => Vec::new(),
                        };
                        let hi_bytes = match hi_node {
                            Some(n) => self.dispatch(n)?,
                            None => Vec::new(),
                        };
                        let lo_opt = (!lo_bytes.is_empty()).then_some(lo_bytes.as_slice());
                        let hi_opt = (!hi_bytes.is_empty()).then_some(hi_bytes.as_slice());
                        out.extend(encode_bigop_template(
                            op_str,
                            &main_bytes,
                            lo_opt,
                            hi_opt,
                            sum_style,
                        ));
                        i = split_end;
                        continue;
                    }
                }
            }

            // 5. Check if children[i] is a single character record followed by one or more prime operators
            if i + 1 < children.len() {
                if let Some(first_prime) = unwrap_leaf_text(&children[i + 1]) {
                    if let Some(mut total_order) = prime_order(first_prime) {
                        let base_bytes = self.dispatch(&children[i])?;
                        if is_single_char_record(&base_bytes) {
                            let mut consumed = 1usize;
                            while total_order < 3 && i + 1 + consumed < children.len() {
                                let Some(next_prime) =
                                    unwrap_leaf_text(&children[i + 1 + consumed])
                                else {
                                    break;
                                };
                                let Some(next_order) = prime_order(next_prime) else {
                                    break;
                                };
                                if total_order + next_order > 3 {
                                    break;
                                }
                                total_order += next_order;
                                consumed += 1;
                            }
                            let emb = match total_order {
                                1 => EMB_1PRIME,
                                2 => EMB_2PRIME,
                                3.. => EMB_3PRIME,
                                0 => unreachable!(),
                            };
                            out.extend(attach_embell(base_bytes, emb));
                            i += 1 + consumed;
                            continue;
                        }
                    }
                }
            }

            out.extend(self.dispatch(&children[i])?);
            i += 1;
        }
        Ok(out)
    }

    /// Look ahead from an opening fence at `open_idx` for a matching closing fence
    /// that is the base of `<msup>`, `<msub>`, or `<msubsup>`.
    fn find_scripted_close_fence(
        &mut self,
        children: &[MathNode],
        open_idx: usize,
        sel: u8,
    ) -> Result<Option<ScriptedCloseFence>> {
        let mut depth = 0usize;
        for (idx, child) in children.iter().enumerate().skip(open_idx + 1) {
            if child.tag == "mo" {
                let t = child.text.trim();
                if open_fence_selector(t) == Some(sel) && !close_fence_matches(sel, t) {
                    depth += 1;
                } else if close_fence_matches(sel, t) {
                    if depth == 0 {
                        // Closed by a plain `<mo>)</mo>`, not scripted
                        return Ok(None);
                    }
                    depth -= 1;
                }
            } else if depth == 0 && matches!(child.tag.as_str(), "msup" | "msub" | "msubsup") {
                if let Some(base) = child.children.first() {
                    if base.tag == "mo" && close_fence_matches(sel, base.text.trim()) {
                        let (sub_bytes, sup_bytes) = match child.tag.as_str() {
                            "msub" => (self.nth(child, 1)?, Vec::new()),
                            "msup" => (Vec::new(), self.nth(child, 1)?),
                            _ => (self.nth(child, 1)?, self.nth(child, 2)?),
                        };
                        return Ok(Some((idx, sub_bytes, sup_bytes)));
                    }
                }
            }
        }
        Ok(None)
    }

    /// Encode an `<mrow data-omml-d="true">` created from an OMML `<m:d>` delimiter element.
    fn try_encode_omml_delimiter(&mut self, node: &MathNode) -> Result<Option<Vec<u8>>> {
        let kids = &node.children;
        if kids.is_empty() {
            return Ok(Some(Vec::new()));
        }

        let first_node = kids
            .first()
            .filter(|n| n.tag == "mo" && n.attr("stretchy") == Some("true"));
        let last_node = if kids.len() > 1 {
            kids.last()
                .filter(|n| n.tag == "mo" && n.attr("stretchy") == Some("true"))
        } else {
            None
        };
        let first_mo = first_node.map(|n| n.text.trim()).filter(|t| !t.is_empty());
        let last_mo = last_node.map(|n| n.text.trim()).filter(|t| !t.is_empty());

        let start = usize::from(first_node.is_some());
        let end = if last_node.is_some() {
            kids.len() - 1
        } else {
            kids.len()
        };
        if start > end {
            return Ok(None);
        }
        let inner_slice = &kids[start..end];

        // 1. Check for Dirac bra-ket `<a | b>`, `|b>`, `<a|`
        let is_left_angle = matches!(first_mo, Some("\u{2329}" | "\u{27E8}" | "<"));
        let is_right_angle = matches!(last_mo, Some("\u{232A}" | "\u{27E9}" | ">"));
        if is_left_angle && is_right_angle {
            if let Some(bar_pos) = inner_slice.iter().position(|n| {
                n.tag == "mo" && n.attr("stretchy") == Some("true") && n.text.trim() == "|"
            }) {
                let left_bytes = self.encode_children_slice(&inner_slice[..bar_pos])?;
                let right_bytes = self.encode_children_slice(&inner_slice[bar_pos + 1..])?;
                return Ok(Some(dirac(
                    Some(&left_bytes),
                    Some(&right_bytes),
                    &[],
                    &[],
                    &[],
                )));
            }
        }
        if first_mo == Some("|") && is_right_angle {
            let right_bytes = self.encode_children_slice(inner_slice)?;
            return Ok(Some(dirac(None, Some(&right_bytes), &[], &[], &[])));
        }
        if is_left_angle && last_mo == Some("|") {
            let left_bytes = self.encode_children_slice(inner_slice)?;
            return Ok(Some(dirac(Some(&left_bytes), None, &[], &[], &[])));
        }

        // 2. Check for unmatched interval fences (`[a, b)`, `(a, b]`, `]a, b[`, `[a, b[`, `]a, b]`)
        if let (Some(l_str), Some(r_str)) = (first_mo, last_mo) {
            if let (Some(l_kind), Some(r_kind)) =
                (interval_fence_kind(l_str), interval_fence_kind(r_str))
            {
                let is_matched_paren = l_str == "(" && r_str == ")";
                let is_matched_brack = l_str == "[" && r_str == "]";
                if !is_matched_paren && !is_matched_brack {
                    let inner_bytes = self.encode_children_slice(inner_slice)?;
                    return Ok(Some(interval(l_kind, r_kind, &inner_bytes, &[], &[])));
                }
            }
        }

        // 3. Standard matched or single-sided fence (`(...)`, `[...]`, `{...}`, `|...|`, `‖...‖`,
        //    `⟨...⟩`, `⌊...⌋`, `⌈...⌉`, `⟦...⟧`)
        match (
            first_mo.and_then(open_fence_selector),
            last_mo.and_then(close_fence_selector),
        ) {
            (Some(l_sel), Some(r_sel)) if l_sel == r_sel => {
                let inner_bytes = self.encode_children_slice(inner_slice)?;
                Ok(Some(fence(l_sel, true, true, &inner_bytes, &[], &[])))
            }
            (Some(l_sel), None) if last_mo.is_none() => {
                let inner_bytes = self.encode_children_slice(inner_slice)?;
                Ok(Some(fence(l_sel, true, false, &inner_bytes, &[], &[])))
            }
            (None, Some(r_sel)) if first_mo.is_none() => {
                let inner_bytes = self.encode_children_slice(inner_slice)?;
                Ok(Some(fence(r_sel, false, true, &inner_bytes, &[], &[])))
            }
            _ => Ok(None),
        }
    }

    fn dispatch_inner(&mut self, node: &MathNode) -> Result<Vec<u8>> {
        match node.tag.as_str() {
            // ── leaves ──────────────────────────────────────────────────
            "mi" => Ok(enc_mi(node)),
            "mn" => Ok(enc_mn(node)),
            "mo" => Ok(enc_mo(node)),
            "mtext" => Ok(enc_mtext(node)),
            "mspace" => {
                let width = crate::symbols::misc::space_width_em(node.attr("width").unwrap_or("0"));
                let code = match width {
                    w if (w - 3.0 / 18.0).abs() < 0.00001 => 0xEF02,
                    w if (w - 4.0 / 18.0).abs() < 0.00001 => 0xEF03,
                    w if (w - 5.0 / 18.0).abs() < 0.00001 => 0xEF04,
                    w if (w - 1.0).abs() < 0.00001 => 0xEF05,
                    w if (w - 2.0).abs() < 0.00001 => 0xEF06,
                    _ => 0xEF08,
                };
                Ok(char_record(crate::mtef::TF_MTEXTRA, code))
            }

            // ── fractions & radicals ────────────────────────────────────
            "mfrac" => {
                let num = self.nth(node, 0)?;
                let den = self.nth(node, 1)?;
                if node.attr("linethickness") == Some("0") {
                    Ok(crate::mtef::pile(
                        crate::mtef::HALIGN_CENTER,
                        crate::mtef::VALIGN_CENTER_BASELINE,
                        &[&num, &den],
                    ))
                } else if node.attr("displaystyle") == Some("false") {
                    Ok(crate::mtef::fraction_variant(
                        &num,
                        &den,
                        crate::mtef::TV_FR_SMALL
                            | if node.attr("bevelled") == Some("true") {
                                crate::mtef::TV_FR_SLASH
                            } else {
                                0
                            },
                    ))
                } else if node.attr("bevelled") == Some("true") {
                    Ok(slash_fraction(&num, &den))
                } else {
                    Ok(fraction(&num, &den))
                }
            }
            "msqrt" => {
                let inner = self.encode_children(node)?;
                Ok(sqrt_radical(&inner))
            }
            "mroot" => {
                let rad = self.nth(node, 0)?;
                let deg = self.nth(node, 1)?;
                Ok(nroot_radical(&rad, &deg))
            }

            // ── scripts ─────────────────────────────────────────────────
            "msub" => {
                let mut base = self.nth(node, 0)?;
                let sub = self.nth(node, 1)?;
                if !sub.is_empty() {
                    base.extend(subscript(&sub));
                }
                Ok(base)
            }
            "msup" => {
                let mut base = self.nth(node, 0)?;
                let sup_text = node
                    .children
                    .get(1)
                    .and_then(unwrap_leaf_text)
                    .unwrap_or("");
                if is_single_char_record(&base) {
                    if let Some(order) = prime_order(sup_text) {
                        let emb = match order {
                            1 => EMB_1PRIME,
                            2 => EMB_2PRIME,
                            3.. => EMB_3PRIME,
                            0 => unreachable!(),
                        };
                        return Ok(attach_embell(base, emb));
                    }

                }
                let sup = self.nth(node, 1)?;
                if !sup.is_empty() {
                    base.extend(superscript(&sup));
                }
                Ok(base)
            }
            "msubsup" | "munderover" => {
                let base_node = node.children.first();
                let sub_node = node.children.get(1);
                let sup_node = node.children.get(2);
                let base_text = base_node.and_then(unwrap_leaf_text).unwrap_or("");
                let sub_text = sub_node.map(|n| n.text.trim()).unwrap_or("");
                let sup_text = sup_node.map(|n| n.text.trim()).unwrap_or("");

                // Case A: Extensible arrow with above and/or below labels
                if is_extensible_arrow_str(base_text)
                    && !is_extensible_arrow_str(sub_text)
                    && !is_extensible_arrow_str(sup_text)
                {
                    let sub = self.nth(node, 1)?; // bottom content
                    let sup = self.nth(node, 2)?; // top content
                    let above = (!sup.is_empty()).then_some(sup.as_slice());
                    let below = (!sub.is_empty()).then_some(sub.as_slice());
                    return Ok(extensible_arrow(base_text, above, below));
                }

                // Case B: Standalone big operator with limits
                if is_bigop_str(base_text) {
                    let sub = self.nth(node, 1)?;
                    let sup = self.nth(node, 2)?;
                    let lo_opt = (!sub.is_empty()).then_some(sub.as_slice());
                    let hi_opt = (!sup.is_empty()).then_some(sup.as_slice());
                    let sum_style = match node.attr("data-limloc") {
                        Some("subSup") => false,
                        Some("undOvr") => true,
                        _ => node.tag == "munderover",
                    };
                    return Ok(encode_bigop_template(
                        base_text,
                        &[],
                        lo_opt,
                        hi_opt,
                        sum_style,
                    ));
                }

                // Case C: `<munderover>` (not an extensible arrow or bigop) -> `tmLIM`
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

                // Case D: Standard sub + sup (with prime attachment if sup is prime)
                let mut out = base;
                if let Some(order) = prime_order(sup_text) {
                    if is_single_char_record(&out) {
                        let emb = match order {
                            1 => EMB_1PRIME,
                            2 => EMB_2PRIME,
                            3.. => EMB_3PRIME,
                            0 => unreachable!(),
                        };
                        out = attach_embell(out, emb);
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
            "mmultiscripts" => self.encode_mmultiscripts(node),

            // ── Overscript (accents, vector, bar, arc, hat, overbrace) ──
            "mover" => {
                let base_node = node.children.first();
                let over_node = node.children.get(1);
                let base_text = base_node.and_then(unwrap_leaf_text).unwrap_or("");
                let over_text = over_node.map(|n| n.text.trim()).unwrap_or("");

                // Case A0: Quotient over `<menclose notation="longdiv">` -> `tmLDIV` with `TV_LD_UPPER`
                if let Some(enc_base) = base_node.filter(|n| {
                    n.tag == "menclose" && n.attr("notation").unwrap_or("").contains("longdiv")
                }) {
                    let dividend = self.encode_children(enc_base)?;
                    let quotient = self.nth(node, 1)?;
                    return Ok(long_division(
                        &dividend,
                        (!quotient.is_empty()).then_some(quotient.as_slice()),
                    ));
                }

                // Case A: Nested <mover><mover>main <mo>⏞</mo></mover> label</mover>
                if let Some(inner_base) = base_node.filter(|n| n.tag == "mover") {
                    let inner_over_text = inner_base
                        .children
                        .get(1)
                        .map(|n| n.text.trim())
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

                // Case B: Standalone overbrace / overbracket without outer label
                if is_overbrace_char(over_text) {
                    let main = self.nth(node, 0)?;
                    return Ok(overbrace(&main, &[]));
                }
                if is_overbracket_char(over_text) {
                    let main = self.nth(node, 0)?;
                    return Ok(overbracket(&main, &[]));
                }

                // Case C: Extensible arrow with label ONLY on top (e.g. A \xrightarrow{x} B)
                if is_extensible_arrow_str(base_text) && !is_extensible_arrow_str(over_text) {
                    let over = self.nth(node, 1)?;
                    return Ok(extensible_arrow(
                        base_text,
                        (!over.is_empty()).then_some(over.as_slice()),
                        None,
                    ));
                }

                // Case D: Big operator with upper limit only
                if is_bigop_str(base_text) {
                    let over = self.nth(node, 1)?;
                    let hi_opt = (!over.is_empty()).then_some(over.as_slice());
                    let sum_style = node.attr("data-limloc") != Some("subSup");
                    return Ok(encode_bigop_template(
                        base_text,
                        &[],
                        None,
                        hi_opt,
                        sum_style,
                    ));
                }

                let base = self.nth(node, 0)?;

                match over_text {
                    // Left-pointing harpoon arrow (\leftharpoonup / \overleftarrow{})
                    "\u{21BC}" | "\u{20D0}" | "\u{21BD}" => Ok(harpoon_arrow(&base, true)),
                    // Right-pointing harpoon arrow (\rightharpoonup / \overrightharpoon{})
                    "\u{21C0}" | "\u{20D1}" | "\u{21C1}" => Ok(harpoon_arrow(&base, false)),
                    // Bidirectional vector arrow over characters (\overleftrightarrow{x})
                    "\u{2194}" | "\u{20E1}" | "\u{27F7}" => Ok(vector_arrow(&base, "↔")),
                    // Left vector arrow over characters (\overleftarrow{x})
                    "\u{2190}" | "\u{20D6}" | "\u{27F5}" => Ok(vector_arrow(&base, "←")),
                    // Standard right vector arrow over characters (\vec{x}, \overrightarrow{AB})
                    "\u{2192}" | "\u{20D7}" | "\u{27F6}" => Ok(vector_arrow(&base, "→")),
                    // Double overbar
                    "\u{033F}" | "==" | "═" => Ok(double_overbar(&base)),
                    // Overbar (\overline{AB}, \bar{x})
                    "\u{0304}" | "\u{0305}" | "\u{00AF}" | "\u{203E}" | "\u{02C9}" | "―" | "-"
                    | "_" => Ok(overbar(&base)),
                    // Tilde over characters (\widetilde{AB}, \tilde{x})
                    "~" | "\u{02DC}" | "\u{0303}" | "\u{223C}" => Ok(over_tilde(&base)),
                    // Arc over characters (\wideparen{AB})
                    "\u{2312}" | "\u{23DC}" | "\u{2322}" | "\u{25DC}" | "\u{23DF}" | "\u{0311}" => {
                        Ok(over_arc(&base))
                    }
                    // Hat / circumflex (\widehat{AB}, \hat{x})
                    "^" | "\u{0302}" | "\u{02C6}" => Ok(over_hat(&base)),
                    // Caron (\check{x})
                    "\u{030C}" | "\u{02C7}" => {
                        Ok(overscript(&base, &char_record(TF_TEXT, 0x02C7)))
                    }
                    // Ring above (\mathring{x})
                    "\u{00B0}" | "\u{02DA}" | "\u{030A}" => {
                        Ok(overscript(&base, &char_record(TF_SYMBOL, 0x00B0)))
                    }
                    // Joint status (\u{FE39})
                    "\u{FE39}" => Ok(joint_status(&base)),
                    // Single-character embellishments (dots, primes, smile)
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
                    "`" | "\u{2035}" if is_single_char_record(&base) => {
                        Ok(attach_embell(base, EMB_BPRIME))
                    }
                    "\u{2323}" | "\u{23DD}" | "\u{0306}" | "\u{02D8}"
                        if is_single_char_record(&base) =>
                    {
                        Ok(attach_embell(base, EMB_SMILE))
                    }
                    // Fallback: overscript limit (`tmLIM` with `TV_BO_UPPER`)
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

            // ── Underscript (under-arrow, underbar, underbrace, limits) ─
            "munder" => {
                let base_node = node.children.first();
                let under_node = node.children.get(1);
                let base_text = base_node.and_then(unwrap_leaf_text).unwrap_or("");
                let under_text = under_node.map(|n| n.text.trim()).unwrap_or("");

                // Case A: Nested <munder><munder>main <mo>⏟</mo></munder> label</munder>
                if let Some(inner_base) = base_node.filter(|n| n.tag == "munder") {
                    let inner_under_text = inner_base
                        .children
                        .get(1)
                        .map(|n| n.text.trim())
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

                // Case B: Standalone underbrace / underbracket without outer label
                if is_underbrace_char(under_text) {
                    let main = self.nth(node, 0)?;
                    return Ok(underbrace(&main, &[]));
                }
                if is_underbracket_char(under_text) {
                    let main = self.nth(node, 0)?;
                    return Ok(underbracket(&main, &[]));
                }

                // Case C: Extensible arrow with label ONLY below (e.g. A \xleftrightarrow[x]{} B)
                if is_extensible_arrow_str(base_text) && !is_extensible_arrow_str(under_text) {
                    let under = self.nth(node, 1)?;
                    return Ok(extensible_arrow(
                        base_text,
                        None,
                        (!under.is_empty()).then_some(under.as_slice()),
                    ));
                }

                // Case D: Big operator with lower limit only
                if is_bigop_str(base_text) {
                    let under = self.nth(node, 1)?;
                    let lo_opt = (!under.is_empty()).then_some(under.as_slice());
                    let sum_style = node.attr("data-limloc") != Some("subSup");
                    return Ok(encode_bigop_template(
                        base_text,
                        &[],
                        lo_opt,
                        None,
                        sum_style,
                    ));
                }

                let base = self.nth(node, 0)?;

                // Case E: Double or single underbar (\underline{AB})
                if matches!(under_text, "\u{0333}" | "==" | "═") {
                    return Ok(double_underbar(&base));
                }
                if is_underbar_str(under_text) {
                    return Ok(underbar(&base));
                }

                // Case F: Harpoons or vector arrows under characters
                if !is_limit_function(base_text) && !base_text.contains("lim") {
                    match under_text {
                        "\u{21BC}" | "\u{20D0}" | "\u{21BD}" => {
                            return Ok(under_harpoon_arrow(&base, true));
                        }
                        "\u{21C0}" | "\u{20D1}" | "\u{21C1}" => {
                            return Ok(under_harpoon_arrow(&base, false));
                        }
                        "\u{20EC}" | "\u{20ED}" | "\u{20EE}" | "\u{20EF}" | "\u{0362}"
                        | "\u{2192}" | "\u{20D7}" | "\u{27F6}" | "\u{2190}" | "\u{20D6}"
                        | "\u{27F5}" | "\u{2194}" | "\u{20E1}" | "\u{27F7}" => {
                            return Ok(under_vector_arrow(&base, under_text));
                        }
                        _ => {}
                    }
                }

                // Case G: Single-character under-embellishments
                if is_single_char_record(&base) {
                    match under_text {
                        "\u{02D9}" | "\u{0307}" | "." | "\u{0323}" => {
                            return Ok(attach_embell(base, EMB_U_1DOT))
                        }
                        "\u{00A8}" | "\u{0308}" | ".." | "\u{0324}" => {
                            return Ok(attach_embell(base, EMB_U_2DOT))
                        }
                        "\u{20DB}" | "..." | "\u{20E8}" => {
                            return Ok(attach_embell(base, EMB_U_3DOT))
                        }
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

                // Case H: Limit / underscript (`\lim_{x->c}`, `\max_{x}`, `\underset{a}{b}`)
                let sub = self.nth(node, 1)?;
                if sub.is_empty() {
                    Ok(base)
                } else {
                    Ok(underscript(&base, &sub))
                }
            }

            // ── Enclosures (box, strike, long division) ─────────────────
            "menclose" => self.encode_menclose(node),

            "mphantom" => Ok(Vec::new()), // phantom = invisible, skip

            // ── table / matrix / pile ───────────────────────────────────
            "mtable" => self.encode_mtable(node),

            // math / mrow / mtr / mtd / mstyle / anything unknown: recurse into children.
            _ => self.encode_children(node),
        }
    }
}

/// Unwrap single-child `<mstyle>` / `<mrow>` / `<mpadded>` wrappers to get leaf text.
fn unwrap_leaf_text(mut node: &MathNode) -> Option<&str> {
    loop {
        if !node.text.trim().is_empty() {
            return Some(node.text.trim());
        }
        if matches!(node.tag.as_str(), "mstyle" | "mrow" | "mpadded") && node.children.len() == 1 {
            node = &node.children[0];
        } else {
            return Some(node.text.trim());
        }
    }
}

fn is_bigop_str(s: &str) -> bool {
    let mut chars = s.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => {
            crate::symbols::operators::is_big_op(c)
                || crate::symbols::operators::is_integral(c)
                || matches!(c, '\u{2232}' | '\u{2233}')
        }
        _ => false,
    }
}

fn is_limit_function(s: &str) -> bool {
    matches!(
        s,
        "lim" | "max" | "min" | "sup" | "inf" | "det" | "gcd" | "lcm" | "Pr" | "limsup" | "liminf"
    )
}

fn extract_bigop_parts(
    node: &MathNode,
) -> Option<(&str, Option<&MathNode>, Option<&MathNode>, bool)> {
    let explicit_sum_style = match node.attr("data-limloc") {
        Some("subSup") => Some(false),
        Some("undOvr") => Some(true),
        _ => None,
    };

    match node.tag.as_str() {
        "mo" | "mstyle" | "mrow" => {
            let t = unwrap_leaf_text(node)?;
            is_bigop_str(t).then_some((t, None, None, explicit_sum_style.unwrap_or(true)))
        }
        "munder" => {
            let base = node.children.first()?;
            let t = unwrap_leaf_text(base)?;
            is_bigop_str(t).then_some((
                t,
                node.children.get(1),
                None,
                explicit_sum_style.unwrap_or(true),
            ))
        }
        "mover" => {
            let base = node.children.first()?;
            let t = unwrap_leaf_text(base)?;
            is_bigop_str(t).then_some((
                t,
                None,
                node.children.get(1),
                explicit_sum_style.unwrap_or(true),
            ))
        }
        "munderover" => {
            let base = node.children.first()?;
            let t = unwrap_leaf_text(base)?;
            is_bigop_str(t).then_some((
                t,
                node.children.get(1),
                node.children.get(2),
                explicit_sum_style.unwrap_or(true),
            ))
        }
        "msub" => {
            let base = node.children.first()?;
            let t = unwrap_leaf_text(base)?;
            is_bigop_str(t).then_some((
                t,
                node.children.get(1),
                None,
                explicit_sum_style.unwrap_or(false),
            ))
        }
        "msup" => {
            let base = node.children.first()?;
            let t = unwrap_leaf_text(base)?;
            is_bigop_str(t).then_some((
                t,
                None,
                node.children.get(1),
                explicit_sum_style.unwrap_or(false),
            ))
        }
        "msubsup" => {
            let base = node.children.first()?;
            let t = unwrap_leaf_text(base)?;
            is_bigop_str(t).then_some((
                t,
                node.children.get(1),
                node.children.get(2),
                explicit_sum_style.unwrap_or(false),
            ))
        }
        _ => None,
    }
}

/// Find the end of the integrand/summand slice following a big operator in an `<mrow>`.
/// Stops before top-level relational operators (`=`, `≤`, `≥`, `≠`, `≈`, `→`, …).
fn find_bigop_operand_end(slice: &[MathNode]) -> usize {
    for (idx, child) in slice.iter().enumerate() {
        if child.tag == "mo" {
            let t = child.text.trim();
            if matches!(
                t,
                "=" | "≠" | "<" | ">" | "≤" | "≥" | "≈" | "≡" | "∼" | "≅" | "→" | "⇒" | "⇔"
            ) {
                return idx;
            }
        }
    }
    slice.len()
}

fn encode_bigop_template(
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

fn interval_fence_kind(s: &str) -> Option<u8> {
    match s {
        "(" => Some(0),
        ")" => Some(1),
        "[" => Some(2),
        "]" => Some(3),
        _ => None,
    }
}

fn is_extensible_arrow_str(s: &str) -> bool {
    !s.is_empty()
        && (s.contains('→')
            || s.contains('←')
            || s.contains('↔')
            || s.contains('⇄')
            || s.contains('⇌')
            || s.contains('\u{27F5}')
            || s.contains('\u{27F6}')
            || s.contains('\u{27F7}')
            || s.contains('\u{21C4}')
            || s.contains('\u{21CC}'))
}

fn is_overbrace_char(s: &str) -> bool {
    matches!(s, "\u{23DE}" | "\u{FE37}")
}

fn is_underbrace_char(s: &str) -> bool {
    matches!(s, "\u{23DF}" | "\u{FE38}")
}

fn is_overbracket_char(s: &str) -> bool {
    matches!(s, "\u{23B4}" | "\u{FE47}")
}

fn is_underbracket_char(s: &str) -> bool {
    matches!(s, "\u{23B5}" | "\u{FE48}")
}

fn is_bar_str(s: &str) -> bool {
    matches!(
        s,
        "\u{0304}" | "\u{0305}" | "\u{00AF}" | "\u{203E}" | "\u{02C9}" | "―" | "-" | "_"
    )
}

fn is_underbar_str(s: &str) -> bool {
    matches!(
        s,
        "_" | "-" | "―" | "\u{02C9}" | "\u{0332}" | "\u{0331}" | "\u{203E}" | "\u{00AF}"
    )
}

/// Map a Unicode combining mark (or inline prime) to its MTEF `EMB_*` code.
fn combining_mark_to_embell(c: char) -> Option<u8> {
    match c {
        '\u{0307}' => Some(EMB_1DOT),
        '\u{0308}' => Some(EMB_2DOT),
        '\u{20DB}' => Some(EMB_3DOT),
        '\u{20DC}' => Some(EMB_4DOT),
        '\u{0302}' => Some(EMB_HAT),
        '\u{0303}' => Some(EMB_TILDE),
        '\u{0304}' | '\u{0305}' => Some(EMB_OBAR),
        '\u{0311}' => Some(EMB_FROWN),
        '\u{0306}' => Some(EMB_SMILE),
        '\u{0338}' | '\u{20D2}' => Some(EMB_NOT),
        '\u{0335}' | '\u{0336}' => Some(EMB_MBAR),
        '\u{20D7}' => Some(EMB_RARROW),
        '\u{20D6}' => Some(EMB_LARROW),
        '\u{20E1}' => Some(EMB_BARROW),
        '\u{20D1}' => Some(EMB_R1ARROW),
        '\u{20D0}' => Some(EMB_L1ARROW),
        '\u{0323}' => Some(EMB_U_1DOT),
        '\u{0324}' => Some(EMB_U_2DOT),
        '\u{20E8}' => Some(EMB_U_3DOT),
        '\u{0331}' | '\u{0332}' => Some(EMB_U_BAR),
        '\u{0330}' => Some(EMB_U_TILDE),
        '\u{032F}' => Some(EMB_U_FROWN),
        '\u{032E}' => Some(EMB_U_SMILE),
        '\u{20EF}' => Some(EMB_U_RARROW),
        '\u{20EE}' => Some(EMB_U_LARROW),
        '\u{034D}' => Some(EMB_U_BARROW),
        '\u{20EC}' => Some(EMB_U_R1ARROW),
        '\u{20ED}' => Some(EMB_U_L1ARROW),
        _ => None,
    }
}

fn emit_char_with_combining_marks(
    chars: &[char],
    idx: &mut usize,
    tf: u8,
    mt_code: u32,
) -> Vec<u8> {
    let mut rec = char_record(tf, mt_code);
    let mut embells = Vec::new();

    // Consume any adjacent prime characters (e.g. y', y'', y''')
    let mut prime_count = 0u8;
    while *idx < chars.len() && is_prime_char(chars[*idx]) {
        match chars[*idx] {
            '\'' | '\u{2032}' | '\u{00B4}' => prime_count = prime_count.saturating_add(1),
            '\u{2033}' => prime_count = prime_count.saturating_add(2),
            '\u{2034}' => prime_count = prime_count.saturating_add(3),
            '`' | '\u{2035}' => embells.push(EMB_BPRIME),
            _ => break,
        }
        *idx += 1;
    }
    match prime_count {
        1 => embells.push(EMB_1PRIME),
        2 => embells.push(EMB_2PRIME),
        3.. => embells.push(EMB_3PRIME),
        0 => {}
    }

    while *idx < chars.len() {
        if let Some(emb) = combining_mark_to_embell(chars[*idx]) {
            embells.push(emb);
            *idx += 1;
        } else {
            break;
        }
    }
    if !embells.is_empty() && records::is_single_char_record(&rec) {
        rec[1] |= 0x01; // OPT_CHAR_EMBELL
        for emb in embells {
            rec.extend_from_slice(&embell_record(emb));
        }
        rec.extend_from_slice(END);
    }
    rec
}

fn is_stretchy_fence_mrow(node: &MathNode) -> bool {
    let kids = &node.children;
    kids.len() >= 2
        && kids.first().is_some_and(|n| {
            n.tag == "mo" && n.attr("stretchy") == Some("true") && n.attr("form") == Some("prefix")
        })
        && kids.last().is_some_and(|n| {
            n.tag == "mo" && n.attr("stretchy") == Some("true") && n.attr("form") == Some("postfix")
        })
        && !kids[1..kids.len() - 1].iter().any(|n| {
            n.tag == "mo"
                && n.attr("stretchy") == Some("true")
                && matches!(n.attr("form"), Some("prefix" | "postfix"))
        })
}

fn open_fence_selector(s: &str) -> Option<u8> {
    match s {
        "(" => Some(TM_PAREN),
        "[" => Some(TM_BRACK),
        "{" => Some(TM_BRACE),
        "|" => Some(TM_BAR),
        "\u{2016}" | "\u{2225}" => Some(TM_DBAR),
        "\u{2329}" | "\u{27E8}" => Some(TM_ANGLE),
        "\u{230A}" => Some(TM_FLOOR),
        "\u{2308}" => Some(TM_CEILING),
        "\u{301A}" | "\u{27E6}" => Some(TM_OBRACK),
        _ => None,
    }
}

fn close_fence_selector(s: &str) -> Option<u8> {
    match s {
        ")" => Some(TM_PAREN),
        "]" => Some(TM_BRACK),
        "}" => Some(TM_BRACE),
        "|" => Some(TM_BAR),
        "\u{2016}" | "\u{2225}" => Some(TM_DBAR),
        "\u{232A}" | "\u{27E9}" => Some(TM_ANGLE),
        "\u{230B}" => Some(TM_FLOOR),
        "\u{2309}" => Some(TM_CEILING),
        "\u{301B}" | "\u{27E7}" => Some(TM_OBRACK),
        _ => None,
    }
}

fn close_fence_matches(open_sel: u8, close_str: &str) -> bool {
    close_fence_selector(close_str) == Some(open_sel)
}

fn enc_mi(node: &MathNode) -> Vec<u8> {
    let text = node.text.trim();
    if text.is_empty() {
        return Vec::new();
    }
    let variant = node.attr("mathvariant");
    let bold = matches!(variant, Some("bold") | Some("bold-italic"));

    // 1. Known function name (`sin`, `cos`, `lim`, `ln`, …)
    if text.chars().count() > 1 && is_function(text) {
        let tf = if bold { TF_VECTOR } else { TF_FUNCTION };
        return text
            .chars()
            .flat_map(|c| char_record(tf, c as u32))
            .collect();
    }

    // 2. Vietnamese / foreign text run inside `<mi>`
    if has_foreign_letters(text) {
        let default_tf = if bold { TF_VECTOR } else { TF_TEXT };
        let mut out = Vec::new();
        for ch in text.chars() {
            if ch == ' ' {
                out.extend(char_record(TF_TEXT, 0x20));
            } else {
                let (tf, mt_code) = map_char_with_variant(ch, variant, default_tf);
                out.extend(char_record(tf, mt_code));
            }
        }
        return out;
    }

    // 3. General `<mi>` run (may be a single variable, a variable with combining marks,
    //    or an OMML compound run such as `"IR"`, `"4Fe"`, `"x∈ℝ"`, `"sinα"`).
    let mut out = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut idx = 0;
    while idx < chars.len() {
        let ch = chars[idx];
        if ch.is_whitespace() || ch == '\u{00A0}' {
            idx += 1;
            continue;
        }
        // Check if an ASCII alphabetic sub-run is a known function name (e.g. `"sin"` in `"sinα"`)
        if ch.is_ascii_alphabetic() {
            let start = idx;
            while idx < chars.len() && chars[idx].is_ascii_alphabetic() {
                idx += 1;
            }
            let word: String = chars[start..idx].iter().collect();
            if word.len() > 1 && is_function(&word) {
                let tf = if bold { TF_VECTOR } else { TF_FUNCTION };
                for c in word.chars() {
                    out.extend(char_record(tf, c as u32));
                }
                continue;
            }
            // Otherwise, step through each letter so trailing combining marks attach properly
            idx = start;
        }

        let (tf, mt_code) = map_char_with_variant(ch, variant, TF_VARIABLE);
        idx += 1;
        out.extend(emit_char_with_combining_marks(
            &chars, &mut idx, tf, mt_code,
        ));
    }
    out
}

fn enc_mn(node: &MathNode) -> Vec<u8> {
    let variant = node.attr("mathvariant");
    let mut out = Vec::new();
    let chars: Vec<char> = node.text.trim().chars().collect();
    let mut idx = 0;
    while idx < chars.len() {
        let ch = chars[idx];
        if ch.is_whitespace() {
            idx += 1;
            continue;
        }
        let (tf, mt_code) = if ch == '.' || ch == ',' {
            (TF_SYMBOL, ch as u32)
        } else {
            map_char_with_variant(ch, variant, TF_NUMBER)
        };
        idx += 1;
        out.extend(emit_char_with_combining_marks(
            &chars, &mut idx, tf, mt_code,
        ));
    }
    out
}

fn enc_mo(node: &MathNode) -> Vec<u8> {
    let text = node.text.trim();
    if text.is_empty() || text == "\u{2061}" {
        return Vec::new();
    }
    if let Some(cp) = op_map(text) {
        let (tf, mt_code) =
            map_char_with_variant(char::from_u32(cp).unwrap_or('='), None, TF_SYMBOL);
        return char_record(tf, mt_code);
    }
    let variant = node.attr("mathvariant");
    let chars: Vec<char> = text
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '\u{2061}')
        .collect();
    let mut out = Vec::new();
    let mut idx = 0;
    while idx < chars.len() {
        let ch = chars[idx];
        let (tf, mt_code) = map_char_with_variant(ch, variant, TF_SYMBOL);
        idx += 1;
        out.extend(emit_char_with_combining_marks(
            &chars, &mut idx, tf, mt_code,
        ));
    }
    out
}

fn enc_mtext(node: &MathNode) -> Vec<u8> {
    if node.text.trim().is_empty() {
        return Vec::new();
    }
    let variant = node.attr("mathvariant");
    let bold = matches!(variant, Some("bold") | Some("bold-italic"));
    let default_tf = if bold { TF_VECTOR } else { TF_TEXT };

    let mut out = Vec::new();
    for ch in node.text.chars() {
        if ch == ' ' || ch == '\u{00A0}' {
            out.extend(char_record(TF_TEXT, 0x20));
        } else if !ch.is_whitespace() {
            let (tf, mt_code) = map_char_with_variant(ch, variant, default_tf);
            out.extend(char_record(tf, mt_code));
        }
    }
    out
}

fn encode_mathml_children(math_el: &MathNode) -> Result<Vec<u8>> {
    MathmlEncoder::new().encode_children(math_el)
}

/// Convert a MathML `<math>` tree to MTEF equation content bytes.
///
/// An empty result means the equation has nothing to encode.
pub fn mathml_to_mtef_bytes(math_el: &MathNode) -> Result<Vec<u8>> {
    encode_mathml_children(math_el)
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn decoded(math: &MathNode) -> MathNode {
        crate::mtef::mtef_to_mathml(&crate::mtef::build_mtef(
            &mathml_to_mtef_bytes(math).unwrap(),
        ))
        .unwrap()
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
                .find(|n| n.tag == "mspace")
                .expect("explicit space must not disappear");
            assert!(
                (crate::symbols::misc::space_width_em(space.attr("width").unwrap())
                    - crate::symbols::misc::space_width_em(width))
                .abs()
                    < 0.00001
            );
        }
        let small = decoded(
            &MathNode::parse(
                "<math><mfrac displaystyle=\"false\"><mi>x</mi><mi>y</mi></mfrac></math>",
            )
            .unwrap(),
        );
        assert!(small
            .iter()
            .iter()
            .any(|n| n.tag == "mfrac" && n.attr("displaystyle") == Some("false")));
        let diag = decoded(
            &MathNode::parse("<math><mfrac bevelled=\"true\"><mi>x</mi><mi>y</mi></mfrac></math>")
                .unwrap(),
        );
        assert!(diag
            .iter()
            .iter()
            .any(|n| n.tag == "mfrac" && n.attr("bevelled") == Some("true")));
        let table=decoded(&MathNode::parse("<math><mtable columnalign=\"right\"><mtr><mtd><mi>x</mi></mtd><mtd><mi>y</mi></mtd></mtr><mtr><mtd><mn>1</mn></mtd><mtd><mn>2</mn></mtd></mtr></mtable></math>").unwrap());
        assert!(table
            .iter()
            .iter()
            .any(|n| n.tag == "mtable" && n.attr("columnalign") == Some("right")));
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
                nodes.iter().any(|n| n.text == arrow),
                "arrow changed: {arrow}: {out:?}"
            );
            assert!(
                nodes.iter().any(|n| n.text == "a") && nodes.iter().any(|n| n.text == "b"),
                "lost label: {arrow}"
            );
        }
    }
}
