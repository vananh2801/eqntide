//! Relational symbols palette (≤ ≠ ≈ …).

/// Relation characters that are neither set-theoretic nor arrows.
pub const RELATIONS: &str = "=≠<>≤≥≈≡∼≅≃∝⊥∥≪≫:≲≳≺≻≼≽≐≑≒≓≔≕≜≝≞≟≦≧≨≩≰≱≮≯≁≄≇≉≭≢≣≙≚≛≍≎≏";

/// `true` when `c` is a relational symbol from this palette.
pub fn contains(c: char) -> bool {
    RELATIONS.contains(c)
}
