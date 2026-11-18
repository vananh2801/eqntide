//! OMML (Office Open XML Math) → MathML converter.
//!
//! Handles the common OMML elements from ECMA-376 Part 1 §22.1. Returns a
//! [`MathNode`] tree rooted at `<math>`. Unknown elements are dropped, exactly
//! like the Python original.

use crate::error::{Error, Result};
use crate::mathml::MathNode;
use crate::xml::Element;

/// OMML namespace.
pub const NS_M: &str = "http://schemas.openxmlformats.org/officeDocument/2006/math";

/// Safety net against pathological nesting (the XML parser already caps depth
/// at 256, so this only matters for hand-built trees).
const MAX_DEPTH: usize = 256;

use crate::symbols::operators::{OMML_OPERATOR_ALIASES as OP_ALIASES, OMML_OPERATOR_CHARS as OPS};
use crate::symbols::prime_order;

fn collect_text(node: &MathNode) -> String {
    let mut s = String::new();
    if !node.text.is_empty() {
        s.push_str(&node.text);
    }
    for c in &node.children {
        s.push_str(&collect_text(c));
    }
    s
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Read `m:val` of the child `m:<child_tag>` of `node`.
///
/// Mirrors the Python `_m_val`: a missing node or child yields `default`; a
/// child without `m:val` also yields `default`.
fn m_val(node: Option<&Element>, child_tag: &str, default: Option<&str>) -> Option<String> {
    let child = node.and_then(|n| n.find(NS_M, child_tag));
    match child {
        None => default.map(str::to_owned),
        Some(c) => c.attr(Some(NS_M), "val").or(default).map(str::to_owned),
    }
}

/// Read an OOXML boolean (`CT_OnOff`) child `m:<child_tag>` of `node`.
///
/// Per ECMA-376, a missing element is `false`, an empty element `<m:tag/>`
/// without `m:val` is `true`, and `m:val="1" | "on" | "true"` is `true`.
fn m_bool(node: Option<&Element>, child_tag: &str) -> bool {
    let Some(c) = node.and_then(|n| n.find(NS_M, child_tag)) else {
        return false;
    };
    match c.attr(Some(NS_M), "val") {
        None => true,
        Some(v) => matches!(v, "1" | "on" | "true"),
    }
}

/// All `m:t` text below `node`, concatenated (`node.iter(m:t)`).
fn get_text(node: &Element) -> String {
    let mut out = String::new();
    for e in node.iter() {
        if e.is(NS_M, "t") {
            out.push_str(&e.text());
        }
    }
    out
}

/// Port of Python's `float(s)` success test, minus the `inf`/`nan` words (an
/// OMML run reading `inf` is the infimum *function*, not a number).
pub fn is_number(s: &str) -> bool {
    let t = s.replace(',', ".");
    let t = t.trim();
    let unsigned = t.strip_prefix(['+', '-']).unwrap_or(t);
    match unsigned.chars().next() {
        None => false,
        Some(c) if c.is_ascii_digit() || c == '.' => t.parse::<f64>().is_ok(),
        Some(_) => false,
    }
}

/// `true` when every character is a known operator symbol.
pub fn is_operator(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| OPS.contains(c) || OP_ALIASES.contains(&c))
}

/// Classify a run of text as `<mn>`, `<mo>` or `<mi>`; `None` when blank.
pub fn text_node(text: &str) -> Option<MathNode> {
    let t = text.trim();
    if t.is_empty() {
        return None;
    }
    Some(if is_number(t) {
        MathNode::leaf("mn", t)
    } else if is_operator(t) {
        MathNode::leaf("mo", t)
    } else {
        MathNode::leaf("mi", t)
    })
}

/// If `el` has exactly one child return that child, else return `el`.
fn unwrap_mrow(mut el: MathNode) -> MathNode {
    if el.children.len() == 1 {
        el.children.remove(0)
    } else {
        el
    }
}

fn mrow() -> MathNode {
    MathNode::new("mrow")
}

fn mel(tag: &str, children: Vec<MathNode>) -> MathNode {
    MathNode::with_children(tag, children)
}

// ── Converter ─────────────────────────────────────────────────────────────────

struct OmmlConverter {
    depth: usize,
}

impl OmmlConverter {
    fn convert(&mut self, omml_root: &Element) -> Result<MathNode> {
        let mut row = mrow();
        for child in omml_root.elements() {
            if let Some(r) = self.dispatch(child)? {
                row.children.push(r);
            }
        }
        Ok(mel("math", vec![row]))
    }

    fn dispatch(&mut self, node: &Element) -> Result<Option<MathNode>> {
        if self.depth >= MAX_DEPTH {
            return Err(Error::TooDeep);
        }
        self.depth += 1;
        let result = self.dispatch_inner(node);
        self.depth -= 1;
        result
    }

    fn dispatch_inner(&mut self, node: &Element) -> Result<Option<MathNode>> {
        // Like the Python original, dispatch on the local name only.
        match node.local.as_str() {
            "oMath" | "oMathPara" => Ok(Some(self.children_to_mrow(node)?)),
            "r" => Ok(self.handle_r(node)),
            "t" => Ok(text_node(&node.text())),
            "f" => self.handle_f(node).map(Some),
            "rad" => self.handle_rad(node).map(Some),
            "sSub" => {
                let base = self.get_e(node)?;
                let sub = self.get_child(node, "sub")?;
                Ok(Some(mel("msub", vec![base, sub])))
            }
            "sSup" => {
                let base = self.get_e(node)?;
                let sup = self.get_child(node, "sup")?;
                let sup_text = collect_text(&sup);
                if let Some(order) = prime_order(&sup_text) {
                    let prime_str = match order {
                        1 => "'",
                        2 => "''",
                        3.. => "'''",
                        0 => unreachable!(),
                    };
                    Ok(Some(mel(
                        "mrow",
                        vec![base, MathNode::leaf("mo", prime_str)],
                    )))
                } else {
                    Ok(Some(mel("msup", vec![base, sup])))
                }
            }
            "sSubSup" => {
                let base = self.get_e(node)?;
                let sub = self.get_child(node, "sub")?;
                let sup = self.get_child(node, "sup")?;
                let sup_text = collect_text(&sup);
                if let Some(order) = prime_order(&sup_text) {
                    let prime_str = match order {
                        1 => "'",
                        2 => "''",
                        3.. => "'''",
                        0 => unreachable!(),
                    };
                    let primed_base = mel("mrow", vec![base, MathNode::leaf("mo", prime_str)]);
                    Ok(Some(mel("msub", vec![primed_base, sub])))
                } else {
                    Ok(Some(mel("msubsup", vec![base, sub, sup])))
                }
            }
            "sPre" => self.handle_s_pre(node).map(Some),
            "limLow" => self.handle_lim(node, "munder").map(Some),
            "limUpp" => self.handle_lim(node, "mover").map(Some),
            "d" => self.handle_d(node).map(Some),
            "func" => self.handle_func(node).map(Some),
            "nary" => self.handle_nary(node).map(Some),
            "acc" => self.handle_acc(node).map(Some),
            "bar" => self.handle_bar(node).map(Some),
            "groupChr" => self.handle_group_chr(node).map(Some),
            "m" => self.handle_matrix(node).map(Some),
            "eqArr" => self.handle_eq_arr(node).map(Some),
            "phant" => self.handle_phant(node),
            "borderBox" => self.handle_border_box(node),
            "box" => match node.find(NS_M, "e") {
                Some(e) => Ok(Some(self.children_to_mrow(e)?)),
                None => Ok(None),
            },
            _ => Ok(None),
        }
    }

    /// Convert every child of `node` and wrap the results in an `<mrow>`.
    fn children_to_mrow(&mut self, node: &Element) -> Result<MathNode> {
        let mut row = mrow();
        for child in node.elements() {
            if let Some(r) = self.dispatch(child)? {
                row.children.push(r);
            }
        }
        Ok(row)
    }

    /// `children_to_mrow` of the optional child `m:<tag>`, or an empty `<mrow>`.
    fn child_mrow(&mut self, node: &Element, tag: &str) -> Result<MathNode> {
        match node.find(NS_M, tag) {
            Some(c) => self.children_to_mrow(c),
            None => Ok(mrow()),
        }
    }

    fn get_e(&mut self, node: &Element) -> Result<MathNode> {
        self.get_child(node, "e")
    }

    /// `_get_e` / `_get_sub` / `_get_sup`: converted child, unwrapped.
    fn get_child(&mut self, node: &Element, tag: &str) -> Result<MathNode> {
        match node.find(NS_M, tag) {
            None => Ok(mrow()),
            Some(c) => Ok(unwrap_mrow(self.children_to_mrow(c)?)),
        }
    }

    // ── Pre-Sub-Superscript (m:sPre) ──────────────────────────────────────

    fn handle_s_pre(&mut self, node: &Element) -> Result<MathNode> {
        let base = self.get_e(node)?;
        let sub = match node.find(NS_M, "sub") {
            Some(s) if !get_text(s).trim().is_empty() => self.get_child(node, "sub")?,
            _ => MathNode::new("none"),
        };
        let sup = match node.find(NS_M, "sup") {
            Some(p) if !get_text(p).trim().is_empty() => self.get_child(node, "sup")?,
            _ => MathNode::new("none"),
        };
        Ok(mel(
            "mmultiscripts",
            vec![base, MathNode::new("mprescripts"), sub, sup],
        ))
    }

    // ── Border Box / Strikethrough (m:borderBox) ──────────────────────────

    fn handle_border_box(&mut self, node: &Element) -> Result<Option<MathNode>> {
        let Some(e) = node.find(NS_M, "e") else {
            return Ok(None);
        };
        let inner = self.children_to_mrow(e)?;
        let b_pr = node.find(NS_M, "borderBoxPr");

        let strike_bltr = m_bool(b_pr, "strikeBLTR");
        let strike_tlbr = m_bool(b_pr, "strikeTLBR");
        let strike_h = m_bool(b_pr, "strikeH");
        let strike_v = m_bool(b_pr, "strikeV");

        let mut notations = Vec::new();
        if strike_bltr {
            notations.push("updiagonalstrike");
        }
        if strike_tlbr {
            notations.push("downdiagonalstrike");
        }
        if strike_h {
            notations.push("horizontalstrike");
        }
        if strike_v {
            notations.push("verticalstrike");
        }

        if !notations.is_empty() {
            let mut enc = mel("menclose", inner.children);
            enc.set_attr("notation", &notations.join(" "));
            return Ok(Some(enc));
        }

        let hide_top = m_bool(b_pr, "hideTop");
        let hide_bot = m_bool(b_pr, "hideBot");
        let hide_left = m_bool(b_pr, "hideLeft");
        let hide_right = m_bool(b_pr, "hideRight");

        if !(hide_top && hide_bot && hide_left && hide_right) {
            let mut enc = mel("menclose", inner.children);
            if !hide_top && !hide_bot && !hide_left && !hide_right {
                enc.set_attr("notation", "box");
            } else if !hide_top && !hide_right && hide_bot && hide_left {
                enc.set_attr("notation", "actuarial");
            } else {
                let mut sides = Vec::new();
                if !hide_top {
                    sides.push("top");
                }
                if !hide_bot {
                    sides.push("bottom");
                }
                if !hide_left {
                    sides.push("left");
                }
                if !hide_right {
                    sides.push("right");
                }
                enc.set_attr("notation", &sides.join(" "));
            }
            return Ok(Some(enc));
        }

        Ok(Some(inner))
    }

    // ── Run ───────────────────────────────────────────────────────────────

    fn handle_r(&mut self, node: &Element) -> Option<MathNode> {
        let text = get_text(node);
        if text.is_empty() {
            return None;
        }
        if text
            .chars()
            .all(|c| crate::symbols::misc::unicode_space_em(c).is_some())
        {
            let width: f32 = text
                .chars()
                .filter_map(crate::symbols::misc::unicode_space_em)
                .sum();
            let mut space = MathNode::new("mspace");
            space.set_attr("width", &format!("{width:.6}em"));
            return Some(space);
        }
        let r_pr = node.find(NS_M, "rPr");
        let sty = m_val(r_pr, "sty", None);
        let scr = m_val(r_pr, "scr", None);
        let is_nor = m_bool(r_pr, "nor");

        // Use `<mtext>` when `<m:nor/>` is set, or when `<m:sty m:val="p"/>` wraps
        // natural-language text (with spaces or non-ASCII Latin/Vietnamese letters).
        // Upright math identifiers (`sin`, `cos`, `log`, `lim`, differential `d`, `Φ`)
        // in Word have `<m:sty m:val="p"/>` without `<m:nor/>` and should stay `<mi>`.
        if (is_nor
            || (sty.as_deref() == Some("p")
                && (crate::symbols::has_foreign_letters(&text) || text.trim().contains(' '))))
            && scr.is_none()
        {
            let mut el = MathNode::leaf("mtext", &text);
            if matches!(sty.as_deref(), Some("b") | Some("bi")) {
                el.set_attr("mathvariant", "bold");
            }
            return Some(el);
        }

        let mut el = text_node(&text).unwrap_or_else(|| MathNode::leaf("mi", &text));
        let variant = match (scr.as_deref(), sty.as_deref()) {
            (Some("double-struck"), _) => Some("double-struck"),
            (Some("fraktur"), Some("b") | Some("bi")) => Some("bold-fraktur"),
            (Some("fraktur"), _) => Some("fraktur"),
            (Some("script"), Some("b") | Some("bi")) => Some("bold-script"),
            (Some("script"), _) => Some("script"),
            (Some("sans-serif"), Some("b") | Some("bi")) => Some("bold"),
            (Some("sans-serif"), _) => Some("sans-serif"),
            (Some("monospace"), _) => Some("monospace"),
            (_, Some("b")) => Some("bold"),
            (_, Some("bi")) => Some("bold-italic"),
            (_, Some("p")) => Some("normal"),
            _ => None,
        };
        if let Some(v) = variant {
            el.set_attr("mathvariant", v);
        }
        Some(el)
    }

    // ── Fraction ──────────────────────────────────────────────────────────

    fn handle_f(&mut self, node: &Element) -> Result<MathNode> {
        let f_type = m_val(node.find(NS_M, "fPr"), "type", Some("bar")).unwrap_or_default();
        let num = self.child_mrow(node, "num")?;
        let den = self.child_mrow(node, "den")?;

        match f_type.as_str() {
            "lin" => {
                let mut row = mrow();
                row.children.push(unwrap_mrow(num));
                row.children.push(MathNode::leaf("mo", "/"));
                row.children.push(unwrap_mrow(den));
                Ok(row)
            }
            "skw" => {
                let mut frac = mel("mfrac", vec![unwrap_mrow(num), unwrap_mrow(den)]);
                frac.set_attr("bevelled", "true");
                Ok(frac)
            }
            "noBar" => {
                let mut frac = mel("mfrac", vec![unwrap_mrow(num), unwrap_mrow(den)]);
                frac.set_attr("linethickness", "0");
                Ok(frac)
            }
            _ => Ok(mel("mfrac", vec![unwrap_mrow(num), unwrap_mrow(den)])),
        }
    }

    // ── Radical ───────────────────────────────────────────────────────────

    fn handle_rad(&mut self, node: &Element) -> Result<MathNode> {
        let rad = self.child_mrow(node, "e")?;

        let rad_pr = node.find(NS_M, "radPr");
        let is_deg_hidden = m_bool(rad_pr, "degHide");

        if !is_deg_hidden {
            if let Some(deg_node) = node.find(NS_M, "deg") {
                let deg_text = get_text(deg_node);
                if !deg_text.trim().is_empty() {
                    let deg = self.children_to_mrow(deg_node)?;
                    return Ok(mel("mroot", vec![unwrap_mrow(rad), unwrap_mrow(deg)]));
                }
            }
        }

        Ok(mel("msqrt", vec![unwrap_mrow(rad)]))
    }

    // ── limLow / limUpp ───────────────────────────────────────────────────

    fn handle_lim(&mut self, node: &Element, tag: &str) -> Result<MathNode> {
        if let Some(e_node) = node.find(NS_M, "e") {
            let e_children: Vec<&Element> = e_node.elements().collect();
            if e_children.len() == 1 {
                let inner = e_children[0];
                if tag == "munder" && inner.is(NS_M, "limUpp") {
                    let base = self.get_e(inner)?;
                    let under = self.child_mrow(node, "lim")?;
                    let over = self.child_mrow(inner, "lim")?;
                    return Ok(mel(
                        "munderover",
                        vec![base, unwrap_mrow(under), unwrap_mrow(over)],
                    ));
                }
                if tag == "mover" && inner.is(NS_M, "limLow") {
                    let base = self.get_e(inner)?;
                    let under = self.child_mrow(inner, "lim")?;
                    let over = self.child_mrow(node, "lim")?;
                    return Ok(mel(
                        "munderover",
                        vec![base, unwrap_mrow(under), unwrap_mrow(over)],
                    ));
                }
                if tag == "munder" && inner.is(NS_M, "groupChr") {
                    let g = self.handle_group_chr(inner)?;
                    if g.tag == "mover"
                        && g.attr("accent").is_none()
                        && g.children.len() == 2
                        && g.children[0].tag == "mo"
                    {
                        let under = self.child_mrow(node, "lim")?;
                        return Ok(mel(
                            "munderover",
                            vec![
                                g.children[0].clone(),
                                unwrap_mrow(under),
                                g.children[1].clone(),
                            ],
                        ));
                    }
                }
                if tag == "mover" && inner.is(NS_M, "groupChr") {
                    let g = self.handle_group_chr(inner)?;
                    if g.tag == "munder"
                        && g.attr("accentunder").is_none()
                        && g.children.len() == 2
                        && g.children[0].tag == "mo"
                    {
                        let over = self.child_mrow(node, "lim")?;
                        return Ok(mel(
                            "munderover",
                            vec![
                                g.children[0].clone(),
                                g.children[1].clone(),
                                unwrap_mrow(over),
                            ],
                        ));
                    }
                }
            }
        }
        let e = self.get_e(node)?;
        let lim = self.child_mrow(node, "lim")?;
        Ok(mel(tag, vec![e, unwrap_mrow(lim)]))
    }

    // ── Delimiter ─────────────────────────────────────────────────────────

    fn handle_d(&mut self, node: &Element) -> Result<MathNode> {
        let d_pr = node.find(NS_M, "dPr");
        let beg = m_val(d_pr, "begChr", Some("(")).unwrap_or_default();
        let end = m_val(d_pr, "endChr", Some(")")).unwrap_or_default();
        let e_nodes = node.find_all(NS_M, "e");
        // Per ECMA-376 §22.1.2.93, default sepChr when multiple <m:e> elements are present is "|".
        let default_sep = if e_nodes.len() > 1 {
            Some("|")
        } else {
            Some("")
        };
        let sep = m_val(d_pr, "sepChr", default_sep).unwrap_or_default();

        let mut row = mrow();
        row.set_attr("data-omml-d", "true");
        if !beg.is_empty() {
            let mut mo = MathNode::leaf("mo", &beg);
            mo.set_attr("stretchy", "true");
            row.children.push(mo);
        }
        let mut first = true;
        for e_node in e_nodes {
            if !first && !sep.is_empty() {
                let mut mo = MathNode::leaf("mo", &sep);
                mo.set_attr("stretchy", "true");
                row.children.push(mo);
            }
            first = false;
            row.children.extend(self.children_to_mrow(e_node)?.children);
        }
        if !end.is_empty() {
            let mut mo = MathNode::leaf("mo", &end);
            mo.set_attr("stretchy", "true");
            row.children.push(mo);
        }
        Ok(row)
    }

    // ── Function ──────────────────────────────────────────────────────────

    fn handle_func(&mut self, node: &Element) -> Result<MathNode> {
        let mut row = mrow();
        if let Some(fname) = node.find(NS_M, "fName") {
            row.children.extend(self.children_to_mrow(fname)?.children);
        }
        row.children.push(MathNode::leaf("mo", "\u{2061}")); // invisible function application
        if let Some(e_node) = node.find(NS_M, "e") {
            row.children
                .push(unwrap_mrow(self.children_to_mrow(e_node)?));
        }
        Ok(row)
    }

    // ── N-ary operator ────────────────────────────────────────────────────

    fn handle_nary(&mut self, node: &Element) -> Result<MathNode> {
        let nary_pr = node.find(NS_M, "naryPr");
        let op_char = m_val(nary_pr, "chr", Some("∫")).unwrap_or_default();
        let lim_loc = m_val(nary_pr, "limLoc", None);
        let sub_hide = m_bool(nary_pr, "subHide");
        let sup_hide = m_bool(nary_pr, "supHide");

        let sub_node = if sub_hide {
            None
        } else {
            node.find(NS_M, "sub")
        };
        let sup_node = if sup_hide {
            None
        } else {
            node.find(NS_M, "sup")
        };

        let lo_opt = match sub_node {
            Some(s) => {
                let m = self.children_to_mrow(s)?;
                (!m.children.is_empty()).then(|| unwrap_mrow(m))
            }
            None => None,
        };
        let hi_opt = match sup_node {
            Some(p) => {
                let m = self.children_to_mrow(p)?;
                (!m.children.is_empty()).then(|| unwrap_mrow(m))
            }
            None => None,
        };

        let op_el = MathNode::leaf("mo", &op_char);
        let base = self.child_mrow(node, "e")?;

        let mut op_with_limits = match (lo_opt, hi_opt) {
            (Some(lo), Some(hi)) => mel("munderover", vec![op_el, lo, hi]),
            (Some(lo), None) => mel("munder", vec![op_el, lo]),
            (None, Some(hi)) => mel("mover", vec![op_el, hi]),
            (None, None) => op_el,
        };
        if let Some(loc) = lim_loc.as_deref() {
            op_with_limits.set_attr("data-limloc", loc);
        }
        let mut row = mrow();
        row.children.push(op_with_limits);
        row.children.extend(base.children);
        Ok(row)
    }

    // ── Accent / bar / group character ────────────────────────────────────

    fn handle_acc(&mut self, node: &Element) -> Result<MathNode> {
        let acc_char = m_val(node.find(NS_M, "accPr"), "chr", Some("^")).unwrap_or_default();
        let base = self.child_mrow(node, "e")?;
        let mut accent = mel(
            "mover",
            vec![unwrap_mrow(base), MathNode::leaf("mo", &acc_char)],
        );
        accent.set_attr("accent", "true");
        Ok(accent)
    }

    fn handle_bar(&mut self, node: &Element) -> Result<MathNode> {
        // Per ECMA-376 §22.1.2.13, default pos is "bot" (underline)
        let pos = m_val(node.find(NS_M, "barPr"), "pos", Some("bot")).unwrap_or_default();
        let base = self.child_mrow(node, "e")?;
        let is_top = pos == "top";
        let bar = MathNode::leaf("mo", if is_top { "\u{00AF}" } else { "_" });
        let tag = if is_top { "mover" } else { "munder" };
        Ok(mel(tag, vec![unwrap_mrow(base), bar]))
    }

    fn handle_group_chr(&mut self, node: &Element) -> Result<MathNode> {
        let g_pr = node.find(NS_M, "groupChrPr");
        let chr_text = m_val(g_pr, "chr", Some("\u{23DF}")).unwrap_or_default();
        let pos = m_val(g_pr, "pos", Some("bot")).unwrap_or_default();
        let vert_jc = m_val(g_pr, "vertJc", Some("bot")).unwrap_or_default();
        let base = self.child_mrow(node, "e")?;
        let ch = if chr_text.is_empty() {
            "\u{23DF}"
        } else {
            chr_text.as_str()
        };
        let gch = MathNode::leaf("mo", ch);
        // Distinguish extensible arrow under/over accents from labeled arrow operators via `vertJc`:
        // - `pos="bot"` & `vertJc="top"`: base `<m:e>` sits on baseline, arrow is an under-accent
        // - `pos="top"` & `vertJc="bot"`: base `<m:e>` sits on baseline, arrow is an over-accent
        // - `pos="bot"` & `vertJc="bot"`: arrow sits on baseline, `<m:e>` is the label above
        // - `pos="top"` & `vertJc="top"`: arrow sits on baseline, `<m:e>` is the label below
        let is_arrow = matches!(
            ch,
            "→" | "←"
                | "↔"
                | "⇐"
                | "⇒"
                | "⇔"
                | "⟵"
                | "⟶"
                | "⟷"
                | "⟸"
                | "⟹"
                | "⟺"
                | "⇄"
                | "⇌"
                | "⇋"
                | "⇆"
                | "⇀"
                | "↼"
                | "⇁"
                | "↽"
                | "↦"
        );
        if is_arrow {
            if pos == "bot" && vert_jc == "top" {
                let mut accent = mel("munder", vec![unwrap_mrow(base), gch]);
                accent.set_attr("accentunder", "true");
                return Ok(accent);
            }
            if pos == "top" && vert_jc == "bot" {
                let mut accent = mel("mover", vec![unwrap_mrow(base), gch]);
                accent.set_attr("accent", "true");
                return Ok(accent);
            }
            let tag = if pos == "top" { "munder" } else { "mover" };
            return Ok(mel(tag, vec![gch, unwrap_mrow(base)]));
        }
        let tag = if pos == "top" { "mover" } else { "munder" };
        let mut accent = mel(tag, vec![unwrap_mrow(base), gch]);
        accent.set_attr(
            if pos == "top" {
                "accent"
            } else {
                "accentunder"
            },
            "true",
        );
        Ok(accent)
    }

    // ── Matrix / equation array ───────────────────────────────────────────

    fn handle_matrix(&mut self, node: &Element) -> Result<MathNode> {
        let mut table = MathNode::new("mtable");
        if let Some(mcs) = node.find(NS_M, "mPr").and_then(|pr| pr.find(NS_M, "mcs")) {
            let mut aligns = Vec::new();
            for mc in mcs.find_all(NS_M, "mc") {
                let pr = mc.find(NS_M, "mcPr");
                let align = m_val(pr, "mcJc", Some("center")).unwrap_or_default();
                let count = m_val(pr, "count", Some("1"))
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(1)
                    .min(256);
                aligns.extend(std::iter::repeat_n(align, count));
            }
            if !aligns.is_empty() {
                let alignment = if aligns.iter().all(|a| a == &aligns[0]) {
                    aligns[0].clone()
                } else {
                    aligns.join(" ")
                };
                table.set_attr("columnalign", &alignment);
            }
        }
        for mr in node.find_all(NS_M, "mr") {
            let mut mtr = MathNode::new("mtr");
            // In ECMA-376 (standard Word OMML), <m:mr> directly contains <m:e> elements.
            let e_cells = mr.find_all(NS_M, "e");
            if !e_cells.is_empty() {
                for e_node in e_cells {
                    let mut mtd = MathNode::new("mtd");
                    mtd.children.extend(self.children_to_mrow(e_node)?.children);
                    mtr.children.push(mtd);
                }
            } else {
                // Fallback for non-standard XML where cells are wrapped in <m:mc>
                for mc in mr.find_all(NS_M, "mc") {
                    let mut mtd = MathNode::new("mtd");
                    if let Some(e_node) = mc.find(NS_M, "e") {
                        mtd.children.extend(self.children_to_mrow(e_node)?.children);
                    }
                    mtr.children.push(mtd);
                }
            }
            table.children.push(mtr);
        }
        Ok(table)
    }

    fn handle_eq_arr(&mut self, node: &Element) -> Result<MathNode> {
        let mut table = MathNode::new("mtable");
        for e_node in node.find_all(NS_M, "e") {
            let mut mtd = MathNode::new("mtd");
            mtd.children.extend(self.children_to_mrow(e_node)?.children);
            table.children.push(mel("mtr", vec![mtd]));
        }
        Ok(table)
    }

    // ── Phantom ───────────────────────────────────────────────────────────

    fn handle_phant(&mut self, node: &Element) -> Result<Option<MathNode>> {
        match node.find(NS_M, "e") {
            None => Ok(None),
            Some(e_node) => {
                let inner = self.children_to_mrow(e_node)?;
                Ok(Some(mel("mphantom", vec![unwrap_mrow(inner)])))
            }
        }
    }
}

/// Convert an `m:oMath` element to a MathML `<math>` tree.
pub fn omml_to_mathml(omml_element: &Element) -> Result<MathNode> {
    OmmlConverter { depth: 0 }.convert(omml_element)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mtef::encode::mathml_to_mtef_bytes;
    use crate::mtef::{
        TM_BAR, TM_BOX, TM_DIRAC, TM_FRACT, TM_INTEG, TM_INTERVAL, TM_STRIKE, TM_SUBSUP,
    };
    use crate::xml;

    fn parse_and_encode(omml_body: &str) -> Vec<u8> {
        let xml_str = format!(
            "<m:oMath xmlns:m=\"http://schemas.openxmlformats.org/officeDocument/2006/math\">{}</m:oMath>",
            omml_body
        );
        let el = xml::parse(xml_str.as_bytes()).expect("valid OMML XML");
        let mathml = omml_to_mathml(&el).expect("OMML -> MathML succeeds");
        mathml_to_mtef_bytes(&mathml).expect("MathML -> MTEF succeeds")
    }

    #[test]
    fn test_spre_prescripts_encode() {
        let bytes = parse_and_encode(
            "<m:sPre><m:sub><m:r><m:t>92</m:t></m:r></m:sub><m:sup><m:r><m:t>235</m:t></m:r></m:sup><m:e><m:r><m:t>U</m:t></m:r></m:e></m:sPre>",
        );
        // Should contain a TMPL record with selector TM_SUBSUP (30) and TV_SU_PRECEDES (0x01)
        assert!(bytes
            .windows(4)
            .any(|w| w[0] == 0x03 && w[2] == TM_SUBSUP && (w[3] & 0x01) != 0));
    }

    #[test]
    fn test_border_box_and_strikethrough_encode() {
        let box_bytes =
            parse_and_encode("<m:borderBox><m:e><m:r><m:t>x</m:t></m:r></m:e></m:borderBox>");
        assert!(box_bytes.windows(3).any(|w| w[0] == 0x03 && w[2] == TM_BOX));

        let strike_bytes = parse_and_encode(
            "<m:borderBox><m:borderBoxPr><m:strikeBLTR/></m:borderBoxPr><m:e><m:r><m:t>x</m:t></m:r></m:e></m:borderBox>",
        );
        assert!(strike_bytes
            .windows(3)
            .any(|w| w[0] == 0x03 && w[2] == TM_STRIKE));
    }

    #[test]
    fn test_skewed_fraction_and_nary_subhide() {
        let skw_bytes = parse_and_encode(
            "<m:f><m:fPr><m:type m:val=\"skw\"/></m:fPr><m:num><m:r><m:t>a</m:t></m:r></m:num><m:den><m:r><m:t>b</m:t></m:r></m:den></m:f>",
        );
        assert!(skw_bytes
            .windows(4)
            .any(|w| w[0] == 0x03 && w[2] == TM_FRACT && w[3] == 0x02));

        let nary_bytes = parse_and_encode(
            "<m:nary><m:naryPr><m:chr m:val=\"∫\"/><m:subHide m:val=\"1\"/><m:supHide m:val=\"1\"/></m:naryPr><m:sub><m:r><m:t>a</m:t></m:r></m:sub><m:sup><m:r><m:t>b</m:t></m:r></m:sup><m:e><m:r><m:t>x</m:t></m:r></m:e></m:nary>",
        );
        assert!(nary_bytes
            .windows(3)
            .any(|w| w[0] == 0x03 && w[2] == TM_INTEG));
    }

    #[test]
    fn test_delimiters_dirac_interval_and_abs() {
        // Dirac <ψ|φ>
        let dirac_bytes = parse_and_encode(
            "<m:d><m:dPr><m:begChr m:val=\"⟨\"/><m:sepChr m:val=\"|\"/><m:endChr m:val=\"⟩\"/></m:dPr><m:e><m:r><m:t>ψ</m:t></m:r></m:e><m:e><m:r><m:t>φ</m:t></m:r></m:e></m:d>",
        );
        assert!(dirac_bytes
            .windows(3)
            .any(|w| w[0] == 0x03 && w[2] == TM_DIRAC));

        // Half-open interval [a, b)
        let interval_bytes = parse_and_encode(
            "<m:d><m:dPr><m:begChr m:val=\"[\"/><m:endChr m:val=\")\"/></m:dPr><m:e><m:r><m:t>a,b</m:t></m:r></m:e></m:d>",
        );
        assert!(interval_bytes
            .windows(3)
            .any(|w| w[0] == 0x03 && w[2] == TM_INTERVAL));

        // Absolute value |x|
        let abs_bytes = parse_and_encode(
            "<m:d><m:dPr><m:begChr m:val=\"|\"/><m:endChr m:val=\"|\"/></m:dPr><m:e><m:r><m:t>x</m:t></m:r></m:e></m:d>",
        );
        assert!(abs_bytes.windows(3).any(|w| w[0] == 0x03 && w[2] == TM_BAR));
    }
}
