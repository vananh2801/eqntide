//! Typeface selection and fallback for rendered math glyphs.

use skrifa::charmap::Charmap;
use skrifa::{FontRef, GlyphId};

use super::{Fonts, MathStyle};

/// Select the primary typeface and glyph ID for `(style, ch)`, falling back across the bundled fonts.
pub(super) fn pick_glyph<'a>(
    style: MathStyle,
    ch: char,
    regular: &'a FontRef<'static>,
    italic: &'a FontRef<'static>,
    bold: &'a FontRef<'static>,
    math: &'a FontRef<'static>,
) -> (&'a FontRef<'static>, GlyphId) {
    let ch = Fonts::to_math_char(style, ch);
    let primary = primary_font(style, regular, bold, math);
    let fallback_chain = match style {
        MathStyle::Variable => [primary, italic, regular],
        MathStyle::LcGreek => [primary, math, regular],
        MathStyle::Symbol | MathStyle::ExtraMath => [primary, regular, italic],
        _ => [primary, math, regular],
    };

    for font in fallback_chain {
        if let Some(id) = Charmap::new(font).map(ch) {
            if id != GlyphId::NOTDEF {
                return (font, id);
            }
        }
    }

    (
        primary,
        Charmap::new(primary).map(ch).unwrap_or(GlyphId::NOTDEF),
    )
}

fn primary_font<'a>(
    style: MathStyle,
    regular: &'a FontRef<'static>,
    bold: &'a FontRef<'static>,
    math: &'a FontRef<'static>,
) -> &'a FontRef<'static> {
    match style {
        MathStyle::Text | MathStyle::Function | MathStyle::Number => regular,
        MathStyle::VectorMatrix => bold,
        MathStyle::Variable
        | MathStyle::LcGreek
        | MathStyle::UcGreek
        | MathStyle::Symbol
        | MathStyle::ExtraMath => math,
    }
}
