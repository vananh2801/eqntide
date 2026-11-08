//! Layout a single horizontal row (`<mrow>`) of math tokens.
//!
//! Applies TeX-like inter-element spacing based on symbol classification
//! (Ord, Op, Bin, Rel, Open, Close, Punct) and standard math spacing
//! constants (thin space = 3/18 em, med space = 4/18 em, thick space = 5/18 em).

use super::*;

pub(super) fn unicode_math_styled_char(ch: char, mathvariant: Option<&str>) -> Option<char> {
    let cp = match mathvariant? {
        "double-struck" => match ch {
            'C' => 0x2102,
            'H' => 0x210D,
            'N' => 0x2115,
            'P' => 0x2119,
            'Q' => 0x211A,
            'R' => 0x211D,
            'Z' => 0x2124,
            'A'..='Z' => 0x1D538 + (ch as u32 - 'A' as u32),
            'a'..='z' => 0x1D552 + (ch as u32 - 'a' as u32),
            '0'..='9' => 0x1D7D8 + (ch as u32 - '0' as u32),
            _ => return None,
        },
        "fraktur" | "bold-fraktur" => match ch {
            'C' => 0x212D,
            'H' => 0x210C,
            'I' => 0x2111,
            'R' => 0x211C,
            'Z' => 0x2128,
            'A'..='Z' => 0x1D504 + (ch as u32 - 'A' as u32),
            'a'..='z' => 0x1D51E + (ch as u32 - 'a' as u32),
            _ => return None,
        },
        "script" | "bold-script" => match ch {
            'B' => 0x212C,
            'E' => 0x2130,
            'F' => 0x2131,
            'H' => 0x210B,
            'I' => 0x2110,
            'L' => 0x2112,
            'M' => 0x2133,
            'R' => 0x211B,
            'A'..='Z' => 0x1D49C + (ch as u32 - 'A' as u32),
            'e' => 0x212F,
            'g' => 0x210A,
            'o' => 0x2134,
            'a'..='z' => 0x1D4B6 + (ch as u32 - 'a' as u32),
            _ => return None,
        },
        _ => return None,
    };
    char::from_u32(cp)
}

impl Layout {
    /// Lay out a run of siblings on one baseline with TeX-style spacing.
    pub(super) fn row(&mut self, kids: &[MathNode], em: f32, ctx: MathContext) -> Result<LBox> {
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

            let ssty = ctx.ssty_level();
            match kid.tag.as_str() {
                "mo" => self.atoms_mo(&kid.text, em, &mut atoms, ssty),
                "mi" => self.atoms_mi(kid, em, &mut atoms, ssty),
                "mn" => {
                    // Strip embedded whitespace inside number runs (e.g. "3 5" -> "35").
                    if !text_clean.is_empty() {
                        let face = if kid.attr("mathvariant") == Some("bold") {
                            Face::VectorMatrix
                        } else {
                            Face::Number
                        };
                        atoms.push(Atom { class: Class::Ord, b: self.text_styled(&text_clean, face, em, ssty), delim: None, is_table: false });
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
                                b: self.text_styled(t, face, em, ssty),
                                delim: None,
                                is_table: false,
                            });
                        } else if t.chars().count() == 1 {
                            // Single character symbol, operator, relation, or punctuation (e.g. '=', '≈', ',')
                            self.atoms_mo(t, em, &mut atoms, ssty);
                        } else {
                            // Normal text words (Vietnamese words, text labels); preserve boundary spaces
                            let face = if bold { Face::VectorMatrix } else { Face::Text };
                            atoms.push(Atom {
                                class: Class::Ord,
                                b: self.text_styled(&kid.text, face, em, ssty),
                                delim: None,
                                is_table: false,
                            });
                        }
                    }
                }
                _ => {
                    let b = if matches!(kid.tag.as_str(), "msub" | "msup" | "msubsup")
                        && kid.children.first().map_or(false, delimiter::is_eval_bar_base)
                        && !atoms.is_empty()
                    {
                        let start_idx = atoms.iter().rposition(|a| matches!(a.class, Class::Rel | Class::Open)).map_or(0, |idx| idx + 1);
                        let expr_atoms = if start_idx < atoms.len() { &atoms[start_idx..] } else { &atoms[..] };
                        let prev_asc = expr_atoms.iter().map(|a| a.b.asc).fold(0.0f32, f32::max);
                        let prev_desc = expr_atoms.iter().map(|a| a.b.desc).fold(0.0f32, f32::max);
                        let h2 = (prev_asc - AXIS * em).max(prev_desc + AXIS * em);
                        let bar_asc = AXIS * em + h2;
                        let bar_desc = h2 - AXIS * em;
                        let (sub_n, sup_n) = match kid.tag.as_str() {
                            "msub" => (kid.children.get(1), None),
                            "msup" => (None, kid.children.get(1)),
                            _ => (kid.children.get(1), kid.children.get(2)),
                        };
                        self.evaluation_bar(sub_n, sup_n, bar_asc, bar_desc, em, ctx)?
                    } else {
                        self.node(kid, em, ctx)?
                    };
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
                x += self.spacing(cls[i - 1], cls[i], em);
            }
            out.place(&atoms[i].b, x, 0.0);
            x += atoms[i].b.w;
        }
        out.w = x;
        Ok(out)
    }

    /// Classification of an element for spacing purposes.
    fn class_of_node(&self, k: &MathNode) -> Class {
        if k.tag == "mo" {
            if let Some(c) = k.text.trim().chars().next() {
                return class_of(c);
            }
        }
        if matches!(k.tag.as_str(), "munder" | "mover" | "munderover" | "msub" | "msup" | "msubsup") {
            if let Some(base) = k.children.first() {
                if base.tag == "mo" {
                    if let Some(c) = base.text.trim().chars().next() {
                        if is_big_op(c) || is_integral(c) {
                            return Class::Op;
                        }
                    }
                }
            }
        }
        Class::Ord
    }

    /// TeX spacing table in units of `em`.
    fn spacing(&self, left: Class, right: Class, em: f32) -> f32 {
        const THIN: f32 = 3.0 / 18.0;
        const MED: f32 = 4.0 / 18.0;
        const THICK: f32 = 5.0 / 18.0;
        use Class::*;
        let t = match (left, right) {
            (Ord, Op) | (Op, Ord) | (Op, Op) => THIN,
            (Ord, Bin) | (Bin, Ord) | (Bin, Op) => MED,
            (Ord, Rel) | (Rel, Ord) | (Rel, Op) | (Op, Rel) => THICK,
            (Punct, Ord) | (Punct, Op) | (Punct, Rel) | (Punct, Punct) => THIN,
            (Op, Open) | (Ord, Ord) => 0.0,
            _ => 0.0,
        };
        t * em
    }

    pub(super) fn atoms_mo(&self, text: &str, em: f32, out: &mut Vec<Atom>, ssty: u8) {
        for ch in text.trim().chars() {
            if ch == '\u{2061}' || ch.is_whitespace() {
                continue;
            }
            let class = class_of(ch);
            let delim = if is_open(ch) || is_close(ch) || is_bar(ch) { Some(ch) } else { None };
            let b = if is_integral(ch) {
                self.integral_without_limits(ch, 1.45 * em, em).unwrap_or_else(|| self.glyph_styled(display_char(ch), Face::Symbol, em, ssty))
            } else if is_big_op(ch) {
                self.big_glyph(ch, Face::ExtraMath, em, 1.45)
            } else {
                self.glyph_styled(display_char(ch), Face::Symbol, em, ssty)
            };
            out.push(Atom { class, b, delim, is_table: false });
        }
    }

    pub(super) fn atoms_mi(&self, kid: &MathNode, em: f32, out: &mut Vec<Atom>, ssty: u8) {
        let t = kid.text.trim();
        if t.is_empty() {
            return;
        }
        let variant = kid.attr("mathvariant");
        let bold = matches!(variant, Some("bold") | Some("bold-italic"));
        if t.chars().count() > 1 && is_function(t) {
            let face = if bold { Face::VectorMatrix } else { Face::Function };
            out.push(Atom { class: Class::Op, b: self.text_styled(t, face, em, ssty), delim: None, is_table: false });
            return;
        }
        if has_foreign_letters(t) {
            // A word with Vietnamese letters, not a run of variables.
            let face = if bold { Face::VectorMatrix } else { Face::Text };
            out.push(Atom { class: Class::Ord, b: self.text_styled(t, face, em, ssty), delim: None, is_table: false });
            return;
        }
        for ch in t.chars() {
            // Skip standard whitespace, zero-width spaces, and non-breaking spaces.
            if ch.is_whitespace() || ch == '\u{00A0}' || ch == '\u{200B}' {
                continue;
            }
            if let Some(styled_ch) = unicode_math_styled_char(ch, variant) {
                out.push(Atom {
                    class: Class::Ord,
                    b: self.glyph_styled(styled_ch, Face::Symbol, em, ssty),
                    delim: None,
                    is_table: false,
                });
            } else if bold {
                out.push(Atom { class: Class::Ord, b: self.glyph_styled(ch, Face::VectorMatrix, em, ssty), delim: None, is_table: false });
            } else if ch.is_ascii_digit() {
                // Digits inside <mi> runs must be upright numbers, not math operators.
                out.push(Atom { class: Class::Ord, b: self.glyph_styled(ch, Face::Number, em, ssty), delim: None, is_table: false });
            } else if is_latin_letter(ch) {
                let face = if variant == Some("normal") {
                    Face::Function
                } else {
                    Face::Variable
                };
                out.push(Atom { class: Class::Ord, b: self.glyph_styled(ch, face, em, ssty), delim: None, is_table: false });
            } else if is_greek_lower(ch) {
                out.push(Atom { class: Class::Ord, b: self.glyph_styled(ch, Face::LcGreek, em, ssty), delim: None, is_table: false });
            } else if is_greek_upper(ch) {
                out.push(Atom { class: Class::Ord, b: self.glyph_styled(ch, Face::UcGreek, em, ssty), delim: None, is_table: false });
            } else {
                self.atoms_mo(&ch.to_string(), em, out, ssty);
            }
        }
    }
}
