//! Fence templates: parentheses, brackets, braces and bars that stretch to
//! their content.

use super::*;

impl Layout {
    pub(super) fn stretch_delims(&self, atoms: &mut [Atom], em: f32) {
        // Match pairs.
        let mut stack: Vec<usize> = Vec::new();
        let mut pairs: Vec<(usize, usize)> = Vec::new();
        for i in 0..atoms.len() {
            let Some(c) = atoms[i].delim else { continue };
            if is_open(c) {
                stack.push(i);
            } else if is_close(c) {
                let want = open_for(c);
                if let Some(pos) = stack.iter().rposition(|&j| atoms[j].delim == Some(want)) {
                    pairs.push((stack[pos], i));
                    stack.truncate(pos);
                }
            } else if let Some(&j) = stack.last() {
                if atoms[j].delim == Some(c) {
                    pairs.push((j, i));
                    stack.pop();
                } else {
                    stack.push(i);
                }
            } else {
                stack.push(i);
            }
        }
        let last = atoms.len().saturating_sub(1);
        if last >= 2
            && atoms[0].delim.is_some()
            && atoms[last].delim.is_some()
            && !pairs
                .iter()
                .any(|&(a, b)| a == 0 || b == 0 || a == last || b == last)
        {
            pairs.push((0, last));
        }
        for (j, i) in pairs.iter().copied() {
            let (mut asc, mut desc) = (0.0f32, 0.0f32);
            for a in &atoms[j + 1..i] {
                asc = asc.max(a.b.asc);
                desc = desc.max(a.b.desc);
            }
            self.stretch_pair(atoms, j, asc, desc, em);
            self.stretch_pair(atoms, i, asc, desc, em);
            for k in (j + 1)..i {
                if atoms[k].delim.is_some_and(is_bar)
                    && !pairs.iter().any(|&(a, b)| a == k || b == k)
                {
                    self.stretch_pair(atoms, k, asc, desc, em);
                }
            }
        }
        // Lone `{` before a table (systems of equations) / lone `}` after one.
        for i in 0..atoms.len() {
            let Some(c) = atoms[i].delim else { continue };
            if is_open(c)
                && atoms.get(i + 1).is_some_and(|a| a.is_table)
                && atoms[i].b.asc < 0.9 * em
            {
                let (a, d) = (atoms[i + 1].b.asc, atoms[i + 1].b.desc);
                self.stretch_pair(atoms, i, a, d, em);
            } else if is_close(c) && i > 0 && atoms[i - 1].is_table && atoms[i].b.asc < 0.9 * em {
                let (a, d) = (atoms[i - 1].b.asc, atoms[i - 1].b.desc);
                self.stretch_pair(atoms, i, a, d, em);
            }
        }
        // Unmatched closing delimiters / bars after an expression (e.g. \left. F(x) \right|)
        // and unmatched opening delimiters before an expression (e.g. \left( \frac{1}{2} \right.
        // or \left\llbracket \frac{1}{2} \right.).
        for i in 0..atoms.len() {
            let Some(c) = atoms[i].delim else { continue };
            if pairs
                .iter()
                .any(|&(open_idx, close_idx)| open_idx == i || close_idx == i)
            {
                continue;
            }
            if (is_bar(c) || is_close(c)) && i > 0 && !(is_close(c) && atoms[i - 1].is_table) {
                let mut asc = 0.0f32;
                let mut desc = 0.0f32;
                for a in &atoms[..i] {
                    asc = asc.max(a.b.asc);
                    desc = desc.max(a.b.desc);
                }
                if asc + desc > 1.25 * em {
                    self.stretch_pair(atoms, i, asc, desc, em);
                    continue;
                }
            }
            if (is_open(c)
                || (is_bar(c)
                    && (i == 0
                        || matches!(
                            atoms[i - 1].class,
                            Class::Rel | Class::Op | Class::Bin | Class::Open | Class::Punct
                        ))))
                && i + 1 < atoms.len()
                && !(is_open(c) && atoms[i + 1].is_table)
            {
                let mut asc = 0.0f32;
                let mut desc = 0.0f32;
                for a in &atoms[i + 1..] {
                    asc = asc.max(a.b.asc);
                    desc = desc.max(a.b.desc);
                }
                if asc + desc > 1.25 * em {
                    self.stretch_pair(atoms, i, asc, desc, em);
                }
            }
        }
    }

    pub(super) fn stretch_pair(
        &self,
        atoms: &mut [Atom],
        idx: usize,
        asc: f32,
        desc: f32,
        em: f32,
    ) {
        if asc + desc <= 1.25 * em {
            return;
        }
        let Some(c) = atoms[idx].delim else { return };
        // Symmetric about the math axis.
        let h2 = (asc - AXIS * em).max(desc + AXIS * em);
        let mut stretched = self.delim_box(c, AXIS * em + h2, h2 - AXIS * em, em);
        for mut hit in std::mem::take(&mut atoms[idx].b.hits) {
            hit.w = stretched.w;
            hit.asc = stretched.asc;
            hit.desc = stretched.desc;
            if !hit.carets.is_empty() {
                hit.carets = vec![0.0, stretched.w];
            }
            stretched.hits.push(hit);
        }
        atoms[idx].b = stretched;
    }

    pub(super) fn delim_box(&self, c: char, asc: f32, desc: f32, em: f32) -> LBox {
        // Parentheses, brackets, braces, floors, ceilings, absolute value, and norm are
        // assembled or outline-scaled from font glyphs so they follow the height of their content.
        if let Some(b) = self.stretchy_delimiter(c, asc, desc, em) {
            return b;
        }
        let h = asc + desc;
        let t = (0.05 * em).max(1.0);
        let pad = 0.04 * em;
        let mut b = LBox {
            asc,
            desc,
            ..Default::default()
        };
        let top = -asc;
        match c {
            '|' => {
                b.items.push(Item::Poly(rect(pad + 0.03 * em, top, t, h)));
                b.w = 0.12 * em + 2.0 * pad;
            }
            '‖' | '∥' => {
                b.items.push(Item::Poly(rect(pad, top, t, h)));
                b.items.push(Item::Poly(rect(pad + 0.1 * em, top, t, h)));
                b.w = 0.2 * em + 2.0 * pad;
            }
            '⟨' | '⟩' => {
                let w = 0.28 * em;
                let pts = if c == '⟨' {
                    [(w, top), (0.0, top + h / 2.0), (w, top + h)]
                } else {
                    [(0.0, top), (w, top + h / 2.0), (0.0, top + h)]
                };
                for q in stroke(
                    &pts.iter().map(|(x, y)| (x + pad, *y)).collect::<Vec<_>>(),
                    0.045 * em,
                ) {
                    b.items.push(Item::Poly(q));
                }
                b.w = w + 2.0 * pad;
            }
            other => {
                return self.glyph(display_char(other), Face::Symbol, em);
            }
        }
        b
    }
}

#[cfg(test)]
mod tests {
    use crate::mathml::MathNode;
    use crate::render::render_interactive;
    use crate::service::parse_latex_to_mathml;

    fn node_at<'a>(mut node: &'a MathNode, path: &[usize]) -> Option<&'a MathNode> {
        for &idx in path {
            node = node.children.get(idx)?;
        }
        Some(node)
    }

    #[test]
    fn unmatched_single_sided_fences_stretch_to_tall_content() {
        let font_pt = 12.0;
        for latex in [
            r"\left\llbracket \frac{1}{2} \right.",
            r"\left( \frac{1}{2} \right.",
            r"\left| \frac{1}{2} \right.",
            r"\left. \frac{1}{2} \right\rrbracket",
            r"\left. \frac{1}{2} \right)",
            r"x = \left\llbracket \frac{1}{2} \right.",
        ] {
            let math = parse_latex_to_mathml(latex).expect("parse latex");
            let preview = render_interactive(&math, font_pt).expect("render");
            let delim = preview
                .regions
                .iter()
                .find(|r| {
                    node_at(&math, &r.path).is_some_and(|n| {
                        n.tag == "mo" && matches!(n.text.trim(), "⟦" | "⟧" | "(" | ")" | "|")
                    })
                })
                .unwrap_or_else(|| {
                    panic!("missing delimiter region for {latex}: {}", math.to_xml())
                });
            assert!(
                delim.height > 1.4 * font_pt,
                "{latex}: expected stretched delimiter height > 1.4*font_pt ({}), got {}",
                1.4 * font_pt,
                delim.height
            );
        }
    }
}
