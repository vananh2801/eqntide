//! Single-equation Flat OPC (`pkg:package`) packager and parser.
//!
//! Provides conversion between [`MathNode`] (MathML) and Word Flat OPC XML
//! (`<pkg:package>`) as used by Word Add-ins (`Office.CoercionType.Ooxml`,
//! `getSelection().getOoxml()`) and WPS Office.
//!
//! # Modes
//!
//! * [`FlatOpcMode::ModernOmml`]: Packages the equation as native Word OMML
//!   (`<m:oMath>`) inside `/word/document.xml`.
//! * [`FlatOpcMode::ModernMathtypeOleEmf`]: Packages the equation as a modern
//!   MathType 7 OLE object (`Equation.DSMT4`) with embedded MTEF 5.0 binary
//!   in `/word/embeddings/oleObject1.bin` and a vector EMF preview carrying
//!   an `EMR_COMMENT` MTEF payload in `/word/media/equation1.emf`.

use std::collections::HashMap;

use crate::error::{Error, Result};
use crate::mathml::MathNode;
use crate::mtef::decode::{mtef_to_mathml, ole_to_mathml};
use crate::mtef::encode::mathml_to_mtef_bytes;
use crate::mtef::make_equation_native;
use crate::ole::{extract_mtef_from_emf, extract_mtef_from_wmf, make_ole};
use crate::omml::decode::{omml_to_mathml, NS_M};
use crate::omml::encode::mathml_to_omml;
use crate::render::emf::render_emf_with_mtef;
use crate::xml::{self, Element, Node};

use super::consts::*;
use super::ole_xml::make_object_run;
use super::to_omml::{
    build_rel_map, collect_imagedata_rids, find_mathtype_ole_in_subtree,
};

/// Open Packaging Conventions (Flat OPC) XML namespace.
pub const NS_PKG: &str = "http://schemas.microsoft.com/office/2006/xml/packaging";

/// Content type for package relationships XML.
pub const CT_RELATIONSHIPS: &str =
    "application/vnd.openxmlformats-package.relationships+xml";

/// Content type for main document XML.
pub const CT_DOCUMENT: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml";

/// Content type for vector EMF pictures.
pub const CT_EMF: &str = "image/x-emf";

/// Relationship type for main office document.
pub const REL_OFFICE_DOCUMENT: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";

/// Target equation packaging mode for single-equation Flat OPC XML generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlatOpcMode {
    /// Modern native Word OMML equation (`<m:oMath>`).
    ModernOmml,
    /// Modern MathType 7 OLE object (`Equation.DSMT4`) + vector EMF with MTEF 5.0 comment.
    ModernMathtypeOleEmf,
}

impl FlatOpcMode {
    /// Alias for [`FlatOpcMode::ModernMathtypeOleEmf`].
    pub const MODERN_MATH_TYPE: FlatOpcMode = FlatOpcMode::ModernMathtypeOleEmf;
}

// ── Base64 Encode / Decode (Zero-dependency, safe Rust) ─────────────────────────

const BASE64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Encode binary bytes to standard base64 string.
pub fn base64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    let chunks = data.chunks_exact(3);
    let remainder = chunks.remainder();

    for chunk in chunks {
        let n = ((chunk[0] as u32) << 16) | ((chunk[1] as u32) << 8) | (chunk[2] as u32);
        out.push(BASE64_ALPHABET[((n >> 18) & 0x3F) as usize] as char);
        out.push(BASE64_ALPHABET[((n >> 12) & 0x3F) as usize] as char);
        out.push(BASE64_ALPHABET[((n >> 6) & 0x3F) as usize] as char);
        out.push(BASE64_ALPHABET[(n & 0x3F) as usize] as char);
    }

    match remainder.len() {
        1 => {
            let n = (remainder[0] as u32) << 16;
            out.push(BASE64_ALPHABET[((n >> 18) & 0x3F) as usize] as char);
            out.push(BASE64_ALPHABET[((n >> 12) & 0x3F) as usize] as char);
            out.push('=');
            out.push('=');
        }
        2 => {
            let n = ((remainder[0] as u32) << 16) | ((remainder[1] as u32) << 8);
            out.push(BASE64_ALPHABET[((n >> 18) & 0x3F) as usize] as char);
            out.push(BASE64_ALPHABET[((n >> 12) & 0x3F) as usize] as char);
            out.push(BASE64_ALPHABET[((n >> 6) & 0x3F) as usize] as char);
            out.push('=');
        }
        _ => {}
    }

    out
}

/// Decode standard base64 string, ignoring whitespace and newlines.
pub fn base64_decode(input: &str) -> Option<Vec<u8>> {
    let mut buf = Vec::with_capacity(input.len() * 3 / 4);
    let mut chunk = [0u8; 4];
    let mut chunk_len = 0;

    for b in input.bytes() {
        if b.is_ascii_whitespace() {
            continue;
        }
        let val = match b {
            b'A'..=b'Z' => b - b'A',
            b'a'..=b'z' => b - b'a' + 26,
            b'0'..=b'9' => b - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => 64, // sentinel for padding
            _ => return None,
        };
        chunk[chunk_len] = val;
        chunk_len += 1;
        if chunk_len == 4 {
            let (c0, c1, c2, c3) = (chunk[0], chunk[1], chunk[2], chunk[3]);
            if c0 == 64 || c1 == 64 {
                return None; // padding cannot be in first two positions
            }
            let b0 = (c0 << 2) | (c1 >> 4);
            buf.push(b0);
            if c2 != 64 {
                let b1 = ((c1 & 0x0F) << 4) | (c2 >> 2);
                buf.push(b1);
                if c3 != 64 {
                    let b2 = ((c2 & 0x03) << 6) | c3;
                    buf.push(b2);
                }
            } else if c3 != 64 {
                return None; // cannot have data after padding
            }
            chunk_len = 0;
        }
    }
    if chunk_len != 0 {
        return None; // truncated input
    }
    Some(buf)
}

// ── Flat OPC Packaging ─────────────────────────────────────────────────────────

fn make_xml_part(name: &str, content_type: &str, xml_root: Element) -> Element {
    let mut part = Element::new(Some(NS_PKG), Some("pkg"), "part");
    part.set_attr(Some(NS_PKG), Some("pkg"), "name", name);
    part.set_attr(Some(NS_PKG), Some("pkg"), "contentType", content_type);

    let mut xml_data = Element::new(Some(NS_PKG), Some("pkg"), "xmlData");
    xml_data.push(xml_root);
    part.push(xml_data);
    part
}

fn make_binary_part(name: &str, content_type: &str, data: &[u8]) -> Element {
    let mut part = Element::new(Some(NS_PKG), Some("pkg"), "part");
    part.set_attr(Some(NS_PKG), Some("pkg"), "name", name);
    part.set_attr(Some(NS_PKG), Some("pkg"), "contentType", content_type);

    let mut bin_data = Element::new(Some(NS_PKG), Some("pkg"), "binaryData");
    bin_data.children.push(Node::Text(base64_encode(data)));
    part.push(bin_data);
    part
}

fn build_root_rels() -> Element {
    let mut rels = Element::new(Some(NS_PKG_REL), None, "Relationships")
        .with_nsdecl(None, NS_PKG_REL);
    let mut rel = Element::new(Some(NS_PKG_REL), None, "Relationship");
    rel.set_attr(None, None, "Id", "rId1");
    rel.set_attr(None, None, "Type", REL_OFFICE_DOCUMENT);
    rel.set_attr(None, None, "Target", "word/document.xml");
    rels.push(rel);
    rels
}

fn build_document_rels(mode: FlatOpcMode) -> Element {
    let mut rels = Element::new(Some(NS_PKG_REL), None, "Relationships")
        .with_nsdecl(None, NS_PKG_REL);
    if mode == FlatOpcMode::ModernMathtypeOleEmf {
        let mut rel1 = Element::new(Some(NS_PKG_REL), None, "Relationship");
        rel1.set_attr(None, None, "Id", "rId1");
        rel1.set_attr(None, None, "Type", REL_OLEOBJECT);
        rel1.set_attr(None, None, "Target", "embeddings/oleObject1.bin");
        rels.push(rel1);

        let mut rel2 = Element::new(Some(NS_PKG_REL), None, "Relationship");
        rel2.set_attr(None, None, "Id", "rId2");
        rel2.set_attr(None, None, "Type", REL_IMAGE);
        rel2.set_attr(None, None, "Target", "media/equation1.emf");
        rels.push(rel2);
    }
    rels
}

fn build_document_xml(mode: FlatOpcMode, content_element: Element) -> Element {
    let mut doc = Element::new(Some(NS_W), Some("w"), "document")
        .with_nsdecl(Some("w"), NS_W)
        .with_nsdecl(Some("m"), NS_M)
        .with_nsdecl(Some("r"), NS_R);
    if mode == FlatOpcMode::ModernMathtypeOleEmf {
        doc = doc
            .with_nsdecl(Some("v"), NS_V)
            .with_nsdecl(Some("o"), NS_O);
    }
    let mut body = Element::new(Some(NS_W), Some("w"), "body");
    let mut p = Element::new(Some(NS_W), Some("w"), "p");
    p.push(content_element);
    body.push(p);
    doc.push(body);
    doc
}

/// Package a single equation (`MathNode`) into a complete Flat OPC (`pkg:package`) XML string.
///
/// * `math`: The MathML intermediate tree representation of the equation.
/// * `mode`: Packaging mode ([`FlatOpcMode::ModernOmml`] or [`FlatOpcMode::ModernMathtypeOleEmf`]).
/// * `font_pt`: Point size used for rendering vector EMF dimensions (e.g. `12.0`).
pub fn mathml_to_flat_opc(math: &MathNode, mode: FlatOpcMode, font_pt: f32) -> Result<String> {
    let mut pkg = Element::new(Some(NS_PKG), Some("pkg"), "package")
        .with_nsdecl(Some("pkg"), NS_PKG);

    // 1. /_rels/.rels
    pkg.push(make_xml_part(
        "/_rels/.rels",
        CT_RELATIONSHIPS,
        build_root_rels(),
    ));

    // 2. /word/_rels/document.xml.rels
    pkg.push(make_xml_part(
        "/word/_rels/document.xml.rels",
        CT_RELATIONSHIPS,
        build_document_rels(mode),
    ));

    // 3. /word/document.xml & binary parts
    match mode {
        FlatOpcMode::ModernOmml => {
            let omml_el = mathml_to_omml(math)?;
            let doc_el = build_document_xml(mode, omml_el);
            pkg.push(make_xml_part("/word/document.xml", CT_DOCUMENT, doc_el));
        }
        FlatOpcMode::ModernMathtypeOleEmf => {
            let mtef_bytes = mathml_to_mtef_bytes(math)?;
            if mtef_bytes.is_empty() {
                return Err(Error::InvalidMtef("empty MTEF stream generated".into()));
            }
            let native_stream = make_equation_native(&mtef_bytes);
            let ole_bin = make_ole(&native_stream)?;
            let emf_pic = render_emf_with_mtef(math, font_pt, Some(&mtef_bytes))?;

            let obj_run = make_object_run(
                "rId1",
                "rId2",
                "_x0000_i1025",
                "1",
                emf_pic.width_pt,
                emf_pic.height_pt,
                emf_pic.depth_pt,
            );
            let doc_el = build_document_xml(mode, obj_run);
            pkg.push(make_xml_part("/word/document.xml", CT_DOCUMENT, doc_el));

            pkg.push(make_binary_part(
                "/word/embeddings/oleObject1.bin",
                CT_OLEOBJECT,
                &ole_bin,
            ));
            pkg.push(make_binary_part(
                "/word/media/equation1.emf",
                CT_EMF,
                &emf_pic.emf,
            ));
        }
    }

    let mut out = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<?mso-application progid=\"Word.Document\"?>\n",
    );
    out.push_str(&xml::to_string(&pkg));
    Ok(out)
}

// ── Flat OPC Parsing / Extraction ──────────────────────────────────────────────

#[derive(Clone)]
struct PartData {
    #[allow(dead_code)]
    content_type: String,
    xml_data: Option<Element>,
    binary_data: Option<Vec<u8>>,
}

fn all_text(e: &Element) -> String {
    let mut out = String::new();
    fn walk(e: &Element, out: &mut String) {
        for child in &e.children {
            match child {
                Node::Text(t) => out.push_str(t),
                Node::Element(el) => walk(el, out),
                _ => {}
            }
        }
    }
    walk(e, &mut out);
    out
}

fn normalize_part_name(name: &str) -> String {
    name.strip_prefix('/').unwrap_or(name).to_owned()
}

fn extract_from_package(pkg_el: &Element) -> Result<MathNode> {
    let mut parts: HashMap<String, PartData> = HashMap::new();

    for child in pkg_el.elements() {
        if !child.is(NS_PKG, "part") && child.local != "part" {
            continue;
        }
        let Some(name) = child
            .attr(Some(NS_PKG), "name")
            .or_else(|| child.attr(None, "name"))
        else {
            continue;
        };
        let content_type = child
            .attr(Some(NS_PKG), "contentType")
            .or_else(|| child.attr(None, "contentType"))
            .unwrap_or("")
            .to_owned();

        let mut xml_data: Option<Element> = None;
        let mut binary_data: Option<Vec<u8>> = None;

        for sub in child.elements() {
            if sub.is(NS_PKG, "xmlData") || sub.local == "xmlData" {
                if let Some(first_el) = sub.elements().next() {
                    xml_data = Some(first_el.clone());
                }
            } else if sub.is(NS_PKG, "binaryData") || sub.local == "binaryData" {
                let text = all_text(sub);
                if let Some(decoded) = base64_decode(&text) {
                    binary_data = Some(decoded);
                }
            }
        }

        let norm_name = normalize_part_name(name);
        parts.insert(
            norm_name,
            PartData {
                content_type,
                xml_data,
                binary_data,
            },
        );
    }

    // Attempt 1: Document XML traversal
    if let Some(doc_part) = parts.get("word/document.xml") {
        if let Some(ref doc_el) = doc_part.xml_data {
            // Build rel_map from word/_rels/document.xml.rels
            let rel_map = parts
                .get("word/_rels/document.xml.rels")
                .and_then(|p| p.xml_data.as_ref())
                .map(|rels| build_rel_map(rels, "word/document.xml"))
                .unwrap_or_default();

            // 1a. MathType OLE object in document.xml
            let ole_nodes = find_mathtype_ole_in_subtree(doc_el);
            for ole in ole_nodes {
                let rid = ole
                    .attr(Some(NS_R), "id")
                    .or_else(|| ole.attr(None, "id"));
                if let Some(rid) = rid {
                    if let Some(target) = rel_map.get(rid) {
                        let norm_target = normalize_part_name(target);
                        if let Some(target_part) = parts.get(&norm_target) {
                            if let Some(ref bin) = target_part.binary_data {
                                if let Some(math) = ole_to_mathml(bin) {
                                    return Ok(math);
                                }
                            }
                        }
                    }
                }
            }

            // 1b. Direct <m:oMath> in document.xml
            for el in doc_el.iter() {
                if el.is(NS_M, "oMath") || el.local == "oMath" {
                    if let Ok(math) = omml_to_mathml(el) {
                        return Ok(math);
                    }
                }
            }

            // 1c. Standalone image with embedded MTEF referenced in document.xml
            let img_rids = collect_imagedata_rids(doc_el);
            for rid in img_rids {
                if let Some(target) = rel_map.get(&rid) {
                    let norm_target = normalize_part_name(target);
                    if let Some(target_part) = parts.get(&norm_target) {
                        if let Some(ref data) = target_part.binary_data {
                            if let Some(mtef) = extract_mtef_from_emf(data)
                                .or_else(|| extract_mtef_from_wmf(data))
                            {
                                if let Some(math) = mtef_to_mathml(&mtef) {
                                    return Ok(math);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Attempt 2: Search any other binary or XML parts in the package
    for (name, part) in &parts {
        if let Some(ref bin) = part.binary_data {
            if name.contains("oleObject") || name.ends_with(".bin") {
                if let Some(math) = ole_to_mathml(bin) {
                    return Ok(math);
                }
            }
            if name.ends_with(".emf") {
                if let Some(mtef) = extract_mtef_from_emf(bin) {
                    if let Some(math) = mtef_to_mathml(&mtef) {
                        return Ok(math);
                    }
                }
            }
            if name.ends_with(".wmf") {
                if let Some(mtef) = extract_mtef_from_wmf(bin) {
                    if let Some(math) = mtef_to_mathml(&mtef) {
                        return Ok(math);
                    }
                }
            }
        }
    }

    for part in parts.values() {
        if let Some(ref xml_el) = part.xml_data {
            for el in xml_el.iter() {
                if el.is(NS_M, "oMath") || el.local == "oMath" {
                    if let Ok(math) = omml_to_mathml(el) {
                        return Ok(math);
                    }
                }
            }
        }
    }

    Err(Error::Xml(
        "no math equation (<m:oMath>, OLE bin, or MTEF image) found in Flat OPC package".into(),
    ))
}

fn extract_from_raw_snippet(root: &Element) -> Result<MathNode> {
    for el in root.iter() {
        if el.is(NS_M, "oMath") || el.local == "oMath" {
            if let Ok(math) = omml_to_mathml(el) {
                return Ok(math);
            }
        }
    }

    Err(Error::Xml(
        "no math equation (<m:oMath>) found in XML snippet".into(),
    ))
}

/// Parse a Flat OPC (`pkg:package`) XML string or raw Word XML selection snippet into MathML ([`MathNode`]).
///
/// Supports:
/// * Full Flat OPC packages with `<m:oMath>`.
/// * Full Flat OPC packages with MathType OLE (`oleObject*.bin`).
/// * Full Flat OPC packages with vector EMF or WMF images containing embedded MTEF (`AppsMFCC`).
/// * Direct `<m:oMath>` or `<w:p>` selection snippets as a fallback.
pub fn flat_opc_to_mathml(xml_str: &str) -> Result<MathNode> {
    let root = xml::parse(xml_str.as_bytes())?;

    if root.is(NS_PKG, "package") || root.local == "package" {
        extract_from_package(&root)
    } else {
        extract_from_raw_snippet(&root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mathml::MathNode;

    #[test]
    fn test_base64_roundtrip() {
        let cases: &[&[u8]] = &[
            b"",
            b"f",
            b"fo",
            b"foo",
            b"foob",
            b"fooba",
            b"foobar",
            b"Hello World \x00\x01\x02\xFF\xFE\xFD",
        ];
        for data in cases {
            let encoded = base64_encode(data);
            let decoded = base64_decode(&encoded).expect("valid decode");
            assert_eq!(&decoded, *data);
        }

        // Test with whitespace and newlines
        let formatted = "SGVs bG8g V29y\r\n bGQ=";
        let decoded = base64_decode(formatted).expect("decode with whitespace");
        assert_eq!(decoded, b"Hello World");
    }

    #[test]
    fn test_flat_opc_roundtrip_omml() {
        let math = MathNode::parse("<math><mi>x</mi><mo>+</mo><mn>1</mn></math>").unwrap();

        let opc_xml = mathml_to_flat_opc(&math, FlatOpcMode::ModernOmml, 12.0)
            .expect("package OMML flat OPC");
        assert!(opc_xml.contains("pkg:package"));
        assert!(opc_xml.contains("/word/document.xml"));
        assert!(opc_xml.contains("m:oMath"));

        let recovered = flat_opc_to_mathml(&opc_xml).expect("parse OMML flat OPC");
        assert_eq!(recovered.find_all("mi")[0].text, "x");
        assert_eq!(recovered.find_all("mo")[0].text, "+");
        assert_eq!(recovered.find_all("mn")[0].text, "1");
    }

    #[test]
    fn test_flat_opc_roundtrip_mathtype_ole() {
        let math = MathNode::parse("<math><mi>y</mi><mo>=</mo><msup><mi>x</mi><mn>2</mn></msup></math>").unwrap();

        let opc_xml = mathml_to_flat_opc(&math, FlatOpcMode::ModernMathtypeOleEmf, 12.0)
            .expect("package MathType flat OPC");
        assert!(opc_xml.contains("pkg:package"));
        assert!(opc_xml.contains("/word/embeddings/oleObject1.bin"));
        assert!(opc_xml.contains("/word/media/equation1.emf"));
        assert!(opc_xml.contains("Equation.DSMT4"));

        let recovered = flat_opc_to_mathml(&opc_xml).expect("parse MathType flat OPC");
        let mi_list = recovered.find_all("mi");
        assert!(mi_list.iter().any(|n| n.text == "y"));
        assert!(mi_list.iter().any(|n| n.text == "x"));
        let mn_list = recovered.find_all("mn");
        assert!(mn_list.iter().any(|n| n.text == "2"));
    }

    #[test]
    fn test_flat_opc_emf_mtef_fallback() {
        let math = MathNode::parse("<math><mi>a</mi><mo>+</mo><mi>b</mi></math>").unwrap();

        let opc_xml = mathml_to_flat_opc(&math, FlatOpcMode::ModernMathtypeOleEmf, 12.0)
            .expect("package MathType flat OPC");

        // Simulate a document where the OLE bin part was stripped or corrupted,
        // leaving only the vector EMF with EMR_COMMENT MTEF.
        let stripped_xml = opc_xml.replace("/word/embeddings/oleObject1.bin", "/word/embeddings/dummy.bin");
        // Also remove the OLEObject node from document.xml to simulate standalone picture
        let stripped_xml = stripped_xml.replace("ProgID=\"Equation.DSMT4\"", "ProgID=\"Unknown\"");

        let recovered = flat_opc_to_mathml(&stripped_xml).expect("parse EMF MTEF fallback");
        let mi_list = recovered.find_all("mi");
        assert!(mi_list.iter().any(|n| n.text == "a"));
        assert!(mi_list.iter().any(|n| n.text == "b"));
    }

    #[test]
    fn test_raw_snippet_fallback() {
        let raw_omml = r#"<w:p xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math"><m:oMath><m:r><m:t>E=mc</m:t></m:r></m:oMath></w:p>"#;
        let recovered = flat_opc_to_mathml(raw_omml).expect("parse raw snippet");
        let text_found = recovered.iter().into_iter().any(|n| n.text.contains("E=mc"));
        assert!(text_found);
    }
}
