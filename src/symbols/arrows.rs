//! Arrow symbols palette (→ ⇔ …).

/// Arrow characters.
pub const ARROWS: &str = "→←↔⇒⇐⇔⟶⟵";

/// `true` when `c` is an arrow from this palette.
pub fn contains(c: char) -> bool {
    ARROWS.contains(c)
}
