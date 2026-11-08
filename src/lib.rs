//! Bidirectional converter between Word OMML equations and MathType OLE objects in `.docx` files.
//!
//! Pure Rust port of `omml-converter` and `mathtypejx`: no Word, no MathType,
//! no Python/lxml/XSLT, no COM — it runs anywhere.
//!
//! ```no_run
//! // OMML -> MathType OLE
//! let summary = omml_converter::convert("input.docx", "output_mathtype.docx")?;
//! println!("{}/{} converted to MathType", summary.converted, summary.found);
//!
//! // MathType OLE / WMF -> OMML
//! let rev = omml_converter::convert_to_omml("output_mathtype.docx", "output_omml.docx")?;
//! println!("{}/{} converted to OMML", rev.converted, rev.found);
//! # Ok::<(), omml_converter::Error>(())
//! ```
//!
//! # Pipeline
//!
//! ```text
//! .docx ──▶ OMML  ──▶  MathML  ──▶  MTEF 5.0  ──▶  OLE (CFB) ──▶ .docx
//!         omml::    mtef::         ole.rs       (re-zipped by docx::)
//!         decode    encode
//!                      └──▶ render:: ──▶ preview EMF / PNG
//!
//! .docx ──▶ OLE / WMF ──▶ MTEF 3/5 ──▶ MathML ──▶ OMML (<m:oMath>) ──▶ .docx
//!             ole.rs       mtef::      omml::     (re-zipped by docx::)
//!                          decode      encode
//! ```
//!
//! # Modules
//!
//! | Module      | Role                                                    |
//! |-------------|---------------------------------------------------------|
//! | [`docx`]    | `.docx` package handling and both conversion pipelines  |
//! | [`omml`]    | OMML ⇄ MathML                                           |
//! | [`mtef`]    | MathML ⇄ MTEF binary                                    |
//! | [`ole`]     | OLE compound file & WMF wrapper/extractor for MTEF      |
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
    convert, convert_bytes, convert_bytes_to_omml, convert_bytes_to_omml_with, convert_bytes_with,
    convert_to_omml, convert_to_omml_with, convert_with, ConversionSummary, EmbedMode, Options,
    PreviewFormat, ToOmmlOptions,
};
pub use error::{Error, Result};
pub use mtef::decode::{mtef_to_mathml, ole_to_mathml};
pub use mtef::encode::mathml_to_mtef_bytes;
pub use mtef::make_equation_native;
pub use ole::{extract_mtef_from_emf, extract_mtef_from_ole, extract_mtef_from_wmf, make_ole};
pub use render::emf::{render_emf, render_emf_with_mtef};
pub use omml::decode::omml_to_mathml;
pub use omml::encode::{apply_math_font, mathml_to_omml, mathml_to_omml_xml};

/// Convenience helper: convert a MathType OLE `.bin` payload directly to an `<m:oMath>` XML string.
pub fn ole_to_omml_xml(ole_data: &[u8]) -> Result<String> {
    let mathml = ole_to_mathml(ole_data)
        .ok_or_else(|| Error::InvalidMtef("could not decode MTEF from OLE data".into()))?;
    mathml_to_omml_xml(&mathml)
}

/// Convenience helper: convert a WMF image containing embedded MathType MTEF data directly to an `<m:oMath>` XML string.
pub fn wmf_to_omml_xml(wmf_data: &[u8]) -> Result<String> {
    let mtef = extract_mtef_from_wmf(wmf_data)
        .ok_or_else(|| Error::InvalidMtef("no MathType MTEF found in WMF data".into()))?;
    let mathml = mtef_to_mathml(&mtef)
        .ok_or_else(|| Error::InvalidMtef("could not decode MTEF from WMF data".into()))?;
    mathml_to_omml_xml(&mathml)
}

/// Convenience helper: convert an EMF image containing embedded MathType MTEF data directly to an `<m:oMath>` XML string.
pub fn emf_to_omml_xml(emf_data: &[u8]) -> Result<String> {
    let mtef = extract_mtef_from_emf(emf_data)
        .ok_or_else(|| Error::InvalidMtef("no MathType MTEF found in EMF data".into()))?;
    let mathml = mtef_to_mathml(&mtef)
        .ok_or_else(|| Error::InvalidMtef("could not decode MTEF from EMF data".into()))?;
    mathml_to_omml_xml(&mathml)
}

/// Crate version (also what `--version` prints).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
