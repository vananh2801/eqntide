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
        for (j, i) in pairs.iter().copied() {
            let (mut asc, mut desc) = (0.0f32, 0.0f32);
            for a in &atoms[j + 1..i] {
                asc = asc.max(a.b.asc);
                desc = desc.max(a.b.desc);
            }
            self.stretch_pair(atoms, j, asc, desc, em);
            self.stretch_pair(atoms, i, asc, desc, em);
        }
        // Lone `{` before a table (systems of equations) / lone `}` after one.
        for i in 0..atoms.len() {
            let Some(c) = atoms[i].delim else { continue };
            if is_open(c) && atoms.get(i + 1).is_some_and(|a| a.is_table) && atoms[i].b.asc < 0.9 * em {
                let (a, d) = (atoms[i + 1].b.asc, atoms[i + 1].b.desc);
                self.stretch_pair(atoms, i, a, d, em);
            } else if is_close(c) && i > 0 && atoms[i - 1].is_table && atoms[i].b.asc < 0.9 * em {
                let (a, d) = (atoms[i - 1].b.asc, atoms[i - 1].b.desc);
                self.stretch_pair(atoms, i, a, d, em);
            }
        }
        // Unmatched closing bar / bracket after an expression (e.g. evaluation bar \left. F(x) \right|):
        for i in 0..atoms.len() {
            let Some(c) = atoms[i].delim else { continue };
            if (is_bar(c) || is_close(c)) && !pairs.iter().any(|&(_, close_idx)| close_idx == i) && i > 0 {
                let mut asc = 0.0f32;
                let mut desc = 0.0f32;
                for a in &atoms[..i] {
                    asc = asc.max(a.b.asc);
                    desc = desc.max(a.b.desc);
                }
                if asc + desc > 1.25 * em {
                    self.stretch_pair(atoms, i, asc, desc, em);
                }
            }
        }
    }

    pub(super) fn stretch_pair(&self, atoms: &mut [Atom], idx: usize, asc: f32, desc: f32, em: f32) {
        if asc + desc <= 1.25 * em {
            return;
        }
        let Some(c) = atoms[idx].delim else { return };
        // Symmetric about the math axis.
        let h2 = (asc - AXIS * em).max(desc + AXIS * em);
        atoms[idx].b = self.delim_box(c, AXIS * em + h2, h2 - AXIS * em, em);
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
        let mut b = LBox { asc, desc, ..Default::default() };
        let top = -asc;
        match c {
            '|' => {
                b.items.push(Item::Poly(rect(pad + 0.03 * em, top, t, h)));
                b.w = 0.12 * em + 2.0 * pad;
            }
            '‖' => {
                b.items.push(Item::Poly(rect(pad, top, t, h)));
                b.items.push(Item::Poly(rect(pad + 0.1 * em, top, t, h)));
                b.w = 0.2 * em + 2.0 * pad;
            }
            '⟨' | '⟩' => {
                let w = 0.28 * em;
                let pts = if c == '⟨' { [(w, top), (0.0, top + h / 2.0), (w, top + h)] } else { [(0.0, top), (w, top + h / 2.0), (0.0, top + h)] };
                for q in stroke(&pts.iter().map(|(x, y)| (x + pad, *y)).collect::<Vec<_>>(), 0.045 * em) {
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
