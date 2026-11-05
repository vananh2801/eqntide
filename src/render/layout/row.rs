//! Rows of atoms: spacing between neighbours (MathType "spaces and ellipses"
//! behaviour) and the conversion of token elements (`mi`, `mn`, `mo`,
//! `mtext`) into atoms.

use super::*;


impl Layout {
    /// Lay out a run of siblings on one baseline with TeX-style spacing.
    pub(super) fn row(&mut self, kids: &[MathNode], em: f32, script: bool) -> Result<LBox> {
        let mut atoms: Vec<Atom> = Vec::new();
        for kid in kids {
            // Ignore standalone spacing elements or empty token nodes.
            let text_clean: String = kid
                .text
                .chars()
                .filter(|c| !c.is_whitespace() && *c != '\u{00A0}' && *c != '\u{200B}')
                .collect();
            if kid.tag == "mspace" || ((kid.tag == "mo" || kid.tag == "mtext") && text_clean.is_empty()) {
                continue;
            }

            match kid.tag.as_str() {
                "mo" => self.atoms_mo(&kid.text, em, &mut atoms),
                "mi" => self.atoms_mi(kid, em, &mut atoms),
                "mn" => {
                    // Strip embedded whitespace inside number runs (e.g. "3 5" -> "35").
                    if !text_clean.is_empty() {
                        let face = if kid.attr("mathvariant") == Some("bold") {
                            Face::VectorMatrix
                        } else {
                            Face::Number
                        };
                        atoms.push(Atom { class: Class::Ord, b: self.text(&text_clean, face, em), delim: None, is_table: false });
                    }
                }
                "mtext" => {
                    let t = kid.text.trim();
                    if !t.is_empty() {
                        let bold = kid.attr("mathvariant") == Some("bold");
                        // If it's a sequence of digits (e.g. "2025" or a single digit "2")
                        if t.chars().all(|c| c.is_ascii_digit()) {
                            let face = if bold { Face::VectorMatrix } else { Face::Number };
                            atoms.push(Atom {
                                class: Class::Ord, // Ord + Ord gets 0.0 spacing (tightly packed)
                                b: self.text(t, face, em),
                                delim: None,
                                is_table: false,
                            });
                        } else if t.chars().count() == 1 {
                            // Single character symbol, operator, relation, or punctuation (e.g. '=', '≈', ',')
                            self.atoms_mo(t, em, &mut atoms);
                        } else {
                            // Normal text words (Vietnamese words, text labels)
                            let face = if bold { Face::VectorMatrix } else { Face::Text };
                            atoms.push(Atom {
                                class: Class::Ord,
                                b: self.text(t, face, em),
                                delim: None,
                                is_table: false,
                            });
                        }
                    }
                }
                _ => {
                    let b = self.node(kid, em, script)?;
                    let class = self.class_of_node(kid);
                    atoms.push(Atom { class, b, delim: None, is_table: kid.tag == "mtable" });
                }
            }
        }
        self.stretch_delims(&mut atoms, em);

        // Bin → Ord where TeX demands it.
        let n = atoms.len();
        let mut cls: Vec<Class> = atoms.iter().map(|a| a.class).collect();
        for i in 0..n {
            if cls[i] == Class::Bin {
                let prev_bad = i == 0 || matches!(cls[i - 1], Class::Bin | Class::Rel | Class::Open | Class::Punct | Class::Op);
                let next_bad = i + 1 == n || matches!(cls[i + 1], Class::Rel | Class::Close | Class::Punct);
                if prev_bad || next_bad {
                    cls[i] = Class::Ord;
                }
            }
        }

        let mut out = LBox::default();
        let mut x = 0.0;
        for i in 0..n {
            if i > 0 {
                let mut sp = self.spacing(cls[i - 1], cls[i], em, script);

                // Add italic correction when a prime/apostrophe follows an italic letter (especially 'f')
                let prev_glyph = atoms[i - 1].b.items.last();
                let curr_glyph = atoms[i].b.items.first();

                let is_prime = curr_glyph.map_or(false, |it| match it {
                    Item::Glyph { ch, .. } => matches!(*ch, '\'' | '\u{2032}' | '\u{2033}' | '\u{2034}'),
                    _ => false,
                });

                if is_prime {
                    match prev_glyph {
                        Some(Item::Glyph { ch: 'f', face: Face::Variable, .. }) => {
                            sp += 0.15 * em; // Generous clearance for the overhanging top hook of italic 'f'
                        }
                        Some(Item::Glyph { face: Face::Variable, .. }) => {
                            sp += 0.05 * em; // Slight clearance for other italic variables
                        }
                        _ => {}
                    }
                }

                x += sp;
            }
            out.place(&atoms[i].b, x, 0.0);
            x += atoms[i].b.w;
        }

        out.w = x;
        Ok(out)
    }

    pub(super) fn class_of_node(&self, k: &MathNode) -> Class {
        if arrow::template_of(k).is_some() {
            return Class::Rel; // a labeled arrow spaces like a relation
        }
        if matches!(k.tag.as_str(), "munder" | "mover" | "munderover" | "msub" | "msup" | "msubsup") {
            if let Some(b) = k.children.first() {
                let t = b.text.trim();
                if b.tag == "mo" && t.chars().next().map_or(false, |c| is_big_op(c) || is_integral(c)) {
                    return Class::Op;
                }
                if b.tag == "mi" && is_function(t) {
                    return Class::Op;
                }
            }
        }
        Class::Ord
    }

    pub(super) fn spacing(&self, l: Class, r: Class, em: f32, script: bool) -> f32 {
        use Class::*;
        if script {
            return 0.0;
        }
        if l == Open || r == Close {
            return 0.0;
        }
        let l = if l == Close { Ord } else { l };
        let r = if r == Open { Ord } else { r };
        let t = match (l, r) {
            (Ord, Op) | (Op, Ord) | (Op, Op) => THIN,
            (Ord, Bin) | (Bin, Ord) | (Bin, Op) => MED,
            (Ord, Rel) | (Rel, Ord) | (Rel, Op) | (Op, Rel) => THICK,
            (Punct, Ord) | (Punct, Op) | (Punct, Rel) | (Punct, Punct) => THIN,
            (Op, Open) | (Ord, Ord) => 0.0,
            _ => 0.0,
        };
        t * em
    }

    pub(super) fn atoms_mo(&self, text: &str, em: f32, out: &mut Vec<Atom>) {
        for ch in text.trim().chars() {
            if ch == '\u{2061}' || ch.is_whitespace() {
                continue;
            }
            let class = class_of(ch);
            let delim = if is_open(ch) || is_close(ch) || is_bar(ch) { Some(ch) } else { None };
            let b = if is_integral(ch) {
                self.integral_without_limits(ch, 1.45 * em, em).unwrap()
            } else if is_big_op(ch) {
                self.big_glyph(ch, Face::ExtraMath, em, 1.45)
            } else {
                self.glyph(display_char(ch), Face::Symbol, em)
            };
            out.push(Atom { class, b, delim, is_table: false });
        }
    }

    pub(super) fn atoms_mi(&self, kid: &MathNode, em: f32, out: &mut Vec<Atom>) {
        let t = kid.text.trim();
        if t.is_empty() {
            return;
        }
        let bold = kid.attr("mathvariant") == Some("bold");
        if t.chars().count() > 1 && is_function(t) {
            let face = if bold { Face::VectorMatrix } else { Face::Function };
            out.push(Atom { class: Class::Op, b: self.text(t, face, em), delim: None, is_table: false });
            return;
        }
        if has_foreign_letters(t) {
            // A word with Vietnamese letters, not a run of variables.
            let face = if bold { Face::VectorMatrix } else { Face::Text };
            out.push(Atom { class: Class::Ord, b: self.text(t, face, em), delim: None, is_table: false });
            return;
        }
        for ch in t.chars() {
            // Skip standard whitespace, zero-width spaces, and non-breaking spaces.
            if ch.is_whitespace() || ch == '\u{00A0}' || ch == '\u{200B}' {
                continue;
            }
            if bold {
                out.push(Atom { class: Class::Ord, b: self.glyph(ch, Face::VectorMatrix, em), delim: None, is_table: false });
            } else if ch.is_ascii_digit() {
                // Digits inside <mi> runs must be upright numbers, not math operators.
                out.push(Atom { class: Class::Ord, b: self.glyph(ch, Face::Number, em), delim: None, is_table: false });
            } else if is_latin_letter(ch) {
                out.push(Atom { class: Class::Ord, b: self.glyph(ch, Face::Variable, em), delim: None, is_table: false });
            } else if is_greek_lower(ch) {
                out.push(Atom { class: Class::Ord, b: self.glyph(ch, Face::LcGreek, em), delim: None, is_table: false });
            } else if is_greek_upper(ch) {
                out.push(Atom { class: Class::Ord, b: self.glyph(ch, Face::UcGreek, em), delim: None, is_table: false });
            } else {
                self.atoms_mo(&ch.to_string(), em, out);
            }
        }
    }
}
