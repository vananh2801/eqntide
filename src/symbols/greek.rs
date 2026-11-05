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
    ("pi", 0x03C0),
    ("rho", 0x03C1),
    ("sigma", 0x03C3),
    ("tau", 0x03C4),
    ("upsilon", 0x03C5),
    ("phi", 0x03C6),
    ("chi", 0x03C7),
    ("psi", 0x03C8),
    ("omega", 0x03C9),
    ("Alpha", 0x0391),
    ("Beta", 0x0392),
    ("Gamma", 0x0393),
    ("Delta", 0x0394),
    ("Epsilon", 0x0395),
    ("Zeta", 0x0396),
    ("Eta", 0x0397),
    ("Theta", 0x0398),
    ("Lambda", 0x039B),
    ("Mu", 0x039C),
    ("Nu", 0x039D),
    ("Xi", 0x039E),
    ("Pi", 0x03A0),
    ("Rho", 0x03A1),
    ("Sigma", 0x03A3),
    ("Tau", 0x03A4),
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
    GREEK.iter().any(|(_, c)| *c == cp)
}

/// `true` for a lowercase Greek letter in the basic block.
pub fn is_greek_lower(c: char) -> bool {
    ('\u{3B1}'..='\u{3C9}').contains(&c)
}

/// `true` for an uppercase Greek letter in the basic block.
pub fn is_greek_upper(c: char) -> bool {
    ('\u{391}'..='\u{3A9}').contains(&c)
}

/// `true` for any character in the Greek and Coptic block.
pub fn in_greek_block(c: char) -> bool {
    ('\u{370}'..='\u{3FF}').contains(&c)
}
