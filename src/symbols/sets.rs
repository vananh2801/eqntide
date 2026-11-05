//! Set theory symbols palette (∈ ∉ ⊂ ∪ ∩ …).

/// Set-theoretic relations.
pub const SET_RELATIONS: &str = "∈∉∋⊂⊃⊆⊇";

/// Set-theoretic binary operators.
pub const SET_OPERATORS: &str = "∪∩∖";

/// `true` for a set-theoretic relation (∈, ⊂, …).
pub fn is_relation(c: char) -> bool {
    SET_RELATIONS.contains(c)
}

/// `true` for a set-theoretic operator (∪, ∩, ∖).
pub fn is_operator(c: char) -> bool {
    SET_OPERATORS.contains(c)
}
