//! TeX-style box layout of a MathML tree, organized around MathType's palettes.
//!
//! `mod.rs` holds the shared typesetting context and recursive tree walk (`node`);
//! every child module implements one MathType palette group as an `impl Layout` block:
//!
//! ```text
//! layout_math(math, font_pt)
//!   └─► Layout::row(children, em_px, MathContext::DISPLAY)
//!         ├─► Layout::node(...)        (mfrac, msqrt/mroot, msub/msup/msubsup, munder/mover, mtable)
//!         ├─► Layout::atoms_{mo,mi}    (symbol classification + font face selection)
//!         ├─► Layout::stretch_delims   (delimiter matching & vertical assembly in `fence`/`delimiter`)
//!         └─► LBox                     (width, asc, desc, draw `Item`s, `RawHit` regions)
//! ```
//!
//! | MathType palette            | Module                |
//! |-----------------------------|-----------------------|
//! | Spaces and ellipses         | `row` (spacing)       |
//! | Embellishments              | `embellish`           |
//! | Fence templates             | `fence`, `delimiter`  |
//! | Fraction and radical        | `fraction`, `radical` |
//! | Subscript and superscript   | `script`              |
//! | Summation / product / set   | `bigop`               |
//! | Integral templates          | `integral`            |
//! | Underbar and overbar        | `bar`                 |
//! | Labeled extensible arrows   | `arrow`               |
//! | Matrix templates            | `matrix`              |
//!
//! The symbol palettes (relations, operators, arrows, logic, sets, Greek, …)
//! are plain data and live in [`crate::symbols`]. Sizes follow MathType's defaults:
//! fraction parts stay at full size while sub/superscripts follow `STIXTwoMath`'s
//! `script_percent_scale_down` constant.

mod arrow;
mod bar;
mod bigop;
mod boxes;
mod delimiter;
mod embellish;
mod enclosure;
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
use crate::render::font::{fonts, Face, Fonts, MathConstants};
use crate::render::PX_PER_PT;
#[allow(unused_imports)]
use crate::symbols::{
    class_of, display_char, has_foreign_letters, is_accent, is_bar, is_big_op, is_close,
    is_function, is_greek_lower, is_greek_upper, is_integral, is_latin_letter, is_open,
    is_prime_char, open_for, Class,
};

pub(crate) use boxes::{Item, LBox};
use geom::{clip_poly_x, clip_poly_y, quad_bezier, rect, stroke};

pub(crate) const MAX_DEPTH: usize = 256;

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

/// The typesetting context tracking the active [`MathVariantStyle`] and whether
/// the style is "cramped" (such as inside denominators or under radical signs).
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
    pub fn scale(&self, constants: &MathConstants) -> f32 {
        match self.style {
            MathVariantStyle::Display | MathVariantStyle::Text => 1.0,
            MathVariantStyle::Script => constants.script_percent_scale_down,
            MathVariantStyle::ScriptScript => constants.script_script_percent_scale_down,
        }
    }

    /// Scale factor for child context relative to this parent context.
    #[inline]
    pub fn rel_scale(&self, child: MathContext, constants: &MathConstants) -> f32 {
        let parent = self.scale(constants);
        if parent > 0.0 {
            child.scale(constants) / parent
        } else {
            1.0
        }
    }

    /// Transition for fraction numerator: in MathType/OMML, fraction parts stay at full size
    /// (`Display` and `Text` -> `Text`, `Script` -> `Script`, `ScriptScript` -> `ScriptScript`).
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

    /// Transition for fraction denominator: fraction parts stay at full size and cramped.
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

    /// Transition for radicand under a square root or n-th root (TeX Rule 11): same style, cramped.
    #[inline]
    pub fn for_radicand(&self) -> Self {
        Self {
            style: self.style,
            cramped: true,
        }
    }

    /// Transition for root index (`deg` in `\sqrt[n]{x}`): cramped `ScriptScript`.
    #[inline]
    pub fn for_root_index(&self) -> Self {
        Self {
            style: MathVariantStyle::ScriptScript,
            cramped: true,
        }
    }

    /// Transition for limits over/under big operators: cramped `Script` / `ScriptScript`.
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

/// Lay out a `<math>` tree; returns the root [`LBox`] and the base em size in pixels.
pub(crate) fn layout_math(math: &MathNode, font_pt: f32) -> Result<(LBox, f32)> {
    let em = font_pt * PX_PER_PT;
    let mut layout = Layout {
        font_set: fonts(),
        depth: 0,
    };
    let children: &[MathNode] = match math.children.as_slice() {
        [only] if only.tag == "mrow" => &only.children,
        all => all,
    };
    let layout_box = layout.row(children, em, MathContext::DISPLAY)?;
    Ok((layout_box, em))
}

/// Role of a delimiter token determined from its MathML `form` attribute.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum FenceRole {
    Neutral,
    Prefix,
    Postfix,
}

/// A run item together with its TeX spacing class, used while laying out a row.
struct Atom {
    class: Class,
    layout_box: LBox,
    delim: Option<char>,
    fence_role: FenceRole,
    is_table: bool,
    is_large_op: bool,
}

/// Box layout engine holding the embedded font set and current recursion depth.
struct Layout {
    font_set: &'static Fonts,
    depth: usize,
}

impl Layout {
    pub(super) fn node(&mut self, node: &MathNode, em: f32, ctx: MathContext) -> Result<LBox> {
        if self.depth >= MAX_DEPTH {
            return Err(Error::TooDeep);
        }
        self.depth += 1;
        let result = self.node_inner(node, em, ctx).map(|mut layout_box| {
            layout_box.mark(node);
            layout_box
        });
        self.depth -= 1;
        result
    }

    pub(super) fn node_inner(
        &mut self,
        node: &MathNode,
        em: f32,
        ctx: MathContext,
    ) -> Result<LBox> {
        let children = &node.children;
        match node.tag.as_str() {
            "mi" | "mn" | "mo" | "mtext" => self.row(std::slice::from_ref(node), em, ctx),
            "mfrac" => self.frac(node, em, ctx),
            "menclose" => self.enclosure(node, em, ctx),
            "mspace" => Ok(LBox {
                w: crate::symbols::misc::space_width_em(node.attr("width").unwrap_or("0")) * em,
                ..Default::default()
            }),
            "mstyle" => {
                let child_ctx = match node.attr("displaystyle") {
                    Some("false") => MathContext::TEXT,
                    Some("true") => MathContext::DISPLAY,
                    _ => ctx,
                };
                self.row(children, em, child_ctx)
            }
            "msqrt" => {
                let radicand = self.row(children, em, ctx.for_radicand())?;
                Ok(self.radical(&radicand, None, em, ctx))
            }
            "mroot" => {
                let radicand = match children.first() {
                    Some(child) => self.node(child, em, ctx.for_radicand())?,
                    None => LBox::default(),
                };
                let degree = match children.get(1) {
                    Some(deg_node) => {
                        let deg_ctx = ctx.for_root_index();
                        let deg_em = em * ctx.rel_scale(deg_ctx, self.font_set.constants());
                        Some(self.node(deg_node, deg_em, deg_ctx)?)
                    }
                    None => None,
                };
                Ok(self.radical(&radicand, degree.as_ref(), em, ctx))
            }
            "msub" | "msup" | "msubsup" => self.scripts(node, em, ctx),
            "mmultiscripts" => self.prescripts(node, em, ctx),
            "munder" | "mover" | "munderover" => self.under_over(node, em, ctx),
            "mtable" => self.table(node, em, ctx),
            "mphantom" => {
                let mut layout_box = self.row(children, em, ctx)?;
                layout_box.items.clear();
                Ok(layout_box)
            }
            _ => self.row(children, em, ctx),
        }
    }

    pub(super) fn glyph(&self, ch: char, face: Face, em: f32) -> LBox {
        self.glyph_styled(ch, face, em, 0)
    }

    pub(super) fn glyph_styled(&self, ch: char, face: Face, em: f32, ssty: u8) -> LBox {
        let (asc, desc) = self.font_set.ink_styled(face, ch, em, ssty);
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
            hits: Vec::new(),
            w: self.font_set.advance_styled(face, ch, em, ssty),
            asc,
            desc,
            items,
        }
    }

    #[allow(dead_code)]
    pub(super) fn text(&self, text: &str, face: Face, em: f32) -> LBox {
        self.text_styled(text, face, em, 0)
    }

    pub(super) fn text_styled(&self, text: &str, face: Face, em: f32, ssty: u8) -> LBox {
        let mut layout_box = LBox::default();
        let mut x = 0.0;
        for ch in text.chars() {
            let glyph_box = self.glyph_styled(ch, face, em, ssty);
            layout_box.place(&glyph_box, x, 0.0);
            x += glyph_box.w;
        }
        layout_box.w = x;
        layout_box
    }

    /// Return the italic correction of the trailing glyph in `layout_box` (or `0.0` if none).
    pub(super) fn italic_correction_of_box(&self, layout_box: &LBox) -> f32 {
        layout_box
            .items
            .iter()
            .rev()
            .find_map(|item| match item {
                Item::Glyph {
                    ch, face, em, ssty, ..
                } => {
                    let ic = self
                        .font_set
                        .italic_correction_styled(*face, *ch, *em, *ssty);
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

    /// Return the top accent attachment x coordinate for `layout_box` if it contains a single glyph.
    pub(super) fn top_accent_attachment_of_box(&self, layout_box: &LBox) -> Option<f32> {
        let glyphs: Vec<_> = layout_box
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Glyph {
                    ch, face, em, x, ..
                } => Some((*ch, *face, *em, *x)),
                _ => None,
            })
            .collect();
        if glyphs.len() == 1 {
            let (ch, face, em, x) = glyphs[0];
            self.font_set
                .top_accent_attachment(face, ch, em)
                .map(|att| x + att)
        } else {
            None
        }
    }

    /// Return the last glyph in `layout_box`, if any.
    pub(super) fn last_glyph_of_box(&self, layout_box: &LBox) -> Option<(Face, char, f32)> {
        layout_box.items.iter().rev().find_map(|item| match item {
            Item::Glyph { ch, face, em, .. } => Some((*face, *ch, *em)),
            _ => None,
        })
    }

    /// Return the first glyph in `layout_box`, if any.
    pub(super) fn first_glyph_of_box(&self, layout_box: &LBox) -> Option<(Face, char, f32)> {
        layout_box.items.iter().find_map(|item| match item {
            Item::Glyph { ch, face, em, .. } => Some((*face, *ch, *em)),
            _ => None,
        })
    }
}
