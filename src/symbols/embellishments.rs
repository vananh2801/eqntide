//! Embellishments palette: accents drawn over or under a base.

/// Characters accepted as an accent over a base.
pub const ACCENTS: &str = "\u{2312}\u{23DC}\u{2322}\u{20D7}\u{2192}\u{27F6}^\u{0302}\u{02C6}\u{00AF}\u{0304}\u{203E}\u{2015}\u{02C9}~\u{0303}\u{02DC}\u{0307}\u{02D9}\u{00B7}.";

/// `true` when `c` is an accent character.
pub fn is_accent(c: char) -> bool {
    ACCENTS.contains(c)
}
