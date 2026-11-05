//! OMML (Office Math Markup Language) support.
//!
//! * [`decode`] – OMML → MathML
//! * `encode`   – MathML → OMML (planned, for the MTEF → OMML direction)

pub mod decode;

pub use decode::{is_number, is_operator, omml_to_mathml, text_node, NS_M};
