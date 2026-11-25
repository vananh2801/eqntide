//! Walk a MathML tree and produce MTEF binary equation content.
//!
//! ```text
//! MathNode tree ──▶ MathmlEncoder::encode_children
//!                        │
//!         ┌──────────────┼──────────────────────┐
//!         ▼              ▼                      ▼
//!   fences / Dirac   tokens / scripts     limits / structural
//!   (m:d, scripted   (mi/mn/mo/mtext,     (big ops, mover/munder,
//!    close fences)    combining primes)    mtable/menclose/mfrac)
//!         │              │                      │
//!         └──────────────┴──────────────────────┘
//!                        ▼
//!       Inner MTEF record bytes (wrapped by `mtef::build_mtef`)
//! ```
//!
//! These bytes are the inner equation records (`CHAR`, `TMPL`, `PILE`,
//! `MATRIX`, ...) that [`build_mtef`](crate::mtef::build_mtef) wraps in a
//! complete MTEF stream.

use crate::error::Result;
use crate::mathml::MathNode;
use crate::mtef::{
    fraction, fraction_variant, nroot_radical, pile, slash_fraction, sqrt_radical, subscript,
    subsuperscript, superscript, EMB_1PRIME, EMB_2PRIME, EMB_3PRIME, HALIGN_CENTER, TV_FR_SLASH,
    TV_FR_SMALL, VALIGN_CENTER_BASELINE,
};
use crate::symbols::prime_order;

mod dispatch;
mod fences;
mod limits;
mod records;
mod structural;
mod tokens;

#[cfg(test)]
mod tests;

use dispatch::unwrap_leaf_text;
use fences::{
    close_fence_matches, close_fence_selector, is_stretchy_fence_mrow, open_fence_selector,
};
use limits::{encode_bigop_template, extract_bigop_parts, find_bigop_operand_end};
use records::{attach_embell, is_single_char_record};
use tokens::{encode_mi, encode_mn, encode_mo, encode_mspace, encode_mtext};

const MAX_DEPTH: usize = 256;

/// Convert a MathML `<math>` tree to MTEF equation content bytes.
///
/// An empty result indicates the equation has nothing to encode.
pub fn mathml_to_mtef_bytes(math_el: &MathNode) -> Result<Vec<u8>> {
    MathmlEncoder::new().encode_children(math_el)
}

/// Stateful recursive encoder walking [`MathNode`] trees into MTEF 5.0 records.
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

    pub(super) fn encode_children_slice(&mut self, children: &[MathNode]) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        let mut idx = 0;
        while idx < children.len() {
            // 1. Check for fence `<mo>` wrapping an `<mtable>` (matrix or piecewise system).
            if idx + 1 < children.len()
                && children[idx].tag == "mo"
                && children[idx + 1].tag == "mtable"
            {
                if let Some(selector) = open_fence_selector(children[idx].text.trim()) {
                    let table_bytes = self.dispatch(&children[idx + 1])?;
                    let has_right = idx + 2 < children.len()
                        && children[idx + 2].tag == "mo"
                        && close_fence_matches(selector, children[idx + 2].text.trim());
                    out.extend(crate::mtef::fence(
                        selector,
                        true,
                        has_right,
                        &table_bytes,
                        &[],
                        &[],
                    ));
                    idx += if has_right { 3 } else { 2 };
                    continue;
                }
            }
            // 2. Also check for `<mtable>` followed by a lone closing fence `<mo>`.
            if idx + 1 < children.len()
                && children[idx].tag == "mtable"
                && children[idx + 1].tag == "mo"
            {
                if let Some(selector) = close_fence_selector(children[idx + 1].text.trim()) {
                    let table_bytes = self.dispatch(&children[idx])?;
                    out.extend(crate::mtef::fence(
                        selector,
                        false,
                        true,
                        &table_bytes,
                        &[],
                        &[],
                    ));
                    idx += 2;
                    continue;
                }
            }

            // 3. Linear `<mo>(</mo> ... <msup><mo>)</mo> sup</msup>` (or `msub`/`msubsup`)
            //    where a script attaches only to the closing delimiter.
            if children[idx].tag == "mo" {
                if let Some(selector) = open_fence_selector(children[idx].text.trim()) {
                    if let Some((close_idx, sub_bytes, sup_bytes)) =
                        self.find_scripted_close_fence(children, idx, selector)?
                    {
                        let inner_bytes =
                            self.encode_children_slice(&children[idx + 1..close_idx])?;
                        out.extend(crate::mtef::fence(
                            selector,
                            true,
                            true,
                            &inner_bytes,
                            &[],
                            &[],
                        ));
                        match (!sub_bytes.is_empty(), !sup_bytes.is_empty()) {
                            (true, true) => out.extend(subsuperscript(&sub_bytes, &sup_bytes)),
                            (true, false) => out.extend(subscript(&sub_bytes)),
                            (false, true) => out.extend(superscript(&sup_bytes)),
                            (false, false) => {}
                        }
                        idx = close_idx + 1;
                        continue;
                    }
                }
            }

            // 4. Big operator (integral, sum, prod, union, ...) with following operand siblings.
            if idx + 1 < children.len() {
                if let Some((op_str, lower_node, upper_node, sum_style)) =
                    extract_bigop_parts(&children[idx])
                {
                    let split_end = find_bigop_operand_end(&children[idx + 1..]) + (idx + 1);
                    if split_end > idx + 1 {
                        let main_bytes =
                            self.encode_children_slice(&children[idx + 1..split_end])?;
                        let lower_bytes = match lower_node {
                            Some(node) => self.dispatch(node)?,
                            None => Vec::new(),
                        };
                        let upper_bytes = match upper_node {
                            Some(node) => self.dispatch(node)?,
                            None => Vec::new(),
                        };
                        let lower_opt = (!lower_bytes.is_empty()).then_some(lower_bytes.as_slice());
                        let upper_opt = (!upper_bytes.is_empty()).then_some(upper_bytes.as_slice());
                        out.extend(encode_bigop_template(
                            op_str,
                            &main_bytes,
                            lower_opt,
                            upper_opt,
                            sum_style,
                        ));
                        idx = split_end;
                        continue;
                    }
                }
            }

            // 5. Single character record followed by one or more prime operators.
            if idx + 1 < children.len() {
                if let Some(first_prime) = unwrap_leaf_text(&children[idx + 1]) {
                    if let Some(mut total_order) = prime_order(first_prime) {
                        let base_bytes = self.dispatch(&children[idx])?;
                        if is_single_char_record(&base_bytes) {
                            let mut consumed = 1usize;
                            while total_order < 3 && idx + 1 + consumed < children.len() {
                                let Some(next_prime) =
                                    unwrap_leaf_text(&children[idx + 1 + consumed])
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
                            let embell = match total_order {
                                1 => EMB_1PRIME,
                                2 => EMB_2PRIME,
                                3.. => EMB_3PRIME,
                                0 => unreachable!(),
                            };
                            out.extend(attach_embell(base_bytes, embell));
                            idx += 1 + consumed;
                            continue;
                        }
                    }
                }
            }

            out.extend(self.dispatch(&children[idx])?);
            idx += 1;
        }
        Ok(out)
    }

    pub(super) fn dispatch_inner(&mut self, node: &MathNode) -> Result<Vec<u8>> {
        match node.tag.as_str() {
            "mi" => Ok(encode_mi(node)),
            "mn" => Ok(encode_mn(node)),
            "mo" => Ok(encode_mo(node)),
            "mtext" => Ok(encode_mtext(node)),
            "mspace" => Ok(encode_mspace(node)),
            "mfrac" => self.encode_mfrac(node),
            "msqrt" => {
                let inner = self.encode_children(node)?;
                Ok(sqrt_radical(&inner))
            }
            "mroot" => {
                let radicand = self.nth(node, 0)?;
                let degree = self.nth(node, 1)?;
                Ok(nroot_radical(&radicand, &degree))
            }
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
                        let embell = match order {
                            1 => EMB_1PRIME,
                            2 => EMB_2PRIME,
                            3.. => EMB_3PRIME,
                            0 => unreachable!(),
                        };
                        return Ok(attach_embell(base, embell));
                    }
                }
                let sup = self.nth(node, 1)?;
                if !sup.is_empty() {
                    base.extend(superscript(&sup));
                }
                Ok(base)
            }
            "msubsup" | "munderover" => self.encode_msubsup_or_munderover(node),
            "mmultiscripts" => self.encode_mmultiscripts(node),
            "mover" => self.encode_mover(node),
            "munder" => self.encode_munder(node),
            "menclose" => self.encode_menclose(node),
            "mphantom" => Ok(Vec::new()),
            "mtable" => self.encode_mtable(node),
            _ => self.encode_children(node),
        }
    }

    fn encode_mfrac(&mut self, node: &MathNode) -> Result<Vec<u8>> {
        let numerator = self.nth(node, 0)?;
        let denominator = self.nth(node, 1)?;
        if node.attr("linethickness") == Some("0") {
            Ok(pile(
                HALIGN_CENTER,
                VALIGN_CENTER_BASELINE,
                &[&numerator, &denominator],
            ))
        } else if node.attr("displaystyle") == Some("false") {
            Ok(fraction_variant(
                &numerator,
                &denominator,
                TV_FR_SMALL
                    | if node.attr("bevelled") == Some("true") {
                        TV_FR_SLASH
                    } else {
                        0
                    },
            ))
        } else if node.attr("bevelled") == Some("true") {
            Ok(slash_fraction(&numerator, &denominator))
        } else {
            Ok(fraction(&numerator, &denominator))
        }
    }
}
