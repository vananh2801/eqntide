//! Symbol tables shared by every converter stage.
//!
//! The upper row of MathType's palette bar (relations, operators, arrows,
//! logic, sets, miscellaneous symbols, Greek) is nothing but *data*: which
//! characters exist, what TeX class they have, which font draws them. That
//! data lives here, one module per palette, so the renderer, the MathML
//! encoders and any future decoder all read the same tables.
//!
//! | MathType palette          | Module          |
//! |---------------------------|-----------------|
//! | Relational symbols        | [`relations`]   |
//! | Operator symbols          | [`operators`]   |
//! | Arrow symbols             | [`arrows`]      |
//! | Logic symbols             | [`logic`]       |
//! | Set theory symbols        | [`sets`]        |
//! | Misc symbols              | [`misc`]        |
//! | Greek (lower / upper)     | [`greek`]       |
//! | Embellishments            | [`embellishments`] |
//! | Fences                    | [`fences`]      |
//! | Function names (sin, …)   | [`functions`]   |

pub mod arrows;
pub mod embellishments;
pub mod fences;
pub mod functions;
pub mod greek;
pub mod logic;
pub mod misc;
pub mod operators;
pub mod relations;
pub mod sets;

pub use embellishments::{is_accent, is_prime_char, prime_order, PRIME_CHARS};
pub use fences::{is_bar, is_close, is_open, open_for};
pub use functions::is_function;
pub use greek::{is_greek_lower, is_greek_upper};
pub use operators::{display_char, is_big_op, is_integral};

/// TeX atom class, which decides the spacing around a symbol.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Class {
    /// Ordinary symbol.
    Ord,
    /// Large operator (∑, ∫, …).
    Op,
    /// Binary operator (+, ×, ∪, …).
    Bin,
    /// Relation (=, ≤, →, ∈, …).
    Rel,
    /// Opening fence.
    Open,
    /// Closing fence.
    Close,
    /// Punctuation.
    Punct,
}

/// TeX class of a single character.
pub fn class_of(c: char) -> Class {
    if is_open(c) {
        Class::Open
    } else if is_close(c) {
        Class::Close
    } else if is_big_op(c) || is_integral(c) {
        Class::Op
    } else if relations::contains(c) || sets::is_relation(c) || arrows::contains(c) {
        Class::Rel
    } else if operators::is_binary(c) || sets::is_operator(c) || logic::contains(c) {
        Class::Bin
    } else if misc::is_punctuation(c) {
        Class::Punct
    } else {
        Class::Ord
    }
}

/// `true` for ASCII letters.
pub fn is_latin_letter(c: char) -> bool {
    c.is_ascii_alphabetic()
}

/// `true` for a run holding non-ASCII natural-language letters (e.g. Vietnamese
/// words with diacritics), which should be drawn as upright text, not as math
/// variables. Excludes Greek letters, Letterlike Symbols (`U+2100..=U+214F`),
/// and Mathematical Alphanumeric Symbols (`U+1D400..=U+1D7FF`).
pub fn has_foreign_letters(s: &str) -> bool {
    s.chars().any(|c| {
        let cp = c as u32;
        c.is_alphabetic()
            && !c.is_ascii()
            && !greek::in_greek_block(c)
            && !(0x2100..=0x214F).contains(&cp)
            && !(0x1D400..=0x1D7FF).contains(&cp)
    })
}
