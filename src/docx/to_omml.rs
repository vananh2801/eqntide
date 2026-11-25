//! MathType (OLE `.bin` / standalone `.wmf` & `.emf`) → OMML conversion pipeline for `.docx` files.
//!
//! ```text
//! input `.docx`
//!   │
//!   ▼
//! discover_content_parts (document.xml, footnotes, endnotes, headers, footers)
//!   │
//!   ▼
//! for each XML part:
//!   ├── collect_to_omml_items  (AlternateContent | OleRun | MetafileRun)
//!   ├── load_transient_batch   (pre-fetch referenced .bin / .wmf / .emf parts)
//!   ├── prepare_to_omml_item   (parallel decode MTEF ─▶ MathML ─▶ <m:oMath>)
//!   └── splice_at_path (rev)   + prune obsolete relationship IDs
//!   │
//!   ▼
//! cleanup_unreferenced_files (remove orphaned .bin / image parts & CT overrides)
//! ```

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use rayon::prelude::*;

use crate::error::{Error, Result};
use crate::mtef::decode::{mtef_to_mathml, ole_to_mathml};
use crate::ole::{extract_mtef_from_emf, extract_mtef_from_wmf};
use crate::omml::decode::NS_M;
use crate::omml::encode::{apply_math_font, mathml_to_omml};
use crate::xml::{self, Element, Node};
use crate::MathNode;

use super::consts::*;
use super::job::element_at;
use super::options::{ConversionSummary, ToOmmlOptions};
use super::package::{Package, PartLookup};

const NS_A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
const TO_OMML_CHUNK_SIZE: usize = 128;

// ── Internal Work & Outcome Types ────────────────────────────────────────────

#[derive(Clone, Copy)]
enum ToOmmlWorkKind {
    AlternateContent,
    OleRun,
    MetafileRun,
}

struct ToOmmlWorkRef {
    path: Vec<usize>,
    kind: ToOmmlWorkKind,
}

enum OmmlConvertOutcome {
    Converted(Element),
    Skipped(&'static str),
}

enum ToOmmlPrepared {
    NoOp,
    AlternateContent {
        ole_rid: Option<String>,
        img_rids: Vec<String>,
        outcome: Result<OmmlConvertOutcome>,
        fallback_omml: Option<Element>,
    },
    OleRun(Vec<OleRunEntryOutcome>),
    MetafileRun {
        rid: String,
        kind_str: &'static str,
        outcome: Result<OmmlConvertOutcome>,
    },
}

enum OleRunEntryOutcome {
    Keep(Element),
    Converted {
        rid: String,
        img_rids: Vec<String>,
        omml: Element,
    },
    Skipped {
        rid: String,
        reason: &'static str,
        kept: Element,
    },
    Failed {
        err: Error,
        kept: Element,
    },
}

// ── Public Entry Points ──────────────────────────────────────────────────────

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
    options: &ToOmmlOptions,
) -> Result<ConversionSummary> {
    convert_to_omml_with_cancel(input_path, output_path, options, None)
}

/// Like [`convert_to_omml_with`], supporting cooperative cancellation before writing output.
pub fn convert_to_omml_with_cancel(
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    options: &ToOmmlOptions,
    cancel: Option<&AtomicBool>,
) -> Result<ConversionSummary> {
    if cancel.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
        return Err(Error::Cancelled);
    }
    let package = Package::open(input_path.as_ref())?;
    let (mut package, summary) = convert_package_to_omml_with_cancel(package, options, cancel)?;
    package.write_to_file_atomic(output_path.as_ref(), cancel)?;
    log::info!("Saved → {}", output_path.as_ref().display());
    Ok(summary)
}

/// Convert every MathType equation (OLE or WMF) in a `.docx` held in memory to OMML.
pub fn convert_bytes_to_omml(input: &[u8]) -> Result<(Vec<u8>, ConversionSummary)> {
    convert_bytes_to_omml_with(input, &ToOmmlOptions::default())
}

/// Like [`convert_bytes_to_omml`], with explicit [`ToOmmlOptions`].
pub fn convert_bytes_to_omml_with(
    input: &[u8],
    options: &ToOmmlOptions,
) -> Result<(Vec<u8>, ConversionSummary)> {
    convert_bytes_to_omml_with_cancel(input, options, None)
}

/// Like [`convert_bytes_to_omml_with`], supporting cooperative cancellation.
pub fn convert_bytes_to_omml_with_cancel(
    input: &[u8],
    options: &ToOmmlOptions,
    cancel: Option<&AtomicBool>,
) -> Result<(Vec<u8>, ConversionSummary)> {
    if cancel.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
        return Err(Error::Cancelled);
    }
    let package = Package::read(input)?;
    let (package, summary) = convert_package_to_omml_with_cancel(package, options, cancel)?;
    Ok((package.write()?, summary))
}

// ── Package & Part Orchestration ─────────────────────────────────────────────

fn convert_package_to_omml_with_cancel(
    mut package: Package,
    options: &ToOmmlOptions,
    cancel: Option<&AtomicBool>,
) -> Result<(Package, ConversionSummary)> {
    if !package.contains(DOC_XML_PATH) {
        return Err(Error::MissingPart(DOC_XML_PATH.into()));
    }

    let effective_opts = if options.math_font.as_deref() == Some("Cambria Math") {
        ToOmmlOptions {
            math_font: Some(
                read_document_math_font(&mut package).unwrap_or_else(|| "Cambria Math".into()),
            ),
        }
    } else {
        options.clone()
    };

    let content_parts = discover_content_parts(&package);
    let mut summary = ConversionSummary::default();
    let mut candidate_files_to_remove: HashSet<String> = HashSet::new();

    for xml_path in content_parts {
        if cancel.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            return Err(Error::Cancelled);
        }
        convert_content_part(
            &mut package,
            &xml_path,
            &effective_opts,
            &mut summary,
            &mut candidate_files_to_remove,
            cancel,
        )?;
    }

    if !candidate_files_to_remove.is_empty() {
        cleanup_unreferenced_files(&mut package, &candidate_files_to_remove)?;
    }

    if cancel.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
        return Err(Error::Cancelled);
    }
    Ok((package, summary))
}

fn read_document_math_font(package: &mut Package) -> Option<String> {
    let settings = package.load_xml("word/settings.xml").ok().flatten()?;
    let math_pr = settings.find(NS_M, "mathPr")?;
    let math_font = math_pr.find(NS_M, "mathFont")?;
    let font_name = math_font
        .attr(Some(NS_M), "val")
        .or_else(|| math_font.attr(None, "val"))?
        .trim();
    (!font_name.is_empty()).then(|| font_name.to_owned())
}

fn convert_content_part(
    package: &mut Package,
    xml_path: &str,
    options: &ToOmmlOptions,
    summary: &mut ConversionSummary,
    candidate_files_to_remove: &mut HashSet<String>,
    cancel: Option<&AtomicBool>,
) -> Result<()> {
    let Some(mut doc_root) = package.load_xml(xml_path)? else {
        return Ok(());
    };
    let rels_path = rels_path_for_part(xml_path);
    let (rels_opt, rel_map) = read_part_relationships(package, xml_path, &rels_path)?;
    let mut converted_rids = HashSet::new();
    let before_converted = summary.converted;

    rewrite_part_parallel(
        &mut doc_root,
        package,
        &rel_map,
        options,
        summary,
        &mut converted_rids,
        cancel,
    )?;
    if summary.converted == before_converted {
        return Ok(());
    }

    ensure_math_namespace(&mut doc_root);
    package.set(xml_path, xml::to_bytes(&doc_root));
    if let Some(mut rels_root) = rels_opt {
        let dead_rids = obsolete_relationship_ids(&doc_root, converted_rids);
        if !dead_rids.is_empty() {
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
            package.set(&rels_path, xml::to_bytes(&rels_root));
        }
    }
    Ok(())
}

// ── Part & Relationship Discovery ────────────────────────────────────────────

/// List XML story parts in a `.docx` that may contain equations.
pub(crate) fn discover_content_parts(package: &Package) -> Vec<String> {
    let mut parts = Vec::new();
    for name in package.part_names() {
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

/// Return the `.rels` path corresponding to an XML part path inside the package.
pub(crate) fn rels_path_for_part(xml_path: &str) -> String {
    match xml_path.rsplit_once('/') {
        Some((dir, file)) => format!("{dir}/_rels/{file}.rels"),
        None => format!("_rels/{xml_path}.rels"),
    }
}

/// Resolve a potentially relative relationship `target` against `part_dir`.
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
    let mut segments = Vec::new();
    for segment in combined.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            part => segments.push(part),
        }
    }
    segments.join("/")
}

/// Build an internal `rId -> resolved package path` map for a `.rels` XML tree.
pub(crate) fn build_rel_map(rels_root: &Element, xml_path: &str) -> HashMap<String, String> {
    let part_dir = xml_path.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("");
    let mut map = HashMap::new();
    for element in rels_root.elements() {
        if element.local == "Relationship" {
            if let (Some(id), Some(target)) =
                (element.attr(None, "Id"), element.attr(None, "Target"))
            {
                if element.attr(None, "TargetMode") == Some("External") {
                    continue;
                }
                map.insert(id.to_owned(), resolve_target_path(part_dir, target));
            }
        }
    }
    map
}

fn read_part_relationships(
    package: &mut Package,
    xml_path: &str,
    rels_path: &str,
) -> Result<(Option<Element>, HashMap<String, String>)> {
    let rels = package.load_xml(rels_path)?;
    let map = rels
        .as_ref()
        .map(|root| build_rel_map(root, xml_path))
        .unwrap_or_default();
    Ok((rels, map))
}

// ── Parallel Preparation & XML Tree Rewriting ────────────────────────────────

fn rewrite_part_parallel(
    doc_root: &mut Element,
    package: &mut Package,
    rel_map: &HashMap<String, String>,
    options: &ToOmmlOptions,
    summary: &mut ConversionSummary,
    converted_rids: &mut HashSet<String>,
    cancel: Option<&AtomicBool>,
) -> Result<()> {
    let mut items = Vec::new();
    collect_to_omml_items(doc_root, &mut Vec::new(), &mut items);
    if items.is_empty() {
        return Ok(());
    }

    // Paths are in original document order. Only multi-object runs can
    // insert siblings, shifting later paths through that same parent.
    let mut extra_children: HashMap<Vec<usize>, usize> = HashMap::new();

    for chunk in items.chunks(TO_OMML_CHUNK_SIZE) {
        if cancel.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            return Err(Error::Cancelled);
        }

        let paths: Vec<Vec<usize>> = chunk
            .iter()
            .map(|item| {
                item.path
                    .iter()
                    .enumerate()
                    .map(|(depth, index)| {
                        index
                            + extra_children
                                .get(&item.path[..depth])
                                .copied()
                                .unwrap_or(0)
                    })
                    .collect()
            })
            .collect();
        let chunk_refs = chunk
            .iter()
            .zip(&paths)
            .map(|(item, path)| {
                let element = element_at(doc_root, path)
                    .ok_or_else(|| Error::Xml("missing equation at its recorded path".into()))?;
                Ok((item, element))
            })
            .collect::<Result<Vec<_>>>()?;

        let part_paths = collect_chunk_part_paths(&chunk_refs, rel_map);
        let chunk_cache = package.load_transient_batch(part_paths)?;

        let prepared: Vec<Result<ToOmmlPrepared>> = {
            let lookup = PartLookup {
                pkg: package,
                cache: &chunk_cache,
            };
            if chunk_refs.len() <= 1 {
                chunk_refs
                    .iter()
                    .map(|(work, element)| {
                        prepare_to_omml_item(work.kind, element, &lookup, rel_map, options, cancel)
                    })
                    .collect()
            } else {
                chunk_refs
                    .par_iter()
                    .map(|(work, element)| {
                        prepare_to_omml_item(work.kind, element, &lookup, rel_map, options, cancel)
                    })
                    .collect()
            }
        };

        drop(chunk_cache);

        if cancel.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            return Err(Error::Cancelled);
        }

        let mut splices = Vec::new();
        for ((item, path), result) in chunk.iter().zip(paths).zip(prepared) {
            let prepared_item = result?;
            match prepared_item {
                ToOmmlPrepared::NoOp => {}
                ToOmmlPrepared::AlternateContent {
                    ole_rid,
                    img_rids,
                    outcome,
                    fallback_omml,
                } => {
                    summary.found += 1;
                    let eq_num = summary.found;
                    match outcome {
                        Ok(OmmlConvertOutcome::Converted(omml)) => {
                            if let Some(rid) = ole_rid {
                                converted_rids.insert(rid);
                            }
                            converted_rids.extend(img_rids);
                            summary.converted += 1;
                            log::info!("Equation {eq_num}: converted AlternateContent → m:oMath");
                            splices.push((path, vec![omml]));
                        }
                        Ok(OmmlConvertOutcome::Skipped(reason)) => {
                            summary.skipped += 1;
                            log::warn!(
                                "Equation {eq_num}: {reason} is not supported in OMML, skipping"
                            );
                        }
                        Err(_) => {
                            if let Some(omml) = fallback_omml {
                                if let Some(rid) = ole_rid {
                                    converted_rids.insert(rid);
                                }
                                converted_rids.extend(img_rids);
                                summary.converted += 1;
                                log::info!(
                                    "Equation {eq_num}: converted AlternateContent → m:oMath"
                                );
                                splices.push((path, vec![omml]));
                            } else {
                                summary.errors += 1;
                                log::error!(
                                    "Equation {eq_num}: failed to convert AlternateContent MathType object"
                                );
                            }
                        }
                    }
                }
                ToOmmlPrepared::OleRun(entries) => {
                    let mut replacements = Vec::with_capacity(entries.len());
                    let mut any_changed = false;
                    for entry in entries {
                        match entry {
                            OleRunEntryOutcome::Keep(kept) => replacements.push(kept),
                            OleRunEntryOutcome::Converted {
                                rid,
                                img_rids,
                                omml,
                            } => {
                                summary.found += 1;
                                let eq_num = summary.found;
                                converted_rids.insert(rid.clone());
                                converted_rids.extend(img_rids);
                                summary.converted += 1;
                                any_changed = true;
                                log::info!("Equation {eq_num}: converted OLE ({rid}) → m:oMath");
                                replacements.push(omml);
                            }
                            OleRunEntryOutcome::Skipped { rid, reason, kept } => {
                                summary.found += 1;
                                let eq_num = summary.found;
                                summary.skipped += 1;
                                log::warn!(
                                    "Equation {eq_num}: {reason} is not supported in OMML (OLE {rid}), skipping"
                                );
                                replacements.push(kept);
                            }
                            OleRunEntryOutcome::Failed { err, kept } => {
                                summary.found += 1;
                                let eq_num = summary.found;
                                summary.errors += 1;
                                log::error!("Equation {eq_num}: {err}");
                                replacements.push(kept);
                            }
                        }
                    }
                    if any_changed {
                        if replacements.len() > 1 {
                            let parent = &item.path[..item.path.len() - 1];
                            *extra_children.entry(parent.to_vec()).or_default() +=
                                replacements.len() - 1;
                        }
                        splices.push((path, replacements));
                    }
                }
                ToOmmlPrepared::MetafileRun {
                    rid,
                    kind_str,
                    outcome,
                } => {
                    summary.found += 1;
                    let eq_num = summary.found;
                    match outcome {
                        Ok(OmmlConvertOutcome::Converted(omml)) => {
                            converted_rids.insert(rid.clone());
                            summary.converted += 1;
                            log::info!("Equation {eq_num}: converted {kind_str} ({rid}) → m:oMath");
                            splices.push((path, vec![omml]));
                        }
                        Ok(OmmlConvertOutcome::Skipped(reason)) => {
                            summary.skipped += 1;
                            log::warn!(
                                "Equation {eq_num}: {reason} is not supported in OMML ({kind_str} {rid}), skipping"
                            );
                        }
                        Err(err) => {
                            summary.errors += 1;
                            log::error!("Equation {eq_num}: {err}");
                        }
                    }
                }
            }
        }
        // Current paths include shifts from earlier chunks only.
        for (path, replacements) in splices.into_iter().rev() {
            splice_at_path(doc_root, &path, replacements);
        }
    }

    Ok(())
}

fn collect_to_omml_items(element: &Element, path: &mut Vec<usize>, out: &mut Vec<ToOmmlWorkRef>) {
    for (index, child) in element.children.iter().enumerate() {
        let Node::Element(child_el) = child else {
            continue;
        };
        path.push(index);
        if (child_el.is(NS_MC, "AlternateContent") || child_el.local == "AlternateContent")
            && !find_mathtype_ole_in_subtree(child_el).is_empty()
        {
            out.push(ToOmmlWorkRef {
                path: path.clone(),
                kind: ToOmmlWorkKind::AlternateContent,
            });
            path.pop();
            continue;
        }
        if child_el.is(NS_W, "r") {
            if !find_mathtype_ole_in_subtree(child_el).is_empty() {
                out.push(ToOmmlWorkRef {
                    path: path.clone(),
                    kind: ToOmmlWorkKind::OleRun,
                });
                path.pop();
                continue;
            }
            if !collect_imagedata_rids(child_el).is_empty() {
                out.push(ToOmmlWorkRef {
                    path: path.clone(),
                    kind: ToOmmlWorkKind::MetafileRun,
                });
                path.pop();
                continue;
            }
        }
        collect_to_omml_items(child_el, path, out);
        path.pop();
    }
}

fn collect_chunk_part_paths<'a>(
    items: &[(&ToOmmlWorkRef, &Element)],
    rel_map: &'a HashMap<String, String>,
) -> Vec<&'a str> {
    let mut paths = Vec::new();
    for (work, element) in items {
        match work.kind {
            ToOmmlWorkKind::AlternateContent | ToOmmlWorkKind::OleRun => {
                for ole in find_mathtype_ole_in_subtree(element) {
                    if let Some(rid) = get_r_attr(ole, "id") {
                        if let Some(part_path) = rel_map.get(rid) {
                            paths.push(part_path.as_str());
                        }
                    }
                }
                for rid in collect_imagedata_rids(element) {
                    if let Some(part_path) = rel_map.get(&rid) {
                        paths.push(part_path.as_str());
                    }
                }
            }
            ToOmmlWorkKind::MetafileRun => {
                for rid in collect_imagedata_rids(element) {
                    if let Some(part_path) = rel_map.get(&rid) {
                        let lower = part_path.to_ascii_lowercase();
                        if lower.ends_with(".wmf") || lower.ends_with(".emf") {
                            paths.push(part_path.as_str());
                        }
                    }
                }
            }
        }
    }
    paths
}

fn prepare_to_omml_item(
    kind: ToOmmlWorkKind,
    element: &Element,
    lookup: &PartLookup<'_>,
    rel_map: &HashMap<String, String>,
    options: &ToOmmlOptions,
    cancel: Option<&AtomicBool>,
) -> Result<ToOmmlPrepared> {
    if cancel.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
        return Err(Error::Cancelled);
    }

    match kind {
        ToOmmlWorkKind::AlternateContent => {
            let ole_nodes = find_mathtype_ole_in_subtree(element);
            let ole_rid = ole_nodes[0]
                .attr(Some(NS_R), "id")
                .or_else(|| ole_nodes[0].attr(None, "id"))
                .map(str::to_owned);
            let img_rids = collect_imagedata_rids(element);
            let outcome = match ole_rid.as_deref() {
                Some(rid) => {
                    convert_ole_rid_to_omml_checked(rid, &img_rids, lookup, rel_map, options)
                }
                None => Err(Error::InvalidOle("OLEObject missing r:id".into())),
            };
            let fallback_omml = if outcome.is_err() {
                element
                    .find(NS_MC, "Fallback")
                    .and_then(|fallback| fallback.find(NS_M, "oMath"))
                    .cloned()
                    .map(|mut omml| {
                        if let Some(ref font) = options.math_font {
                            apply_math_font(&mut omml, font);
                        }
                        omml
                    })
            } else {
                None
            };
            Ok(ToOmmlPrepared::AlternateContent {
                ole_rid,
                img_rids,
                outcome,
                fallback_omml,
            })
        }
        ToOmmlWorkKind::OleRun => {
            let run_el = element;
            let obj_children: Vec<&Element> = run_el
                .elements()
                .filter(|child| child.is(NS_W, "object") || child.local == "object")
                .collect();

            if obj_children.len() > 1 {
                let run_props = run_el.find(NS_W, "rPr").cloned();
                let mut entries = Vec::with_capacity(obj_children.len());
                for obj in obj_children {
                    let oles = find_mathtype_ole_in_subtree(obj);
                    let make_kept = || {
                        let mut kept_run = Element::new(Some(NS_W), Some("w"), "r");
                        if let Some(ref props) = run_props {
                            kept_run.push(props.clone());
                        }
                        kept_run.push(obj.clone());
                        kept_run
                    };
                    if oles.is_empty() {
                        entries.push(OleRunEntryOutcome::Keep(make_kept()));
                        continue;
                    }
                    let ole_rid = get_r_attr(oles[0], "id");
                    let img_rids = collect_imagedata_rids(obj);
                    match ole_rid
                        .ok_or_else(|| Error::InvalidOle("OLEObject missing r:id".into()))
                        .and_then(|rid| {
                            let outcome = convert_ole_rid_to_omml_checked(
                                rid, &img_rids, lookup, rel_map, options,
                            )?;
                            Ok((rid, outcome))
                        }) {
                        Ok((rid, OmmlConvertOutcome::Converted(omml))) => {
                            entries.push(OleRunEntryOutcome::Converted {
                                rid: rid.to_owned(),
                                img_rids,
                                omml,
                            });
                        }
                        Ok((rid, OmmlConvertOutcome::Skipped(reason))) => {
                            entries.push(OleRunEntryOutcome::Skipped {
                                rid: rid.to_owned(),
                                reason,
                                kept: make_kept(),
                            });
                        }
                        Err(err) => {
                            entries.push(OleRunEntryOutcome::Failed {
                                err,
                                kept: make_kept(),
                            });
                        }
                    }
                }
                Ok(ToOmmlPrepared::OleRun(entries))
            } else {
                let oles = find_mathtype_ole_in_subtree(run_el);
                let ole_rid = oles.first().and_then(|ole| get_r_attr(ole, "id"));
                let img_rids = collect_imagedata_rids(run_el);
                let entry = match ole_rid
                    .ok_or_else(|| Error::InvalidOle("OLEObject missing r:id".into()))
                    .and_then(|rid| {
                        let outcome = convert_ole_rid_to_omml_checked(
                            rid, &img_rids, lookup, rel_map, options,
                        )?;
                        Ok((rid, outcome))
                    }) {
                    Ok((rid, OmmlConvertOutcome::Converted(omml))) => {
                        OleRunEntryOutcome::Converted {
                            rid: rid.to_owned(),
                            img_rids,
                            omml,
                        }
                    }
                    Ok((rid, OmmlConvertOutcome::Skipped(reason))) => OleRunEntryOutcome::Skipped {
                        rid: rid.to_owned(),
                        reason,
                        kept: run_el.clone(),
                    },
                    Err(err) => OleRunEntryOutcome::Failed {
                        err,
                        kept: run_el.clone(),
                    },
                };
                Ok(ToOmmlPrepared::OleRun(vec![entry]))
            }
        }
        ToOmmlWorkKind::MetafileRun => {
            let img_rids = collect_imagedata_rids(element);
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
                let Some(img_bytes) = lookup.get(target_path) else {
                    continue;
                };
                let Some(mtef_bytes) = mtef_from_metafile(target_path, img_bytes) else {
                    continue;
                };
                let kind_str = if is_wmf { "WMF" } else { "EMF" };
                let outcome = mtef_to_mathml(&mtef_bytes)
                    .ok_or_else(|| {
                        Error::InvalidMtef(format!(
                            "failed to decode {kind_str} MTEF in {target_path}"
                        ))
                    })
                    .and_then(|mathml| convert_mathml_to_omml_checked(&mathml, options));
                return Ok(ToOmmlPrepared::MetafileRun {
                    rid,
                    kind_str,
                    outcome,
                });
            }
            Ok(ToOmmlPrepared::NoOp)
        }
    }
}

fn splice_at_path(root: &mut Element, path: &[usize], replacements: Vec<Element>) {
    let Some((&slot, parent_path)) = path.split_last() else {
        return;
    };
    let mut current = root;
    for &idx in parent_path {
        match current.children.get_mut(idx) {
            Some(Node::Element(child)) => current = child,
            _ => return,
        }
    }
    current
        .children
        .splice(slot..=slot, replacements.into_iter().map(Node::Element));
}

// ── Decoding & Conversion Helpers ────────────────────────────────────────────

/// Return whether `prog_id` identifies a MathType or legacy Equation Editor OLE object.
pub(crate) fn is_mathtype_prog_id(prog_id: &str) -> bool {
    matches!(prog_id, "Equation.DSMT4" | "Equation.3" | "Equation.2")
        || prog_id.starts_with("Equation.DSMT")
}

/// Return whether an XML element is an `o:OLEObject` with a MathType `ProgID`.
pub(crate) fn is_mathtype_ole_element(element: &Element) -> bool {
    (element.is(NS_O, "OLEObject") || element.local == "OLEObject")
        && element
            .attr(None, "ProgID")
            .is_some_and(is_mathtype_prog_id)
}

/// Read an attribute in the OOXML relationships namespace (`r:`) or without namespace.
pub(crate) fn get_r_attr<'a>(element: &'a Element, local: &str) -> Option<&'a str> {
    element
        .attr(Some(NS_R), local)
        .or_else(|| element.attr(None, local))
}

/// Find every MathType `o:OLEObject` descendant within `element`.
pub(crate) fn find_mathtype_ole_in_subtree(element: &Element) -> Vec<&Element> {
    element
        .iter()
        .into_iter()
        .filter(|child| is_mathtype_ole_element(child))
        .collect()
}

/// Collect relationship IDs referenced by VML `<v:imagedata>` or DrawingML `<a:blip>` elements.
pub(crate) fn collect_imagedata_rids(element: &Element) -> Vec<String> {
    let mut out = Vec::new();
    for descendant in element.iter() {
        if descendant.is(NS_V, "imagedata") || descendant.local == "imagedata" {
            if let Some(rid) = get_r_attr(descendant, "id") {
                out.push(rid.to_owned());
            }
        } else if descendant.is(NS_A, "blip") || descendant.local == "blip" {
            if let Some(rid) = get_r_attr(descendant, "embed") {
                out.push(rid.to_owned());
            }
        }
    }
    out
}

/// Decode MTEF embedded inside an equation's preview metafile, repairing legacy streams if needed.
pub(crate) fn preferred_metafile_math(
    image_ids: &[String],
    ole_id: &str,
    lookup: &PartLookup<'_>,
    rel_map: &HashMap<String, String>,
) -> Option<MathNode> {
    image_ids.iter().find_map(|id| {
        let path = rel_map.get(id)?;
        let ole = rel_map
            .get(ole_id)
            .and_then(|ole_path| lookup.get(ole_path));
        let mtef = crate::ole::repair_legacy_metafile_mtef(
            mtef_from_metafile(path, lookup.get(path)?)?,
            ole,
        );
        mtef_to_mathml(&mtef)
    })
}

fn convert_ole_rid_to_omml_checked(
    rid: &str,
    image_ids: &[String],
    lookup: &PartLookup<'_>,
    rel_map: &HashMap<String, String>,
    options: &ToOmmlOptions,
) -> Result<OmmlConvertOutcome> {
    let mathml = decode_ole_rid_to_mathml(rid, image_ids, lookup, rel_map)?;
    convert_mathml_to_omml_checked(&mathml, options)
}

fn decode_ole_rid_to_mathml(
    rid: &str,
    image_ids: &[String],
    lookup: &PartLookup<'_>,
    rel_map: &HashMap<String, String>,
) -> Result<MathNode> {
    if let Some(math) = preferred_metafile_math(image_ids, rid, lookup, rel_map) {
        return Ok(math);
    }
    let target_path = rel_map
        .get(rid)
        .ok_or_else(|| Error::InvalidOle(format!("missing relationship target for {rid}")))?;
    let ole_bytes = lookup
        .get(target_path)
        .ok_or_else(|| Error::MissingPart(target_path.clone()))?;
    ole_to_mathml(ole_bytes)
        .ok_or_else(|| Error::InvalidMtef(format!("failed to decode MTEF from {target_path}")))
}

fn convert_mathml_to_omml_checked(
    mathml: &MathNode,
    options: &ToOmmlOptions,
) -> Result<OmmlConvertOutcome> {
    if let Some(reason) = crate::omml::check_omml_unsupported(mathml) {
        return Ok(OmmlConvertOutcome::Skipped(reason));
    }
    let mut omml = mathml_to_omml(mathml)?;
    if let Some(ref font) = options.math_font {
        apply_math_font(&mut omml, font);
    }
    Ok(OmmlConvertOutcome::Converted(omml))
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

// ── Namespace & Orphan Cleanup ───────────────────────────────────────────────

fn ensure_math_namespace(root: &mut Element) {
    if !root
        .nsdecls
        .iter()
        .any(|(prefix, uri)| prefix.as_deref() == Some("m") || uri == NS_M)
    {
        root.nsdecls.push((
            Some(std::borrow::Cow::Borrowed("m")),
            std::borrow::Cow::Borrowed(NS_M),
        ));
    }
}

fn obsolete_relationship_ids(root: &Element, converted: HashSet<String>) -> HashSet<String> {
    let remaining = collect_referenced_rids(root);
    converted
        .into_iter()
        .filter(|rid| !remaining.contains(rid))
        .collect()
}

/// Collect all `rId*` attribute values still referenced inside an XML tree.
pub(crate) fn collect_referenced_rids(root: &Element) -> HashSet<String> {
    let mut rids = HashSet::new();
    for element in root.iter() {
        for attr in &element.attrs {
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

/// Remove candidate part files that are no longer referenced by any `.rels` part in the package.
pub(crate) fn cleanup_unreferenced_files(
    package: &mut Package,
    candidates: &HashSet<String>,
) -> Result<()> {
    // Collect all targets still referenced across ALL .rels files in the package.
    let mut still_referenced: HashSet<String> = HashSet::new();
    for name in package.part_names() {
        if name.ends_with(".rels") {
            let rels_root = package
                .load_xml(&name)?
                .ok_or_else(|| Error::MissingPart(name.clone()))?;
            let owner_dir = name
                .split("/_rels/")
                .next()
                .filter(|&dir| dir != name)
                .unwrap_or("");
            for element in rels_root.elements() {
                if element.local == "Relationship" {
                    if let Some(target) = element.attr(None, "Target") {
                        if element.attr(None, "TargetMode") != Some("External") {
                            still_referenced.insert(resolve_target_path(owner_dir, target));
                        }
                    }
                }
            }
        }
    }

    let mut removed_parts: HashSet<String> = HashSet::new();
    for candidate in candidates {
        if !still_referenced.contains(candidate) && package.remove(candidate) {
            removed_parts.insert(format!("/{candidate}"));
        }
    }

    if !removed_parts.is_empty() {
        if let Some(mut ct_root) = package.load_xml(CT_PATH)? {
            ct_root.children.retain(|node| match node {
                Node::Element(element) if element.local == "Override" => element
                    .attr(None, "PartName")
                    .is_none_or(|part_name| !removed_parts.contains(part_name)),
                _ => true,
            });
            package.set(CT_PATH, xml::to_bytes(&ct_root));
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
        let mut zip_writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let file_opts = zip::write::SimpleFileOptions::default();
        zip_writer
            .start_file("[Content_Types].xml", file_opts)
            .unwrap();
        zip_writer.write_all(ct.as_bytes()).unwrap();
        zip_writer
            .start_file("word/document.xml", file_opts)
            .unwrap();
        zip_writer.write_all(doc.as_bytes()).unwrap();
        zip_writer.finish().unwrap().into_inner()
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

        let mut zip_writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let file_opts = zip::write::SimpleFileOptions::default();
        zip_writer
            .start_file("[Content_Types].xml", file_opts)
            .unwrap();
        zip_writer.write_all(ct_xml.as_bytes()).unwrap();
        zip_writer
            .start_file("word/document.xml", file_opts)
            .unwrap();
        zip_writer.write_all(doc_xml.as_bytes()).unwrap();
        zip_writer
            .start_file("word/_rels/document.xml.rels", file_opts)
            .unwrap();
        zip_writer.write_all(rels_xml.as_bytes()).unwrap();
        zip_writer
            .start_file("word/media/equation1.emf", file_opts)
            .unwrap();
        zip_writer.write_all(&preview.emf).unwrap();
        let src_docx = zip_writer.finish().unwrap().into_inner();

        let (omml_docx, summary) = convert_bytes_to_omml(&src_docx).unwrap();
        assert_eq!(summary.found, 1);
        assert_eq!(summary.converted, 1);
        assert_eq!(summary.errors, 0);

        let package = Package::read(&omml_docx).unwrap();
        let doc_el = xml::parse(package.get("word/document.xml").unwrap()).unwrap();
        let omaths: Vec<_> = doc_el
            .iter()
            .into_iter()
            .filter(|element| element.is(NS_M, "oMath"))
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
        let package = Package::read(&omml_docx).unwrap();
        assert!(!package.contains("word/embeddings/oleObject1.bin"));
        let doc_el = xml::parse(package.get("word/document.xml").unwrap()).unwrap();
        let omaths: Vec<_> = doc_el
            .iter()
            .into_iter()
            .filter(|element| element.is(NS_M, "oMath"))
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

        let mut zip_writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let file_opts = zip::write::SimpleFileOptions::default();
        zip_writer
            .start_file("[Content_Types].xml", file_opts)
            .unwrap();
        zip_writer.write_all(ct_xml.as_bytes()).unwrap();
        zip_writer
            .start_file("word/document.xml", file_opts)
            .unwrap();
        zip_writer.write_all(doc_xml.as_bytes()).unwrap();
        zip_writer
            .start_file("word/_rels/document.xml.rels", file_opts)
            .unwrap();
        zip_writer.write_all(rels_xml.as_bytes()).unwrap();
        zip_writer
            .start_file("word/embeddings/oleObject1.bin", file_opts)
            .unwrap();
        zip_writer.write_all(&ole1).unwrap();
        zip_writer
            .start_file("word/media/equation1.emf", file_opts)
            .unwrap();
        zip_writer.write_all(&emf1).unwrap();
        zip_writer
            .start_file("word/embeddings/oleObject2.bin", file_opts)
            .unwrap();
        zip_writer.write_all(&ole2).unwrap();
        zip_writer
            .start_file("word/media/equation2.emf", file_opts)
            .unwrap();
        zip_writer.write_all(&emf2).unwrap();
        let src_docx = zip_writer.finish().unwrap().into_inner();

        let (omml_docx, summary) = convert_bytes_to_omml(&src_docx).unwrap();
        assert_eq!(summary.found, 2);
        assert_eq!(summary.converted, 1);
        assert_eq!(summary.skipped, 1);
        assert_eq!(summary.errors, 0);

        let package = Package::read(&omml_docx).unwrap();
        assert!(!package.contains("word/embeddings/oleObject1.bin"));
        assert!(!package.contains("word/media/equation1.emf"));
        assert!(package.contains("word/embeddings/oleObject2.bin"));
        assert!(package.contains("word/media/equation2.emf"));

        let doc_el = xml::parse(package.get("word/document.xml").unwrap()).unwrap();
        let omaths: Vec<_> = doc_el
            .iter()
            .into_iter()
            .filter(|element| element.is(NS_M, "oMath"))
            .collect();
        assert_eq!(omaths.len(), 1);
        assert!(omaths[0].find(NS_M, "f").is_some());
        let oles: Vec<_> = doc_el
            .iter()
            .into_iter()
            .filter(|element| element.local == "OLEObject")
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

        let mut zip_writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let file_opts = zip::write::SimpleFileOptions::default();
        zip_writer
            .start_file("[Content_Types].xml", file_opts)
            .unwrap();
        zip_writer.write_all(ct_xml.as_bytes()).unwrap();
        zip_writer
            .start_file("word/document.xml", file_opts)
            .unwrap();
        zip_writer.write_all(doc_xml.as_bytes()).unwrap();
        zip_writer
            .start_file("word/_rels/document.xml.rels", file_opts)
            .unwrap();
        zip_writer.write_all(rels_xml.as_bytes()).unwrap();
        zip_writer
            .start_file("word/media/equation1.emf", file_opts)
            .unwrap();
        zip_writer.write_all(&preview.emf).unwrap();
        let src_docx = zip_writer.finish().unwrap().into_inner();

        let (omml_docx, summary) = convert_bytes_to_omml(&src_docx).unwrap();
        assert_eq!(summary.found, 1);
        assert_eq!(summary.converted, 0);
        assert_eq!(summary.skipped, 1);
        assert_eq!(summary.errors, 0);

        let package = Package::read(&omml_docx).unwrap();
        assert!(package.contains("word/media/equation1.emf"));
        let doc_el = xml::parse(package.get("word/document.xml").unwrap()).unwrap();
        assert_eq!(
            doc_el
                .iter()
                .into_iter()
                .filter(|element| element.is(NS_M, "oMath"))
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

    #[test]
    fn unreadable_relationships_abort_in_place_cleanup() {
        let src = make_minimal_omml_docx("<m:oMath><m:r><m:t>x</m:t></m:r></m:oMath>");
        let (mt, _) = convert_bytes_with(&src, &Options::default()).unwrap();
        let mut package = Package::read(&mt).unwrap();
        package.set(
            "customXml/_rels/item1.xml.rels",
            b"<Relationships><broken></Relationships>".to_vec(),
        );
        let original = package.write().unwrap();
        let path = std::env::temp_dir().join(format!("eqntide-rels-{}.docx", uuid::Uuid::new_v4()));
        std::fs::write(&path, &original).unwrap();
        assert!(matches!(convert_to_omml(&path, &path), Err(Error::Xml(_))));
        assert_eq!(std::fs::read(&path).unwrap(), original);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn multi_object_run_at_chunk_boundary_keeps_later_sibling_paths() {
        let src = make_minimal_omml_docx("<m:oMath><m:r><m:t>x</m:t></m:r></m:oMath>");
        let (mt, _) = convert_bytes_with(&src, &Options::default()).unwrap();
        let mut package = Package::read(&mt).unwrap();
        let mut root = package.load_xml(DOC_XML_PATH).unwrap().unwrap();
        let run = root
            .iter()
            .into_iter()
            .find(|element| element.is(NS_W, "r") && element.find(NS_W, "object").is_some())
            .unwrap()
            .clone();
        let mut para = Element::new(Some(NS_W), Some("w"), "p");
        for _ in 1..TO_OMML_CHUNK_SIZE {
            para.push(run.clone());
        }
        let mut double = run.clone();
        double.push(run.find(NS_W, "object").unwrap().clone());
        para.push(double);
        let mut link = Element::new(Some(NS_W), Some("w"), "hyperlink");
        link.push(run.clone());
        para.push(link);
        para.push(run);
        let mut body = Element::new(Some(NS_W), Some("w"), "body");
        body.push(para);
        root.children.clear();
        root.push(body);
        package.set(DOC_XML_PATH, xml::to_bytes(&root));

        let (mut out, summary) =
            convert_package_to_omml_with_cancel(package, &ToOmmlOptions::default(), None).unwrap();
        assert_eq!(summary.converted, TO_OMML_CHUNK_SIZE + 3);
        let result = out.load_xml(DOC_XML_PATH).unwrap().unwrap();
        let para = result.find(NS_W, "body").unwrap().find(NS_W, "p").unwrap();
        let mut children = para.elements();
        for _ in 0..=TO_OMML_CHUNK_SIZE {
            assert!(children.next().unwrap().is(NS_M, "oMath"));
        }
        assert!(children.next().unwrap().find(NS_M, "oMath").is_some());
        assert!(children.next().unwrap().is(NS_M, "oMath"));
        assert!(children.next().is_none());
    }

    #[test]
    fn test_chunked_mathtype_to_omml_over_300_equations_matches_bytes_and_cleans_orphans() {
        let mut body = String::new();
        for _ in 0..300 {
            body.push_str("<w:p><m:oMath><m:r><m:t>x</m:t></m:r><m:r><m:t>+</m:t></m:r><m:r><m:t>1</m:t></m:r></m:oMath></w:p>");
        }
        let doc_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math">
  <w:body>{body}</w:body>
</w:document>"#
        );
        let ct_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#;

        let mut zip_writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let file_opts = zip::write::SimpleFileOptions::default();
        zip_writer
            .start_file("[Content_Types].xml", file_opts)
            .unwrap();
        zip_writer.write_all(ct_xml.as_bytes()).unwrap();
        zip_writer
            .start_file("word/document.xml", file_opts)
            .unwrap();
        zip_writer.write_all(doc_xml.as_bytes()).unwrap();
        zip_writer
            .start_file("word/media/untouched.png", file_opts)
            .unwrap();
        zip_writer.write_all(b"keep-image").unwrap();
        let omml_src = zip_writer.finish().unwrap().into_inner();

        let (mt_docx, sum1) = convert_bytes_with(&omml_src, &Options::default()).unwrap();
        assert_eq!(sum1.converted, 300);

        let (bytes_omml, sum_bytes) = convert_bytes_to_omml(&mt_docx).unwrap();
        assert_eq!(sum_bytes.found, 300);
        assert_eq!(sum_bytes.converted, 300);
        assert_eq!(sum_bytes.errors, 0);

        let dir = std::env::temp_dir().join(format!("eqntide_chunked_m2o_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mt_path = dir.join("mt.docx");
        let omml_path = dir.join("omml.docx");
        std::fs::write(&mt_path, &mt_docx).unwrap();

        let sum_file = convert_to_omml(&mt_path, &omml_path).unwrap();
        assert_eq!(sum_file, sum_bytes);

        let pkg_bytes = Package::read(&bytes_omml).unwrap();
        let pkg_file = Package::read(&std::fs::read(&omml_path).unwrap()).unwrap();
        assert_eq!(pkg_bytes.part_names(), pkg_file.part_names());
        assert_eq!(
            pkg_file.get("word/document.xml"),
            pkg_bytes.get("word/document.xml")
        );
        assert_eq!(
            pkg_file.get("word/media/untouched.png").unwrap(),
            b"keep-image"
        );
        assert!(!pkg_file.contains("word/embeddings/oleObject1.bin"));
        assert!(!pkg_file.contains("word/media/equation1.emf"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
