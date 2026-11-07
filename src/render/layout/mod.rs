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
//! sub/superscripts are 70% (matching `STIXTwoMath`'s `script_percent_scale_down`).

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
    is_function, is_greek_lower, is_greek_upper, is_integral, is_latin_letter, is_open, open_for,
    Class,
};

pub(crate) use boxes::{Item, LBox};
use geom::{quad_bezier, rect, stroke};

pub(crate) const MAX_DEPTH: usize = 256;
pub(crate) const SCRIPT: f32 = 0.70;
pub(crate) const AXIS: f32 = 0.258;
pub(crate) const RULE: f32 = 0.068;
const THIN: f32 = 0.167;
const MED: f32 = 0.222;
const THICK: f32 = 0.278;

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
        let (a, d) = self.f.ink(face, ch, em);
        let items = if ch == ' ' { vec![] } else { vec![Item::Glyph { ch, face, em, x: 0.0, y: 0.0 }] };
        LBox { w: self.f.advance(face, ch, em), asc: a, desc: d, items }
    }

    pub(super) fn text(&self, s: &str, face: Face, em: f32) -> LBox {
        let mut b = LBox::default();
        let mut x = 0.0;
        for ch in s.chars() {
            let g = self.glyph(ch, face, em);
            b.place(&g, x, 0.0);
            x += g.w;
        }
        b.w = x;
        b
    }

    // ── tree walk ────────────────────────────────────────────────────────────

    pub(super) fn node(&mut self, n: &MathNode, em: f32, script: bool) -> Result<LBox> {
        if self.depth >= MAX_DEPTH {
            return Err(Error::TooDeep);
        }
        self.depth += 1;
        let r = self.node_inner(n, em, script);
        self.depth -= 1;
        r
    }

    pub(super) fn node_inner(&mut self, n: &MathNode, em: f32, script: bool) -> Result<LBox> {
        let k = &n.children;
        match n.tag.as_str() {
            "mi" | "mn" | "mo" | "mtext" => self.row(std::slice::from_ref(n), em, script),
            "mfrac" => self.frac(n, em, script),
            "msqrt" => {
                let c = self.row(k, em, script)?;
                Ok(self.radical(&c, None, em))
            }
            "mroot" => {
                let c = match k.first() {
                    Some(b) => self.node(b, em, script)?,
                    None => LBox::default(),
                };
                let d = match k.get(1) {
                    Some(d) => Some(self.node(d, em * SCRIPT, true)?),
                    None => None,
                };
                Ok(self.radical(&c, d.as_ref(), em))
            }
            "msub" | "msup" | "msubsup" => self.scripts(n, em, script),
            "mmultiscripts" => self.prescripts(n, em, script),
            "munder" | "mover" | "munderover" => self.under_over(n, em, script),
            "mtable" => self.table(n, em, script),
            "mphantom" => {
                let mut b = self.row(k, em, script)?;
                b.items.clear();
                Ok(b)
            }
            _ => self.row(k, em, script),
        }
    }
}

/// Lay out a `<math>` tree; returns the box and the em size in pixels.
pub(crate) fn layout_math(math: &MathNode, font_pt: f32) -> Result<(LBox, f32)> {
    let em = font_pt * PX_PER_PT;
    let mut l = Layout { f: fonts(), depth: 0 };
    let kids: &[MathNode] = match math.children.as_slice() {
        [only] if only.tag == "mrow" => &only.children,
        all => all,
    };
    let b = l.row(kids, em, false)?;
    Ok((b, em))
}
