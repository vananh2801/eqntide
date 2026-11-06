//! Walk a MathML tree and produce MTEF binary equation content.
//!
//! These bytes are the "inner" equation records (CHAR, TMPL, PILE, MATRIX, …)
//! that [`mtef::build_mtef`](crate::mtef::build_mtef) wraps in the full MTEF stream.

use crate::error::{Error, Result};
use crate::mathml::MathNode;
use crate::mtef::{
    big_operator, boxed, char_record, dirac, double_overbar, double_underbar, embell_record,
    extensible_arrow, fence, fraction, harpoon_arrow, integral, interval, joint_status,
    long_division, map_char_with_variant, matrix, nroot_radical, over_arc, over_hat, over_tilde,
    overbar, overbrace, overbracket, overscript, pile, presubscript, presubsuperscript,
    presuperscript, slash_fraction, sqrt_radical, strike_through, subscript, subsuperscript,
    superscript, under_harpoon_arrow, under_overscript, under_vector_arrow, underbar, underbrace,
    underbracket, underscript, vector_arrow, EMB_1DOT, EMB_1PRIME, EMB_2DOT, EMB_2PRIME, EMB_3DOT,
    EMB_3PRIME, EMB_4DOT, EMB_BPRIME, EMB_SMILE, EMB_U_1DOT, EMB_U_2DOT, EMB_U_3DOT, EMB_U_4DOT,
    EMB_U_FROWN, EMB_U_SMILE, EMB_U_TILDE, END, HALIGN_CENTER, HALIGN_LEFT, TF_FUNCTION,
    TF_NUMBER, TF_SYMBOL, TF_TEXT, TF_VARIABLE, TF_VECTOR, TM_ANGLE, TM_BAR, TM_BRACE, TM_BRACK,
    TM_CEILING, TM_COPROD, TM_DBAR, TM_FLOOR, TM_INTER, TM_OBRACK, TM_PAREN, TM_PROD, TM_SUM,
    TM_SUMOP, TM_UNION, TV_BO_SUM, TV_INT_1, TV_INT_2, TV_INT_3, TV_INT_CCW_LOOP, TV_INT_CW_LOOP,
    TV_INT_LOOP, TV_ST_DOWN, TV_ST_HORIZ, TV_ST_UP, VALIGN_CENTER_BASELINE,
};
use crate::symbols::{has_foreign_letters, is_function};

const MAX_DEPTH: usize = 256;

use crate::symbols::operators::ascii_operator_spelling as op_map;

struct MathmlEncoder {
    depth: usize,
}

impl MathmlEncoder {
    fn encode_children(&mut self, node: &MathNode) -> Result<Vec<u8>> {
        if node.tag == "mrow" && node.attr("data-omml-d") == Some("true") {
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
            if i + 1 < children.len()
                && children[i].tag == "mo"
                && children[i + 1].tag == "mtable"
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
            if i + 1 < children.len()
                && children[i].tag == "mtable"
                && children[i + 1].tag == "mo"
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
                        let main_bytes =
                            self.encode_children_slice(&children[i + 1..split_end])?;
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
    ) -> Result<Option<(usize, Vec<u8>, Vec<u8>)>> {
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

        let first_mo = kids
            .first()
            .filter(|n| n.tag == "mo" && n.attr("stretchy") == Some("true"))
            .map(|n| n.text.trim());
        let last_mo = if kids.len() > 1 {
            kids.last()
                .filter(|n| n.tag == "mo" && n.attr("stretchy") == Some("true"))
                .map(|n| n.text.trim())
        } else {
            None
        };

        let start = usize::from(first_mo.is_some());
        let end = if last_mo.is_some() {
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
            if let Some(bar_pos) = inner_slice
                .iter()
                .position(|n| n.tag == "mo" && n.attr("stretchy") == Some("true") && n.text.trim() == "|")
            {
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
            if let (Some(l_kind), Some(r_kind)) = (interval_fence_kind(l_str), interval_fence_kind(r_str)) {
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

    fn dispatch(&mut self, node: &MathNode) -> Result<Vec<u8>> {
        if self.depth >= MAX_DEPTH {
            return Err(Error::TooDeep);
        }
        self.depth += 1;
        let r = self.dispatch_inner(node);
        self.depth -= 1;
        r
    }

    /// Encode child `idx` of `node`, or nothing when it does not exist.
    fn nth(&mut self, node: &MathNode, idx: usize) -> Result<Vec<u8>> {
        match node.children.get(idx) {
            Some(c) => self.dispatch(c),
            None => Ok(Vec::new()),
        }
    }

    fn dispatch_inner(&mut self, node: &MathNode) -> Result<Vec<u8>> {
        match node.tag.as_str() {
            // ── leaves ──────────────────────────────────────────────────
            "mi" => Ok(enc_mi(node)),
            "mn" => Ok(enc_mn(node)),
            "mo" => Ok(enc_mo(node)),
            "mtext" => Ok(enc_mtext(node)),

            // ── fractions & radicals ────────────────────────────────────
            "mfrac" => {
                let num = self.nth(node, 0)?;
                let den = self.nth(node, 1)?;
                if node.attr("bevelled") == Some("true") {
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
                let sup_text = node.children.get(1).map(|n| n.text.trim()).unwrap_or("");
                if is_single_char_record(&base) {
                    match sup_text {
                        "'" | "\u{2032}" | "\u{00B4}" => {
                            return Ok(attach_embell(base, EMB_1PRIME));
                        }
                        "''" | "\u{2033}" => {
                            return Ok(attach_embell(base, EMB_2PRIME));
                        }
                        "'''" | "\u{2034}" => {
                            return Ok(attach_embell(base, EMB_3PRIME));
                        }
                        "`" | "\u{2035}" => {
                            return Ok(attach_embell(base, EMB_BPRIME));
                        }
                        _ => {}
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
                let base_text = base_node.map(|n| n.text.trim()).unwrap_or("");
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
                    return Ok(encode_bigop_template(
                        base_text,
                        &[],
                        lo_opt,
                        hi_opt,
                        node.tag == "munderover",
                    ));
                }

                // Case C: `<munderover>` on a limit function (`lim`, `max`, …) -> `tmLIM`
                let base = self.nth(node, 0)?;
                let sub = self.nth(node, 1)?;
                let sup = self.nth(node, 2)?;
                if node.tag == "munderover" && is_limit_function(base_text) {
                    return Ok(match (!sub.is_empty(), !sup.is_empty()) {
                        (true, true) => under_overscript(&base, &sub, &sup),
                        (true, false) => underscript(&base, &sub),
                        (false, true) => overscript(&base, &sup),
                        (false, false) => base,
                    });
                }

                // Case D: Standard sub + sup
                let mut out = base;
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
                let base_text = base_node.map(|n| n.text.trim()).unwrap_or("");
                let over_text = over_node.map(|n| n.text.trim()).unwrap_or("");

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
                    return Ok(encode_bigop_template(base_text, &[], None, hi_opt, true));
                }

                let base = self.nth(node, 0)?;

                match over_text {
                    // Left-pointing harpoon arrow (\leftharpoonup / \overleftarrow{})
                    "\u{21BC}" | "\u{20D0}" | "\u{21BD}" => Ok(harpoon_arrow(&base, true)),
                    // Right-pointing harpoon arrow (\rightharpoonup / \overrightharpoon{})
                    "\u{21C0}" | "\u{20D1}" | "\u{21C1}" => Ok(harpoon_arrow(&base, false)),
                    // Bidirectional arrow over characters (\overleftrightarrow{x} / \overset{\leftrightarrow}{x})
                    "\u{2194}" | "\u{27F7}" => Ok(extensible_arrow("↔", None, Some(&base))),
                    // Left arrow over characters (\overleftarrow{x})
                    "\u{2190}" | "\u{20D6}" | "\u{27F5}" => {
                        Ok(extensible_arrow("←", None, Some(&base)))
                    }
                    // Standard vector arrow over characters (\vec{x})
                    "\u{2192}" | "\u{20D7}" | "\u{27F6}" => Ok(vector_arrow(&base, "→")),
                    // Double overbar
                    "\u{033F}" | "==" | "═" => Ok(double_overbar(&base)),
                    // Overbar (\overline{AB}, \bar{x})
                    "\u{0304}" | "\u{0305}" | "\u{00AF}" | "\u{203E}" | "\u{02C9}" | "―"
                    | "-" | "_" => Ok(overbar(&base)),
                    // Tilde over characters (\widetilde{AB}, \tilde{x})
                    "~" | "\u{02DC}" | "\u{0303}" | "\u{223C}" => Ok(over_tilde(&base)),
                    // Arc over characters (\wideparen{AB})
                    "\u{2312}" | "\u{23DC}" | "\u{2322}" | "\u{25DC}" | "\u{23DF}" => {
                        Ok(over_arc(&base))
                    }
                    // Hat / circumflex (\widehat{AB}, \hat{x})
                    "^" | "\u{0302}" | "\u{02C6}" | "\u{030C}" | "ˇ" => Ok(over_hat(&base)),
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
                    "\u{20DC}" if is_single_char_record(&base) => {
                        Ok(attach_embell(base, EMB_4DOT))
                    }
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
                    "\u{2323}" | "\u{23DD}" if is_single_char_record(&base) => {
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
                let base_text = base_node.map(|n| n.text.trim()).unwrap_or("");
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
                    return Ok(encode_bigop_template(base_text, &[], lo_opt, None, true));
                }

                let base = self.nth(node, 0)?;

                // Case E: Double or single underbar (\underline{AB})
                if matches!(under_text, "\u{0333}" | "==" | "═") {
                    return Ok(double_underbar(&base));
                }
                if is_underbar_str(under_text) {
                    return Ok(underbar(&base));
                }

                // Case F: Harpoons or arrows under characters
                if !is_limit_function(base_text) && !base_text.contains("lim") {
                    match under_text {
                        "\u{21BC}" | "\u{20D0}" | "\u{21BD}" => {
                            return Ok(under_harpoon_arrow(&base, true));
                        }
                        "\u{21C0}" | "\u{20D1}" | "\u{21C1}" => {
                            return Ok(under_harpoon_arrow(&base, false));
                        }
                        "\u{20EC}" | "\u{20ED}" | "\u{20EE}" | "\u{20EF}" | "\u{0362}" => {
                            return Ok(under_vector_arrow(&base, under_text));
                        }
                        "\u{2192}" | "\u{20D7}" | "\u{27F6}" | "\u{2190}" | "\u{20D6}"
                        | "\u{27F5}" | "\u{2194}" | "\u{27F7}" => {
                            return Ok(extensible_arrow(under_text, Some(&base), None));
                        }
                        _ => {}
                    }
                }

                // Case G: Single-character under-embellishments
                if is_single_char_record(&base) {
                    match under_text {
                        "\u{02D9}" | "\u{0307}" | "." => return Ok(attach_embell(base, EMB_U_1DOT)),
                        "\u{00A8}" | "\u{0308}" | ".." => {
                            return Ok(attach_embell(base, EMB_U_2DOT))
                        }
                        "\u{20DB}" | "..." => return Ok(attach_embell(base, EMB_U_3DOT)),
                        "\u{20DC}" => return Ok(attach_embell(base, EMB_U_4DOT)),
                        "~" | "\u{02DC}" | "\u{0303}" => {
                            return Ok(attach_embell(base, EMB_U_TILDE))
                        }
                        "\u{2322}" | "\u{23DC}" => return Ok(attach_embell(base, EMB_U_FROWN)),
                        "\u{2323}" | "\u{23DD}" => return Ok(attach_embell(base, EMB_U_SMILE)),
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

    fn encode_mmultiscripts(&mut self, node: &MathNode) -> Result<Vec<u8>> {
        let base = self.nth(node, 0)?;
        if let Some(prescripts_pos) = node
            .children
            .iter()
            .position(|c| c.tag == "mprescripts")
        {
            let presub_node = node.children.get(prescripts_pos + 1);
            let presup_node = node.children.get(prescripts_pos + 2);
            let presub = match presub_node {
                Some(n) if n.tag != "none" => self.dispatch(n)?,
                _ => Vec::new(),
            };
            let presup = match presup_node {
                Some(n) if n.tag != "none" => self.dispatch(n)?,
                _ => Vec::new(),
            };
            let mut out = match (!presub.is_empty(), !presup.is_empty()) {
                (true, true) => presubsuperscript(&base, &presub, &presup),
                (true, false) => presubscript(&base, &presub),
                (false, true) => presuperscript(&base, &presup),
                (false, false) => base,
            };
            // Also encode any postscripts that appeared before <mprescripts/>
            if prescripts_pos > 1 {
                let post_sub = match node.children.get(1) {
                    Some(n) if n.tag != "none" => self.dispatch(n)?,
                    _ => Vec::new(),
                };
                let post_sup = match node.children.get(2) {
                    Some(n) if prescripts_pos > 2 && n.tag != "none" => self.dispatch(n)?,
                    _ => Vec::new(),
                };
                match (!post_sub.is_empty(), !post_sup.is_empty()) {
                    (true, true) => out.extend(subsuperscript(&post_sub, &post_sup)),
                    (true, false) => out.extend(subscript(&post_sub)),
                    (false, true) => out.extend(superscript(&post_sup)),
                    (false, false) => {}
                }
            }
            return Ok(out);
        }

        // No <mprescripts/>: treat like <msubsup>
        let mut out = base;
        let sub = match node.children.get(1) {
            Some(n) if n.tag != "none" => self.dispatch(n)?,
            _ => Vec::new(),
        };
        let sup = match node.children.get(2) {
            Some(n) if n.tag != "none" => self.dispatch(n)?,
            _ => Vec::new(),
        };
        match (!sub.is_empty(), !sup.is_empty()) {
            (true, true) => out.extend(subsuperscript(&sub, &sup)),
            (true, false) => out.extend(subscript(&sub)),
            (false, true) => out.extend(superscript(&sup)),
            (false, false) => {}
        }
        Ok(out)
    }

    fn encode_menclose(&mut self, node: &MathNode) -> Result<Vec<u8>> {
        let inner = self.encode_children(node)?;
        let notation = node.attr("notation").unwrap_or("box");
        if notation.contains("longdiv") {
            return Ok(long_division(&inner, None));
        }

        let mut strike_bits = 0u8;
        if notation.contains("horizontalstrike") {
            strike_bits |= TV_ST_HORIZ;
        }
        if notation.contains("updiagonalstrike") {
            strike_bits |= TV_ST_UP;
        }
        if notation.contains("downdiagonalstrike") {
            strike_bits |= TV_ST_DOWN;
        }
        if strike_bits != 0 {
            return Ok(strike_through(&inner, strike_bits));
        }

        let rounded = notation.contains("roundedbox");
        let all_sides = notation.contains("box") || rounded;
        let actuarial = notation.contains("actuarial");
        let left = all_sides || notation.contains("left");
        let right = all_sides || actuarial || notation.contains("right");
        let top = all_sides || actuarial || notation.contains("top");
        let bottom = all_sides || actuarial || notation.contains("bottom");
        Ok(boxed(&inner, rounded, left, right, top, bottom))
    }

    fn encode_mtable(&mut self, node: &MathNode) -> Result<Vec<u8>> {
        let nrows = node.children.len();
        if nrows == 0 {
            return Ok(Vec::new());
        }
        let ncols = node
            .children
            .iter()
            .map(|mtr| mtr.children.len())
            .max()
            .unwrap_or(0);

        // Single-column stack (e.g. from <m:eqArr> or <pile>): encode as MTEF PILE.
        if ncols <= 1 {
            let mut encoded_rows: Vec<Vec<u8>> = Vec::with_capacity(nrows);
            for mtr in &node.children {
                let row_bytes = match mtr.children.first() {
                    Some(mtd) => self.encode_children(mtd)?,
                    None => self.encode_children(mtr)?,
                };
                encoded_rows.push(row_bytes);
            }
            if nrows == 1 {
                return Ok(encoded_rows.pop().unwrap_or_default());
            }
            let row_refs: Vec<&[u8]> = encoded_rows.iter().map(Vec::as_slice).collect();
            let halign = match node.attr("columnalign") {
                Some("center") => HALIGN_CENTER,
                _ => HALIGN_LEFT,
            };
            return Ok(pile(halign, VALIGN_CENTER_BASELINE, &row_refs));
        }

        // Multi-column grid (e.g. from <m:m>): encode as MTEF MATRIX.
        let mut encoded_cells: Vec<Vec<u8>> = Vec::with_capacity(nrows * ncols);
        for mtr in &node.children {
            for col_idx in 0..ncols {
                let cell_bytes = match mtr.children.get(col_idx) {
                    Some(mtd) => self.encode_children(mtd)?,
                    None => Vec::new(),
                };
                encoded_cells.push(cell_bytes);
            }
        }
        let cell_refs: Vec<&[u8]> = encoded_cells.iter().map(Vec::as_slice).collect();
        let h_just = match node.attr("columnalign") {
            Some("left") => HALIGN_LEFT,
            _ => HALIGN_CENTER,
        };
        Ok(matrix(
            VALIGN_CENTER_BASELINE,
            h_just,
            VALIGN_CENTER_BASELINE,
            nrows.min(u8::MAX as usize) as u8,
            ncols.min(u8::MAX as usize) as u8,
            &cell_refs,
        ))
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
    match node.tag.as_str() {
        "mo" => {
            let t = node.text.trim();
            is_bigop_str(t).then_some((t, None, None, true))
        }
        "munder" => {
            let base = node.children.first()?;
            let t = base.text.trim();
            is_bigop_str(t).then_some((t, node.children.get(1), None, true))
        }
        "mover" => {
            let base = node.children.first()?;
            let t = base.text.trim();
            is_bigop_str(t).then_some((t, None, node.children.get(1), true))
        }
        "munderover" => {
            let base = node.children.first()?;
            let t = base.text.trim();
            is_bigop_str(t).then_some((t, node.children.get(1), node.children.get(2), true))
        }
        "msub" => {
            let base = node.children.first()?;
            let t = base.text.trim();
            is_bigop_str(t).then_some((t, node.children.get(1), None, false))
        }
        "msup" => {
            let base = node.children.first()?;
            let t = base.text.trim();
            is_bigop_str(t).then_some((t, None, node.children.get(1), false))
        }
        "msubsup" => {
            let base = node.children.first()?;
            let t = base.text.trim();
            is_bigop_str(t).then_some((t, node.children.get(1), node.children.get(2), false))
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
        '∮' => integral(
            main,
            upper,
            lower,
            &[],
            TV_INT_1 | TV_INT_LOOP | sum_bit,
        ),
        '∯' => integral(
            main,
            upper,
            lower,
            &[],
            TV_INT_2 | TV_INT_LOOP | sum_bit,
        ),
        '∰' => integral(
            main,
            upper,
            lower,
            &[],
            TV_INT_3 | TV_INT_LOOP | sum_bit,
        ),
        '∲' => integral(
            main,
            upper,
            lower,
            &[],
            TV_INT_1 | TV_INT_CW_LOOP | sum_bit,
        ),
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
        _ => big_operator(TM_SUMOP, main, upper, lower, &op_char, sum_bit),
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

fn is_single_char_record(bytes: &[u8]) -> bool {
    bytes.len() == 5 && bytes[0] == 0x02 && bytes[1] == 0x00
}

fn attach_embell(mut char_bytes: Vec<u8>, embell_code: u8) -> Vec<u8> {
    if is_single_char_record(&char_bytes) {
        char_bytes[1] |= 0x01; // OPT_CHAR_EMBELL
        char_bytes.extend_from_slice(&embell_record(embell_code));
        char_bytes.extend_from_slice(END);
    }
    char_bytes
}

fn open_fence_selector(s: &str) -> Option<u8> {
    match s {
        "(" => Some(TM_PAREN),
        "[" => Some(TM_BRACK),
        "{" => Some(TM_BRACE),
        "|" => Some(TM_BAR),
        "\u{2016}" => Some(TM_DBAR),
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
        "\u{2016}" => Some(TM_DBAR),
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

    // 3. General `<mi>` run (may be a single variable or an OMML compound run
    //    such as `"IR"`, `"4Fe"`, `"x∈ℝ"`, `"sinα"`).
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
            } else {
                for c in word.chars() {
                    let (tf, mt_code) = map_char_with_variant(c, variant, TF_VARIABLE);
                    out.extend(char_record(tf, mt_code));
                }
            }
            continue;
        }

        let (tf, mt_code) = map_char_with_variant(ch, variant, TF_VARIABLE);
        out.extend(char_record(tf, mt_code));
        idx += 1;
    }
    out
}

fn enc_mn(node: &MathNode) -> Vec<u8> {
    let variant = node.attr("mathvariant");
    let mut out = Vec::new();
    for ch in node.text.trim().chars() {
        if ch.is_whitespace() {
            continue;
        }
        if ch == '.' || ch == ',' {
            out.extend(char_record(TF_SYMBOL, ch as u32));
        } else {
            let (tf, mt_code) = map_char_with_variant(ch, variant, TF_NUMBER);
            out.extend(char_record(tf, mt_code));
        }
    }
    out
}

fn enc_mo(node: &MathNode) -> Vec<u8> {
    let text = node.text.trim();
    if text.is_empty() || text == "\u{2061}" {
        return Vec::new();
    }
    if let Some(cp) = op_map(text) {
        let (tf, mt_code) = map_char_with_variant(char::from_u32(cp).unwrap_or('='), None, TF_SYMBOL);
        return char_record(tf, mt_code);
    }
    let variant = node.attr("mathvariant");
    text.chars()
        .filter(|c| !c.is_whitespace() && *c != '\u{2061}')
        .flat_map(|c| {
            let (tf, mt_code) = map_char_with_variant(c, variant, TF_SYMBOL);
            char_record(tf, mt_code)
        })
        .collect()
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

/// Convert a MathML `<math>` tree to MTEF equation content bytes.
///
/// An empty result means the equation has nothing to encode.
pub fn mathml_to_mtef_bytes(math_el: &MathNode) -> Result<Vec<u8>> {
    MathmlEncoder { depth: 0 }.encode_children(math_el)
}
