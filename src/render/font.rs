//! Embedded fonts and glyph helpers used to draw equation previews.

use std::sync::OnceLock;

use ab_glyph::{Font, FontRef, GlyphId, PxScale};

use super::canvas::Canvas;

static FONT_REGULAR: &[u8] = include_bytes!("../../assets/fonts/stixfonts-2.13b171/STIXTwoText-Regular.ttf");
static FONT_ITALIC: &[u8] = include_bytes!("../../assets/fonts/stixfonts-2.13b171/STIXTwoText-Italic.ttf");
static FONT_BOLD: &[u8] = include_bytes!("../../assets/fonts/stixfonts-2.13b171/STIXTwoText-Bold.ttf");
static FONT_MATH: &[u8] = include_bytes!("../../assets/fonts/stixfonts-2.13b171/STIXTwoMath-Regular.ttf");

/// Which face to draw a character in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MathStyle {
    Text,         // Times New Roman
    Function,     // Times New Roman (sin, cos, lim...)
    Variable,     // Times New Roman Italic (x, y, z...)
    LcGreek,      // Lowercase Greek Italic (α, β, γ...)
    UcGreek,      // Uppercase Greek Regular (Γ, Δ, Ω...)
    Symbol,       // Symbol (+, -, =, ±...)
    VectorMatrix, // Times New Roman Bold
    Number,       // Times New Roman (0-9)
    ExtraMath,    // MT Extra (∫, ∑, √, large brackets, ...)
}

pub type Face = MathStyle;

/// One drawing command of a glyph outline, in absolute pixel coordinates (y down).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Seg {
    /// Start a new contour.
    Move(f32, f32),
    /// Straight line to a point.
    Line(f32, f32),
    /// Quadratic Bézier: control point, end point.
    Quad(f32, f32, f32, f32),
    /// Cubic Bézier: two control points, end point.
    Cubic(f32, f32, f32, f32, f32, f32),
    /// Close the current contour.
    Close,
}

impl MathStyle {
    pub fn auto_detect(c: char) -> Self {
        match c {
            '0'..='9' => MathStyle::Number,
            'a'..='z' | 'A'..='Z' => MathStyle::Variable,
            '\u{03B1}'..='\u{03C9}' => MathStyle::LcGreek,
            '\u{0391}'..='\u{03A9}' => MathStyle::UcGreek,
            '∫' | '∬' | '∭' | '∑' | '∏' | '√' => MathStyle::ExtraMath,
            '+' | '-' | '=' | '<' | '>' | '±' | '×' | '÷' | '≤' | '≥' | '≠' | '≈' | '∈' | '∉' => {
                MathStyle::Symbol
            }
            _ => MathStyle::Text,
        }
    }
}

/// The font set.
pub struct Fonts {
    regular: FontRef<'static>,
    italic: FontRef<'static>,
    bold: FontRef<'static>,
    math: FontRef<'static>,
}

/// Access the lazily-loaded font set.
pub fn fonts() -> &'static Fonts {
    static F: OnceLock<Fonts> = OnceLock::new();
    F.get_or_init(|| {
        let load = |d: &'static [u8]| FontRef::try_from_slice(d).expect("bundled font is valid");
        Fonts {
            regular: load(FONT_REGULAR),
            italic: load(FONT_ITALIC),
            bold: load(FONT_BOLD),
            math: load(FONT_MATH),
        }
    })
}

impl Fonts {
    fn pick(&self, style: MathStyle, ch: char) -> (&FontRef<'static>, GlyphId) {

        let mut ch = ch;
        if style == MathStyle::LcGreek && ('\u{03B1}'..='\u{03C9}').contains(&ch) {
            ch = std::char::from_u32(ch as u32 + 0x1D34B).unwrap_or(ch);
        }

        let primary = match style {
            MathStyle::Text | MathStyle::Function | MathStyle::Number => &self.regular,
            MathStyle::Variable => &self.italic,
            MathStyle::VectorMatrix => &self.bold,
            MathStyle::LcGreek | MathStyle::UcGreek => &self.math,
            MathStyle::Symbol | MathStyle::ExtraMath => &self.math,
        };

        let fallback_chain = match style {
            MathStyle::LcGreek => [primary, &self.math, &self.regular],
            MathStyle::Symbol | MathStyle::ExtraMath => [primary, &self.regular, &self.italic],
            _ => [primary, &self.math, &self.regular],
        };

        for f in fallback_chain {
            let id = f.glyph_id(ch);
            if id.0 != 0 {
                return (f, id);
            }
        }

        (primary, primary.glyph_id(ch))
    }

    fn scale_for(font: &FontRef<'static>, em_px: f32) -> PxScale {
        let upem = font.units_per_em().unwrap_or(1000.0);
        PxScale::from(em_px * font.height_unscaled() / upem)
    }

    pub fn advance(&self, style: MathStyle, ch: char, em_px: f32) -> f32 {
        let (f, id) = self.pick(style, ch);
        let adv = f.h_advance_unscaled(id) * em_px / f.units_per_em().unwrap_or(1000.0);
        adv
    }

    pub fn ink(&self, style: MathStyle, ch: char, em_px: f32) -> (f32, f32) {
        let (f, id) = self.pick(style, ch);
        let g = id.with_scale_and_position(Self::scale_for(f, em_px), ab_glyph::point(0.0, 0.0));
        match f.outline_glyph(g) {
            Some(o) => {
                let b = o.px_bounds();
                ((-b.min.y).max(0.0), b.max.y.max(0.0))
            }
            None => (0.0, 0.0),
        }
    }

    pub fn ink_x(&self, style: MathStyle, ch: char, em_px: f32) -> (f32, f32) {
        let (f, id) = self.pick(style, ch);
        let g = id.with_scale_and_position(Self::scale_for(f, em_px), ab_glyph::point(0.0, 0.0));
        match f.outline_glyph(g) {
            Some(o) => {
                let b = o.px_bounds();
                (b.min.x, b.max.x)
            }
            None => (0.0, 0.0),
        }
    }

    pub fn draw(&self, style: MathStyle, ch: char, em_px: f32, x: f32, y: f32, canvas: &mut Canvas) {
        let (f, id) = self.pick(style, ch);
        let g = id.with_scale_and_position(Self::scale_for(f, em_px), ab_glyph::point(x, y));
        if let Some(o) = f.outline_glyph(g) {
            let b = o.px_bounds();
            let (bx, by) = (b.min.x.floor() as i64, b.min.y.floor() as i64);
            o.draw(|gx, gy, c| canvas.add(bx + gx as i64, by + gy as i64, c));
        }
    }

    /// The outline of `ch` as drawing commands in pixels (y pointing down), with its
    /// baseline origin at `(x, y)` and `em_px` pixels per em. `None` when the glyph has
    /// no outline (e.g. a space).
    pub fn glyph_segments(&self, style: MathStyle, ch: char, em_px: f32, x: f32, y: f32) -> Option<Vec<Seg>> {
        use ab_glyph::OutlineCurve as C;

        let (f, id) = self.pick(style, ch);
        let outline = f.outline(id)?; // font units, y axis pointing up
        let k = em_px / f.units_per_em().unwrap_or(1000.0);
        let pt = |p: ab_glyph::Point| (x + p.x * k, y - p.y * k); // flip y: pixels grow downwards

        let mut segs = Vec::with_capacity(outline.curves.len() + 8);
        let mut pen: Option<(f32, f32)> = None; // end of the previous segment
        for c in &outline.curves {
            let start = match c {
                C::Line(a, _) | C::Quad(a, _, _) | C::Cubic(a, _, _, _) => pt(*a),
            };
            if pen != Some(start) {
                if pen.is_some() {
                    segs.push(Seg::Close);
                }
                segs.push(Seg::Move(start.0, start.1));
            }
            pen = Some(match c {
                C::Line(_, b) => {
                    let b = pt(*b);
                    segs.push(Seg::Line(b.0, b.1));
                    b
                }
                C::Quad(_, b, e) => {
                    let (b, e) = (pt(*b), pt(*e));
                    segs.push(Seg::Quad(b.0, b.1, e.0, e.1));
                    e
                }
                C::Cubic(_, b, c2, e) => {
                    let (b, c2, e) = (pt(*b), pt(*c2), pt(*e));
                    segs.push(Seg::Cubic(b.0, b.1, c2.0, c2.1, e.0, e.1));
                    e
                }
            });
        }
        if pen.is_some() {
            segs.push(Seg::Close);
        }
        Some(segs)
    }

    /// The outline of `ch` flattened to polygons (one per contour), drawn with
    /// independent horizontal and vertical scales: `em_x` / `em_y` pixels per em.
    /// The baseline origin sits at `(x, y)`; pixels grow downwards. This is how a
    /// glyph is stretched vertically without thickening its vertical strokes.
    pub fn glyph_polys(&self, style: MathStyle, ch: char, em_x: f32, em_y: f32, x: f32, y: f32) -> Vec<Vec<(f32, f32)>> {
        let Some(segs) = self.glyph_segments(style, ch, em_y, 0.0, 0.0) else {
            return Vec::new();
        };
        let k = if em_y != 0.0 { em_x / em_y } else { 1.0 };
        let map = |px: f32, py: f32| (x + px * k, y + py);
        const STEPS: usize = 12;

        let mut polys: Vec<Vec<(f32, f32)>> = Vec::new();
        let mut cur: Vec<(f32, f32)> = Vec::new();
        let mut pen = (0.0f32, 0.0f32);
        for seg in segs {
            match seg {
                Seg::Move(a, b) => {
                    if cur.len() >= 3 {
                        polys.push(std::mem::take(&mut cur));
                    }
                    cur.clear();
                    pen = (a, b);
                    cur.push(map(a, b));
                }
                Seg::Line(a, b) => {
                    pen = (a, b);
                    cur.push(map(a, b));
                }
                Seg::Quad(cx, cy, ex, ey) => {
                    let (sx, sy) = pen;
                    for i in 1..=STEPS {
                        let t = i as f32 / STEPS as f32;
                        let u = 1.0 - t;
                        cur.push(map(u * u * sx + 2.0 * u * t * cx + t * t * ex, u * u * sy + 2.0 * u * t * cy + t * t * ey));
                    }
                    pen = (ex, ey);
                }
                Seg::Cubic(c1x, c1y, c2x, c2y, ex, ey) => {
                    let (sx, sy) = pen;
                    for i in 1..=STEPS {
                        let t = i as f32 / STEPS as f32;
                        let u = 1.0 - t;
                        let (b0, b1, b2, b3) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
                        cur.push(map(b0 * sx + b1 * c1x + b2 * c2x + b3 * ex, b0 * sy + b1 * c1y + b2 * c2y + b3 * ey));
                    }
                    pen = (ex, ey);
                }
                Seg::Close => {
                    if cur.len() >= 3 {
                        polys.push(std::mem::take(&mut cur));
                    }
                    cur.clear();
                }
            }
        }
        if cur.len() >= 3 {
            polys.push(cur);
        }
        polys
    }

    pub fn has_glyph(&self, ch: char) -> bool {
        self.pick(MathStyle::Text, ch).1 .0 != 0
    }
}
