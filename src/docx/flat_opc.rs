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
use super::to_omml::{build_rel_map, collect_imagedata_rids};

/// Open Packaging Conventions (Flat OPC) XML namespace.
pub const NS_PKG: &str = "http://schemas.microsoft.com/office/2006/xmlPackage";
pub const NS_PKG_LEGACY: &str = "http://schemas.microsoft.com/office/2006/xml/packaging";

/// Content type for package relationships XML.
pub const CT_RELATIONSHIPS: &str = "application/vnd.openxmlformats-package.relationships+xml";

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
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    let (chunks, remainder) = data.as_chunks::<3>();

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
    let mut rels =
        Element::new(Some(NS_PKG_REL), None, "Relationships").with_nsdecl(None, NS_PKG_REL);
    let mut rel = Element::new(Some(NS_PKG_REL), None, "Relationship");
    rel.set_attr(None, None, "Id", "rId1");
    rel.set_attr(None, None, "Type", REL_OFFICE_DOCUMENT);
    rel.set_attr(None, None, "Target", "word/document.xml");
    rels.push(rel);
    rels
}

fn build_document_rels(
    mode: FlatOpcMode,
    ole_rid: &str,
    img_rid: &str,
    ole_target: &str,
    img_target: &str,
) -> Element {
    let mut rels =
        Element::new(Some(NS_PKG_REL), None, "Relationships").with_nsdecl(None, NS_PKG_REL);
    if mode == FlatOpcMode::ModernMathtypeOleEmf {
        let mut rel1 = Element::new(Some(NS_PKG_REL), None, "Relationship");
        rel1.set_attr(None, None, "Id", ole_rid);
        rel1.set_attr(None, None, "Type", REL_OLEOBJECT);
        rel1.set_attr(None, None, "Target", ole_target);
        rels.push(rel1);

        let mut rel2 = Element::new(Some(NS_PKG_REL), None, "Relationship");
        rel2.set_attr(None, None, "Id", img_rid);
        rel2.set_attr(None, None, "Type", REL_IMAGE);
        rel2.set_attr(None, None, "Target", img_target);
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
    // Place inline equation directly inside <w:p>
    p.push(content_element);

    body.push(p);
    doc.push(body);
    doc
}

fn build_omml_package_parts(pkg: &mut Element, math: &MathNode) -> Result<()> {
    pkg.push(make_xml_part(
        "/word/_rels/document.xml.rels",
        CT_RELATIONSHIPS,
        build_document_rels(FlatOpcMode::ModernOmml, "", "", "", ""),
    ));
    let omml_el = mathml_to_omml(math)?;
    pkg.push(make_xml_part(
        "/word/document.xml",
        CT_DOCUMENT,
        build_document_xml(FlatOpcMode::ModernOmml, omml_el),
    ));
    Ok(())
}

struct MathTypePartNames {
    uid: String,
    shape_id: String,
    object_id: String,
    ole_target: String,
    image_target: String,
}

fn make_mathtype_part_names() -> MathTypePartNames {
    let id = uuid::Uuid::new_v4();
    let uid = id.simple().to_string();
    MathTypePartNames {
        ole_target: format!("embeddings/oleObject{uid}.bin"),
        image_target: format!("media/equation{uid}.emf"),
        shape_id: format!("_x0000_i{uid}"),
        object_id: format!("_{}", id.as_u128()),
        uid,
    }
}

fn serialize_flat_opc_package(pkg: &Element) -> String {
    let mut out = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<?mso-application progid=\"Word.Document\"?>\n",
    );
    out.push_str(&xml::to_string(pkg));
    out
}

/// Package a single equation (`MathNode`) into a complete Flat OPC (`pkg:package`) XML string.
///
/// * `math`: The MathML intermediate tree representation of the equation.
/// * `mode`: Packaging mode ([`FlatOpcMode::ModernOmml`] or [`FlatOpcMode::ModernMathtypeOleEmf`]).
/// * `font_pt`: Point size used for rendering vector EMF dimensions (e.g. `12.0`).
pub fn mathml_to_flat_opc(math: &MathNode, mode: FlatOpcMode, font_pt: f32) -> Result<String> {
    let mut pkg =
        Element::new(Some(NS_PKG), Some("pkg"), "package").with_nsdecl(Some("pkg"), NS_PKG);

    // 1. /_rels/.rels
    pkg.push(make_xml_part(
        "/_rels/.rels",
        CT_RELATIONSHIPS,
        build_root_rels(),
    ));

    // 2. /word/_rels/document.xml.rels & 3. /word/document.xml + binary parts
    match mode {
        FlatOpcMode::ModernOmml => {
            build_omml_package_parts(&mut pkg, math)?;
        }
        FlatOpcMode::ModernMathtypeOleEmf => {
            let names = make_mathtype_part_names();

            // Local relationship IDs inside Flat OPC snippet (Word/WPS remaps them into document.xml.rels)
            let ole_rid = format!("rIdOle{}", names.uid);
            let img_rid = format!("rIdImage{}", names.uid);

            let bin_part_name = format!("/word/{}", names.ole_target);
            let img_part_name = format!("/word/{}", names.image_target);

            pkg.push(make_xml_part(
                "/word/_rels/document.xml.rels",
                CT_RELATIONSHIPS,
                build_document_rels(
                    mode,
                    &ole_rid,
                    &img_rid,
                    &names.ole_target,
                    &names.image_target,
                ),
            ));

            let mtef_bytes = mathml_to_mtef_bytes(math)?;
            if mtef_bytes.is_empty() {
                return Err(Error::InvalidMtef("empty MTEF stream generated".into()));
            }
            let native_stream = make_equation_native(&mtef_bytes);
            let ole_bin = make_ole(&native_stream)?;
            // Template record tags (3/5) overlap MTEF version bytes.
            // Pass a complete stream, never let the renderer guess from raw records.
            let complete_mtef = crate::mtef::build_mtef(&mtef_bytes);
            let emf_pic = render_emf_with_mtef(math, font_pt, Some(&complete_mtef))?;

            let obj_run = make_object_run(
                &ole_rid,
                &img_rid,
                &names.shape_id,
                &names.object_id,
                emf_pic.width_pt,
                emf_pic.height_pt,
                emf_pic.depth_pt,
            );
            let doc_el = build_document_xml(mode, obj_run);
            pkg.push(make_xml_part("/word/document.xml", CT_DOCUMENT, doc_el));

            pkg.push(make_binary_part(&bin_part_name, CT_OLEOBJECT, &ole_bin));
            pkg.push(make_binary_part(&img_part_name, CT_EMF, &emf_pic.emf));
        }
    }

    Ok(serialize_flat_opc_package(&pkg))
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

fn parse_package_part(child: &Element) -> Option<(String, PartData)> {
    if !child.is(NS_PKG, "part") && !child.is(NS_PKG_LEGACY, "part") && child.local != "part" {
        return None;
    }
    let name = child
        .attrs
        .iter()
        .find(|a| a.local == "name")
        .map(|a| a.value.as_str())
        .or_else(|| child.attr(Some(NS_PKG), "name"))
        .or_else(|| child.attr(Some(NS_PKG_LEGACY), "name"))
        .or_else(|| child.attr(None, "name"))?;
    let content_type = child
        .attrs
        .iter()
        .find(|a| a.local.eq_ignore_ascii_case("contenttype"))
        .map(|a| a.value.as_str())
        .or_else(|| child.attr(Some(NS_PKG), "contentType"))
        .or_else(|| child.attr(Some(NS_PKG_LEGACY), "contentType"))
        .or_else(|| child.attr(None, "contentType"))
        .unwrap_or("")
        .to_owned();
    let mut xml_data = None;
    let mut binary_data = None;
    for sub in child.elements() {
        if sub.local.eq_ignore_ascii_case("xmldata") {
            xml_data = sub.elements().next().cloned();
        } else if sub.local.eq_ignore_ascii_case("binarydata") {
            binary_data = base64_decode(&all_text(sub));
        }
    }
    Some((
        normalize_part_name(name).to_ascii_lowercase(),
        PartData {
            content_type,
            xml_data,
            binary_data,
        },
    ))
}

fn find_package_part<'a>(
    parts: &'a HashMap<String, PartData>,
    canonical_name: &str,
    suffix: &str,
) -> Option<&'a PartData> {
    parts.get(canonical_name).or_else(|| {
        parts
            .iter()
            .find(|(name, _)| name.ends_with(suffix))
            .map(|(_, part)| part)
    })
}

fn document_relationships(parts: &HashMap<String, PartData>) -> HashMap<String, String> {
    find_package_part(parts, "word/_rels/document.xml.rels", "document.xml.rels")
        .and_then(|part| part.xml_data.as_ref())
        .map(|rels| build_rel_map(rels, "word/document.xml"))
        .unwrap_or_default()
}

fn mathml_from_metafile(data: &[u8]) -> Option<MathNode> {
    let mtef = extract_mtef_from_emf(data).or_else(|| extract_mtef_from_wmf(data))?;
    mtef_to_mathml(&mtef)
}

fn mathml_from_binary(data: &[u8]) -> Option<MathNode> {
    mathml_from_metafile(data).or_else(|| ole_to_mathml(data))
}

fn first_omml_math(root: &Element) -> Option<MathNode> {
    root.iter().into_iter().find_map(|element| {
        (element.is(NS_M, "oMath") || element.local == "oMath")
            .then(|| omml_to_mathml(element).ok())
            .flatten()
    })
}

fn related_binary<'a>(
    parts: &'a HashMap<String, PartData>,
    relationships: &HashMap<String, String>,
    relationship_id: &str,
) -> Option<&'a [u8]> {
    let target = relationships.get(relationship_id)?;
    let name = normalize_part_name(target).to_ascii_lowercase();
    parts.get(&name)?.binary_data.as_deref()
}

fn ole_math_in_document(
    document: &Element,
    parts: &HashMap<String, PartData>,
    relationships: &HashMap<String, String>,
) -> Option<MathNode> {
    document.iter().into_iter().find_map(|element| {
        let is_candidate = element.is(NS_O, "OLEObject")
            || element.local == "OLEObject"
            || element.is(NS_W, "object")
            || element.local == "object";
        if !is_candidate {
            return None;
        }
        let relationship_id = element
            .attr(Some(NS_R), "id")
            .or_else(|| {
                element
                    .attrs
                    .iter()
                    .find(|attr| attr.local == "id")
                    .map(|attr| attr.value.as_str())
            })
            .or_else(|| element.attr(None, "id"))?;
        related_binary(parts, relationships, relationship_id).and_then(ole_to_mathml)
    })
}

fn metafile_math_in_document(
    document: &Element,
    parts: &HashMap<String, PartData>,
    relationships: &HashMap<String, String>,
) -> Option<MathNode> {
    let ole = document.iter().into_iter().find_map(|element| {
        if element.local != "OLEObject" {
            return None;
        }
        let id = element
            .attr(Some(NS_R), "id")
            .or_else(|| element.attr(None, "id"))?;
        related_binary(parts, relationships, id)
    });
    collect_imagedata_rids(document)
        .into_iter()
        .find_map(|relationship_id| {
            let data = related_binary(parts, relationships, &relationship_id)?;
            let mtef = extract_mtef_from_emf(data).or_else(|| extract_mtef_from_wmf(data))?;
            let mtef = crate::ole::repair_legacy_metafile_mtef(mtef, ole);
            mtef_to_mathml(&mtef)
        })
}

fn document_math(parts: &HashMap<String, PartData>) -> Option<MathNode> {
    let document = find_package_part(parts, "word/document.xml", "document.xml")?
        .xml_data
        .as_ref()?;
    let relationships = document_relationships(parts);
    metafile_math_in_document(document, parts, &relationships)
        .or_else(|| ole_math_in_document(document, parts, &relationships))
        .or_else(|| first_omml_math(document))
}

fn fallback_binary_math(parts: &HashMap<String, PartData>) -> Option<MathNode> {
    let binaries: Vec<_> = parts
        .values()
        .filter_map(|part| part.binary_data.as_deref())
        .collect();
    binaries
        .iter()
        .find_map(|data| mathml_from_metafile(data))
        .or_else(|| binaries.iter().find_map(|data| ole_to_mathml(data)))
}

fn fallback_xml_math(parts: &HashMap<String, PartData>) -> Option<MathNode> {
    parts
        .values()
        .filter_map(|part| part.xml_data.as_ref())
        .find_map(first_omml_math)
}

fn extract_from_package(pkg_el: &Element) -> Result<MathNode> {
    let mut parts: HashMap<String, PartData> = HashMap::new();

    for child in pkg_el.elements() {
        if let Some((name, part)) = parse_package_part(child) {
            parts.insert(name, part);
        }
    }

    if let Some(math) = document_math(&parts)
        .or_else(|| fallback_binary_math(&parts))
        .or_else(|| fallback_xml_math(&parts))
    {
        return Ok(math);
    }

    Err(Error::Xml(
        "no math equation (<m:oMath>, OLE bin, or MTEF image) found in Flat OPC package".into(),
    ))
}

fn extract_from_raw_snippet(root: &Element) -> Result<MathNode> {
    if let Some(math) = first_omml_math(root) {
        return Ok(math);
    }

    let mut binaries = Vec::new();
    for el in root.iter() {
        if el.local == "binData" || el.local == "binaryData" {
            if let Some(data) = base64_decode(all_text(el).trim()) {
                binaries.push(data);
            }
        } else {
            for child in &el.children {
                if let Node::Text(text) = child {
                    if text.trim().len() > 100 {
                        if let Some(data) = base64_decode(text.trim()) {
                            binaries.push(data);
                        }
                    }
                }
            }
        }
    }
    // The order of binary parts in a selection must not decide which copy wins.
    if let Some(math) = binaries
        .iter()
        .find_map(|data| mathml_from_metafile(data))
        .or_else(|| binaries.iter().find_map(|data| mathml_from_binary(data)))
    {
        return Ok(math);
    }

    Err(Error::Xml(
        "no math equation (<m:oMath> or embedded MTEF) found in XML snippet".into(),
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

    if root.is(NS_PKG, "package") || root.is(NS_PKG_LEGACY, "package") || root.local == "package" {
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
        let math =
            MathNode::parse("<math><mi>y</mi><mo>=</mo><msup><mi>x</mi><mn>2</mn></msup></math>")
                .unwrap();

        let opc_xml = mathml_to_flat_opc(&math, FlatOpcMode::ModernMathtypeOleEmf, 12.0)
            .expect("package MathType flat OPC");
        assert!(opc_xml.contains("pkg:package"));
        assert!(opc_xml.contains("/word/embeddings/oleObject"));
        assert!(opc_xml.contains("/word/media/equation"));
        assert!(opc_xml.contains("Equation.DSMT4"));

        let recovered = flat_opc_to_mathml(&opc_xml).expect("parse MathType flat OPC");
        let mi_list = recovered.find_all("mi");
        assert!(mi_list.iter().any(|n| n.text == "y"));
        assert!(mi_list.iter().any(|n| n.text == "x"));
        let mn_list = recovered.find_all("mn");
        assert!(mn_list.iter().any(|n| n.text == "2"));
    }

    #[test]
    fn ole_and_emf_embed_identical_independently_readable_mtef() {
        for source in [
            "<math><mi>x</mi><mo>+</mo><mn>1</mn></math>",
            "<math><mfrac><mi>π</mi><mn>3</mn></mfrac><mo>+</mo><mn>1</mn></math>",
            "<math><mtable><mtr><mtd><mi>x</mi></mtd><mtd><mn>2</mn></mtd></mtr></mtable></math>",
        ] {
            let math = MathNode::parse(source).unwrap();
            let expected = crate::mtef::build_mtef(&mathml_to_mtef_bytes(&math).unwrap());
            let package =
                mathml_to_flat_opc(&math, FlatOpcMode::ModernMathtypeOleEmf, 12.0).unwrap();
            let root = xml::parse(package.as_bytes()).unwrap();
            let mut ole_mtef = None;
            let mut emf_mtef = None;
            for part in root.elements() {
                let Some(binary) = part.find(NS_PKG, "binaryData") else {
                    continue;
                };
                let bytes = base64_decode(&binary.text()).unwrap();
                match part.attr(Some(NS_PKG), "contentType") {
                    Some(CT_OLEOBJECT) => ole_mtef = crate::ole::extract_mtef_from_ole(&bytes),
                    Some(CT_EMF) => emf_mtef = extract_mtef_from_emf(&bytes),
                    _ => {}
                }
            }
            let ole = ole_mtef.expect("OLE must contain MTEF");
            let emf = emf_mtef.expect("EMF preview must also contain MTEF");
            assert_eq!(ole, expected);
            assert_eq!(emf, expected);
            let from_ole = mtef_to_mathml(&ole).expect("decode OLE without its EMF preview");
            let from_emf = mtef_to_mathml(&emf).expect("decode EMF without its OLE object");
            assert_eq!(from_ole.to_xml(), from_emf.to_xml());
        }
    }

    #[test]
    fn legacy_headerless_preview_is_recovered_only_when_ole_matches() {
        let math = MathNode::parse("<math><mfrac><mi>π</mi><mn>3</mn></mfrac></math>").unwrap();
        let raw = mathml_to_mtef_bytes(&math).unwrap();
        assert_eq!(raw[0], 3); // Template tag was mistaken for MTEF v3.
        let complete = crate::mtef::build_mtef(&raw);
        let ole = make_ole(&make_equation_native(&raw)).unwrap();
        assert_eq!(
            crate::ole::repair_legacy_metafile_mtef(raw.clone(), Some(&ole)),
            complete
        );
        assert_eq!(
            crate::ole::repair_legacy_metafile_mtef(raw.clone(), None),
            raw
        );
        let package = mathml_to_flat_opc(&math, FlatOpcMode::ModernMathtypeOleEmf, 12.0).unwrap();
        let root = xml::parse(package.as_bytes()).unwrap();
        let preview = root
            .elements()
            .find(|part| part.attr(Some(NS_PKG), "contentType") == Some(CT_EMF))
            .unwrap();
        let original = preview.find(NS_PKG, "binaryData").unwrap().text();
        let legacy = render_emf_with_mtef(&math, 12.0, Some(&raw)).unwrap();
        let package = package.replace(&original, &base64_encode(&legacy.emf));
        let recovered = flat_opc_to_mathml(&package).unwrap();
        assert_eq!(recovered.to_latex(), r"\frac{\pi}{3}");
    }

    #[test]
    fn prefer_emf_mtef_and_fall_back_to_ole_when_preview_is_unreadable() {
        let old = MathNode::parse("<math><mi>x</mi></math>").unwrap();
        let new = MathNode::parse("<math><mi>y</mi></math>").unwrap();
        let package = mathml_to_flat_opc(&old, FlatOpcMode::ModernMathtypeOleEmf, 12.0).unwrap();
        let root = xml::parse(package.as_bytes()).unwrap();
        let preview = root
            .elements()
            .find(|part| part.attr(Some(NS_PKG), "contentType") == Some(CT_EMF))
            .unwrap();
        let original = preview.find(NS_PKG, "binaryData").unwrap().text();
        let replacement = crate::render_emf(&new, 12.0).unwrap();
        let updated = package.replace(&original, &base64_encode(&replacement.emf));
        assert_eq!(flat_opc_to_mathml(&updated).unwrap().to_latex(), "y");
        let damaged = package.replace(&original, &base64_encode(b"damaged EMF"));
        assert_eq!(flat_opc_to_mathml(&damaged).unwrap().to_latex(), "x");
    }

    #[test]
    fn repeated_imports_have_distinct_parts_shapes_and_relationships() {
        let math = MathNode::parse("<math><mi>x</mi></math>").unwrap();
        let mut seen = std::collections::HashSet::new();
        for _ in 0..16 {
            let names = make_mathtype_part_names();
            for id in [
                names.shape_id,
                names.object_id,
                names.ole_target,
                names.image_target,
                names.uid,
            ] {
                assert!(seen.insert(id));
            }
            let package =
                mathml_to_flat_opc(&math, FlatOpcMode::ModernMathtypeOleEmf, 12.0).unwrap();
            let root = xml::parse(package.as_bytes()).unwrap();
            let rels = root
                .elements()
                .find(|part| {
                    part.attr(Some(NS_PKG), "name") == Some("/word/_rels/document.xml.rels")
                })
                .unwrap();
            let data = rels.find(NS_PKG, "xmlData").unwrap();
            for relationship in data.elements().flat_map(|r| r.elements()) {
                assert!(seen.insert(relationship.attr(None, "Id").unwrap().to_owned()));
            }
        }
    }

    #[test]
    fn package_keeps_relationships_and_media_parts_linked() {
        let math = MathNode::parse("<math><mi>x</mi></math>").unwrap();
        let opc = mathml_to_flat_opc(&math, FlatOpcMode::ModernMathtypeOleEmf, 12.0).unwrap();
        let root = xml::parse(opc.as_bytes()).unwrap();
        let parts: Vec<_> = root.elements().collect();

        assert!(opc.contains("Id=\"rIdOle"));
        assert!(opc.contains("Id=\"rIdImage"));
        assert!(parts.iter().any(|part| {
            part.attr(Some(NS_PKG), "name")
                .is_some_and(|name| name.starts_with("/word/embeddings/oleObject"))
        }));
        assert!(parts.iter().any(|part| {
            part.attr(Some(NS_PKG), "name")
                .is_some_and(|name| name.starts_with("/word/media/equation"))
        }));
    }

    #[test]
    fn fraction_picture_remains_editable_after_repeated_saves() {
        let mut math =
            MathNode::parse("<math><mfrac><mi>π</mi><mn>3</mn></mfrac><mo>+</mo><mn>1</mn></math>")
                .unwrap();
        for _ in 0..3 {
            let opc = mathml_to_flat_opc(&math, FlatOpcMode::ModernMathtypeOleEmf, 12.0).unwrap();
            let picture_only = opc
                .replace("/word/embeddings/oleObject", "/word/embeddings/dummy")
                .replace("ProgID=\"Equation.DSMT4\"", "ProgID=\"Unknown\"");
            math = flat_opc_to_mathml(&picture_only).unwrap();
            assert_eq!(math.find_all("mfrac").len(), 1);
            assert!(math.find_all("mi").iter().any(|n| n.text == "π"));
            assert!(math.find_all("mn").iter().any(|n| n.text == "3"));
            assert!(math.find_all("mn").iter().any(|n| n.text == "1"));
            assert!(math.find_all("mo").iter().any(|n| n.text == "+"));
        }
    }

    #[test]
    fn test_flat_opc_emf_mtef_fallback() {
        let math = MathNode::parse("<math><mi>a</mi><mo>+</mo><mi>b</mi></math>").unwrap();

        let opc_xml = mathml_to_flat_opc(&math, FlatOpcMode::ModernMathtypeOleEmf, 12.0)
            .expect("package MathType flat OPC");

        // Simulate a document where the OLE bin part was stripped or corrupted,
        // leaving only the vector EMF with EMR_COMMENT MTEF.
        let stripped_xml = opc_xml.replace("/word/embeddings/oleObject", "/word/embeddings/dummy");
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
        let text_found = recovered
            .iter()
            .into_iter()
            .any(|n| n.text.contains("E=mc"));
        assert!(text_found);
    }
    #[test]
    fn test_wps_xmlpackage_namespace() {
        let math = MathNode::parse("<math><mi>x</mi><mo>+</mo><mn>1</mn></math>").unwrap();
        let opc_xml = mathml_to_flat_opc(&math, FlatOpcMode::ModernOmml, 12.0).unwrap();
        let wps_opc = opc_xml.replace(
            "http://schemas.microsoft.com/office/2006/xml/packaging",
            "http://schemas.microsoft.com/office/2006/xmlPackage",
        );
        let recovered = flat_opc_to_mathml(&wps_opc).expect("should parse WPS xmlPackage");
        assert_eq!(recovered.find_all("mi")[0].text, "x");
    }
    #[test]
    fn test_wps_xmlpackage_mathtype_ole() {
        let math =
            MathNode::parse("<math><mi>y</mi><mo>=</mo><msup><mi>x</mi><mn>2</mn></msup></math>")
                .unwrap();
        let opc_xml = mathml_to_flat_opc(&math, FlatOpcMode::ModernMathtypeOleEmf, 12.0).unwrap();
        let wps_opc = opc_xml.replace(
            "http://schemas.microsoft.com/office/2006/xml/packaging",
            "http://schemas.microsoft.com/office/2006/xmlPackage",
        );
        let recovered = flat_opc_to_mathml(&wps_opc).expect("should parse WPS MathType xmlPackage");
        assert!(recovered.find_all("mi").iter().any(|n| n.text == "y"));
        assert!(recovered.find_all("mi").iter().any(|n| n.text == "x"));
        assert!(recovered.find_all("mn").iter().any(|n| n.text == "2"));
    }

    #[test]
    fn test_bridge_decode_xmlpackage_omml() {
        let xml = r#"<?xml version="1.0" standalone="yes"?>
<?mso-application progid="Word.Document"?>
<pkg:package xmlns:pkg="http://schemas.microsoft.com/office/2006/xmlPackage">
  <pkg:part pkg:name="/word/document.xml" pkg:contentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml">
    <pkg:xmlData>
      <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math">
        <w:body>
          <w:p>
            <m:oMath><m:r><m:t>x+1</m:t></m:r></m:oMath>
          </w:p>
        </w:body>
      </w:document>
    </pkg:xmlData>
  </pkg:part>
</pkg:package>"#;
        let m = crate::docx::flat_opc_to_mathml(xml).expect("decode xmlpackage");
        assert_eq!(m.to_latex(), "x+1");
    }
}
