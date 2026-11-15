//! Conversion services shared by the CLI bridge and native desktop.
use crate::bridge::{
    ConvertDocumentRequest, ConvertDocumentResponse, ConvertSnippetRequest, ConvertSnippetResponse,
};
use crate::mtef::decode::{mtef_to_mathml, ole_to_mathml};
use crate::ole::{extract_mtef_from_emf, extract_mtef_from_wmf};
use crate::{docx::*, MathNode};
use crate::{
    mathml_to_mtef_bytes, mathml_to_omml_xml, render_emf, render_emf_with_mtef, render_preview,
};

#[derive(Debug)]
pub struct ServiceError {
    pub status: u16,
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for ServiceError {}

// Normalize complete control words rather than replacing prefixes (\le must
// not corrupt \left or \leq). Preserve escaped punctuation and row separators.
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
            while chars.peek().is_some_and(char::is_ascii_alphabetic) {
                command.push(chars.next().unwrap());
            }
            output.push_str(match command.as_str() {
                "le" => "leq",
                "ge" => "geq",
                "ne" => "neq",
                "tfrac" | "dfrac" => "frac",
                _ => &command,
            });
        } else if let Some(escaped) = chars.next() {
            output.push(escaped);
        }
    }
    output
}

/// Parse a LaTeX math string into a [`MathNode`] via latex2mathml.
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
    // These imported LaTeX environments have equivalent native table representations.
    let normalized = latex_aliases(clean)
        .replace(r"\begin{cases}", r"\left\{\begin{matrix}")
        .replace(r"\end{cases}", r"\end{matrix}\right.")
        .replace(r"\begin{aligned}", r"\begin{align}")
        .replace(r"\end{aligned}", r"\end{align}")
        .replace(r"\begin{gathered}", r"\begin{matrix}")
        .replace(r"\end{gathered}", r"\end{matrix}");
    let mml = latex2mathml::latex_to_mathml(&normalized, latex2mathml::DisplayStyle::Inline)
        .map_err(|e| crate::error::Error::Xml(format!("LaTeX parsing error: {e}")))?;
    // latex2mathml 0.2 emits Undefined AST nodes as apparent mtext successes.
    // Treat these as parse failures instead of exporting diagnostic text to Office.
    if mml.contains("<mtext>[PARSE ERROR:") {
        return Err(crate::Error::Xml(
            "unsupported or invalid LaTeX command".into(),
        ));
    }
    let mut math = MathNode::parse(
        &mml.replace("&lang;", "⟨")
            .replace("&rang;", "⟩")
            .replace("columnalign=left", "columnalign=\"left\"")
            .replace("columnalign=right", "columnalign=\"right\""),
    )?;
    normalize_latex_integrals(&mut math);
    Ok(math)
}

// The editor's integral templates use limits above/below the sign. The LaTeX
// dependency emits ordinary side scripts for integrals, so normalize only the
// LaTeX path. Imported MathML/OMML keeps its explicit placement and structure.
fn normalize_latex_integrals(node: &mut MathNode) {
    if node.children.first().is_some_and(|base| {
        base.tag == "mo"
            && base.text.trim().chars().count() == 1
            && base
                .text
                .trim()
                .chars()
                .next()
                .is_some_and(crate::symbols::is_integral)
    }) {
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

/// Helper to decode any supported equation input representation into [`MathNode`].
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

    // Fallback: Attempt Flat OPC / XML parse
    if let Ok(m) = flat_opc_to_mathml(trimmed) {
        return Ok(m);
    }

    // Fallback: Attempt MathNode parse
    if let Ok(m) = MathNode::parse(trimmed) {
        return Ok(m);
    }

    Err(crate::error::Error::Xml(format!(
        "unrecognized equation input format: '{format}'"
    )))
}

/// Convert a document using the bridge-compatible request and aliases.
/// An omitted output retains the bridge's existing in-place behavior.
/// Interactive callers should always supply a distinct output path.
pub fn convert_document(
    req: &ConvertDocumentRequest,
) -> std::result::Result<ConvertDocumentResponse, ServiceError> {
    let input = std::path::Path::new(&req.input_path);
    if !input.exists() {
        return Err(ServiceError::new(
            404,
            format!("input file does not exist: {}", input.display()),
        ));
    }
    if !input
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("docx"))
    {
        return Err(ServiceError::new(
            400,
            "only .docx files are supported for document conversion",
        ));
    }
    let output = req.output_path.as_deref().unwrap_or(&req.input_path);
    let preview = if req.preview.eq_ignore_ascii_case("png") {
        PreviewFormat::Png
    } else {
        PreviewFormat::Emf
    };
    let summary = match req.target.as_str() {
        "omml" | "mathtype_to_omml" => {
            convert_to_omml_with(input, output, &ToOmmlOptions::default())
        }
        "mathtype" | "omml_to_mathtype" => convert_with(
            input,
            output,
            &Options {
                font_pt: req.font_pt,
                preview,
                ..Default::default()
            },
        ),
        "upgrade_omml" | "legacy_to_omml" | "upgrade_modern_omml" => {
            upgrade_legacy_mathtype_docx_with(
                input,
                output,
                &UpgradeOptions {
                    target: UpgradeTarget::ModernOmml,
                    font_pt: req.font_pt,
                    math_font: None,
                },
            )
        }
        "upgrade_mathtype" | "legacy_to_mathtype" | "upgrade_modern_mathtype" => {
            upgrade_legacy_mathtype_docx_with(
                input,
                output,
                &UpgradeOptions {
                    target: UpgradeTarget::ModernMathType,
                    font_pt: req.font_pt,
                    math_font: None,
                },
            )
        }
        other => {
            return Err(ServiceError::new(
                400,
                format!("unknown document target '{other}'"),
            ))
        }
    }
    .map_err(|e| ServiceError::new(500, format!("document conversion error: {e}")))?;
    Ok(ConvertDocumentResponse {
        status: "ok".into(),
        found: summary.found,
        converted: summary.converted,
        skipped: summary.skipped,
        errors: summary.errors,
        output_path: output.into(),
    })
}

pub fn convert_snippet(
    req: &ConvertSnippetRequest,
) -> std::result::Result<ConvertSnippetResponse, ServiceError> {
    let math = decode_input_to_mathml(&req.data, &req.from)
        .map_err(|e| ServiceError::new(422, format!("failed to parse input equation: {e}")))?;
    let mut response = ConvertSnippetResponse {
        status: "ok".into(),
        result: String::new(),
        emf_path: None,
        width_pt: None,
        height_pt: None,
        depth_pt: None,
    };
    response.result = match req.to.as_str() {
        "flat_opc_omml" => mathml_to_flat_opc(&math, FlatOpcMode::ModernOmml, req.font_pt),
        "flat_opc_mathtype" | "mathtype" => {
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
        "omml" => mathml_to_omml_xml(&math),
        "mathml" => Ok(math.to_xml()),
        "latex" => Ok(math.to_latex()),
        "emf" | "emf_base64" => render_emf(&math, req.font_pt).map(|p| base64_encode(&p.emf)),
        "png" | "png_base64" => render_preview(&math, req.font_pt).map(|p| base64_encode(&p.png)),
        other => {
            return Err(ServiceError::new(
                400,
                format!("unknown target format '{other}'"),
            ))
        }
    }
    .map_err(|e| ServiceError::new(500, format!("conversion failed: {e}")))?;
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dfrac_converts_through_both_office_toggle_targets() {
        for to in ["flat_opc_omml", "flat_opc_mathtype"] {
            let response = convert_snippet(&ConvertSnippetRequest {
                data: r"\dfrac{1}{2}".into(),
                from: "latex".into(),
                to: to.into(),
                font_pt: 12.0,
            })
            .unwrap();
            let recovered = flat_opc_to_mathml(&response.result).unwrap();
            assert!(
                recovered.iter().iter().any(|node| node.tag == "mfrac"),
                "{to}"
            );
            if let Some(path) = response.emf_path {
                std::fs::remove_file(path).unwrap();
            }
        }
    }

    #[test]
    fn unknown_latex_commands_are_errors_instead_of_equation_text() {
        assert!(parse_latex_to_mathml(r"x+\thiscommanddoesnotexist{a}").is_err());
    }
    #[test]
    fn aliases_preserve_delimiters_and_longer_control_words() {
        let math = parse_latex_to_mathml(r"\left(x\le0\right)\geq1").unwrap();
        let text: String = math.iter().iter().map(|n| n.text.as_str()).collect();
        assert!(text.contains('≤'));
        assert!(text.contains('≥'));
        assert!(!text.contains("PARSE ERROR"));
        assert!(parse_latex_to_mathml(r"\begin{cases}x & x\le0 \\ x^2 & x>0\end{cases}").is_ok());
    }
    #[test]
    fn latex_integral_limits_use_the_editors_under_over_layout() {
        for (source, tag) in [
            (r"\int_a^b x dx", "munderover"),
            (r"\int_a x dx", "munder"),
            (r"\int^b x dx", "mover"),
            (r"\iint_a^b x dx", "munderover"),
            (r"\iiint_a^b x dx", "munderover"),
            (r"\oint_a^b x dx", "munderover"),
        ] {
            let math = parse_latex_to_mathml(source).unwrap();
            assert!(
                math.iter().iter().any(|n| n.tag == tag
                    && n.children.first().is_some_and(|b| b.tag == "mo"
                        && b.text
                            .chars()
                            .next()
                            .is_some_and(crate::symbols::is_integral))),
                "{source}"
            );
            let omml = mathml_to_omml_xml(&math).unwrap();
            assert!(omml.contains("undOvr"), "{source}");
        }
        for source in ["x_a", "x^b", "x_a^b"] {
            let original =
                latex2mathml::latex_to_mathml(source, latex2mathml::DisplayStyle::Inline).unwrap();
            assert_eq!(
                parse_latex_to_mathml(source).unwrap(),
                MathNode::parse(&original).unwrap()
            );
        }
    }
    #[test]
    fn mathml_import_keeps_explicit_side_limits() {
        let xml = r#"<math><munderover data-limloc="subSup"><mo>∫</mo><mi>a</mi><mi>b</mi></munderover></math>"#;
        let math = decode_input_to_mathml(xml, "mathml").unwrap();
        assert_eq!(math, MathNode::parse(xml).unwrap());
        assert!(mathml_to_omml_xml(&math).unwrap().contains("subSup"));
    }
}
