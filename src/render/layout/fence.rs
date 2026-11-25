//! Fence templates: parentheses, brackets, braces, and bars that stretch to their content.

use super::*;

impl Layout {
    /// Pair delimiters across `atoms` and stretch them to enclose their spans or adjacent tables.
    pub(super) fn stretch_delims(&self, atoms: &mut [Atom], em: f32) {
        let mut stack: Vec<usize> = Vec::new();
        let mut pairs: Vec<(usize, usize)> = Vec::new();
        for i in 0..atoms.len() {
            let Some(delim_ch) = atoms[i].delim else {
                continue;
            };
            match atoms[i].fence_role {
                FenceRole::Prefix => stack.push(i),
                FenceRole::Postfix => {
                    let want = open_for(delim_ch);
                    if let Some(pos) = stack
                        .iter()
                        .rposition(|&j| atoms[j].delim == Some(want))
                        .or_else(|| {
                            stack
                                .iter()
                                .rposition(|&j| atoms[j].fence_role == FenceRole::Prefix)
                        })
                    {
                        pairs.push((stack[pos], i));
                        stack.truncate(pos);
                    }
                }
                FenceRole::Neutral => {
                    if is_open(delim_ch) {
                        stack.push(i);
                    } else if is_close(delim_ch) {
                        let want = open_for(delim_ch);
                        if let Some(pos) = stack.iter().rposition(|&j| atoms[j].delim == Some(want))
                        {
                            pairs.push((stack[pos], i));
                            stack.truncate(pos);
                        }
                    } else if let Some(&j) = stack.last() {
                        if atoms[j].delim == Some(delim_ch) {
                            atoms[j].class = Class::Open;
                            atoms[i].class = Class::Close;
                            pairs.push((j, i));
                            stack.pop();
                        } else {
                            stack.push(i);
                        }
                    } else {
                        stack.push(i);
                    }
                }
            }
        }
        let last = atoms.len().saturating_sub(1);
        if last >= 2
            && atoms[0].delim.is_some()
            && atoms[last].delim.is_some()
            && !pairs.iter().any(|&(open_idx, close_idx)| {
                open_idx == 0 || close_idx == 0 || open_idx == last || close_idx == last
            })
        {
            if atoms[0].delim.is_some_and(is_bar) {
                atoms[0].class = Class::Open;
            }
            if atoms[last].delim.is_some_and(is_bar) {
                atoms[last].class = Class::Close;
            }
            pairs.push((0, last));
        }
        for (j, i) in pairs.iter().copied() {
            let (mut asc, mut desc) = (0.0f32, 0.0f32);
            for atom in &atoms[j + 1..i] {
                asc = asc.max(atom.layout_box.asc);
                desc = desc.max(atom.layout_box.desc);
            }
            self.stretch_pair(atoms, j, asc, desc, em);
            self.stretch_pair(atoms, i, asc, desc, em);
            for mid_idx in (j + 1)..i {
                if atoms[mid_idx].delim.is_some_and(is_bar)
                    && !pairs
                        .iter()
                        .any(|&(open_idx, close_idx)| open_idx == mid_idx || close_idx == mid_idx)
                {
                    self.stretch_pair(atoms, mid_idx, asc, desc, em);
                }
            }
        }
        // Lone `{` before a table (systems of equations) or lone `}` after one.
        for i in 0..atoms.len() {
            let Some(delim_ch) = atoms[i].delim else {
                continue;
            };
            let opens_next = atoms[i].fence_role == FenceRole::Prefix || is_open(delim_ch);
            let closes_prev = atoms[i].fence_role == FenceRole::Postfix || is_close(delim_ch);
            if opens_next
                && atoms.get(i + 1).is_some_and(|atom| atom.is_table)
                && atoms[i].layout_box.asc < 0.9 * em
            {
                let (asc, desc) = (atoms[i + 1].layout_box.asc, atoms[i + 1].layout_box.desc);
                self.stretch_pair(atoms, i, asc, desc, em);
            } else if closes_prev
                && i > 0
                && atoms[i - 1].is_table
                && atoms[i].layout_box.asc < 0.9 * em
            {
                let (asc, desc) = (atoms[i - 1].layout_box.asc, atoms[i - 1].layout_box.desc);
                self.stretch_pair(atoms, i, asc, desc, em);
            }
        }
        // Unmatched closing delimiters / bars after an expression (e.g. \left. F(x) \right|)
        // and unmatched opening delimiters before an expression (e.g. \left( \frac{1}{2} \right.).
        for i in 0..atoms.len() {
            let Some(delim_ch) = atoms[i].delim else {
                continue;
            };
            if pairs
                .iter()
                .any(|&(open_idx, close_idx)| open_idx == i || close_idx == i)
            {
                continue;
            }
            let closes_prev = atoms[i].fence_role == FenceRole::Postfix || is_close(delim_ch);
            let opens_next = atoms[i].fence_role == FenceRole::Prefix || is_open(delim_ch);
            let is_closing_delim = atoms[i].fence_role == FenceRole::Postfix
                || (atoms[i].fence_role == FenceRole::Neutral
                    && (is_bar(delim_ch) || is_close(delim_ch)));
            if is_closing_delim && i > 0 && !(closes_prev && atoms[i - 1].is_table) {
                let mut asc = 0.0f32;
                let mut desc = 0.0f32;
                for atom in &atoms[..i] {
                    asc = asc.max(atom.layout_box.asc);
                    desc = desc.max(atom.layout_box.desc);
                }
                if asc + desc > 1.25 * em {
                    self.stretch_pair(atoms, i, asc, desc, em);
                    continue;
                }
            }
            let is_opening_delim = atoms[i].fence_role == FenceRole::Prefix
                || (atoms[i].fence_role == FenceRole::Neutral
                    && (is_open(delim_ch)
                        || (is_bar(delim_ch)
                            && (i == 0
                                || matches!(
                                    atoms[i - 1].class,
                                    Class::Rel
                                        | Class::Op
                                        | Class::Bin
                                        | Class::Open
                                        | Class::Punct
                                )))));
            if is_opening_delim && i + 1 < atoms.len() && !(opens_next && atoms[i + 1].is_table) {
                let mut asc = 0.0f32;
                let mut desc = 0.0f32;
                for atom in &atoms[i + 1..] {
                    asc = asc.max(atom.layout_box.asc);
                    desc = desc.max(atom.layout_box.desc);
                }
                if asc + desc > 1.25 * em {
                    self.stretch_pair(atoms, i, asc, desc, em);
                }
            }
        }
    }

    /// Replace the delimiter at `idx` with a vertically stretched box covering `(asc, desc)`.
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
        let Some(delim_ch) = atoms[idx].delim else {
            return;
        };
        // Center delimiter symmetry around the math axis.
        let half_span = (asc - AXIS * em).max(desc + AXIS * em);
        let mut stretched =
            self.delim_box(delim_ch, AXIS * em + half_span, half_span - AXIS * em, em);
        for mut hit in std::mem::take(&mut atoms[idx].layout_box.hits) {
            hit.w = stretched.w;
            hit.asc = stretched.asc;
            hit.desc = stretched.desc;
            if !hit.carets.is_empty() {
                hit.carets = vec![0.0, stretched.w];
            }
            stretched.hits.push(hit);
        }
        atoms[idx].layout_box = stretched;
    }

    /// Construct a stretched delimiter box for `ch`, falling back to vector bars or angles.
    pub(super) fn delim_box(&self, ch: char, asc: f32, desc: f32, em: f32) -> LBox {
        if let Some(layout_box) = self.stretchy_delimiter(ch, asc, desc, em) {
            return layout_box;
        }
        let height = asc + desc;
        let thick = (0.05 * em).max(1.0);
        let pad = 0.04 * em;
        let mut layout_box = LBox {
            asc,
            desc,
            ..Default::default()
        };
        let top = -asc;
        match ch {
            '|' => {
                layout_box
                    .items
                    .push(Item::Poly(rect(pad + 0.03 * em, top, thick, height)));
                layout_box.w = 0.12 * em + 2.0 * pad;
            }
            '‖' | '∥' => {
                layout_box
                    .items
                    .push(Item::Poly(rect(pad, top, thick, height)));
                layout_box
                    .items
                    .push(Item::Poly(rect(pad + 0.1 * em, top, thick, height)));
                layout_box.w = 0.2 * em + 2.0 * pad;
            }
            '⟨' | '⟩' => {
                let width = 0.28 * em;
                let points = if ch == '⟨' {
                    [
                        (width, top),
                        (0.0, top + height / 2.0),
                        (width, top + height),
                    ]
                } else {
                    [(0.0, top), (width, top + height / 2.0), (0.0, top + height)]
                };
                for quad in stroke(
                    &points
                        .iter()
                        .map(|(x, y)| (x + pad, *y))
                        .collect::<Vec<_>>(),
                    0.045 * em,
                ) {
                    layout_box.items.push(Item::Poly(quad));
                }
                layout_box.w = width + 2.0 * pad;
            }
            other => {
                return self.glyph(display_char(other), Face::Symbol, em);
            }
        }
        layout_box
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
    fn matched_norms_bars_and_interval_fences_stretch_equally() {
        let font_pt = 12.0;
        for latex in [
            r"\left\Vert \frac{1}{2}x \right\Vert",
            r"a = \left\Vert \frac{1}{2}x \right\Vert",
            r"\left| \frac{1}{2}x \right|",
            r"a = \left] \frac{1}{2}, x \right[",
            r"a = \left[ \frac{1}{2}, x \right[",
            r"a = \left] \frac{1}{2}, x \right]",
        ] {
            let math = parse_latex_to_mathml(latex).expect("parse latex");
            let preview = render_interactive(&math, font_pt).expect("render");
            let delims: Vec<_> = preview
                .regions
                .iter()
                .filter(|region| {
                    node_at(&math, &region.path).is_some_and(|node| {
                        node.tag == "mo" && matches!(node.text.trim(), "∥" | "‖" | "|" | "[" | "]")
                    })
                })
                .collect();
            assert_eq!(
                delims.len(),
                2,
                "{latex}: expected 2 delimiter regions, got {} in {}",
                delims.len(),
                math.to_xml()
            );
            assert!(
                delims[0].height > 1.4 * font_pt && delims[1].height > 1.4 * font_pt,
                "{latex}: expected both delimiters > {}, got left={}, right={}",
                1.4 * font_pt,
                delims[0].height,
                delims[1].height
            );
            assert!(
                (delims[0].height - delims[1].height).abs() < 0.05,
                "{latex}: expected matching delimiter heights, got left={}, right={}",
                delims[0].height,
                delims[1].height
            );
        }
    }

    #[test]
    fn unmatched_single_sided_fences_stretch_to_tall_content() {
        let font_pt = 12.0;
        for latex in [
            r"\left\llbracket \frac{1}{2} \right.",
            r"\left( \frac{1}{2} \right.",
            r"\left| \frac{1}{2} \right.",
            r"\left] \frac{1}{2} \right.",
            r"x = \left] \frac{1}{2} \right.",
            r"\left. \frac{1}{2} \right\rrbracket",
            r"\left. \frac{1}{2} \right)",
            r"\left. \frac{1}{2} \right[",
            r"x = \left\llbracket \frac{1}{2} \right.",
        ] {
            let math = parse_latex_to_mathml(latex).expect("parse latex");
            let preview = render_interactive(&math, font_pt).expect("render");
            let delim = preview
                .regions
                .iter()
                .find(|region| {
                    node_at(&math, &region.path).is_some_and(|node| {
                        node.tag == "mo"
                            && matches!(node.text.trim(), "⟦" | "⟧" | "(" | ")" | "|" | "[" | "]")
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
