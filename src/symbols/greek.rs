//! Greek letters palette (lowercase and uppercase).

/// Greek letter name → Unicode code point.
pub const GREEK: &[(&str, u32)] = &[
    ("alpha", 0x03B1),
    ("beta", 0x03B2),
    ("gamma", 0x03B3),
    ("delta", 0x03B4),
    ("epsilon", 0x03B5),
    ("zeta", 0x03B6),
    ("eta", 0x03B7),
    ("theta", 0x03B8),
    ("iota", 0x03B9),
    ("kappa", 0x03BA),
    ("lambda", 0x03BB),
    ("mu", 0x03BC),
    ("nu", 0x03BD),
    ("xi", 0x03BE),
    ("omicron", 0x03BF),
    ("pi", 0x03C0),
    ("rho", 0x03C1),
    ("varsigma", 0x03C2),
    ("sigma", 0x03C3),
    ("tau", 0x03C4),
    ("upsilon", 0x03C5),
    ("phi", 0x03C6),
    ("chi", 0x03C7),
    ("psi", 0x03C8),
    ("omega", 0x03C9),
    ("vartheta", 0x03D1),
    ("varphi", 0x03D5),
    ("varpi", 0x03D6),
    ("digamma", 0x03DD),
    ("varkappa", 0x03F0),
    ("varrho", 0x03F1),
    ("varepsilon", 0x03F5),
    ("Alpha", 0x0391),
    ("Beta", 0x0392),
    ("Gamma", 0x0393),
    ("Delta", 0x0394),
    ("Epsilon", 0x0395),
    ("Zeta", 0x0396),
    ("Eta", 0x0397),
    ("Theta", 0x0398),
    ("Iota", 0x0399),
    ("Kappa", 0x039A),
    ("Lambda", 0x039B),
    ("Mu", 0x039C),
    ("Nu", 0x039D),
    ("Xi", 0x039E),
    ("Omicron", 0x039F),
    ("Pi", 0x03A0),
    ("Rho", 0x03A1),
    ("Sigma", 0x03A3),
    ("Tau", 0x03A4),
    ("Upsilon", 0x03A5),
    ("Phi", 0x03A6),
    ("Chi", 0x03A7),
    ("Psi", 0x03A8),
    ("Omega", 0x03A9),
];

/// Look a Greek letter name up.
pub fn greek(name: &str) -> Option<u32> {
    GREEK.iter().find(|(n, _)| *n == name).map(|(_, cp)| *cp)
}

/// `true` when `cp` is one of the Greek letters in [`GREEK`].
pub fn is_greek_codepoint(cp: u32) -> bool {
    matches!(
        cp,
        0x0391..=0x03A9
            | 0x03B1..=0x03C9
            | 0x03D0..=0x03D7
            | 0x03DC..=0x03DD
            | 0x03F0..=0x03F5
    )
}

/// `true` for a lowercase Greek letter (including symbol variants such as ϑ, ϕ, ϖ, ϱ, ϵ).
pub fn is_greek_lower(c: char) -> bool {
    matches!(
        c,
        '\u{03B1}'..='\u{03C9}'
            | '\u{03D0}'..='\u{03D1}'
            | '\u{03D5}'..='\u{03D7}'
            | '\u{03DD}'
            | '\u{03F0}'..='\u{03F3}'
            | '\u{03F5}'
    )
}

/// `true` for an uppercase Greek letter (including ϒ, Ϝ, ϴ).
pub fn is_greek_upper(c: char) -> bool {
    matches!(
        c,
        '\u{0391}'..='\u{03A9}' | '\u{03D2}'..='\u{03D4}' | '\u{03DC}' | '\u{03F4}'
    )
}

/// `true` for any character in the Greek and Coptic block.
pub fn in_greek_block(c: char) -> bool {
    ('\u{370}'..='\u{3FF}').contains(&c)
}
