//! Function names that are drawn upright (sin, cos, lim, …).

/// Known function names.
pub const FUNCTIONS: &[&str] = &[
    "sin", "cos", "tan", "cot", "sec", "csc", "arcsin", "arccos", "arctan", "arccot", "arcsec",
    "arccsc", "sinh", "cosh", "tanh", "coth", "sech", "csch", "arsinh", "arcosh", "artanh",
    "arcoth", "tg", "ctg", "cotg", "cosec", "log", "ln", "lg", "exp", "lim", "limsup", "liminf",
    "lim sup", "lim inf", "max", "min", "sup", "inf", "det", "dim", "ker", "hom", "deg", "arg",
    "gcd", "lcm", "sgn", "mod", "Pr", "Re", "Im",
];

/// `true` when `s` is a known function name.
pub fn is_function(s: &str) -> bool {
    FUNCTIONS.contains(&s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_full_function_set() {
        for name in [
            "sin", "cos", "tan", "cot", "sec", "csc", "arcsin", "arccos", "arctan", "arccot",
            "arcsec", "arccsc", "sinh", "cosh", "tanh", "coth", "sech", "csch", "arsinh", "arcosh",
            "artanh", "arcoth", "tg", "ctg", "cotg", "cosec", "log", "ln", "lg", "exp", "lim",
            "limsup", "liminf", "lim sup", "lim inf", "max", "min", "sup", "inf", "det", "dim",
            "ker", "hom", "deg", "arg", "gcd", "lcm", "sgn", "mod", "Pr", "Re", "Im",
        ] {
            assert!(is_function(name), "missing function {name}");
        }
        assert!(!is_function("foo"));
        assert!(!is_function("x"));
    }
}
