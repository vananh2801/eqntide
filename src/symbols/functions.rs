//! Function names that are drawn upright (sin, cos, lim, …).

/// Known function names.
pub const FUNCTIONS: &[&str] = &[
    "sin", "cos", "tan", "cot", "sec", "csc", "arcsin", "arccos", "arctan", "arccot", "sinh",
    "cosh", "tanh", "coth", "log", "ln", "lg", "lim", "max", "min", "sup", "inf", "det", "exp",
    "gcd", "deg", "dim", "ker", "lcm", "mod", "Pr", "arg", "sgn", "Re", "Im", "limsup", "liminf",
    "tg", "cotg",
];

/// `true` when `s` is a known function name.
pub fn is_function(s: &str) -> bool {
    FUNCTIONS.contains(&s)
}
