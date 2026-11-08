//! Bidirectional OMML ⇄ MathType conversion pipeline for `.docx` files.
//!
//! * **OMML → MathType** ([`convert`], [`convert_with`], [`convert_bytes`], [`convert_bytes_with`]):
//!   Finds every `m:oMath` equation in a `.docx`, converts each to a MathType OLE
//!   object (`Equation.DSMT4`), and writes a new `.docx` with those objects
//!   embedded in place of the original OMML equations.
//! * **MathType → OMML** ([`convert_to_omml`], [`convert_to_omml_with`], [`convert_bytes_to_omml`], [`convert_bytes_to_omml_with`]):
//!   Finds every MathType OLE object (`Equation.DSMT4`, `Equation.3`, `Equation.2`)
//!   and standalone WMF equation in a `.docx`, converts each to an `<m:oMath>`
//!   element, and removes orphaned `.bin` and preview image files.
//!
//! | Module       | Responsibility                                            |
//! |--------------|-----------------------------------------------------------|
//! | `options`    | [`Options`], [`ToOmmlOptions`], [`EmbedMode`], [`ConversionSummary`] |
//! | `consts`     | namespaces, relationship types, package part names        |
//! | `ole_xml`    | XML that embeds an OLE object (`w:object`, `v:shape`, …)  |
//! | `package`    | zip entries, relationships and content types              |
//! | `job`        | per-equation work: OMML → MathML → MTEF → OLE (+ picture) |
//! | `pipeline`   | public `convert*` entry points (OMML → MathType)          |
//! | `to_omml`    | public `convert_to_omml*` entry points (MathType → OMML)  |
//! | `flat_opc`   | single-equation Flat OPC (`pkg:package`) packager & parser|

mod consts;
mod flat_opc;
mod job;
mod ole_xml;
mod options;
mod package;
mod pipeline;
mod to_omml;
mod upgrade;

pub use flat_opc::{flat_opc_to_mathml, mathml_to_flat_opc, FlatOpcMode};
pub use ole_xml::{make_object_run, make_ole_xml};
pub use options::{ConversionSummary, EmbedMode, Options, PreviewFormat, ToOmmlOptions, UpgradeOptions, UpgradeTarget};
pub use package::{add_relationship, ensure_content_type, ensure_default_extension};
pub use pipeline::{convert, convert_bytes, convert_bytes_with, convert_with};
pub use to_omml::{
    convert_bytes_to_omml, convert_bytes_to_omml_with, convert_to_omml, convert_to_omml_with,
};

pub use upgrade::{
    upgrade_legacy_mathtype_bytes, upgrade_legacy_mathtype_bytes_with,
    upgrade_legacy_mathtype_docx, upgrade_legacy_mathtype_docx_with,
};
