//! Walk a MathML tree and produce MTEF binary equation content.
//!
//! These bytes are the "inner" equation records (CHAR, TMPL, …) that
//! [`mtef::build_mtef`](crate::mtef::build_mtef) wraps in the full MTEF stream.

use crate::error::{Error, Result};
use crate::mathml::MathNode;
use crate::mtef::{
    char_record, fraction, is_greek_codepoint, nroot_radical, py_isdigit, sqrt_radical, subscript,
    subsuperscript, superscript, TF_FUNCTION, TF_LCGREEK, TF_NUMBER, TF_SYMBOL, TF_TEXT,
    TF_UCGREEK, TF_VARIABLE,
};

const MAX_DEPTH: usize = 256;

use crate::symbols::operators::ascii_operator_spelling as op_map;

struct MathmlEncoder {
    depth: usize,
}

impl MathmlEncoder {
    fn encode_children(&mut self, node: &MathNode) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        for child in &node.children {
            out.extend(self.dispatch(child)?);
        }
        Ok(out)
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
            "mi" => Ok(enc_mi(&node.text)),
            "mn" => Ok(enc_mn(&node.text)),
            "mo" => Ok(enc_mo(&node.text)),
            "mtext" => Ok(enc_mtext(&node.text)),

            // ── structure ───────────────────────────────────────────────
            "mfrac" => {
                let num = self.nth(node, 0)?;
                let den = self.nth(node, 1)?;
                Ok(fraction(&num, &den))
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
            "msub" => {
                let mut base = self.nth(node, 0)?;
                let sub = self.nth(node, 1)?;
                base.extend(subscript(&sub));
                Ok(base)
            }
            "msup" => {
                let mut base = self.nth(node, 0)?;
                let sup = self.nth(node, 1)?;
                base.extend(superscript(&sup));
                Ok(base)
            }
            // Standard sub + sup (also handles big operators like \int_a^b, \sum_{k=0}^n)
            "msubsup" | "munderover" => {
                // Case A: Extensible arrow with BOTH above and below text (A \xleftrightarrow[under]{over} B)
                // This case is for the future because WPS/Word doesn't support this case natively
                // let base_node = node.children.get(0);
                // let base_text = base_node.map(|n| n.text.trim()).unwrap_or("");
                // if base_text.contains('→') || base_text.contains('←') || base_text.contains('↔') {
                //     let sub = self.nth(node, 1)?; // bottom content
                //     let sup = self.nth(node, 2)?; // top content
                //     return Ok(crate::mtef::extensible_arrow(base_text, Some(&sup), Some(&sub)));
                // }

                // Case B: Standard sub + sup (or limits for \int_a^b, \sum_{k=0}^n)
                let mut base = self.nth(node, 0)?;
                let sub = self.nth(node, 1)?;
                let sup = self.nth(node, 2)?;
                base.extend(subsuperscript(&sub, &sup));
                Ok(base)
            }

            // ── Overscript (accents, vector, bar, arc, hat) ─────────────
            "mover" => {
                let base_node = node.children.get(0);
                let over_node = node.children.get(1);
                let base_text = base_node.map(|n| n.text.trim()).unwrap_or("");
                let over_text = over_node.map(|n| n.text.trim()).unwrap_or("");

                // Case A: Extensible arrow with label ONLY on top (e.g. A \xrightarrow{x} B)
                if (base_text.contains('→') || base_text.contains('←') || base_text.contains('↔'))
                    && !over_text.contains('→') && !over_text.contains('←') && !over_text.contains('↔')
                {
                    let over = self.nth(node, 1)?;
                    return Ok(crate::mtef::extensible_arrow(base_text, Some(&over), None));
                }

                let base = self.nth(node, 0)?;

                match over_text {
                    // Left-pointing harpoon arrow (\leftharpoonup / \overleftarrow{})
                    "\u{21BC}" | "\u{20D0}" | "\u{21BD}" => {
                        Ok(crate::mtef::harpoon_arrow(&base, true))
                    }
                    // Right-pointing harpoon arrow (\rightharpoonup / \overrightharpoon{})
                    "\u{21C0}" | "\u{20D1}" | "\u{21C1}" => {
                        Ok(crate::mtef::harpoon_arrow(&base, false))
                    }
                    // Bidirectional arrow over characters (\overleftrightarrow{x} / \overset{\leftrightarrow}{x})
                    "\u{2194}" | "\u{27F7}" => {
                        Ok(crate::mtef::extensible_arrow("↔", None, Some(&base)))
                    }
                    // Left arrow over characters (\overleftarrow{x})
                    "\u{2190}" | "\u{20D6}" | "\u{27F5}" => {
                        Ok(crate::mtef::extensible_arrow("←", None, Some(&base)))
                    }
                    // Standard vector arrow over characters (\vec{x})
                    "\u{2192}" | "\u{20D7}" | "\u{27F6}" => {
                        Ok(crate::mtef::vector_arrow(&base, "→"))
                    }
                    // Overbar (\overline{AB}, \bar{x})
                    "\u{0304}" | "\u{0305}" | "\u{00AF}" | "\u{203E}" | "\u{02C9}" | "―" | "-" | "_" => {
                        Ok(crate::mtef::overbar(&base))
                    }
                    // Arc over characters (\wideparen{AB})
                    "\u{2312}" | "\u{23DC}" | "\u{2322}" | "\u{25DC}" | "\u{23DF}" => {
                        Ok(crate::mtef::over_arc(&base))
                    }
                    // Hat / circumflex (\widehat{AB}, \hat{x})
                    "^" | "\u{0302}" | "\u{02C6}" => {
                        Ok(crate::mtef::over_hat(&base))
                    }
                    // Fallback for regular operators (e.g. \int^b, \sum^n): treat as superscript
                    _ => {
                        let mut b = base;
                        let sup = self.nth(node, 1)?;
                        b.extend(superscript(&sup));
                        Ok(b)
                    }
                }
            }

            // ── Underscript (under-arrow, underbar, limits) ─────────────
            "munder" => {
                let base_node = node.children.get(0);
                let under_node = node.children.get(1);
                let base_text = base_node.map(|n| n.text.trim()).unwrap_or("");
                let under_text = under_node.map(|n| n.text.trim()).unwrap_or("");

                // Case A: Extensible arrow with label ONLY below (e.g. A \xleftrightarrow[x]{} B)
                if (base_text.contains('→') || base_text.contains('←') || base_text.contains('↔'))
                    && !under_text.contains('→') && !under_text.contains('←') && !under_text.contains('↔')
                {
                    let under = self.nth(node, 1)?;
                    return Ok(crate::mtef::extensible_arrow(base_text, None, Some(&under)));
                }

                let base = self.nth(node, 0)?;

                // Case B: Underbar (\underline{AB})
                if under_text == "_" || under_text == "-" || under_text == "―" || under_text == "\u{02C9}" || under_text == "\u{0332}" || under_text == "\u{0331}" || under_text == "\u{203E}" {
                    return Ok(crate::mtef::underbar(&base));
                }

                // Case C: Arrows under characters (\underset{\rightarrow}{x}, \underset{\leftrightarrow}{x})
                // Arrow is under base -> arrow_is_above = false
                let is_pure_arrow = matches!(
                    under_text,
                    "\u{2192}" | "\u{20D7}" | "\u{27F6}" |
                    "\u{2190}" | "\u{20D6}" | "\u{27F5}" |
                    "\u{2194}" | "\u{27F7}"
                );
                if is_pure_arrow && base_text != "lim" && !base_text.contains("lim") {
                    return Ok(crate::mtef::extensible_arrow(under_text, Some(&base), None));
                }

                // Case D: Default for operators & limits (\lim_{x->c}, \int_a, \sum_{k=0})
                let mut b = base;
                let sub = self.nth(node, 1)?;
                b.extend(subscript(&sub));
                Ok(b)
            }

            "mphantom" => Ok(Vec::new()), // phantom = invisible, skip

            // ── table / matrix: flattened row by row ────────────────────
            "mtable" => {
                let mut out = Vec::new();
                for (i, mtr) in node.children.iter().enumerate() {
                    if i > 0 {
                        out.extend(char_record(TF_SYMBOL, ';' as u32)); // row separator
                    }
                    for (j, mtd) in mtr.children.iter().enumerate() {
                        if j > 0 {
                            out.extend(char_record(TF_SYMBOL, ',' as u32));
                        }
                        out.extend(self.encode_children(mtd)?);
                    }
                }
                Ok(out)
            }

            // math / mrow / mtr / mtd / anything unknown: recurse into children.
            _ => self.encode_children(node),
        }
    }
}

fn enc_mi(text: &str) -> Vec<u8> {
    let text = text.trim();
    if text.is_empty() {
        return Vec::new();
    }
    let mut chars = text.chars();
    let first = chars.next().expect("non-empty");
    let single = chars.next().is_none();

    if single {
        let cp = first as u32;
        if is_greek_codepoint(cp) {
            if (0x03B1..=0x03C9).contains(&cp) {
                return char_record(TF_LCGREEK, cp);
            }
            if (0x0391..=0x03A9).contains(&cp) {
                return char_record(TF_UCGREEK, cp);
            }
        }
        return char_record(TF_VARIABLE, cp);
    }
    // Multi-char <mi> is a function name.
    text.chars()
        .flat_map(|c| char_record(TF_FUNCTION, c as u32))
        .collect()
}

fn enc_mn(text: &str) -> Vec<u8> {
    let mut out = Vec::new();
    for ch in text.trim().chars() {
        if py_isdigit(ch) {
            out.extend(char_record(TF_NUMBER, ch as u32));
        } else if ch == '.' || ch == ',' {
            out.extend(char_record(TF_SYMBOL, ch as u32));
        } else {
            out.extend(char_record(TF_VARIABLE, ch as u32));
        }
    }
    out
}

fn enc_mo(text: &str) -> Vec<u8> {
    let text = text.trim();
    if text.is_empty() {
        return Vec::new();
    }
    if let Some(cp) = op_map(text) {
        return char_record(TF_SYMBOL, cp);
    }
    if text == "\u{2061}" {
        return Vec::new(); // invisible function application
    }
    text.chars()
        .flat_map(|c| char_record(TF_SYMBOL, c as u32))
        .collect()
}

fn enc_mtext(text: &str) -> Vec<u8> {
    text.trim()
        .chars()
        .flat_map(|c| char_record(TF_TEXT, c as u32))
        .collect()
}

/// Convert a MathML `<math>` tree to MTEF equation content bytes.
///
/// An empty result means the equation has nothing to encode.
pub fn mathml_to_mtef_bytes(math_el: &MathNode) -> Result<Vec<u8>> {
    MathmlEncoder { depth: 0 }.encode_children(math_el)
}
