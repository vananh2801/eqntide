//! Conversion services shared by the CLI, HTTP bridge, and desktop UI.
//!
//! ```text
//!                         ┌──▶ latex_extensions::parse (markers + ScriptRewriter)
//!                         │           │
//!   parse_latex_to_mathml ┤           ▼
//!                         │    parse_standard (latex2mathml + XML fixups)
//!                         │           │
//!                         └──▶ normalize_latex_integrals ──▶ MathNode tree
//!
//!   decode_input_to_mathml (latex / flat_opc / omml / mathml / ole / emf / wmf)
//!          │
//!          ├──▶ convert_snippet  ──▶ flat_opc_* / omml / mathml / latex / emf / png
//!          └──▶ convert_document(_with_cancel) ──▶ docx::{to_omml, pipeline, upgrade}
//! ```

use crate::bridge::{
    ConvertDocumentRequest, ConvertDocumentResponse, ConvertSnippetRequest, ConvertSnippetResponse,
};
use crate::mtef::decode::{mtef_to_mathml, ole_to_mathml};
use crate::ole::{extract_mtef_from_emf, extract_mtef_from_wmf};
use crate::{docx::*, MathNode};
use crate::{
    mathml_to_mtef_bytes, mathml_to_omml_xml, render_emf, render_emf_with_mtef, render_preview,
};

mod latex_extensions;

#[cfg(test)]
mod tests;

/// Error returned by document and snippet conversion services, paired with an
/// HTTP-compatible status code (`400`, `404`, `422`, `499`, `500`).
#[derive(Debug)]
pub struct ServiceError {
    /// HTTP-style status code describing the failure class.
    pub status: u16,
    /// Human-readable diagnostic message.
    pub message: String,
}

impl ServiceError {
    fn new(status: u16, message: impl ToString) -> Self {
        Self {
            status,
            message: message.to_string(),
        }
    }
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.write_str(&self.message)
    }
}

impl std::error::Error for ServiceError {}

/// Parse a LaTeX math string into a [`MathNode`] via `latex_extensions` and `latex2mathml`.
pub fn parse_latex_to_mathml(latex_str: &str) -> crate::error::Result<MathNode> {
    let mut clean = latex_str.trim();
    if clean.starts_with("$$") && clean.ends_with("$$") && clean.len() >= 4 {
        clean = clean[2..clean.len() - 2].trim();
    } else if clean.starts_with('$') && clean.ends_with('$') && clean.len() >= 2 {
        clean = clean[1..clean.len() - 1].trim();
    } else if (clean.starts_with(r"\[") && clean.ends_with(r"\]")
        || clean.starts_with(r"\(") && clean.ends_with(r"\)"))
        && clean.len() >= 4
    {
        clean = clean[2..clean.len() - 2].trim();
    }
    let mut math = latex_extensions::parse(clean)?;
    normalize_latex_integrals(&mut math);
    Ok(math)
}

/// Decode any supported equation input representation into a [`MathNode`].
pub fn decode_input_to_mathml(data: &str, format: &str) -> crate::error::Result<MathNode> {
    let trimmed = data.trim();

    if format == "latex"
        || trimmed.starts_with('$')
        || trimmed.starts_with(r"\[")
        || trimmed.starts_with(r"\(")
    {
        return parse_latex_to_mathml(trimmed);
    }

    if format == "flat_opc" || trimmed.contains("<pkg:package") || trimmed.starts_with("<?xml") {
        return flat_opc_to_mathml(trimmed);
    }

    if format == "omml"
        || trimmed.contains("<m:oMath")
        || trimmed.contains("<w:p")
        || trimmed.contains("<w:wordDocument")
        || trimmed.contains("<m:oMathPara")
    {
        return flat_opc_to_mathml(trimmed);
    }

    if format == "mathml" || trimmed.starts_with("<math") {
        return MathNode::parse(trimmed);
    }

    if format == "ole_base64" {
        let bin = base64_decode(trimmed).ok_or_else(|| {
            crate::error::Error::Xml("invalid base64 encoding for OLE data".into())
        })?;
        return ole_to_mathml(&bin).ok_or_else(|| {
            crate::error::Error::InvalidMtef("failed to decode MTEF from OLE data".into())
        });
    }

    if format == "emf_base64" {
        let bin = base64_decode(trimmed).ok_or_else(|| {
            crate::error::Error::Xml("invalid base64 encoding for EMF data".into())
        })?;
        let mtef = extract_mtef_from_emf(&bin).ok_or_else(|| {
            crate::error::Error::InvalidMtef("no embedded MTEF comment found in EMF".into())
        })?;
        return mtef_to_mathml(&mtef).ok_or_else(|| {
            crate::error::Error::InvalidMtef("failed to decode MTEF from EMF comment".into())
        });
    }

    if format == "wmf_base64" {
        let bin = base64_decode(trimmed).ok_or_else(|| {
            crate::error::Error::Xml("invalid base64 encoding for WMF data".into())
        })?;
        let mtef = extract_mtef_from_wmf(&bin).ok_or_else(|| {
            crate::error::Error::InvalidMtef("no embedded MTEF found in WMF".into())
        })?;
        return mtef_to_mathml(&mtef).ok_or_else(|| {
            crate::error::Error::InvalidMtef("failed to decode MTEF from WMF".into())
        });
    }

    // Fallback: attempt Flat OPC / XML parse, then MathNode parse.
    if let Ok(math) = flat_opc_to_mathml(trimmed) {
        return Ok(math);
    }
    if let Ok(math) = MathNode::parse(trimmed) {
        return Ok(math);
    }

    Err(crate::error::Error::Xml(format!(
        "unrecognized equation input format: '{format}'"
    )))
}

/// Convert a `.docx` document using the bridge-compatible request and target aliases.
///
/// An omitted `output_path` modifies the input file in place; interactive callers
/// should supply a distinct output path.
pub fn convert_document(
    req: &ConvertDocumentRequest,
) -> std::result::Result<ConvertDocumentResponse, ServiceError> {
    convert_document_with_cancel(req, None)
}

/// Like [`convert_document`], supporting cooperative cancellation via `cancel`.
pub fn convert_document_with_cancel(
    req: &ConvertDocumentRequest,
    cancel: Option<&std::sync::atomic::AtomicBool>,
) -> std::result::Result<ConvertDocumentResponse, ServiceError> {
    if req.input_path.contains('\0')
        || req
            .output_path
            .as_deref()
            .is_some_and(|path| path.contains('\0'))
    {
        return Err(ServiceError::new(400, "invalid null byte in path"));
    }
    if !req.font_pt.is_finite() || !(1.0..=200.0).contains(&req.font_pt) {
        return Err(ServiceError::new(
            400,
            "font_pt must be a finite number between 1.0 and 200.0",
        ));
    }
    let input = std::path::Path::new(&req.input_path);
    if !input.exists() {
        return Err(ServiceError::new(
            404,
            format!("input file does not exist: {}", input.display()),
        ));
    }
    if !input
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("docx"))
    {
        return Err(ServiceError::new(
            400,
            "only .docx files are supported for document conversion",
        ));
    }
    let output = req.output_path.as_deref().unwrap_or(&req.input_path);
    if !std::path::Path::new(output)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("docx"))
    {
        return Err(ServiceError::new(
            400,
            "output_path must have a .docx extension",
        ));
    }
    let preview = if req.preview.eq_ignore_ascii_case("png") {
        PreviewFormat::Png
    } else {
        PreviewFormat::Emf
    };
    let summary = match req.target.as_str() {
        "omml" | "mathtype_to_omml" => {
            convert_to_omml_with_cancel(input, output, &ToOmmlOptions::default(), cancel)
        }
        "mathtype" | "omml_to_mathtype" => convert_with_cancel(
            input,
            output,
            &Options {
                font_pt: req.font_pt,
                preview,
                ..Default::default()
            },
            cancel,
        ),
        "upgrade_omml" | "legacy_to_omml" | "upgrade_modern_omml" => {
            upgrade_legacy_mathtype_docx_with_cancel(
                input,
                output,
                &UpgradeOptions {
                    target: UpgradeTarget::ModernOmml,
                    font_pt: req.font_pt,
                    ..Default::default()
                },
                cancel,
            )
        }
        "upgrade_mathtype" | "legacy_to_mathtype" | "upgrade_modern_mathtype" => {
            upgrade_legacy_mathtype_docx_with_cancel(
                input,
                output,
                &UpgradeOptions {
                    target: UpgradeTarget::ModernMathType,
                    font_pt: req.font_pt,
                    ..Default::default()
                },
                cancel,
            )
        }
        other => {
            return Err(ServiceError::new(
                400,
                format!("unknown document target '{other}'"),
            ))
        }
    }
    .map_err(|err| match err {
        crate::Error::Cancelled => ServiceError::new(499, "document conversion cancelled"),
        other => ServiceError::new(500, format!("document conversion error: {other}")),
    })?;
    Ok(ConvertDocumentResponse {
        status: "ok".into(),
        found: summary.found,
        converted: summary.converted,
        skipped: summary.skipped,
        errors: summary.errors,
        output_path: output.into(),
    })
}

/// Convert a single equation snippet between supported formats (`omml`, `mathtype`,
/// `flat_opc_*`, `mathml`, `latex`, `emf`, `png`).
pub fn convert_snippet(
    req: &ConvertSnippetRequest,
) -> std::result::Result<ConvertSnippetResponse, ServiceError> {
    if !req.font_pt.is_finite() || !(1.0..=200.0).contains(&req.font_pt) {
        return Err(ServiceError::new(
            400,
            "font_pt must be a finite number between 1.0 and 200.0",
        ));
    }
    let math = decode_input_to_mathml(&req.data, &req.from)
        .map_err(|err| ServiceError::new(422, format!("failed to parse input equation: {err}")))?;
    let mut response = ConvertSnippetResponse {
        status: "ok".into(),
        result: String::new(),
        emf_path: None,
        width_pt: None,
        height_pt: None,
        depth_pt: None,
    };
    response.result = match req.to.as_str() {
        "flat_opc_omml" => {
            if let Some(reason) = crate::omml::check_omml_unsupported(&math) {
                return Err(ServiceError::new(
                    422,
                    format!("{reason} is not supported in OMML"),
                ));
            }
            mathml_to_flat_opc(&math, FlatOpcMode::ModernOmml, req.font_pt)
        }
        "flat_opc_mathtype" | "mathtype" => {
            if let Some(reason) = crate::mtef::check_mathtype_unsupported(&math) {
                return Err(ServiceError::new(
                    422,
                    format!("{reason} is not supported in MathType"),
                ));
            }
            let mtef = mathml_to_mtef_bytes(&math)
                .ok()
                .map(|records| crate::mtef::build_mtef(&records));
            if let Ok(pic) = render_emf_with_mtef(&math, req.font_pt, mtef.as_deref()) {
                let path = std::env::temp_dir()
                    .join(format!("eqntide_snippet_{}.emf", uuid::Uuid::new_v4()));
                if std::fs::write(&path, &pic.emf).is_ok() {
                    response.emf_path = Some(path.to_string_lossy().into_owned());
                    response.width_pt = Some(pic.width_pt);
                    response.height_pt = Some(pic.height_pt);
                    response.depth_pt = Some(pic.depth_pt);
                }
            }
            mathml_to_flat_opc(&math, FlatOpcMode::ModernMathtypeOleEmf, req.font_pt)
        }
        "omml" => {
            if let Some(reason) = crate::omml::check_omml_unsupported(&math) {
                return Err(ServiceError::new(
                    422,
                    format!("{reason} is not supported in OMML"),
                ));
            }
            mathml_to_omml_xml(&math)
        }
        "mathml" => Ok(math.to_xml()),
        "latex" => Ok(math.to_latex()),
        "emf" | "emf_base64" => render_emf(&math, req.font_pt).map(|pic| base64_encode(&pic.emf)),
        "png" | "png_base64" => {
            render_preview(&math, req.font_pt).map(|pic| base64_encode(&pic.png))
        }
        other => {
            return Err(ServiceError::new(
                400,
                format!("unknown target format '{other}'"),
            ))
        }
    }
    .map_err(|err| ServiceError::new(500, format!("conversion failed: {err}")))?;
    Ok(response)
}

/// Normalize environments and run `latex2mathml` with XML fixups for `latex_extensions`.
pub(super) fn parse_standard(clean: &str) -> crate::error::Result<MathNode> {
    // Imported LaTeX environments have equivalent native table representations.
    let normalized = latex_aliases(clean)
        .replace(r"\begin{cases}", r"\left\{\begin{matrix}")
        .replace(r"\end{cases}", r"\end{matrix}\right.")
        .replace(r"\begin{aligned}", r"\begin{align}")
        .replace(r"\end{aligned}", r"\end{align}")
        .replace(r"\begin{gathered}", r"\begin{matrix}")
        .replace(r"\end{gathered}", r"\end{matrix}");
    let mml = latex2mathml::latex_to_mathml(&normalized, latex2mathml::DisplayStyle::Inline)
        .map_err(|err| crate::error::Error::Xml(format!("LaTeX parsing error: {err}")))?;
    // latex2mathml 0.2 emits Undefined AST nodes as apparent mtext successes;
    // treat these as parse failures instead of exporting diagnostic text to Office.
    if mml.contains("<mtext>[PARSE ERROR:") {
        return Err(crate::Error::Xml(
            "unsupported or invalid LaTeX command".into(),
        ));
    }
    // latex2mathml 0.2.3 writes raw `<` operators without XML escaping and `</mro>` for sized parens.
    MathNode::parse(
        &mml.replace("&lang;", "⟨")
            .replace("&rang;", "⟩")
            .replace("><</", ">&lt;</")
            .replace("<mo><&#x0338;</mo>", "<mo>&lt;&#x0338;</mo>")
            .replace("</mro>", "</mo>")
            .replace("columnalign=left", "columnalign=\"left\"")
            .replace("columnalign=right", "columnalign=\"right\""),
    )
}

// Normalize complete control words rather than replacing prefixes (`\le` must
// not corrupt `\left` or `\leq`). Preserves escaped punctuation and row separators.
fn latex_aliases(input: &str) -> String {
    let mut chars = input.chars().peekable();
    let mut output = String::with_capacity(input.len());
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            output.push(ch);
            continue;
        }
        output.push(ch);
        if chars.peek().is_some_and(char::is_ascii_alphabetic) {
            let mut command = String::new();
            while let Some(&letter) = chars.peek().filter(|letter| letter.is_ascii_alphabetic()) {
                chars.next();
                command.push(letter);
            }
            output.push_str(match command.as_str() {
                "le" => "leq",
                "ge" => "geq",
                "ne" => "neq",
                "to" => "rightarrow",
                "gets" => "leftarrow",
                "implies" => "Rightarrow",
                "iff" => "Leftrightarrow",
                "land" => "wedge",
                "lor" => "vee",
                "lnot" => "neg",
                _ => &command,
            });
        } else if let Some(escaped) = chars.next() {
            output.push(escaped);
        }
    }
    output
}

// The editor's integral templates use limits above/below the sign. `latex2mathml`
// emits ordinary side scripts for integrals, so normalize only the LaTeX path.
// Imported MathML/OMML preserves its explicit placement and structure.
fn normalize_latex_integrals(node: &mut MathNode) {
    if node.attr("data-limloc") != Some("subSup")
        && node.children.first().is_some_and(|base| {
            base.tag == "mo"
                && base.text.trim().chars().count() == 1
                && base
                    .text
                    .trim()
                    .chars()
                    .next()
                    .is_some_and(crate::symbols::is_integral)
        })
    {
        node.tag = match node.tag.as_str() {
            "msub" => "munder".into(),
            "msup" => "mover".into(),
            "msubsup" => "munderover".into(),
            _ => node.tag.clone(),
        };
    }
    for child in &mut node.children {
        normalize_latex_integrals(child);
    }
}
