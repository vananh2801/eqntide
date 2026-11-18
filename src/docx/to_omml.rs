//! MathType (OLE `.bin` / standalone `.wmf`) → OMML conversion pipeline for `.docx` files.
//!
//! Scans `.docx` XML parts (`word/document.xml`, headers, footers, footnotes,
//! endnotes) for embedded MathType OLE objects (`Equation.DSMT4`, `Equation.3`,
//! `Equation.2`) and standalone WMF pictures containing MathType MTEF data,
//! converts each to an Office Open XML `<m:oMath>` element, replaces the
//! enclosing `<w:r>` (or `<mc:AlternateContent>`), and cleans up orphaned `.bin`
//! and preview image resources.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::error::{Error, Result};
use crate::mtef::decode::{mtef_to_mathml, ole_to_mathml};
use crate::ole::{extract_mtef_from_emf, extract_mtef_from_wmf};
use crate::omml::decode::NS_M;
use crate::omml::encode::{apply_math_font, mathml_to_omml};
use crate::xml::{self, Element, Node};
use crate::MathNode;

use super::consts::*;
use super::options::{ConversionSummary, ToOmmlOptions};
use super::package::Package;

const NS_A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";

// ── Public Entry Points ──────────────────────────────────────────────────────

/// Convert every MathType equation (OLE or WMF) in a `.docx` held in memory to OMML.
pub fn convert_bytes_to_omml(input: &[u8]) -> Result<(Vec<u8>, ConversionSummary)> {
    convert_bytes_to_omml_with(input, &ToOmmlOptions::default())
}

fn read_document_math_font(pkg: &Package) -> Option<String> {
    let settings_bytes = pkg.get("word/settings.xml")?;
    let settings = xml::parse(settings_bytes).ok()?;
    let math_pr = settings.find(NS_M, "mathPr")?;
    let math_font = math_pr.find(NS_M, "mathFont")?;
    let val = math_font
        .attr(Some(NS_M), "val")
        .or_else(|| math_font.attr(None, "val"))?
        .trim();
    (!val.is_empty()).then(|| val.to_owned())
}

/// Like [`convert_bytes_to_omml`], with explicit [`ToOmmlOptions`].
pub fn convert_bytes_to_omml_with(
    input: &[u8],
    opts: &ToOmmlOptions,
) -> Result<(Vec<u8>, ConversionSummary)> {
    let mut pkg = Package::read(input)?;
    if !pkg.contains(DOC_XML_PATH) {
        return Err(Error::MissingPart(DOC_XML_PATH.into()));
    }

    let effective_opts = if opts.math_font.as_deref() == Some("Cambria Math") {
        ToOmmlOptions {
            math_font: Some(read_document_math_font(&pkg).unwrap_or_else(|| "Cambria Math".into())),
        }
    } else {
        opts.clone()
    };

    let content_parts = discover_content_parts(&pkg);
    let mut summary = ConversionSummary::default();
    let mut candidate_files_to_remove: HashSet<String> = HashSet::new();

    for xml_path in content_parts {
        convert_content_part(
            &mut pkg,
            &xml_path,
            &effective_opts,
            &mut summary,
            &mut candidate_files_to_remove,
        )?;
    }

    if !candidate_files_to_remove.is_empty() {
        cleanup_unreferenced_files(&mut pkg, &candidate_files_to_remove)?;
    }

    Ok((pkg.write()?, summary))
}

fn convert_content_part(
    pkg: &mut Package,
    xml_path: &str,
    opts: &ToOmmlOptions,
    summary: &mut ConversionSummary,
    candidate_files_to_remove: &mut HashSet<String>,
) -> Result<()> {
    let Some(xml_bytes) = pkg.get(xml_path) else {
        return Ok(());
    };
    let mut doc_root = xml::parse(xml_bytes)?;
    let rels_path = rels_path_for_part(xml_path);
    let (rels_opt, rel_map) = read_part_relationships(pkg, xml_path, &rels_path)?;
    let mut converted_rids = HashSet::new();
    let before_converted = summary.converted;

    rewrite_element(
        &mut doc_root,
        pkg,
        &rel_map,
        opts,
        summary,
        &mut converted_rids,
    );
    if summary.converted == before_converted {
        return Ok(());
    }

    ensure_math_namespace(&mut doc_root);
    pkg.set(xml_path, xml::to_bytes(&doc_root));
    if let Some(mut rels_root) = rels_opt {
        if remove_obsolete_relationships(
            &mut rels_root,
            &doc_root,
            converted_rids,
            &rel_map,
            candidate_files_to_remove,
        ) {
            pkg.set(&rels_path, xml::to_bytes(&rels_root));
        }
    }
    Ok(())
}

fn remove_obsolete_relationships(
    rels_root: &mut Element,
    doc_root: &Element,
    converted_rids: HashSet<String>,
    rel_map: &HashMap<String, String>,
    candidate_files_to_remove: &mut HashSet<String>,
) -> bool {
    let dead_rids = obsolete_relationship_ids(doc_root, converted_rids);
    if dead_rids.is_empty() {
        return false;
    }
    for rid in &dead_rids {
        if let Some(target_zip_path) = rel_map.get(rid) {
            candidate_files_to_remove.insert(target_zip_path.clone());
        }
    }
    rels_root.children.retain(|node| match node {
        Node::Element(element) if element.local == "Relationship" => element
            .attr(None, "Id")
            .is_none_or(|id| !dead_rids.contains(id)),
        _ => true,
    });
    true
}

/// Convert every MathType equation in `input_path` to OMML and write `output_path`.
pub fn convert_to_omml(
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
) -> Result<ConversionSummary> {
    convert_to_omml_with(input_path, output_path, &ToOmmlOptions::default())
}

/// Like [`convert_to_omml`], with explicit [`ToOmmlOptions`].
pub fn convert_to_omml_with(
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    opts: &ToOmmlOptions,
) -> Result<ConversionSummary> {
    let input = std::fs::read(input_path.as_ref())?;
    let (output, summary) = convert_bytes_to_omml_with(&input, opts)?;
    std::fs::write(output_path.as_ref(), output)?;
    log::info!("Saved → {}", output_path.as_ref().display());
    Ok(summary)
}

// ── Part & Relationship Discovery ────────────────────────────────────────────

pub(crate) fn discover_content_parts(pkg: &Package) -> Vec<String> {
    let mut parts = Vec::new();
    for name in pkg.part_names() {
        if name == "word/document.xml"
            || name == "word/footnotes.xml"
            || name == "word/endnotes.xml"
            || (name.starts_with("word/header") && name.ends_with(".xml"))
            || (name.starts_with("word/footer") && name.ends_with(".xml"))
        {
            parts.push(name);
        }
    }
    parts
}

pub(crate) fn rels_path_for_part(xml_path: &str) -> String {
    match xml_path.rsplit_once('/') {
        Some((dir, file)) => format!("{dir}/_rels/{file}.rels"),
        None => format!("_rels/{xml_path}.rels"),
    }
}

fn read_part_relationships(
    pkg: &Package,
    xml_path: &str,
    rels_path: &str,
) -> Result<(Option<Element>, HashMap<String, String>)> {
    let rels = pkg.get(rels_path).map(xml::parse).transpose()?;
    let map = rels
        .as_ref()
        .map(|root| build_rel_map(root, xml_path))
        .unwrap_or_default();
    Ok((rels, map))
}

pub(crate) fn resolve_target_path(part_dir: &str, target: &str) -> String {
    if let Some(stripped) = target.strip_prefix('/') {
        return stripped.to_owned();
    }
    let combined = if part_dir.is_empty() {
        target.to_owned()
    } else {
        format!("{part_dir}/{target}")
    };
    // Normalize `..` segments
    let mut segs = Vec::new();
    for seg in combined.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                segs.pop();
            }
            s => segs.push(s),
        }
    }
    segs.join("/")
}

pub(crate) fn build_rel_map(rels_root: &Element, xml_path: &str) -> HashMap<String, String> {
    let part_dir = xml_path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
    let mut map = HashMap::new();
    for el in rels_root.elements() {
        if el.local == "Relationship" {
            if let (Some(id), Some(target)) = (el.attr(None, "Id"), el.attr(None, "Target")) {
                if el.attr(None, "TargetMode") == Some("External") {
                    continue;
                }
                map.insert(id.to_owned(), resolve_target_path(part_dir, target));
            }
        }
    }
    map
}

// ── XML Tree Rewriting ───────────────────────────────────────────────────────

fn ensure_math_namespace(root: &mut Element) {
    if !root
        .nsdecls
        .iter()
        .any(|(prefix, uri)| prefix.as_deref() == Some("m") || uri == NS_M)
    {
        root.nsdecls.push((Some("m".to_owned()), NS_M.to_owned()));
    }
}

fn obsolete_relationship_ids(root: &Element, converted: HashSet<String>) -> HashSet<String> {
    let remaining = collect_referenced_rids(root);
    converted
        .into_iter()
        .filter(|rid| !remaining.contains(rid))
        .collect()
}

pub(crate) fn is_mathtype_prog_id(prog_id: &str) -> bool {
    matches!(prog_id, "Equation.DSMT4" | "Equation.3" | "Equation.2")
        || prog_id.starts_with("Equation.DSMT")
}

pub(crate) fn is_mathtype_ole_element(el: &Element) -> bool {
    (el.is(NS_O, "OLEObject") || el.local == "OLEObject")
        && el.attr(None, "ProgID").is_some_and(is_mathtype_prog_id)
}

pub(crate) fn get_r_attr<'a>(el: &'a Element, local: &str) -> Option<&'a str> {
    el.attr(Some(NS_R), local).or_else(|| el.attr(None, local))
}

pub(crate) fn find_mathtype_ole_in_subtree(el: &Element) -> Vec<&Element> {
    el.iter()
        .into_iter()
        .filter(|e| is_mathtype_ole_element(e))
        .collect()
}

pub(crate) fn collect_imagedata_rids(el: &Element) -> Vec<String> {
    let mut out = Vec::new();
    for d in el.iter() {
        if d.is(NS_V, "imagedata") || d.local == "imagedata" {
            if let Some(rid) = get_r_attr(d, "id") {
                out.push(rid.to_owned());
            }
        } else if d.is(NS_A, "blip") || d.local == "blip" {
            if let Some(rid) = get_r_attr(d, "embed") {
                out.push(rid.to_owned());
            }
        }
    }
    out
}

pub(crate) fn preferred_metafile_math(
    image_ids: &[String],
    ole_id: &str,
    pkg: &Package,
    rel_map: &HashMap<String, String>,
) -> Option<MathNode> {
    image_ids.iter().find_map(|id| {
        let path = rel_map.get(id)?;
        let ole = rel_map.get(ole_id).and_then(|path| pkg.get(path));
        let mtef =
            crate::ole::repair_legacy_metafile_mtef(mtef_from_metafile(path, pkg.get(path)?)?, ole);
        mtef_to_mathml(&mtef)
    })
}

enum OmmlConvertOutcome {
    Converted(Element),
    Skipped(&'static str),
}

fn decode_ole_rid_to_mathml(
    rid: &str,
    image_ids: &[String],
    pkg: &Package,
    rel_map: &HashMap<String, String>,
) -> Result<MathNode> {
    if let Some(math) = preferred_metafile_math(image_ids, rid, pkg, rel_map) {
        return Ok(math);
    }
    let target_path = rel_map
        .get(rid)
        .ok_or_else(|| Error::InvalidOle(format!("missing relationship target for {rid}")))?;
    let ole_bytes = pkg
        .get(target_path)
        .ok_or_else(|| Error::MissingPart(target_path.clone()))?;
    ole_to_mathml(ole_bytes)
        .ok_or_else(|| Error::InvalidMtef(format!("failed to decode MTEF from {target_path}")))
}

fn convert_mathml_to_omml_checked(
    mathml: &MathNode,
    opts: &ToOmmlOptions,
) -> Result<OmmlConvertOutcome> {
    if let Some(reason) = crate::omml::check_omml_unsupported(mathml) {
        return Ok(OmmlConvertOutcome::Skipped(reason));
    }
    let mut omml = mathml_to_omml(mathml)?;
    if let Some(ref font) = opts.math_font {
        apply_math_font(&mut omml, font);
    }
    Ok(OmmlConvertOutcome::Converted(omml))
}

fn convert_ole_rid_to_omml_checked(
    rid: &str,
    image_ids: &[String],
    pkg: &Package,
    rel_map: &HashMap<String, String>,
    opts: &ToOmmlOptions,
) -> Result<OmmlConvertOutcome> {
    let mathml = decode_ole_rid_to_mathml(rid, image_ids, pkg, rel_map)?;
    convert_mathml_to_omml_checked(&mathml, opts)
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

fn rewrite_element(
    parent: &mut Element,
    pkg: &Package,
    rel_map: &HashMap<String, String>,
    opts: &ToOmmlOptions,
    summary: &mut ConversionSummary,
    converted_rids: &mut HashSet<String>,
) {
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

                let outcome = match ole_rid {
                    Some(rid) => {
                        convert_ole_rid_to_omml_checked(rid, &img_rids, pkg, rel_map, opts)
                    }
                    None => Err(Error::InvalidOle("OLEObject missing r:id".into())),
                };

                match outcome {
                    Ok(OmmlConvertOutcome::Converted(omml)) => {
                        if let Some(rid) = ole_rid {
                            converted_rids.insert(rid.to_owned());
                        }
                        converted_rids.extend(img_rids);
                        summary.converted += 1;
                        log::info!("Equation {eq_num}: converted AlternateContent → m:oMath");
                        new_children.push(Node::Element(omml));
                        continue;
                    }
                    Ok(OmmlConvertOutcome::Skipped(reason)) => {
                        summary.skipped += 1;
                        log::warn!(
                            "Equation {eq_num}: {reason} is not supported in OMML, skipping"
                        );
                        new_children.push(Node::Element(el));
                        continue;
                    }
                    Err(_) => {
                        // Fallback: if `<mc:Fallback>` already holds an `<m:oMath>`, use it
                        if let Some(mut omml) = el
                            .find(NS_MC, "Fallback")
                            .and_then(|fb| fb.find(NS_M, "oMath"))
                            .cloned()
                        {
                            if let Some(ref font) = opts.math_font {
                                apply_math_font(&mut omml, font);
                            }
                            if let Some(rid) = ole_rid {
                                converted_rids.insert(rid.to_owned());
                            }
                            converted_rids.extend(img_rids);
                            summary.converted += 1;
                            log::info!("Equation {eq_num}: converted AlternateContent → m:oMath");
                            new_children.push(Node::Element(omml));
                            continue;
                        }
                        summary.errors += 1;
                        log::error!(
                            "Equation {eq_num}: failed to convert AlternateContent MathType object"
                        );
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
                let replacements =
                    convert_ole_run(&el, pkg, rel_map, opts, summary, converted_rids);
                for rep in replacements {
                    new_children.push(Node::Element(rep));
                }
                continue;
            }

            // Case 3: `<w:r>` without `<o:OLEObject>`, but with a standalone `.wmf` or `.emf` image
            // containing embedded MathType MTEF data.
            if let Some(omml) =
                try_convert_metafile_run(&el, pkg, rel_map, opts, summary, converted_rids)
            {
                new_children.push(Node::Element(omml));
                continue;
            }
        }

        // Recurse into child elements
        rewrite_element(&mut el, pkg, rel_map, opts, summary, converted_rids);
        new_children.push(Node::Element(el));
    }

    parent.children = new_children;
}

/// Convert a `<w:r>` containing one or more MathType OLE objects.
fn convert_ole_run(
    run_el: &Element,
    pkg: &Package,
    rel_map: &HashMap<String, String>,
    opts: &ToOmmlOptions,
    summary: &mut ConversionSummary,
    converted_rids: &mut HashSet<String>,
) -> Vec<Element> {
    // Check direct `<w:object>` children of `<w:r>` so multi-object runs are split cleanly
    let obj_children: Vec<&Element> = run_el
        .elements()
        .filter(|c| c.is(NS_W, "object") || c.local == "object")
        .collect();

    if obj_children.len() > 1 {
        let rpr = run_el.find(NS_W, "rPr").cloned();
        let mut out = Vec::with_capacity(obj_children.len());
        for obj in obj_children {
            let oles = find_mathtype_ole_in_subtree(obj);
            if oles.is_empty() {
                let mut kept_run = Element::new(Some(NS_W), Some("w"), "r");
                if let Some(ref p) = rpr {
                    kept_run.push(p.clone());
                }
                kept_run.push(obj.clone());
                out.push(kept_run);
                continue;
            }
            summary.found += 1;
            let eq_num = summary.found;
            let ole_rid = get_r_attr(oles[0], "id");
            let img_rids = collect_imagedata_rids(obj);
            match ole_rid
                .ok_or_else(|| Error::InvalidOle("OLEObject missing r:id".into()))
                .and_then(|rid| {
                    let res = convert_ole_rid_to_omml_checked(rid, &img_rids, pkg, rel_map, opts)?;
                    Ok((rid, res))
                }) {
                Ok((rid, OmmlConvertOutcome::Converted(omml))) => {
                    converted_rids.insert(rid.to_owned());
                    converted_rids.extend(img_rids);
                    summary.converted += 1;
                    log::info!("Equation {eq_num}: converted OLE ({rid}) → m:oMath");
                    out.push(omml);
                }
                Ok((rid, OmmlConvertOutcome::Skipped(reason))) => {
                    summary.skipped += 1;
                    log::warn!(
                        "Equation {eq_num}: {reason} is not supported in OMML (OLE {rid}), skipping"
                    );
                    let mut kept_run = Element::new(Some(NS_W), Some("w"), "r");
                    if let Some(ref p) = rpr {
                        kept_run.push(p.clone());
                    }
                    kept_run.push(obj.clone());
                    out.push(kept_run);
                }
                Err(e) => {
                    summary.errors += 1;
                    log::error!("Equation {eq_num}: {e}");
                    let mut kept_run = Element::new(Some(NS_W), Some("w"), "r");
                    if let Some(ref p) = rpr {
                        kept_run.push(p.clone());
                    }
                    kept_run.push(obj.clone());
                    out.push(kept_run);
                }
            }
        }
        return out;
    }

    // Single MathType OLE object inside `<w:r>`
    summary.found += 1;
    let eq_num = summary.found;
    let oles = find_mathtype_ole_in_subtree(run_el);
    let ole_rid = oles.first().and_then(|o| get_r_attr(o, "id"));
    let img_rids = collect_imagedata_rids(run_el);

    match ole_rid
        .ok_or_else(|| Error::InvalidOle("OLEObject missing r:id".into()))
        .and_then(|rid| {
            let res = convert_ole_rid_to_omml_checked(rid, &img_rids, pkg, rel_map, opts)?;
            Ok((rid, res))
        }) {
        Ok((rid, OmmlConvertOutcome::Converted(omml))) => {
            converted_rids.insert(rid.to_owned());
            converted_rids.extend(img_rids);
            summary.converted += 1;
            log::info!("Equation {eq_num}: converted OLE ({rid}) → m:oMath");
            vec![omml]
        }
        Ok((rid, OmmlConvertOutcome::Skipped(reason))) => {
            summary.skipped += 1;
            log::warn!(
                "Equation {eq_num}: {reason} is not supported in OMML (OLE {rid}), skipping"
            );
            vec![run_el.clone()]
        }
        Err(e) => {
            summary.errors += 1;
            log::error!("Equation {eq_num}: {e}");
            vec![run_el.clone()]
        }
    }
}

/// Inspect a `<w:r>` without `<o:OLEObject>`: if it references a `.wmf` or `.emf` image that
/// embeds MathType MTEF data, convert it to `<m:oMath>`.
fn try_convert_metafile_run(
    run_el: &Element,
    pkg: &Package,
    rel_map: &HashMap<String, String>,
    opts: &ToOmmlOptions,
    summary: &mut ConversionSummary,
    converted_rids: &mut HashSet<String>,
) -> Option<Element> {
    let img_rids = collect_imagedata_rids(run_el);
    for rid in img_rids {
        let Some(target_path) = rel_map.get(&rid) else {
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

        // Found a standalone WMF/EMF equation with embedded MTEF!
        summary.found += 1;
        let eq_num = summary.found;
        let kind_str = if is_wmf { "WMF" } else { "EMF" };
        match mtef_to_mathml(&mtef_bytes)
            .ok_or_else(|| {
                Error::InvalidMtef(format!("failed to decode {kind_str} MTEF in {target_path}"))
            })
            .and_then(|mml| convert_mathml_to_omml_checked(&mml, opts))
        {
            Ok(OmmlConvertOutcome::Converted(omml)) => {
                converted_rids.insert(rid.clone());
                summary.converted += 1;
                log::info!("Equation {eq_num}: converted {kind_str} ({rid}) → m:oMath");
                return Some(omml);
            }
            Ok(OmmlConvertOutcome::Skipped(reason)) => {
                summary.skipped += 1;
                log::warn!(
                    "Equation {eq_num}: {reason} is not supported in OMML ({kind_str} {rid}), skipping"
                );
                return None;
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

// ── Orphan Cleanup ───────────────────────────────────────────────────────────

pub(crate) fn collect_referenced_rids(root: &Element) -> HashSet<String> {
    let mut rids = HashSet::new();
    for el in root.iter() {
        for attr in &el.attrs {
            if (attr.ns.as_deref() == Some(NS_R)
                || matches!(attr.local.as_str(), "id" | "embed" | "link"))
                && attr.value.starts_with("rId")
            {
                rids.insert(attr.value.clone());
            }
        }
    }
    rids
}

pub(crate) fn cleanup_unreferenced_files(
    pkg: &mut Package,
    candidates: &HashSet<String>,
) -> Result<()> {
    // Collect all targets still referenced across ALL .rels files in the package
    let mut still_referenced: HashSet<String> = HashSet::new();
    for name in pkg.part_names() {
        if name.ends_with(".rels") {
            if let Some(bytes) = pkg.get(&name) {
                if let Ok(rels_root) = xml::parse(bytes) {
                    let owner_dir = name
                        .split("/_rels/")
                        .next()
                        .filter(|&d| d != name)
                        .unwrap_or("");
                    for el in rels_root.elements() {
                        if el.local == "Relationship" {
                            if let Some(target) = el.attr(None, "Target") {
                                if el.attr(None, "TargetMode") != Some("External") {
                                    still_referenced.insert(resolve_target_path(owner_dir, target));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let mut removed_parts: HashSet<String> = HashSet::new();
    for cand in candidates {
        if !still_referenced.contains(cand) && pkg.remove(cand) {
            removed_parts.insert(format!("/{cand}"));
        }
    }

    if !removed_parts.is_empty() {
        if let Some(ct_bytes) = pkg.get(CT_PATH) {
            if let Ok(mut ct_root) = xml::parse(ct_bytes) {
                ct_root.children.retain(|n| match n {
                    Node::Element(e) if e.local == "Override" => e
                        .attr(None, "PartName")
                        .is_none_or(|pn| !removed_parts.contains(pn)),
                    _ => true,
                });
                pkg.set(CT_PATH, xml::to_bytes(&ct_root));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docx::{convert_bytes_with, EmbedMode, Options, PreviewFormat};
    use std::io::{Cursor, Write};

    fn make_minimal_omml_docx(omml_xml: &str) -> Vec<u8> {
        let ct = r#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#;
        let doc = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math">
  <w:body><w:p>{omml_xml}</w:p></w:body>
</w:document>"#
        );
        let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let opts = zip::write::FileOptions::default();
        w.start_file("[Content_Types].xml", opts).unwrap();
        w.write_all(ct.as_bytes()).unwrap();
        w.start_file("word/document.xml", opts).unwrap();
        w.write_all(doc.as_bytes()).unwrap();
        w.finish().unwrap().into_inner()
    }

    #[test]
    fn relationships_resolve_relative_targets_and_skip_external_resources() {
        let rels = xml::parse(
            br#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Target="media/equation.emf"/>
<Relationship Id="rId2" Target="../embeddings/equation.bin"/>
<Relationship Id="rId3" Target="https://example.test/equation" TargetMode="External"/>
</Relationships>"#,
        )
        .unwrap();

        let map = build_rel_map(&rels, "word/header1.xml");
        assert_eq!(map.get("rId1"), Some(&"word/media/equation.emf".to_owned()));
        assert_eq!(map.get("rId2"), Some(&"embeddings/equation.bin".to_owned()));
        assert!(!map.contains_key("rId3"));
    }

    #[test]
    fn test_docx_convert_standalone_emf_with_mtef_to_omml() {
        let node = crate::mathml::MathNode::with_children(
            "math",
            vec![crate::mathml::MathNode::leaf("mi", "z")],
        );
        let preview = crate::render::emf::render_emf(&node, 12.0).unwrap();

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
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/equation1.emf"/>
</Relationships>"#;

        let ct_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
  <Default Extension="emf" ContentType="image/x-emf"/>
</Types>"#;

        let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let opts = zip::write::FileOptions::default();
        w.start_file("[Content_Types].xml", opts).unwrap();
        w.write_all(ct_xml.as_bytes()).unwrap();
        w.start_file("word/document.xml", opts).unwrap();
        w.write_all(doc_xml.as_bytes()).unwrap();
        w.start_file("word/_rels/document.xml.rels", opts).unwrap();
        w.write_all(rels_xml.as_bytes()).unwrap();
        w.start_file("word/media/equation1.emf", opts).unwrap();
        w.write_all(&preview.emf).unwrap();
        let src_docx = w.finish().unwrap().into_inner();

        let (omml_docx, summary) = convert_bytes_to_omml(&src_docx).unwrap();
        assert_eq!(summary.found, 1);
        assert_eq!(summary.converted, 1);
        assert_eq!(summary.errors, 0);

        let pkg = Package::read(&omml_docx).unwrap();
        let doc_el = xml::parse(pkg.get("word/document.xml").unwrap()).unwrap();
        let omaths: Vec<_> = doc_el
            .iter()
            .into_iter()
            .filter(|e| e.is(NS_M, "oMath"))
            .collect();
        assert_eq!(omaths.len(), 1);
    }

    #[test]
    fn test_docx_roundtrip_preview_and_fallback_modes() {
        let omml = r#"<m:oMath><m:f><m:num><m:r><m:t>x</m:t></m:r><m:r><m:t>+</m:t></m:r><m:r><m:t>1</m:t></m:r></m:num><m:den><m:rad><m:radPr><m:degHide m:val="1"/></m:radPr><m:deg/><m:e><m:r><m:t>y</m:t></m:r></m:e></m:rad></m:den></m:f></m:oMath>"#;
        let src_docx = make_minimal_omml_docx(omml);

        // 1. Convert OMML -> MathType (Preview mode)
        let (mt_docx, sum1) = convert_bytes_with(&src_docx, &Options::default()).unwrap();
        assert_eq!(sum1.found, 1);
        assert_eq!(sum1.converted, 1);

        // 2. Convert MathType -> OMML with math_font
        let (omml_docx, sum2) = convert_bytes_to_omml_with(
            &mt_docx,
            &ToOmmlOptions {
                math_font: Some("Cambria Math".into()),
            },
        )
        .unwrap();
        assert_eq!(sum2.found, 1);
        assert_eq!(sum2.converted, 1);
        assert_eq!(sum2.errors, 0);

        // Verify oleObject1.bin and preview image were cleaned up
        let pkg = Package::read(&omml_docx).unwrap();
        assert!(!pkg.contains("word/embeddings/oleObject1.bin"));
        let doc_el = xml::parse(pkg.get("word/document.xml").unwrap()).unwrap();
        let omaths: Vec<_> = doc_el
            .iter()
            .into_iter()
            .filter(|e| e.is(NS_M, "oMath"))
            .collect();
        assert_eq!(omaths.len(), 1);
        assert!(omaths[0].find(NS_M, "f").is_some());

        // 3. Also test OmmlFallback mode -> OMML
        let (fb_docx, _) = convert_bytes_with(
            &src_docx,
            &Options {
                mode: EmbedMode::OmmlFallback,
                font_pt: 12.0,
                preview: PreviewFormat::Emf,
            },
        )
        .unwrap();
        let (_, sum3) = convert_bytes_to_omml(&fb_docx).unwrap();
        assert_eq!(sum3.found, 1);
        assert_eq!(sum3.converted, 1);
    }

    #[test]
    fn test_docx_to_omml_skips_unsupported_ole_and_preserves_resources() {
        let convertible = crate::service::parse_latex_to_mathml(r"\frac{a}{b}").unwrap();
        let unsupported = crate::service::parse_latex_to_mathml(r"\enclose{longdiv}{144}").unwrap();

        let make_ole_and_emf = |math: &MathNode| -> (Vec<u8>, Vec<u8>) {
            let eq_bytes = crate::mtef::encode::mathml_to_mtef_bytes(math).unwrap();
            let native = crate::mtef::make_equation_native(&eq_bytes);
            let ole_bin = crate::ole::make_ole(&native).unwrap();
            let complete_mtef = crate::mtef::build_mtef(&eq_bytes);
            let preview =
                crate::render::emf::render_emf_with_mtef(math, 12.0, Some(&complete_mtef)).unwrap();
            (ole_bin, preview.emf)
        };

        let (ole1, emf1) = make_ole_and_emf(&convertible);
        let (ole2, emf2) = make_ole_and_emf(&unsupported);

        let doc_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
            xmlns:v="urn:schemas-microsoft-com:vml"
            xmlns:o="urn:schemas-microsoft-com:office:office">
  <w:body>
    <w:p>
      <w:r><w:object><v:shape id="_x0000_i1025" style="width:10pt;height:10pt"><v:imagedata r:id="rIdImg1"/></v:shape><o:OLEObject Type="Embed" ProgID="Equation.DSMT4" ShapeID="_x0000_i1025" r:id="rIdOle1"/></w:object></w:r>
      <w:r><w:object><v:shape id="_x0000_i1026" style="width:10pt;height:10pt"><v:imagedata r:id="rIdImg2"/></v:shape><o:OLEObject Type="Embed" ProgID="Equation.DSMT4" ShapeID="_x0000_i1026" r:id="rIdOle2"/></w:object></w:r>
    </w:p>
  </w:body>
</w:document>"#;

        let rels_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdOle1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/oleObject" Target="embeddings/oleObject1.bin"/>
  <Relationship Id="rIdImg1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/equation1.emf"/>
  <Relationship Id="rIdOle2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/oleObject" Target="embeddings/oleObject2.bin"/>
  <Relationship Id="rIdImg2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/equation2.emf"/>
</Relationships>"#;

        let ct_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
  <Default Extension="emf" ContentType="image/x-emf"/>
  <Default Extension="bin" ContentType="application/vnd.openxmlformats-officedocument.oleObject"/>
</Types>"#;

        let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let opts = zip::write::FileOptions::default();
        w.start_file("[Content_Types].xml", opts).unwrap();
        w.write_all(ct_xml.as_bytes()).unwrap();
        w.start_file("word/document.xml", opts).unwrap();
        w.write_all(doc_xml.as_bytes()).unwrap();
        w.start_file("word/_rels/document.xml.rels", opts).unwrap();
        w.write_all(rels_xml.as_bytes()).unwrap();
        w.start_file("word/embeddings/oleObject1.bin", opts)
            .unwrap();
        w.write_all(&ole1).unwrap();
        w.start_file("word/media/equation1.emf", opts).unwrap();
        w.write_all(&emf1).unwrap();
        w.start_file("word/embeddings/oleObject2.bin", opts)
            .unwrap();
        w.write_all(&ole2).unwrap();
        w.start_file("word/media/equation2.emf", opts).unwrap();
        w.write_all(&emf2).unwrap();
        let src_docx = w.finish().unwrap().into_inner();

        let (omml_docx, summary) = convert_bytes_to_omml(&src_docx).unwrap();
        assert_eq!(summary.found, 2);
        assert_eq!(summary.converted, 1);
        assert_eq!(summary.skipped, 1);
        assert_eq!(summary.errors, 0);

        let pkg = Package::read(&omml_docx).unwrap();
        assert!(!pkg.contains("word/embeddings/oleObject1.bin"));
        assert!(!pkg.contains("word/media/equation1.emf"));
        assert!(pkg.contains("word/embeddings/oleObject2.bin"));
        assert!(pkg.contains("word/media/equation2.emf"));

        let doc_el = xml::parse(pkg.get("word/document.xml").unwrap()).unwrap();
        let omaths: Vec<_> = doc_el
            .iter()
            .into_iter()
            .filter(|e| e.is(NS_M, "oMath"))
            .collect();
        assert_eq!(omaths.len(), 1);
        assert!(omaths[0].find(NS_M, "f").is_some());
        let oles: Vec<_> = doc_el
            .iter()
            .into_iter()
            .filter(|e| e.local == "OLEObject")
            .collect();
        assert_eq!(oles.len(), 1);
    }

    #[test]
    fn test_docx_to_omml_skips_unsupported_standalone_emf() {
        let unsupported =
            crate::service::parse_latex_to_mathml(r"\enclose{roundedbox}{x+y}").unwrap();
        let eq_bytes = crate::mtef::encode::mathml_to_mtef_bytes(&unsupported).unwrap();
        let mtef = crate::mtef::build_mtef(&eq_bytes);
        let preview =
            crate::render::emf::render_emf_with_mtef(&unsupported, 12.0, Some(&mtef)).unwrap();

        let doc_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
            xmlns:v="urn:schemas-microsoft-com:vml">
  <w:body><w:p><w:r><w:pict><v:shape id="_x0000_i1025" style="width:10pt;height:10pt"><v:imagedata r:id="rId1"/></v:shape></w:pict></w:r></w:p></w:body>
</w:document>"#;
        let rels_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/equation1.emf"/>
</Relationships>"#;
        let ct_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
  <Default Extension="emf" ContentType="image/x-emf"/>
</Types>"#;

        let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let opts = zip::write::FileOptions::default();
        w.start_file("[Content_Types].xml", opts).unwrap();
        w.write_all(ct_xml.as_bytes()).unwrap();
        w.start_file("word/document.xml", opts).unwrap();
        w.write_all(doc_xml.as_bytes()).unwrap();
        w.start_file("word/_rels/document.xml.rels", opts).unwrap();
        w.write_all(rels_xml.as_bytes()).unwrap();
        w.start_file("word/media/equation1.emf", opts).unwrap();
        w.write_all(&preview.emf).unwrap();
        let src_docx = w.finish().unwrap().into_inner();

        let (omml_docx, summary) = convert_bytes_to_omml(&src_docx).unwrap();
        assert_eq!(summary.found, 1);
        assert_eq!(summary.converted, 0);
        assert_eq!(summary.skipped, 1);
        assert_eq!(summary.errors, 0);

        let pkg = Package::read(&omml_docx).unwrap();
        assert!(pkg.contains("word/media/equation1.emf"));
        let doc_el = xml::parse(pkg.get("word/document.xml").unwrap()).unwrap();
        assert_eq!(
            doc_el
                .iter()
                .into_iter()
                .filter(|e| e.is(NS_M, "oMath"))
                .count(),
            0
        );
    }

    #[test]
    fn test_docx_to_omml_honors_settings_math_font_and_defaults_to_cambria_math() {
        let omml = r#"<m:oMath><m:f><m:num><m:r><m:t>x</m:t></m:r></m:num><m:den><m:r><m:t>2</m:t></m:r></m:den></m:f></m:oMath>"#;
        let src_docx = make_minimal_omml_docx(omml);
        let (mt_docx, _) = convert_bytes_with(&src_docx, &Options::default()).unwrap();

        // 1. Default conversion without settings.xml emits Cambria Math on m:r and m:ctrlPr
        let (default_omml_docx, _) = convert_bytes_to_omml(&mt_docx).unwrap();
        let pkg_default = Package::read(&default_omml_docx).unwrap();
        let doc_default =
            String::from_utf8(pkg_default.get("word/document.xml").unwrap().to_vec()).unwrap();
        assert!(doc_default.contains(r#"w:ascii="Cambria Math""#));
        assert!(doc_default.contains("m:ctrlPr"));

        // 2. Document with word/settings.xml <m:mathFont m:val="Latin Modern Math"/> is honored
        let mut pkg_with_settings = Package::read(&mt_docx).unwrap();
        let settings_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:settings xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math">
  <m:mathPr><m:mathFont m:val="Latin Modern Math"/></m:mathPr>
</w:settings>"#;
        pkg_with_settings.set("word/settings.xml", settings_xml.to_vec());
        let mt_with_settings = pkg_with_settings.write().unwrap();

        let (honored_omml_docx, _) = convert_bytes_to_omml(&mt_with_settings).unwrap();
        let pkg_honored = Package::read(&honored_omml_docx).unwrap();
        let doc_honored =
            String::from_utf8(pkg_honored.get("word/document.xml").unwrap().to_vec()).unwrap();
        assert!(doc_honored.contains(r#"w:ascii="Latin Modern Math""#));
        assert!(!doc_honored.contains("Cambria Math"));
    }
}
