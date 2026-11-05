//! Convert OMML equations in `.docx` files to MathType OLE objects.
//!
//! Pure Rust port of the Python `omml-converter` package: no Word, no
//! MathType, no COM — it runs anywhere.
//!
//! ```no_run
//! let summary = omml_converter::convert("input.docx", "output.docx")?;
//! println!("{}/{} converted", summary.converted, summary.found);
//! # Ok::<(), omml_converter::Error>(())
//! ```
//!
//! # Pipeline
//!
//! ```text
//! .docx ──▶ OMML  ──▶  MathML  ──▶  MTEF 5.0  ──▶  OLE (CFB) ──▶ .docx
//!         omml::    mtef::         ole.rs       (re-zipped by docx::)
//!         decode    encode
//!                      └──▶ render:: ──▶ preview PNG
//! ```
//!
//! MathML ([`mathml::MathNode`]) is the hub: every format only needs a
//! `decode` (format → MathML) and an `encode` (MathML → format) module, so
//! the reverse direction (MTEF → OMML) slots in as `mtef::decode` +
//! `omml::encode` without touching the rest.
//!
//! # Modules
//!
//! | Module      | Role                                                    |
//! |-------------|---------------------------------------------------------|
//! | [`docx`]    | `.docx` package handling and the conversion pipeline    |
//! | [`omml`]    | OMML ⇄ MathML                                           |
//! | [`mtef`]    | MathML ⇄ MTEF binary                                    |
//! | [`ole`]     | OLE compound file wrapper for the MTEF stream           |
//! | [`render`]  | preview picture (layout grouped by MathType palettes)   |
//! | [`symbols`] | shared symbol tables (one module per MathType palette)  |
//! | [`mathml`]  | the intermediate MathML tree                            |
//! | [`xml`]     | small XML tree used for the `.docx` parts               |
//! | [`cli`]     | command-line front end                                  |

#![forbid(unsafe_code)]

pub mod cli;
pub mod docx;
pub mod error;
pub mod mathml;
pub mod mtef;
pub mod ole;
pub mod omml;
pub mod render;
pub mod symbols;
pub mod xml;


pub use docx::{
    convert, convert_bytes, convert_bytes_with, convert_with, ConversionSummary, EmbedMode, Options,
};
pub use error::{Error, Result};
pub use mtef::encode::mathml_to_mtef_bytes;
pub use mtef::make_equation_native;
pub use ole::make_ole;
pub use omml::decode::omml_to_mathml;

/// Crate version (also what `--version` prints).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
