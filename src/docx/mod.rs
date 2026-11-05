//! OMML → MathType conversion pipeline for `.docx` files.
//!
//! Finds every `m:oMath` equation in a `.docx`, converts each to a MathType OLE
//! object (`Equation.DSMT4`), and writes a new `.docx` with those objects
//! embedded in place of the original OMML equations. The original OMML is kept
//! as a fallback so the equations stay visible in every Word environment.
//!
//! | Module       | Responsibility                                            |
//! |--------------|-----------------------------------------------------------|
//! | `options`    | [`Options`], [`EmbedMode`], [`ConversionSummary`]         |
//! | `consts`     | namespaces, relationship types, package part names        |
//! | `ole_xml`    | XML that embeds an OLE object (`w:object`, `v:shape`, …)  |
//! | `package`    | zip entries, relationships and content types              |
//! | `job`        | per-equation work: OMML → MathML → MTEF → OLE (+ picture) |
//! | `pipeline`   | public `convert*` entry points                            |

mod consts;
mod job;
mod ole_xml;
mod options;
mod package;
mod pipeline;


pub use ole_xml::{make_object_run, make_ole_xml};
pub use options::{ConversionSummary, EmbedMode, Options, PreviewFormat};
pub use package::{add_relationship, ensure_content_type, ensure_default_extension};
pub use pipeline::{convert, convert_bytes, convert_bytes_with, convert_with};
