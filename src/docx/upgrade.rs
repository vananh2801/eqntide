//! MathType Upgrade & Re-typesetting Pipeline for `.docx` files.
//!
//! Upgrades legacy equations (Microsoft Equation 3.0 `Equation.3` / `Equation.2`,
//! MTEF 3.0 streams, old blurry `.wmf` equations) and re-typesets existing MathType
//! equations using this crate's high-quality TeX/STIX Two Math layout engine into:
//!
//! * **Target A: Modern Add-in format (Native OMML `<m:oMath>`)**:
//!   Native Word 365 & WPS Office equation markup, directly editable by modern
//!   Word and modern Web Add-ins.
//! * **Target B: Modern MathType 7 format (`Equation.DSMT4`)**:
//!   Genuine MTEF 5.0 OLE compound binary objects (`word/embeddings/oleObject*.bin`)
//!   accompanied by crisp vector EMF previews (`word/media/equation*.emf`) carrying
//!   embedded MTEF in an `EMR_COMMENT` record.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::omml::decode::NS_M;

use crate::error::{Error, Result};
use crate::mathml::MathNode;
use crate::mtef::decode::{mtef_to_mathml, ole_to_mathml};
use crate::mtef::encode::mathml_to_mtef_bytes;
use crate::mtef::make_equation_native;
use crate::ole::{extract_mtef_from_emf, extract_mtef_from_wmf, make_ole};
use crate::render::emf::render_emf_with_mtef;
use crate::xml::{self, Element, Node};

use super::consts::*;
use super::ole_xml::{make_object_run, sanitize_document_baselines};
use super::options::{ConversionSummary, ToOmmlOptions, UpgradeOptions, UpgradeTarget};
use super::package::{add_relationship, ensure_content_type, ensure_default_extension, Package};
use super::to_omml::{
    build_rel_map, cleanup_unreferenced_files, collect_imagedata_rids, collect_referenced_rids,
    convert_bytes_to_omml_with, discover_content_parts, find_mathtype_ole_in_subtree, get_r_attr,
    rels_path_for_part,
};

// ── Public Entry Points ──────────────────────────────────────────────────────

/// Upgrade legacy MathType equations in a `.docx` file on disk with default options.
pub fn upgrade_legacy_mathtype_docx(
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
) -> Result<ConversionSummary> {
    upgrade_legacy_mathtype_docx_with(input_path, output_path, &UpgradeOptions::default())
}

/// Like [`upgrade_legacy_mathtype_docx`], with explicit [`UpgradeOptions`].
pub fn upgrade_legacy_mathtype_docx_with(
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    opts: &UpgradeOptions,
) -> Result<ConversionSummary> {
    let input = std::fs::read(input_path.as_ref())?;
    let (output, summary) = upgrade_legacy_mathtype_bytes_with(&input, opts)?;
    std::fs::write(output_path.as_ref(), output)?;
    log::info!("Saved upgraded docx → {}", output_path.as_ref().display());
    Ok(summary)
}

/// Upgrade legacy MathType equations in a `.docx` held in memory with default options.
pub fn upgrade_legacy_mathtype_bytes(input: &[u8]) -> Result<(Vec<u8>, ConversionSummary)> {
    upgrade_legacy_mathtype_bytes_with(input, &UpgradeOptions::default())
}

/// Upgrade legacy MathType equations in a `.docx` held in memory.
pub fn upgrade_legacy_mathtype_bytes_with(
    input: &[u8],
    opts: &UpgradeOptions,
) -> Result<(Vec<u8>, ConversionSummary)> {
    if opts.target == UpgradeTarget::ModernOmml {
        return convert_bytes_to_omml_with(
            input,
            &ToOmmlOptions {
                math_font: opts.math_font.clone(),
            },
        );
    }

    let mut pkg = Package::read(input)?;
    if !pkg.contains(DOC_XML_PATH) {
        return Err(Error::MissingPart(DOC_XML_PATH.into()));
    }

    let mut ct_root = xml::parse(
        pkg.get(CT_PATH)
            .ok_or_else(|| Error::MissingPart(CT_PATH.into()))?,
    )?;

    let (mut ole_index, mut img_index) = next_package_part_indices(&pkg);

    let content_parts = discover_content_parts(&pkg);
    let mut summary = ConversionSummary::default();
    let mut candidate_files_to_remove: HashSet<String> = HashSet::new();

    for xml_path in content_parts {
        let Some(xml_bytes) = pkg.get(&xml_path) else {
            continue;
        };
        let mut doc_root = xml::parse(xml_bytes)?;

        let rels_path = rels_path_for_part(&xml_path);
        let rels_opt = pkg.get(&rels_path).map(xml::parse).transpose()?;
        let rel_map = rels_opt
            .as_ref()
            .map(|r| build_rel_map(r, &xml_path))
            .unwrap_or_default();

        let mut rels_root = rels_opt.unwrap_or_else(default_relationships_root);
        let mut existing_ids: HashSet<String> = rels_root
            .elements()
            .filter_map(|c| c.attr(None, "Id").map(str::to_owned))
            .collect();

        let mut converted_rids: HashSet<String> = HashSet::new();
        let before_converted = summary.converted;

        upgrade_element(
            &mut doc_root,
            &mut pkg,
            &mut rels_root,
            &mut ct_root,
            &rel_map,
            &mut existing_ids,
            &mut ole_index,
            &mut img_index,
            opts,
            &mut summary,
            &mut converted_rids,
        )?;

        sanitize_document_baselines(&mut doc_root);

        if summary.converted > before_converted {
            pkg.set(&xml_path, xml::to_bytes(&doc_root));

            let dead_rids = obsolete_relationship_ids(&doc_root, converted_rids);

            if !dead_rids.is_empty() {
                for rid in &dead_rids {
                    if let Some(target_zip_path) = rel_map.get(rid) {
                        candidate_files_to_remove.insert(target_zip_path.clone());
                    }
                }
                rels_root.children.retain(|n| match n {
                    Node::Element(e) if e.local == "Relationship" => {
                        e.attr(None, "Id").is_none_or(|id| !dead_rids.contains(id))
                    }
                    _ => true,
                });
            }
            pkg.set(&rels_path, xml::to_bytes(&rels_root));
        }
    }

    if !candidate_files_to_remove.is_empty() {
        cleanup_unreferenced_files(&mut pkg, &candidate_files_to_remove)?;
    }

    pkg.set(CT_PATH, xml::to_bytes(&ct_root));
    Ok((pkg.write()?, summary))
}

// ── XML Tree Rewriting ───────────────────────────────────────────────────────

fn next_package_part_indices(pkg: &Package) -> (usize, usize) {
    let mut ole_index = 1;
    while pkg.contains(&format!("word/embeddings/oleObject{ole_index}.bin")) {
        ole_index += 1;
    }
    let mut img_index = 1;
    while pkg.contains(&format!("word/media/equation{img_index}.emf"))
        || pkg.contains(&format!("word/media/equation{img_index}.wmf"))
        || pkg.contains(&format!("word/media/equation{img_index}.png"))
    {
        img_index += 1;
    }
    (ole_index, img_index)
}

fn default_relationships_root() -> Element {
    xml::parse(EMPTY_RELS).expect("built-in relationship XML is valid")
}

fn obsolete_relationship_ids(root: &Element, converted: HashSet<String>) -> HashSet<String> {
    let remaining = collect_referenced_rids(root);
    converted
        .into_iter()
        .filter(|rid| !remaining.contains(rid))
        .collect()
}

fn next_unique_id(existing: &mut HashSet<String>, prefix: &str) -> String {
    let mut i = 1;
    loop {
        let id = format!("{prefix}{i}");
        if !existing.contains(&id) {
            existing.insert(id.clone());
            return id;
        }
        i += 1;
    }
}

fn convert_ole_rid_to_mathml(
    rid: &str,
    pkg: &Package,
    rel_map: &HashMap<String, String>,
) -> Result<MathNode> {
    let target_path = rel_map
        .get(rid)
        .ok_or_else(|| Error::InvalidOle(format!("missing relationship target for {rid}")))?;
    let ole_bytes = pkg
        .get(target_path)
        .ok_or_else(|| Error::MissingPart(target_path.clone()))?;
    ole_to_mathml(ole_bytes)
        .ok_or_else(|| Error::InvalidMtef(format!("failed to decode MTEF from {target_path}")))
}

fn mtef_from_metafile(path: &str, bytes: &[u8]) -> Option<Vec<u8>> {
    if path.to_ascii_lowercase().ends_with(".wmf") {
        extract_mtef_from_wmf(bytes).or_else(|| extract_mtef_from_emf(bytes))
    } else if path.to_ascii_lowercase().ends_with(".emf") {
        extract_mtef_from_emf(bytes).or_else(|| extract_mtef_from_wmf(bytes))
    } else {
        None
    }
}

#[allow(clippy::too_many_arguments)]
fn build_modern_ole_run(
    mathml: &MathNode,
    font_pt: f32,
    eq_num: usize,
    pkg: &mut Package,
    rels_root: &mut Element,
    ct_root: &mut Element,
    existing_ids: &mut HashSet<String>,
    ole_index: &mut usize,
    img_index: &mut usize,
) -> Result<Element> {
    // 1. MathML → MTEF 5.0 inner equation bytes
    let eq_bytes = mathml_to_mtef_bytes(mathml)?;
    if eq_bytes.is_empty() {
        return Err(Error::InvalidMtef("empty equation content".into()));
    }

    // 2. Wrap in Equation Native stream & OLE CFB
    let native_stream = make_equation_native(&eq_bytes);
    let ole_bin = make_ole(&native_stream)?;

    // 3. Render vector EMF preview with embedded MTEF in EMR_COMMENT
    let pv = render_emf_with_mtef(mathml, font_pt, Some(&eq_bytes))?;

    // 4. Save into docx package
    let mut bin_name = format!("word/embeddings/oleObject{ole_index}.bin");
    while pkg.contains(&bin_name) {
        *ole_index += 1;
        bin_name = format!("word/embeddings/oleObject{ole_index}.bin");
    }
    *ole_index += 1;
    pkg.set(&bin_name, ole_bin);
    let bin_target = format!("embeddings/oleObject{}.bin", *ole_index - 1);

    let mut img_name = format!("word/media/equation{img_index}.emf");
    while pkg.contains(&img_name) {
        *img_index += 1;
        img_name = format!("word/media/equation{img_index}.emf");
    }
    *img_index += 1;
    pkg.set(&img_name, pv.emf);
    let img_target = format!("media/equation{}.emf", *img_index - 1);

    // 5. Add relationships
    let ole_rid = next_unique_id(existing_ids, "rIdOle");
    add_relationship(rels_root, &ole_rid, REL_OLEOBJECT, &bin_target);

    let img_rid = next_unique_id(existing_ids, "rIdImg");
    add_relationship(rels_root, &img_rid, REL_IMAGE, &img_target);

    // 6. Ensure Content Types
    ensure_content_type(ct_root, &format!("/{bin_name}"), CT_OLEOBJECT);
    ensure_default_extension(ct_root, "emf", "image/x-emf");

    // 7. Make object run
    let shape_id = format!("_x0000_i{}", 1024 + eq_num);
    let obj_id = format!(
        "_{}",
        1_000_000_000u128 + uuid::Uuid::new_v4().as_u128() % 9_000_000_000u128
    );
    Ok(make_object_run(
        &ole_rid,
        &img_rid,
        &shape_id,
        &obj_id,
        pv.width_pt,
        pv.height_pt,
        pv.depth_pt,
    ))
}

#[allow(clippy::too_many_arguments)]
fn upgrade_element(
    parent: &mut Element,
    pkg: &mut Package,
    rels_root: &mut Element,
    ct_root: &mut Element,
    rel_map: &HashMap<String, String>,
    existing_ids: &mut HashSet<String>,
    ole_index: &mut usize,
    img_index: &mut usize,
    opts: &UpgradeOptions,
    summary: &mut ConversionSummary,
    converted_rids: &mut HashSet<String>,
) -> Result<()> {
    let mut new_children: Vec<Node> = Vec::with_capacity(parent.children.len());

    for child in std::mem::take(&mut parent.children) {
        let Node::Element(mut el) = child else {
            new_children.push(child);
            continue;
        };

        // Case 1: `<mc:AlternateContent>` wrapping a MathType OLE object in `<mc:Choice>`
        if el.is(NS_MC, "AlternateContent") || el.local == "AlternateContent" {
            let ole_nodes = find_mathtype_ole_in_subtree(&el);
            if !ole_nodes.is_empty() {
                summary.found += 1;
                let eq_num = summary.found;
                let ole_rid = ole_nodes[0]
                    .attr(Some(NS_R), "id")
                    .or_else(|| ole_nodes[0].attr(None, "id"));
                let img_rids = collect_imagedata_rids(&el);

                let mathml_opt = match ole_rid {
                    Some(rid) => convert_ole_rid_to_mathml(rid, pkg, rel_map).ok(),
                    None => None,
                }
                .or_else(|| {
                    el.find(NS_MC, "Fallback")
                        .and_then(|fb| fb.find(NS_M, "oMath"))
                        .and_then(|omath| crate::omml::decode::omml_to_mathml(omath).ok())
                });

                let modern_run_res = match mathml_opt {
                    Some(mml) => build_modern_ole_run(
                        &mml,
                        opts.font_pt,
                        eq_num,
                        pkg,
                        rels_root,
                        ct_root,
                        existing_ids,
                        ole_index,
                        img_index,
                    ),
                    None => Err(Error::InvalidMtef(
                        "failed to decode AlternateContent formula".into(),
                    )),
                };

                match modern_run_res {
                    Ok(modern_run) => {
                        if let Some(rid) = ole_rid {
                            converted_rids.insert(rid.to_owned());
                        }
                        converted_rids.extend(img_rids);
                        summary.converted += 1;
                        log::info!(
                            "Equation {eq_num}: upgraded AlternateContent → Equation.DSMT4 OLE"
                        );
                        new_children.push(Node::Element(modern_run));
                        continue;
                    }
                    Err(e) => {
                        summary.errors += 1;
                        log::error!("Equation {eq_num}: {e}");
                        new_children.push(Node::Element(el));
                        continue;
                    }
                }
            }
        }

        // Case 2: `<w:r>` containing one or more MathType `<o:OLEObject>` elements
        if el.is(NS_W, "r") {
            let ole_count = find_mathtype_ole_in_subtree(&el).len();
            if ole_count > 0 {
                let replacements = upgrade_ole_run(
                    &el,
                    pkg,
                    rels_root,
                    ct_root,
                    rel_map,
                    existing_ids,
                    ole_index,
                    img_index,
                    opts,
                    summary,
                    converted_rids,
                );
                for rep in replacements {
                    new_children.push(Node::Element(rep));
                }
                continue;
            }

            // Case 3: `<w:r>` with a standalone `.wmf` or `.emf` image containing MathType MTEF
            if let Some(modern_run) = try_upgrade_metafile_run(
                &el,
                pkg,
                rels_root,
                ct_root,
                rel_map,
                existing_ids,
                ole_index,
                img_index,
                opts,
                summary,
                converted_rids,
            ) {
                new_children.push(Node::Element(modern_run));
                continue;
            }
        }

        // Recurse into child elements
        upgrade_element(
            &mut el,
            pkg,
            rels_root,
            ct_root,
            rel_map,
            existing_ids,
            ole_index,
            img_index,
            opts,
            summary,
            converted_rids,
        )?;
        new_children.push(Node::Element(el));
    }

    parent.children = new_children;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn upgrade_ole_run(
    run_el: &Element,
    pkg: &mut Package,
    rels_root: &mut Element,
    ct_root: &mut Element,
    rel_map: &HashMap<String, String>,
    existing_ids: &mut HashSet<String>,
    ole_index: &mut usize,
    img_index: &mut usize,
    opts: &UpgradeOptions,
    summary: &mut ConversionSummary,
    converted_rids: &mut HashSet<String>,
) -> Vec<Element> {
    summary.found += 1;
    let eq_num = summary.found;
    let oles = find_mathtype_ole_in_subtree(run_el);
    let ole_rid = oles.first().and_then(|o| get_r_attr(o, "id"));
    let img_rids = collect_imagedata_rids(run_el);

    let mathml_res = match ole_rid {
        Some(rid) => convert_ole_rid_to_mathml(rid, pkg, rel_map),
        None => Err(Error::InvalidOle("OLEObject missing r:id".into())),
    };

    match mathml_res.and_then(|mml| {
        build_modern_ole_run(
            &mml,
            opts.font_pt,
            eq_num,
            pkg,
            rels_root,
            ct_root,
            existing_ids,
            ole_index,
            img_index,
        )
    }) {
        Ok(modern_run) => {
            if let Some(rid) = ole_rid {
                converted_rids.insert(rid.to_owned());
            }
            converted_rids.extend(img_rids);
            summary.converted += 1;
            log::info!("Equation {eq_num}: upgraded OLE → Equation.DSMT4");
            vec![modern_run]
        }
        Err(e) => {
            summary.errors += 1;
            log::error!("Equation {eq_num}: {e}");
            vec![run_el.clone()]
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn try_upgrade_metafile_run(
    run_el: &Element,
    pkg: &mut Package,
    rels_root: &mut Element,
    ct_root: &mut Element,
    rel_map: &HashMap<String, String>,
    existing_ids: &mut HashSet<String>,
    ole_index: &mut usize,
    img_index: &mut usize,
    opts: &UpgradeOptions,
    summary: &mut ConversionSummary,
    converted_rids: &mut HashSet<String>,
) -> Option<Element> {
    let img_rids = collect_imagedata_rids(run_el);
    for rid in &img_rids {
        let Some(target_path) = rel_map.get(rid) else {
            continue;
        };
        let lower = target_path.to_ascii_lowercase();
        let is_wmf = lower.ends_with(".wmf");
        let is_emf = lower.ends_with(".emf");
        if !is_wmf && !is_emf {
            continue;
        }
        let Some(img_bytes) = pkg.get(target_path) else {
            continue;
        };
        let mtef_bytes = mtef_from_metafile(target_path, img_bytes);
        let Some(mtef_bytes) = mtef_bytes else {
            continue;
        };

        summary.found += 1;
        let eq_num = summary.found;
        let kind_str = if is_wmf { "WMF" } else { "EMF" };
        let mathml_opt = mtef_to_mathml(&mtef_bytes);
        let modern_run_res = match mathml_opt {
            Some(mml) => build_modern_ole_run(
                &mml,
                opts.font_pt,
                eq_num,
                pkg,
                rels_root,
                ct_root,
                existing_ids,
                ole_index,
                img_index,
            ),
            None => Err(Error::InvalidMtef(format!(
                "failed to decode {kind_str} MTEF in {target_path}"
            ))),
        };

        match modern_run_res {
            Ok(modern_run) => {
                converted_rids.insert(rid.clone());
                summary.converted += 1;
                log::info!(
                    "Equation {eq_num}: upgraded standalone {kind_str} ({rid}) → Equation.DSMT4 OLE"
                );
                return Some(modern_run);
            }
            Err(e) => {
                summary.errors += 1;
                log::error!("Equation {eq_num}: {e}");
                return None;
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};

    #[test]
    fn allocates_the_first_available_relationship_id() {
        let mut existing = HashSet::from(["rId1".to_owned(), "rId3".to_owned()]);
        assert_eq!(next_unique_id(&mut existing, "rId"), "rId2");
        assert_eq!(next_unique_id(&mut existing, "rId"), "rId4");
    }

    fn make_legacy_wmf_docx(preview_emf: &[u8]) -> Vec<u8> {
        let doc_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
            xmlns:v="urn:schemas-microsoft-com:vml">
  <w:body>
    <w:p>
      <w:r>
        <w:pict>
          <v:shape id="_x0000_i1025" style="width:10pt;height:10pt">
            <v:imagedata r:id="rId1"/>
          </v:shape>
        </w:pict>
      </w:r>
    </w:p>
  </w:body>
</w:document>"#;

        let rels_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/legacy_equation.emf"/>
</Relationships>"#;

        let ct_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
  <Default Extension="wmf" ContentType="image/x-wmf"/>
</Types>"#;

        let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let opts = zip::write::FileOptions::default();
        w.start_file("[Content_Types].xml", opts).unwrap();
        w.write_all(ct_xml.as_bytes()).unwrap();
        w.start_file("word/document.xml", opts).unwrap();
        w.write_all(doc_xml.as_bytes()).unwrap();
        w.start_file("word/_rels/document.xml.rels", opts).unwrap();
        w.write_all(rels_xml.as_bytes()).unwrap();
        w.start_file("word/media/legacy_equation.emf", opts)
            .unwrap();
        w.write_all(preview_emf).unwrap();
        w.finish().unwrap().into_inner()
    }

    #[test]
    fn test_upgrade_legacy_wmf_to_modern_mathtype() {
        let node = MathNode::with_children("math", vec![MathNode::leaf("mi", "x")]);
        let preview = crate::render::emf::render_emf(&node, 12.0).unwrap();
        let src_docx = make_legacy_wmf_docx(&preview.emf);

        // Upgrade with default options (target = ModernMathType)
        let (upgraded_docx, summary) = upgrade_legacy_mathtype_bytes(&src_docx).unwrap();

        assert_eq!(summary.found, 1);
        assert_eq!(summary.converted, 1);
        assert_eq!(summary.errors, 0);

        let pkg = Package::read(&upgraded_docx).unwrap();
        // Old image should be cleaned up
        assert!(!pkg.contains("word/media/legacy_equation.emf"));
        // New oleObject1.bin and equation1.emf should exist
        assert!(pkg.contains("word/embeddings/oleObject1.bin"));
        assert!(pkg.contains("word/media/equation1.emf"));

        // Verify document.xml has Equation.DSMT4 ProgID
        let doc_str = String::from_utf8(pkg.get("word/document.xml").unwrap().to_vec()).unwrap();
        assert!(doc_str.contains("Equation.DSMT4"));

        // Verify the generated EMF contains valid embedded MTEF
        let new_emf = pkg.get("word/media/equation1.emf").unwrap();
        let extracted_mtef =
            extract_mtef_from_emf(new_emf).expect("extract MTEF from upgraded EMF");
        assert_eq!(extracted_mtef[0], 5);
    }

    #[test]
    fn test_upgrade_legacy_wmf_to_modern_omml() {
        let node = MathNode::with_children("math", vec![MathNode::leaf("mi", "y")]);
        let preview = crate::render::emf::render_emf(&node, 12.0).unwrap();
        let src_docx = make_legacy_wmf_docx(&preview.emf);

        // Upgrade with target = ModernOmml
        let (upgraded_docx, summary) = upgrade_legacy_mathtype_bytes_with(
            &src_docx,
            &UpgradeOptions {
                target: UpgradeTarget::ModernOmml,
                font_pt: 12.0,
                math_font: None,
            },
        )
        .unwrap();

        assert_eq!(summary.found, 1);
        assert_eq!(summary.converted, 1);
        assert_eq!(summary.errors, 0);

        let pkg = Package::read(&upgraded_docx).unwrap();
        let doc_el = xml::parse(pkg.get("word/document.xml").unwrap()).unwrap();
        let omaths: Vec<_> = doc_el
            .iter()
            .into_iter()
            .filter(|e| e.is(NS_M, "oMath"))
            .collect();
        assert_eq!(omaths.len(), 1);
    }

    #[test]
    fn test_upgrade_retypesets_existing_mathtype_dsmt4() {
        // Create an OMML equation and convert it to MathType DSMT4 first
        let omml = r#"<m:oMath><m:r><m:t>x</m:t></m:r><m:r><m:t>+</m:t></m:r><m:r><m:t>1</m:t></m:r></m:oMath>"#;
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buf);
            let opts = zip::write::FileOptions::default();
            zip.start_file("[Content_Types].xml", opts).unwrap();
            zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#).unwrap();
            zip.start_file("word/document.xml", opts).unwrap();
            let doc_xml = format!(
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math">
  <w:body><w:p>{omml}</w:p></w:body>
</w:document>"#
            );
            zip.write_all(doc_xml.as_bytes()).unwrap();
            zip.finish().unwrap();
        }
        let src_omml_docx = buf.into_inner();
        let (dsmt4_docx, _) = crate::docx::convert_bytes(&src_omml_docx).unwrap();

        // Now run upgrade on the docx that already has Equation.DSMT4
        let (retypeset_docx, summary) = upgrade_legacy_mathtype_bytes(&dsmt4_docx).unwrap();
        assert_eq!(summary.found, 1);
        assert_eq!(summary.converted, 1);
        assert_eq!(summary.errors, 0);

        let pkg = Package::read(&retypeset_docx).unwrap();
        let doc_str = String::from_utf8(pkg.get("word/document.xml").unwrap().to_vec()).unwrap();
        assert!(doc_str.contains("Equation.DSMT4"));
    }
}
