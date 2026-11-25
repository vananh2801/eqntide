//! Lay out a single horizontal row (`<mrow>`) of math tokens.
//!
//! Applies TeX-like inter-element spacing based on symbol classification
//! (`Ord`, `Op`, `Bin`, `Rel`, `Open`, `Close`, `Punct`) and standard math
//! spacing constants (thin = 3/18 em, med = 4/18 em, thick = 5/18 em).

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
            let atom_start = atoms.len();
            // Skip empty tokens while keeping explicit text and math spaces.
            let text_clean: String = kid
                .text
                .chars()
                .filter(|ch| !ch.is_whitespace() && *ch != '\u{00A0}' && *ch != '\u{200B}')
                .collect();
            if (kid.tag == "mo" || kid.tag == "mtext")
                && text_clean.is_empty()
                && !(kid.tag == "mtext" && kid.text.contains([' ', '\u{00A0}']))
                && !kid
                    .text
                    .chars()
                    .any(|ch| crate::symbols::misc::unicode_space_em(ch).is_some())
            {
                continue;
            }

            let ssty = ctx.ssty_level();
            match kid.tag.as_str() {
                "mspace" => {
                    atoms.push(Atom {
                        class: Class::Ord,
                        layout_box: self.node(kid, em, ctx)?,
                        delim: None,
                        fence_role: FenceRole::Neutral,
                        is_table: false,
                    });
                }
                "mtext"
                    if kid
                        .text
                        .chars()
                        .all(|ch| crate::symbols::misc::unicode_space_em(ch).is_some()) =>
                {
                    let width: f32 = kid
                        .text
                        .chars()
                        .filter_map(crate::symbols::misc::unicode_space_em)
                        .sum();
                    atoms.push(Atom {
                        class: Class::Ord,
                        layout_box: LBox {
                            w: width * em,
                            ..Default::default()
                        },
                        delim: None,
                        fence_role: FenceRole::Neutral,
                        is_table: false,
                    });
                }
                "mo" => self.atoms_mo(&kid.text, kid.attr("form"), em, &mut atoms, ssty),
                "mi" => self.atoms_mi(kid, em, &mut atoms, ssty),
                "mn" => {
                    // Strip embedded whitespace inside number runs (e.g. "3 5" -> "35").
                    if !text_clean.is_empty() {
                        let face = if kid.attr("mathvariant") == Some("bold") {
                            Face::VectorMatrix
                        } else {
                            Face::Number
                        };
                        atoms.push(Atom {
                            class: Class::Ord,
                            layout_box: self.text_styled(&text_clean, face, em, ssty),
                            delim: None,
                            fence_role: FenceRole::Neutral,
                            is_table: false,
                        });
                    }
                }
                "mtext" => {
                    let text = &kid.text;
                    let digits = text.chars().all(|ch| ch.is_ascii_digit());
                    if !digits && text.chars().count() == 1 && !text_clean.is_empty() {
                        self.atoms_mo(text, None, em, &mut atoms, ssty);
                    } else {
                        let face = if kid.attr("mathvariant") == Some("bold") {
                            Face::VectorMatrix
                        } else if digits {
                            Face::Number
                        } else {
                            Face::Text
                        };
                        let mut layout_box = self.text_styled(text, face, em, ssty);
                        let mut x = 0.0;
                        let mut carets = vec![x];
                        for ch in text.chars() {
                            x += self.font_set.advance_styled(face, ch, em, ssty);
                            carets.push(x);
                        }
                        layout_box.mark_with_carets(kid, carets);
                        if layout_box.asc == 0.0 && layout_box.desc == 0.0 {
                            if let Some(hit) = layout_box.hits.last_mut() {
                                hit.asc = 0.72 * em;
                                hit.desc = 0.22 * em;
                            }
                        }
                        atoms.push(Atom {
                            class: Class::Ord,
                            layout_box,
                            delim: None,
                            fence_role: FenceRole::Neutral,
                            is_table: false,
                        });
                    }
                }
                _ => {
                    let layout_box = if matches!(kid.tag.as_str(), "msub" | "msup" | "msubsup")
                        && kid
                            .children
                            .first()
                            .is_some_and(delimiter::is_eval_bar_base)
                        && !atoms.is_empty()
                    {
                        let start_idx = atoms
                            .iter()
                            .rposition(|atom| matches!(atom.class, Class::Rel | Class::Open))
                            .map_or(0, |idx| idx + 1);
                        let expr_atoms = if start_idx < atoms.len() {
                            &atoms[start_idx..]
                        } else {
                            &atoms[..]
                        };
                        let prev_asc = expr_atoms
                            .iter()
                            .map(|atom| atom.layout_box.asc)
                            .fold(0.0f32, f32::max);
                        let prev_desc = expr_atoms
                            .iter()
                            .map(|atom| atom.layout_box.desc)
                            .fold(0.0f32, f32::max);
                        let half_span = (prev_asc - AXIS * em).max(prev_desc + AXIS * em);
                        let bar_asc = AXIS * em + half_span;
                        let bar_desc = half_span - AXIS * em;
                        let (sub_node, sup_node) = match kid.tag.as_str() {
                            "msub" => (kid.children.get(1), None),
                            "msup" => (None, kid.children.get(1)),
                            _ => (kid.children.get(1), kid.children.get(2)),
                        };
                        self.evaluation_bar(sub_node, sup_node, bar_asc, bar_desc, em, ctx)?
                    } else {
                        self.node(kid, em, ctx)?
                    };
                    let class = self.class_of_node(kid);
                    atoms.push(Atom {
                        class,
                        layout_box,
                        delim: None,
                        fence_role: FenceRole::Neutral,
                        is_table: is_wrapped_table(kid),
                    });
                }
            }
            for atom in &mut atoms[atom_start..] {
                if !atom
                    .layout_box
                    .hits
                    .iter()
                    .any(|hit| hit.key == kid as *const _ as usize)
                {
                    atom.layout_box.mark(kid);
                }
            }
        }
        self.stretch_delims(&mut atoms, em);

        // Downgrade Bin -> Ord where TeX spacing requires it.
        let count = atoms.len();
        let mut classes: Vec<Class> = atoms.iter().map(|atom| atom.class).collect();
        for i in 0..count {
            if classes[i] == Class::Bin {
                let prev_disallows = i == 0
                    || matches!(
                        classes[i - 1],
                        Class::Bin | Class::Rel | Class::Open | Class::Punct | Class::Op
                    );
                let next_disallows = i + 1 == count
                    || matches!(classes[i + 1], Class::Rel | Class::Close | Class::Punct);
                if prev_disallows || next_disallows {
                    classes[i] = Class::Ord;
                }
            }
        }

        let mut out = LBox::default();
        let mut x = 0.0;
        for i in 0..count {
            if i > 0 {
                x += self.spacing(classes[i - 1], classes[i], em);

                // Ensure prime marks clear slanted ascenders or italic hooks.
                let is_prime = atoms[i]
                    .layout_box
                    .items
                    .first()
                    .is_some_and(|item| match item {
                        Item::Glyph { ch, .. } => is_prime_char(*ch),
                        _ => false,
                    });
                if is_prime {
                    let last_glyph = self.last_glyph_of_box(&atoms[i - 1].layout_box);
                    let (overhang, ic) = last_glyph.map_or((0.0, 0.0), |(face, ch, glyph_em)| {
                        let (_, x_max) = self.font_set.ink_x(face, ch, glyph_em);
                        let adv = self.font_set.advance(face, ch, glyph_em);
                        (
                            (x_max - adv).max(0.0),
                            self.font_set.italic_correction(face, ch, glyph_em),
                        )
                    });
                    let clearance = ic.max(overhang);
                    x += clearance.max(0.04 * em) + 0.03 * em;
                }
            }
            out.place(&atoms[i].layout_box, x, 0.0);
            x += atoms[i].layout_box.w;
        }
        out.w = x;
        Ok(out)
    }

    /// Push laid-out operator/delimiter atoms for an `<mo>` text string.
    pub(super) fn atoms_mo(
        &self,
        text: &str,
        form: Option<&str>,
        em: f32,
        out: &mut Vec<Atom>,
        ssty: u8,
    ) {
        for ch in text.trim().chars() {
            if ch == '\u{2061}' || ch.is_whitespace() {
                continue;
            }
            let delim = if is_open(ch) || is_close(ch) || is_bar(ch) {
                Some(ch)
            } else {
                None
            };
            let (fence_role, class) = if delim.is_some() {
                match form {
                    Some("prefix") => (FenceRole::Prefix, Class::Open),
                    Some("postfix") => (FenceRole::Postfix, Class::Close),
                    _ => (FenceRole::Neutral, class_of(ch)),
                }
            } else {
                (FenceRole::Neutral, class_of(ch))
            };
            let layout_box = if is_integral(ch) {
                self.integral_without_limits(ch, 1.45 * em, em)
                    .unwrap_or_else(|| self.glyph_styled(display_char(ch), Face::Symbol, em, ssty))
            } else if is_big_op(ch) {
                self.big_glyph(ch, Face::ExtraMath, em, 1.45)
            } else {
                self.glyph_styled(display_char(ch), Face::Symbol, em, ssty)
            };
            out.push(Atom {
                class,
                layout_box,
                delim,
                fence_role,
                is_table: false,
            });
        }
    }

    /// Push laid-out identifier, function-name, or text atoms for an `<mi>` node.
    pub(super) fn atoms_mi(&self, kid: &MathNode, em: f32, out: &mut Vec<Atom>, ssty: u8) {
        let text = kid.text.trim();
        if text.is_empty() {
            return;
        }
        let variant = kid.attr("mathvariant");
        let bold = matches!(variant, Some("bold") | Some("bold-italic"));
        if text.chars().count() > 1 && is_function(text) {
            let face = if bold {
                Face::VectorMatrix
            } else {
                Face::Function
            };
            out.push(Atom {
                class: Class::Op,
                layout_box: self.text_styled(text, face, em, ssty),
                delim: None,
                fence_role: FenceRole::Neutral,
                is_table: false,
            });
            return;
        }
        if has_foreign_letters(text) {
            let face = if bold { Face::VectorMatrix } else { Face::Text };
            out.push(Atom {
                class: Class::Ord,
                layout_box: self.text_styled(text, face, em, ssty),
                delim: None,
                fence_role: FenceRole::Neutral,
                is_table: false,
            });
            return;
        }
        for ch in text.chars() {
            if ch.is_whitespace() || ch == '\u{00A0}' || ch == '\u{200B}' {
                continue;
            }
            if let Some(styled_ch) = unicode_math_styled_char(ch, variant) {
                out.push(Atom {
                    class: Class::Ord,
                    layout_box: self.glyph_styled(styled_ch, Face::Symbol, em, ssty),
                    delim: None,
                    fence_role: FenceRole::Neutral,
                    is_table: false,
                });
            } else if bold {
                out.push(Atom {
                    class: Class::Ord,
                    layout_box: self.glyph_styled(ch, Face::VectorMatrix, em, ssty),
                    delim: None,
                    fence_role: FenceRole::Neutral,
                    is_table: false,
                });
            } else if ch.is_ascii_digit() {
                // Digits inside <mi> runs must be upright numbers, not math operators.
                out.push(Atom {
                    class: Class::Ord,
                    layout_box: self.glyph_styled(ch, Face::Number, em, ssty),
                    delim: None,
                    fence_role: FenceRole::Neutral,
                    is_table: false,
                });
            } else if is_latin_letter(ch) {
                let face = if variant == Some("normal") {
                    Face::Function
                } else {
                    Face::Variable
                };
                out.push(Atom {
                    class: Class::Ord,
                    layout_box: self.glyph_styled(ch, face, em, ssty),
                    delim: None,
                    fence_role: FenceRole::Neutral,
                    is_table: false,
                });
            } else if is_greek_lower(ch) {
                out.push(Atom {
                    class: Class::Ord,
                    layout_box: self.glyph_styled(ch, Face::LcGreek, em, ssty),
                    delim: None,
                    fence_role: FenceRole::Neutral,
                    is_table: false,
                });
            } else if is_greek_upper(ch) {
                out.push(Atom {
                    class: Class::Ord,
                    layout_box: self.glyph_styled(ch, Face::UcGreek, em, ssty),
                    delim: None,
                    fence_role: FenceRole::Neutral,
                    is_table: false,
                });
            } else {
                self.atoms_mo(&ch.to_string(), None, em, out, ssty);
            }
        }
    }

    fn class_of_node(&self, node: &MathNode) -> Class {
        if node.tag == "mo" {
            if let Some(ch) = node.text.trim().chars().next() {
                return class_of(ch);
            }
        }
        if matches!(
            node.tag.as_str(),
            "munder" | "mover" | "munderover" | "msub" | "msup" | "msubsup"
        ) {
            if let Some(base) = node.children.first() {
                if base.tag == "mo" {
                    if let Some(ch) = base.text.trim().chars().next() {
                        if is_big_op(ch) || is_integral(ch) {
                            return Class::Op;
                        }
                        if arrow::arrow_dir(ch).is_some() {
                            return Class::Rel;
                        }
                    }
                }
            }
        }
        Class::Ord
    }

    /// TeX spacing table in units of `em` (The TeXbook, Ch. 18, p. 170).
    fn spacing(&self, left: Class, right: Class, em: f32) -> f32 {
        const THIN: f32 = 3.0 / 18.0;
        const MED: f32 = 4.0 / 18.0;
        const THICK: f32 = 5.0 / 18.0;
        use Class::*;
        let units = match (left, right) {
            (Ord | Close, Op) | (Op, Ord | Op) => THIN,
            (Ord | Close, Bin) | (Bin, Ord | Op | Open) => MED,
            (Ord | Close | Op, Rel) | (Rel, Ord | Op | Open) => THICK,
            (Punct, Ord | Op | Rel | Open | Punct) => THIN,
            _ => 0.0,
        };
        units * em
    }
}

fn is_wrapped_table(mut node: &MathNode) -> bool {
    while matches!(node.tag.as_str(), "mrow" | "mstyle" | "mpadded" | "math")
        && node.children.len() == 1
    {
        node = &node.children[0];
    }
    node.tag == "mtable"
}

#[cfg(test)]
mod wrapped_table_tests {
    use super::*;
    use crate::render::render_interactive;
    use crate::service::parse_latex_to_mathml;

    #[test]
    fn lone_brace_stretches_identically_around_wrapped_and_direct_tables() {
        let table = "<mtable><mtr><mtd><mfrac><mi>x</mi><mn>2</mn></mfrac></mtd></mtr><mtr><mtd><mn>4</mn></mtd></mtr></mtable>";
        let direct = MathNode::parse(&format!("<math><mo>{{</mo>{table}</math>")).unwrap();
        let wrapped = MathNode::parse(&format!(
            "<math><mo>{{</mo><mrow><mrow>{table}</mrow></mrow></math>"
        ))
        .unwrap();
        let (direct_box, _) = layout_math(&direct, 12.0).unwrap();
        let (wrapped_box, _) = layout_math(&wrapped, 12.0).unwrap();
        assert!(
            (direct_box.w - wrapped_box.w).abs() < 0.01,
            "wrapper must not leave a small brace"
        );
        assert_eq!(direct_box.items.len(), wrapped_box.items.len());
    }

    #[test]
    fn closing_and_opening_fences_receive_tex_operator_spacing() {
        let layout = Layout {
            font_set: fonts(),
            depth: 0,
        };
        let em = 18.0;
        let med = 4.0 / 18.0 * em;
        let thick = 5.0 / 18.0 * em;
        let thin = 3.0 / 18.0 * em;
        assert!((layout.spacing(Class::Close, Class::Bin, em) - med).abs() < 1e-4);
        assert!((layout.spacing(Class::Bin, Class::Open, em) - med).abs() < 1e-4);
        assert!((layout.spacing(Class::Close, Class::Rel, em) - thick).abs() < 1e-4);
        assert!((layout.spacing(Class::Rel, Class::Open, em) - thick).abs() < 1e-4);
        assert!((layout.spacing(Class::Close, Class::Op, em) - thin).abs() < 1e-4);
        assert!((layout.spacing(Class::Punct, Class::Open, em) - thin).abs() < 1e-4);
    }

    #[test]
    fn double_bar_fences_use_open_close_class_without_relation_gap() {
        let layout = Layout {
            font_set: fonts(),
            depth: 0,
        };
        let mut atoms = Vec::new();
        layout.atoms_mo("∥", Some("prefix"), 18.0, &mut atoms, 0);
        layout.atoms_mo("∥", Some("postfix"), 18.0, &mut atoms, 0);
        assert_eq!(atoms[0].fence_role, FenceRole::Prefix);
        assert_eq!(atoms[0].class, Class::Open);
        assert_eq!(atoms[1].fence_role, FenceRole::Postfix);
        assert_eq!(atoms[1].class, Class::Close);

        let norm_math = parse_latex_to_mathml(r"\left\Vert \frac{1}{2}x \right\Vert").unwrap();
        let preview = render_interactive(&norm_math, 12.0).unwrap();
        let left = preview
            .regions
            .iter()
            .find(|region| region.path == [0, 0])
            .unwrap();
        let inner = preview
            .regions
            .iter()
            .find(|region| region.path == [0, 1])
            .unwrap();
        let right = preview
            .regions
            .iter()
            .find(|region| region.path == [0, 2])
            .unwrap();
        let left_gap = inner.x - (left.x + left.width);
        let right_gap = right.x - (inner.x + inner.width);
        assert!(
            left_gap < 0.05 && right_gap < 0.05,
            "unexpected gap inside norm fences: left={left_gap}, right={right_gap}"
        );
    }
}
