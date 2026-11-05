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

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Read `m:val` of the child `m:<child_tag>` of `node`.
///
/// Mirrors the Python `_m_val`: a missing node or child yields `default`; a
/// child without `m:val` also yields `default`.
fn m_val(node: Option<&Element>, child_tag: &str, default: Option<&str>) -> Option<String> {
    let child = node.and_then(|n| n.find(NS_M, child_tag));
    match child {
        None => default.map(str::to_owned),
        Some(c) => c
            .attr(Some(NS_M), "val")
            .or(default)
            .map(str::to_owned),
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
    !s.is_empty() && s.chars().all(|c| OPS.contains(c) || OP_ALIASES.contains(&c))
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
                Ok(Some(mel("msup", vec![base, sup])))
            }
            "sSubSup" => {
                let base = self.get_e(node)?;
                let sub = self.get_child(node, "sub")?;
                let sup = self.get_child(node, "sup")?;
                Ok(Some(mel("msubsup", vec![base, sub, sup])))
            }
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
            "box" | "borderBox" => match node.find(NS_M, "e") {
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

    // ── Run ───────────────────────────────────────────────────────────────

    fn handle_r(&mut self, node: &Element) -> Option<MathNode> {
        let text = get_text(node);
        if text.is_empty() {
            return None;
        }
        let r_pr = node.find(NS_M, "rPr");
        let sty = m_val(r_pr, "sty", None);
        if sty.as_deref() == Some("p") {
            return Some(MathNode::leaf("mtext", &text));
        }
        let mut el = text_node(&text).unwrap_or_else(|| MathNode::leaf("mi", &text));
        if matches!(sty.as_deref(), Some("b") | Some("bi")) {
            el.set_attr("mathvariant", "bold");
        }
        Some(el)
    }

    // ── Fraction ──────────────────────────────────────────────────────────

    fn handle_f(&mut self, node: &Element) -> Result<MathNode> {
        let num = self.child_mrow(node, "num")?;
        let den = self.child_mrow(node, "den")?;
        let ftype = m_val(node.find(NS_M, "fPr"), "type", None);
        match ftype.as_deref() {
            Some("lin") => Ok(mel(
                "mrow",
                vec![unwrap_mrow(num), MathNode::leaf("mo", "/"), unwrap_mrow(den)],
            )),
            Some("noBar") | Some("skw") => {
                let mut f = mel("mfrac", vec![unwrap_mrow(num), unwrap_mrow(den)]);
                f.set_attr("linethickness", "0");
                Ok(f)
            }
            _ => Ok(mel("mfrac", vec![unwrap_mrow(num), unwrap_mrow(den)])),
        }
    }

    // ── Radical ───────────────────────────────────────────────────────────

    fn handle_rad(&mut self, node: &Element) -> Result<MathNode> {
        let rad = self.child_mrow(node, "e")?;

        let rad_pr = node.find(NS_M, "radPr");
        let deg_hide = m_val(rad_pr, "degHide", Some("0")).unwrap_or_default();
        let is_deg_hidden = deg_hide == "1" || deg_hide == "on" || deg_hide == "true";

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
        let e = self.get_e(node)?;
        let lim = self.child_mrow(node, "lim")?;
        Ok(mel(tag, vec![e, unwrap_mrow(lim)]))
    }

    // ── Delimiter ─────────────────────────────────────────────────────────

    fn handle_d(&mut self, node: &Element) -> Result<MathNode> {
        let d_pr = node.find(NS_M, "dPr");
        let beg = m_val(d_pr, "begChr", Some("(")).unwrap_or_default();
        let end = m_val(d_pr, "endChr", Some(")")).unwrap_or_default();
        let sep = m_val(d_pr, "sepChr", Some("")).unwrap_or_default();

        let mut row = mrow();
        if !beg.is_empty() {
            row.children.push(MathNode::leaf("mo", &beg));
        }
        let mut first = true;
        for e_node in node.find_all(NS_M, "e") {
            if !first && !sep.is_empty() {
                row.children.push(MathNode::leaf("mo", &sep));
            }
            first = false;
            row.children.extend(self.children_to_mrow(e_node)?.children);
        }
        if !end.is_empty() {
            row.children.push(MathNode::leaf("mo", &end));
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
        let op_char = m_val(node.find(NS_M, "naryPr"), "chr", Some("∫")).unwrap_or_default();
        let sub_node = node.find(NS_M, "sub");
        let sup_node = node.find(NS_M, "sup");
        let op_el = MathNode::leaf("mo", &op_char);
        let base = self.child_mrow(node, "e")?;

        let op_with_limits = match (sub_node, sup_node) {
            (Some(s), Some(p)) => {
                let lo = self.children_to_mrow(s)?;
                let hi = self.children_to_mrow(p)?;
                mel("munderover", vec![op_el, unwrap_mrow(lo), unwrap_mrow(hi)])
            }
            (Some(s), None) => {
                let lo = self.children_to_mrow(s)?;
                mel("munder", vec![op_el, unwrap_mrow(lo)])
            }
            (None, Some(p)) => {
                let hi = self.children_to_mrow(p)?;
                mel("mover", vec![op_el, unwrap_mrow(hi)])
            }
            (None, None) => op_el,
        };
        let mut row = mrow();
        row.children.push(op_with_limits);
        row.children.extend(base.children);
        Ok(row)
    }

    // ── Accent / bar / group character ────────────────────────────────────

    fn handle_acc(&mut self, node: &Element) -> Result<MathNode> {
        let acc_char = m_val(node.find(NS_M, "accPr"), "chr", Some("^")).unwrap_or_default();
        let base = self.child_mrow(node, "e")?;
        Ok(mel(
            "mover",
            vec![unwrap_mrow(base), MathNode::leaf("mo", &acc_char)],
        ))
    }

    fn handle_bar(&mut self, node: &Element) -> Result<MathNode> {
        let pos = m_val(node.find(NS_M, "barPr"), "pos", Some("top")).unwrap_or_default();
        let base = self.child_mrow(node, "e")?;
        let bar = MathNode::leaf("mo", "\u{00AF}"); // macron
        let tag = if pos == "bot" { "munder" } else { "mover" };
        Ok(mel(tag, vec![unwrap_mrow(base), bar]))
    }

    fn handle_group_chr(&mut self, node: &Element) -> Result<MathNode> {
        let g_pr = node.find(NS_M, "groupChrPr");
        let chr_text = m_val(g_pr, "chr", Some("\u{23DF}")).unwrap_or_default();
        let pos = m_val(g_pr, "pos", Some("bot")).unwrap_or_default();
        let base = self.child_mrow(node, "e")?;
        let ch = if chr_text.is_empty() {
            "\u{23DF}"
        } else {
            chr_text.as_str()
        };
        let gch = MathNode::leaf("mo", ch);
        let tag = if pos == "top" { "mover" } else { "munder" };
        Ok(mel(tag, vec![unwrap_mrow(base), gch]))
    }

    // ── Matrix / equation array ───────────────────────────────────────────

    fn handle_matrix(&mut self, node: &Element) -> Result<MathNode> {
        let mut table = MathNode::new("mtable");
        for mr in node.find_all(NS_M, "mr") {
            let mut mtr = MathNode::new("mtr");
            // In ECMA-376 (standard Word OMML), <m:mr> directly contains <m:e> elements.
            let e_cells = mr.find_all(NS_M, "e");
            if !e_cells.is_empty() {
                for e_node in e_cells {
                    let mut mtd = MathNode::new("mtd");
                    mtd.children
                        .extend(self.children_to_mrow(e_node)?.children);
                    mtr.children.push(mtd);
                }
            } else {
                // Fallback for non-standard XML where cells are wrapped in <m:mc>
                for mc in mr.find_all(NS_M, "mc") {
                    let mut mtd = MathNode::new("mtd");
                    if let Some(e_node) = mc.find(NS_M, "e") {
                        mtd.children
                            .extend(self.children_to_mrow(e_node)?.children);
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
            mtd.children
                .extend(self.children_to_mrow(e_node)?.children);
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
