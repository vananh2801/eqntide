//! OMML (Office Math Markup Language) support.
//!
//! * [`decode`] – OMML → MathML
//! * [`encode`] – MathML → OMML

pub mod decode;
pub mod encode;

pub use decode::{is_number, is_operator, omml_to_mathml, text_node, NS_M};
pub use encode::{apply_math_font, mathml_to_omml, mathml_to_omml_xml};
