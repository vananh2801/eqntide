//! Logic symbols palette (∧ ∨ …).

/// Logic operators that behave as binary operators.
pub const LOGIC_OPERATORS: &str = "∧∨";

/// `true` when `c` is a logic operator.
pub fn contains(c: char) -> bool {
    LOGIC_OPERATORS.contains(c)
}
