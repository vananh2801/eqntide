//! Typeface selection and fallback for rendered math glyphs.

use skrifa::charmap::Charmap;
use skrifa::{FontRef, GlyphId};

use super::MathStyle;

pub(super) fn pick_glyph<'a>(
    style: MathStyle,
    ch: char,
    regular: &'a FontRef<'static>,
    italic: &'a FontRef<'static>,
    bold: &'a FontRef<'static>,
    math: &'a FontRef<'static>,
) -> (&'a FontRef<'static>, GlyphId) {
    let ch = normalized_character(style, ch);
    let primary = primary_font(style, regular, italic, bold, math);
    let fallback_chain = match style {
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

fn normalized_character(style: MathStyle, ch: char) -> char {
    if style == MathStyle::LcGreek && ('\u{03B1}'..='\u{03C9}').contains(&ch) {
        std::char::from_u32(ch as u32 + 0x1D34B).unwrap_or(ch)
    } else {
        ch
    }
}

fn primary_font<'a>(
    style: MathStyle,
    regular: &'a FontRef<'static>,
    italic: &'a FontRef<'static>,
    bold: &'a FontRef<'static>,
    math: &'a FontRef<'static>,
) -> &'a FontRef<'static> {
    match style {
        MathStyle::Text | MathStyle::Function | MathStyle::Number => regular,
        MathStyle::Variable => italic,
        MathStyle::VectorMatrix => bold,
        MathStyle::LcGreek | MathStyle::UcGreek | MathStyle::Symbol | MathStyle::ExtraMath => math,
    }
}
