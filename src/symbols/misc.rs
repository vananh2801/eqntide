//! Miscellaneous symbols palette (∂ ∞ ℓ …) and punctuation.

/// Punctuation characters (TeX class `Punct`).
pub const PUNCTUATION: &str = ",;";

/// `true` for punctuation.
pub fn is_punctuation(c: char) -> bool {
    PUNCTUATION.contains(c)
}
