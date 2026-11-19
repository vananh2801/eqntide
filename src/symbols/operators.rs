//! Operator symbols palette (± • ⊗ …), big operators and the operator tables
//! used by the OMML / MTEF converters.

/// Binary operators from the operator palette (set and logic operators live in
/// [`super::sets`] and [`super::logic`]).
pub const BINARY: &str = "+−-±∓×÷·⋅∗*∘∔\u{2295}\u{2297}\u{2299}";

/// Large operators that take limits (summation, product, big set operators).
pub const BIG_OPERATORS: &str =
    "\u{2211}\u{2210}\u{220F}\u{22C3}\u{22C2}\u{22C1}\u{22C0}\u{2A02}\u{2A01}\u{2A00}";

/// Integral signs (including loop, double, triple, cw and ccw loop integrals).
pub const INTEGRALS: &str = "\u{222B}\u{222C}\u{222D}\u{222E}\u{222F}\u{2230}\u{2232}\u{2233}";

/// `true` for a binary operator of this palette.
pub fn is_binary(c: char) -> bool {
    BINARY.contains(c)
}

/// `true` for a large operator such as ∑ or ∏.
pub fn is_big_op(c: char) -> bool {
    BIG_OPERATORS.contains(c)
}

/// `true` for an integral sign.
pub fn is_integral(c: char) -> bool {
    INTEGRALS.contains(c)
}

/// The glyph actually drawn for an ASCII operator (`-` → U+2212, …).
pub fn display_char(c: char) -> char {
    match c {
        '-' => '\u{2212}',
        '*' => '\u{2217}',
        '\'' => '\u{2032}',
        c => c,
    }
}

/// MTEF: operators that should use the symbol typeface.
pub const MTEF_SYMBOL_OPERATORS: &str = "+-=<>*/÷×·±∓∞∂∇∑∏∫√∧∨∩∪≤≥≠≈≡←→↑↓↔⟨⟩";

/// OMML → MathML: characters treated as operators (`<mo>`) when they make up
/// an entire run.
pub const OMML_OPERATOR_CHARS: &str = "+-=<>*/÷×·±∓∞∂∇∑∏∫√∧∨∩∪≤≥≠≈≡←→↑↓↔⟨⟩(),.:;!?|'′″‴";

/// OMML → MathML: extra operator spellings accepted as operators.
pub const OMML_OPERATOR_ALIASES: &[char] = &[
    '⋅', '−', '×', '÷', '≤', '≥', '≠', '≈', '≡', '∖', '∘', '∗', '⊕', '⊗', '⊙', '•', '⋄', '†', '‡',
    '⨿', '∈', '∉', '∋', '∌', '⊂', '⊃', '⊆', '⊇', '⊄', '⊅', '⊈', '⊉', '⊊', '⊋', '∅', '⊎', '⊓', '⊔',
    '∀', '∃', '∍', '∄', '¬', '⊤', '⊥', '⊢', '⊣', '⊨', '∴', '∵', '⇒', '⇐', '⇔', '↕', '⇕', '⇑', '⇓',
    '↗', '↘', '↙', '↖', '⤢', '⤡', '⇆', '⇄', '⇌', '⇋', '⇀', '⇁', '↼', '↽', '↦', '↩', '↪', '⟶', '⟵',
    '⟷', '⟹', '⟸', '⟺', '↶', '↷', '≪', '≫', '≺', '≻', '◁', '▷', '∼', '≃', '≅', '≐', '≜', '≙', '≒',
    '≓', '≍', '∝', '≰', '≱', '≼', '≽', '∥', '∬', '∭', '∮', '∯', '∰', '∲', '∳', '∐', '⋂', '⋃', '⋀',
    '⋁', '⨂', '⨁', '⨀', '°', '⁗', '‵', '…', '⋯', '⋮', '⋱', '⋰', '⟦', '⟧', '‖', '∠', '∡', '∢', '△',
    '□', '▭', '▱', '◯', '◊', '℧', '○', '↵',
];

/// MathML → MTEF: operator spellings typed as ASCII that map to one symbol.
pub fn ascii_operator_spelling(text: &str) -> Option<u32> {
    match text {
        "<=" => Some(0x2264),  // ≤
        ">=" => Some(0x2265),  // ≥
        "!=" => Some(0x2260),  // ≠
        "->" => Some(0x2192),  // →
        "<-" => Some(0x2190),  // ←
        "..." => Some(0x2026), // …
        _ => None,
    }
}
