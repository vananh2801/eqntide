//! Miscellaneous symbols palette (∂ ∞ ℓ …) and punctuation.

/// Punctuation characters (TeX class `Punct`).
pub const PUNCTUATION: &str = ",;";

/// `true` for punctuation.
pub fn is_punctuation(c: char) -> bool {
    PUNCTUATION.contains(c)
}

/// Convert an explicit MathML spacing width to em units (12 pt reference size).
/// Invalid and non-finite lengths are ignored rather than poisoning layout.
pub fn space_width_em(width: &str) -> f32 {
    let width = width.trim();
    let (number, scale) = if let Some(n) = width.strip_suffix("em") {
        (n, 1.0)
    } else if let Some(n) = width.strip_suffix("ex") {
        (n, 0.5)
    } else if let Some(n) = width.strip_suffix("mu") {
        (n, 1.0 / 18.0)
    } else if let Some(n) = width.strip_suffix("pt") {
        (n, 1.0 / 12.0)
    } else if let Some(n) = width.strip_suffix("px") {
        (n, 1.0 / 16.0)
    } else {
        (width, 1.0)
    };
    number
        .parse::<f32>()
        .ok()
        .filter(|v| v.is_finite())
        .map_or(0.0, |v| (v * scale).clamp(-20.0, 20.0))
}

/// Standard typographic spaces used to carry explicit math spacing in Office.
pub fn unicode_space_em(c: char) -> Option<f32> {
    Some(match c {
        '\u{200A}' => 1.0 / 12.0,
        '\u{2009}' => 3.0 / 18.0,
        '\u{205F}' => 4.0 / 18.0,
        '\u{2005}' => 5.0 / 18.0,
        '\u{2003}' => 1.0,
        '\u{2002}' => 0.5,
        _ => return None,
    })
}
