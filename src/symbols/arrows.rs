//! Arrow symbols palette (→ ⇔ …).

/// Arrow characters.
pub const ARROWS: &str = "→←↔⇒⇐⇔⟶⟵";

/// `true` when `c` is an arrow from this palette.
pub fn contains(c: char) -> bool {
    ARROWS.contains(c) || matches!(c as u32, 0x2190..=0x21FF | 0x27F0..=0x27FF | 0x2900..=0x297F)
}
