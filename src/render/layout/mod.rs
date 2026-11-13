//! TeX-style box layout of a MathML tree, organised like MathType's palettes.
//!
//! `mod.rs` holds the shared state and the tree walk (`node`); every child
//! module implements one MathType palette group as an `impl Layout` block.
//!
//! | MathType palette            | Module          |
//! |-----------------------------|-----------------|
//! | Spaces and ellipses         | `row` (spacing) |
//! | Embellishments              | `embellish`     |
//! | Fence templates             | `fence`         |
//! | Fraction and radical        | `fraction`, `radical` |
//! | Subscript and superscript   | `script`        |
//! | Summation / product / set   | `bigop`         |
//! | Integral templates          | `integral`      |
//! | Underbar and overbar        | `bar`           |
//! | Matrix templates            | `matrix`        |
//!
//! The symbol palettes (relations, operators, arrows, logic, sets, Greek, …)
//! are plain data and live in [`crate::symbols`].
//!
//! Sizes follow MathType's defaults: fraction parts stay at full size,
//! sub/superscripts are scaled according to `STIXTwoMath`'s `script_percent_scale_down`.

mod arrow;
mod bar;
mod bigop;
mod boxes;
mod delimiter;
mod embellish;
mod fence;
mod fraction;
mod geom;
mod integral;
mod matrix;
mod radical;
mod row;
mod script;

#[allow(unused_imports)]
use crate::error::{Error, Result};
use crate::mathml::MathNode;
use crate::render::font::{fonts, Face, Fonts};
use crate::render::PX_PER_PT;
#[allow(unused_imports)]
use crate::symbols::{
    class_of, display_char, has_foreign_letters, is_accent, is_bar, is_big_op, is_close,
    is_function, is_greek_lower, is_greek_upper, is_integral, is_latin_letter, is_open,
    is_prime_char, open_for, Class,
};

pub(crate) use boxes::{Item, LBox};
use geom::{clip_poly_y, quad_bezier, rect, stroke};

pub(crate) const MAX_DEPTH: usize = 256;
pub(crate) const AXIS: f32 = 0.258;

/// TeX / OpenType MATH style levels (The TeXbook, Appendix G, Rule 1).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MathVariantStyle {
    /// Display style (D): large unconstrained equations on their own line.
    Display,
    /// Text style (T): equations inline within a paragraph.
    Text,
    /// Script style (S): first-level subscripts, superscripts, and fraction parts.
    Script,
    /// ScriptScript style (SS): second-level (nested) scripts and root indices.
    ScriptScript,
}

/// The typesetting context tracking the active `MathVariantStyle` and whether
/// the style is "cramped" (e.g. inside denominators or under radical signs).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MathContext {
    pub style: MathVariantStyle,
    pub cramped: bool,
}

impl MathContext {
    pub const DISPLAY: Self = Self {
        style: MathVariantStyle::Display,
        cramped: false,
    };

    #[allow(dead_code)]
    pub const DISPLAY_CRAMPED: Self = Self {
        style: MathVariantStyle::Display,
        cramped: true,
    };

    pub const TEXT: Self = Self {
        style: MathVariantStyle::Text,
        cramped: false,
    };

    #[allow(dead_code)]
    pub const TEXT_CRAMPED: Self = Self {
        style: MathVariantStyle::Text,
        cramped: true,
    };

    /// Whether this context is display style (un-cramped display).
    #[inline]
    pub fn is_display(&self) -> bool {
        self.style == MathVariantStyle::Display && !self.cramped
    }

    /// Whether this context is script or scriptscript level.
    #[inline]
    pub fn is_script(&self) -> bool {
        matches!(
            self.style,
            MathVariantStyle::Script | MathVariantStyle::ScriptScript
        )
    }

    /// Optical size alternate level for OpenType GSUB `ssty` feature:
    /// 0 for Display/Text, 1 for Script, 2 for ScriptScript.
    #[inline]
    pub fn ssty_level(&self) -> u8 {
        match self.style {
            MathVariantStyle::Display | MathVariantStyle::Text => 0,
            MathVariantStyle::Script => 1,
            MathVariantStyle::ScriptScript => 2,
        }
    }

    /// Scale factor for font size based on the OpenType MATH constants.
    #[inline]
    pub fn scale(&self, c: &crate::render::font::MathConstants) -> f32 {
        match self.style {
            MathVariantStyle::Display | MathVariantStyle::Text => 1.0,
            MathVariantStyle::Script => c.script_percent_scale_down,
            MathVariantStyle::ScriptScript => c.script_script_percent_scale_down,
        }
    }

    /// Scale factor for child context relative to this parent context.
    #[inline]
    pub fn rel_scale(&self, child: MathContext, c: &crate::render::font::MathConstants) -> f32 {
        let parent = self.scale(c);
        if parent > 0.0 {
            child.scale(c) / parent
        } else {
            1.0
        }
    }

    /// Transition for fraction numerator: in MathType/OMML, fraction parts stay at full size
    /// (Display and Text -> Text, Script -> Script, ScriptScript -> ScriptScript).
    #[inline]
    pub fn for_numerator(&self) -> Self {
        Self {
            style: match self.style {
                MathVariantStyle::Display | MathVariantStyle::Text => MathVariantStyle::Text,
                MathVariantStyle::Script | MathVariantStyle::ScriptScript => self.style,
            },
            cramped: self.cramped,
        }
    }

    /// Transition for fraction denominator: in MathType/OMML, fraction parts stay at full size (always cramped).
    #[inline]
    pub fn for_denominator(&self) -> Self {
        Self {
            style: match self.style {
                MathVariantStyle::Display | MathVariantStyle::Text => MathVariantStyle::Text,
                MathVariantStyle::Script | MathVariantStyle::ScriptScript => self.style,
            },
            cramped: true,
        }
    }

    /// Transition for superscript (TeX Rule 18a): D, T -> S; S, SS -> SS (preserves cramped).
    #[inline]
    pub fn for_superscript(&self) -> Self {
        Self {
            style: match self.style {
                MathVariantStyle::Display | MathVariantStyle::Text => MathVariantStyle::Script,
                MathVariantStyle::Script | MathVariantStyle::ScriptScript => {
                    MathVariantStyle::ScriptScript
                }
            },
            cramped: self.cramped,
        }
    }

    /// Transition for subscript (TeX Rule 18b): D, T -> S'; S, SS -> SS' (always cramped).
    #[inline]
    pub fn for_subscript(&self) -> Self {
        Self {
            style: match self.style {
                MathVariantStyle::Display | MathVariantStyle::Text => MathVariantStyle::Script,
                MathVariantStyle::Script | MathVariantStyle::ScriptScript => {
                    MathVariantStyle::ScriptScript
                }
            },
            cramped: true,
        }
    }

    /// Transition for radicand under a square root or n-th root (TeX Rule 11): same style, cramped = true.
    #[inline]
    pub fn for_radicand(&self) -> Self {
        Self {
            style: self.style,
            cramped: true,
        }
    }

    /// Transition for root index (deg in \sqrt[n]{x}): SS' (cramped ScriptScript).
    #[inline]
    pub fn for_root_index(&self) -> Self {
        Self {
            style: MathVariantStyle::ScriptScript,
            cramped: true,
        }
    }

    /// Transition for limits over/under big operators: cramped Script/ScriptScript.
    #[inline]
    pub fn for_limits(&self) -> Self {
        Self {
            style: match self.style {
                MathVariantStyle::Display | MathVariantStyle::Text => MathVariantStyle::Script,
                MathVariantStyle::Script | MathVariantStyle::ScriptScript => {
                    MathVariantStyle::ScriptScript
                }
            },
            cramped: true,
        }
    }
}

/// A run item together with its TeX class, used while spacing a row.
struct Atom {
    class: Class,
    b: LBox,
    delim: Option<char>,
    is_table: bool,
}

/// The layout engine: font set plus the current recursion depth.
struct Layout {
    f: &'static Fonts,
    depth: usize,
}

impl Layout {
    pub(super) fn glyph(&self, ch: char, face: Face, em: f32) -> LBox {
        self.glyph_styled(ch, face, em, 0)
    }

    pub(super) fn glyph_styled(&self, ch: char, face: Face, em: f32, ssty: u8) -> LBox {
        let (a, d) = self.f.ink_styled(face, ch, em, ssty);
        let items = if ch == ' ' {
            vec![]
        } else {
            vec![Item::Glyph {
                ch,
                face,
                em,
                x: 0.0,
                y: 0.0,
                ssty,
            }]
        };
        LBox {
            w: self.f.advance_styled(face, ch, em, ssty),
            asc: a,
            desc: d,
            items,
        }
    }

    #[allow(dead_code)]
    pub(super) fn text(&self, s: &str, face: Face, em: f32) -> LBox {
        self.text_styled(s, face, em, 0)
    }

    pub(super) fn text_styled(&self, s: &str, face: Face, em: f32, ssty: u8) -> LBox {
        let mut b = LBox::default();
        let mut x = 0.0;
        for ch in s.chars() {
            let g = self.glyph_styled(ch, face, em, ssty);
            b.place(&g, x, 0.0);
            x += g.w;
        }
        b.w = x;
        b
    }

    /// Returns the italic correction of the trailing glyph in `b` (or 0.0 if none).
    pub(super) fn italic_correction_of_box(&self, b: &LBox) -> f32 {
        b.items
            .iter()
            .rev()
            .find_map(|it| match it {
                Item::Glyph {
                    ch, face, em, ssty, ..
                } => {
                    let ic = self.f.italic_correction_styled(*face, *ch, *em, *ssty);
                    if ic > 0.0 {
                        Some(ic)
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .unwrap_or(0.0)
    }

    /// Returns the top accent attachment x coordinate for `b` if `b` contains a single glyph.
    pub(super) fn top_accent_attachment_of_box(&self, b: &LBox) -> Option<f32> {
        let glyphs: Vec<_> = b
            .items
            .iter()
            .filter_map(|it| match it {
                Item::Glyph {
                    ch, face, em, x, ..
                } => Some((*ch, *face, *em, *x)),
                _ => None,
            })
            .collect();
        if glyphs.len() == 1 {
            let (ch, face, em, x) = glyphs[0];
            self.f
                .top_accent_attachment(face, ch, em)
                .map(|att| x + att)
        } else {
            None
        }
    }

    /// Returns the last glyph in `b` if any.
    pub(super) fn last_glyph_of_box(&self, b: &LBox) -> Option<(Face, char, f32)> {
        b.items.iter().rev().find_map(|it| match it {
            Item::Glyph { ch, face, em, .. } => Some((*face, *ch, *em)),
            _ => None,
        })
    }

    /// Returns the first glyph in `b` if any.
    pub(super) fn first_glyph_of_box(&self, b: &LBox) -> Option<(Face, char, f32)> {
        b.items.iter().find_map(|it| match it {
            Item::Glyph { ch, face, em, .. } => Some((*face, *ch, *em)),
            _ => None,
        })
    }

    // ── tree walk ────────────────────────────────────────────────────────────

    pub(super) fn node(&mut self, n: &MathNode, em: f32, ctx: MathContext) -> Result<LBox> {
        if self.depth >= MAX_DEPTH {
            return Err(Error::TooDeep);
        }
        self.depth += 1;
        let r = self.node_inner(n, em, ctx);
        self.depth -= 1;
        r
    }

    pub(super) fn node_inner(&mut self, n: &MathNode, em: f32, ctx: MathContext) -> Result<LBox> {
        let k = &n.children;
        match n.tag.as_str() {
            "mi" | "mn" | "mo" | "mtext" => self.row(std::slice::from_ref(n), em, ctx),
            "mfrac" => self.frac(n, em, ctx),
            "msqrt" => {
                let c = self.row(k, em, ctx.for_radicand())?;
                Ok(self.radical(&c, None, em, ctx))
            }
            "mroot" => {
                let c = match k.first() {
                    Some(b) => self.node(b, em, ctx.for_radicand())?,
                    None => LBox::default(),
                };
                let d = match k.get(1) {
                    Some(d) => {
                        let deg_ctx = ctx.for_root_index();
                        let deg_em = em * ctx.rel_scale(deg_ctx, self.f.constants());
                        Some(self.node(d, deg_em, deg_ctx)?)
                    }
                    None => None,
                };
                Ok(self.radical(&c, d.as_ref(), em, ctx))
            }
            "msub" | "msup" | "msubsup" => self.scripts(n, em, ctx),
            "mmultiscripts" => self.prescripts(n, em, ctx),
            "munder" | "mover" | "munderover" => self.under_over(n, em, ctx),
            "mtable" => self.table(n, em, ctx),
            "mphantom" => {
                let mut b = self.row(k, em, ctx)?;
                b.items.clear();
                Ok(b)
            }
            _ => self.row(k, em, ctx),
        }
    }
}

/// Lay out a `<math>` tree; returns the box and the em size in pixels.
pub(crate) fn layout_math(math: &MathNode, font_pt: f32) -> Result<(LBox, f32)> {
    let em = font_pt * PX_PER_PT;
    let mut l = Layout {
        f: fonts(),
        depth: 0,
    };
    let kids: &[MathNode] = match math.children.as_slice() {
        [only] if only.tag == "mrow" => &only.children,
        all => all,
    };
    let b = l.row(kids, em, MathContext::DISPLAY)?;
    Ok((b, em))
}
