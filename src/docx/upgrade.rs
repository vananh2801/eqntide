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
use std::sync::atomic::{AtomicBool, Ordering};

use rayon::prelude::*;

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
use super::job::{element_at, element_at_mut};
use super::ole_xml::{make_object_run, sanitize_document_baselines};
use super::options::{ConversionSummary, ToOmmlOptions, UpgradeOptions, UpgradeTarget};
use super::package::{
    add_relationship, collect_override_part_names, ensure_content_type_indexed,
    ensure_default_extension, Package, PartLookup,
};
use super::to_omml::{
    build_rel_map, cleanup_unreferenced_files, collect_imagedata_rids, collect_referenced_rids,
    convert_bytes_to_omml_with_cancel, convert_to_omml_with_cancel, discover_content_parts,
    find_mathtype_ole_in_subtree, get_r_attr, rels_path_for_part,
};

const UPGRADE_CHUNK_SIZE: usize = 128;

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
    upgrade_legacy_mathtype_docx_with_cancel(input_path, output_path, opts, None)
}

/// Like [`upgrade_legacy_mathtype_docx_with`], supporting cooperative cancellation before writing output.
pub fn upgrade_legacy_mathtype_docx_with_cancel(
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    opts: &UpgradeOptions,
    cancel: Option<&AtomicBool>,
) -> Result<ConversionSummary> {
    if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
        return Err(Error::Cancelled);
    }
    if opts.target == UpgradeTarget::ModernOmml {
        return convert_to_omml_with_cancel(
            input_path,
            output_path,
            &ToOmmlOptions {
                math_font: opts.math_font.clone(),
            },
            cancel,
        );
    }
    let pkg = Package::open(input_path.as_ref())?;
    let (mut pkg, summary) = upgrade_package_with_cancel(pkg, opts, cancel)?;
    pkg.write_to_file_atomic(output_path.as_ref(), cancel)?;
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
    upgrade_legacy_mathtype_bytes_with_cancel(input, opts, None)
}

/// Like [`upgrade_legacy_mathtype_bytes_with`], supporting cooperative cancellation.
pub fn upgrade_legacy_mathtype_bytes_with_cancel(
    input: &[u8],
    opts: &UpgradeOptions,
    cancel: Option<&AtomicBool>,
) -> Result<(Vec<u8>, ConversionSummary)> {
    if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
        return Err(Error::Cancelled);
    }
    if opts.target == UpgradeTarget::ModernOmml {
        return convert_bytes_to_omml_with_cancel(
            input,
            &ToOmmlOptions {
                math_font: opts.math_font.clone(),
            },
            cancel,
        );
    }

    let pkg = Package::read(input)?;
    let (pkg, summary) = upgrade_package_with_cancel(pkg, opts, cancel)?;
    Ok((pkg.write()?, summary))
}

fn upgrade_package_with_cancel(
    mut pkg: Package,
    opts: &UpgradeOptions,
    cancel: Option<&AtomicBool>,
) -> Result<(Package, ConversionSummary)> {
    if !pkg.contains(DOC_XML_PATH) {
        return Err(Error::MissingPart(DOC_XML_PATH.into()));
    }

    let mut ct_root = pkg
        .load_xml(CT_PATH)?
        .ok_or_else(|| Error::MissingPart(CT_PATH.into()))?;
    let mut ct_overrides = collect_override_part_names(&ct_root);

    let (mut ole_index, mut img_index) = next_package_part_indices(&pkg);

    let content_parts = discover_content_parts(&pkg);
    let mut summary = ConversionSummary::default();
    let mut candidate_files_to_remove: HashSet<String> = HashSet::new();

    for xml_path in content_parts {
        if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
            return Err(Error::Cancelled);
        }
        upgrade_content_part(
            &mut pkg,
            &xml_path,
            &mut ct_root,
            &mut ct_overrides,
            &mut ole_index,
            &mut img_index,
            opts,
            &mut summary,
            &mut candidate_files_to_remove,
            cancel,
        )?;
    }

    if !candidate_files_to_remove.is_empty() {
        cleanup_unreferenced_files(&mut pkg, &candidate_files_to_remove)?;
    }

    if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
        return Err(Error::Cancelled);
    }
    pkg.set(CT_PATH, xml::to_bytes(&ct_root));
    Ok((pkg, summary))
}

#[allow(clippy::too_many_arguments)]
fn upgrade_content_part(
    pkg: &mut Package,
    xml_path: &str,
    ct_root: &mut Element,
    ct_overrides: &mut HashSet<String>,
    ole_index: &mut usize,
    img_index: &mut usize,
    opts: &UpgradeOptions,
    summary: &mut ConversionSummary,
    candidate_files_to_remove: &mut HashSet<String>,
    cancel: Option<&AtomicBool>,
) -> Result<()> {
    let Some(mut doc_root) = pkg.load_xml(xml_path)? else {
        return Ok(());
    };
    let rels_path = rels_path_for_part(xml_path);
    let rels_opt = pkg.load_xml(&rels_path)?;
    let rel_map = rels_opt
        .as_ref()
        .map(|rels| build_rel_map(rels, xml_path))
        .unwrap_or_default();
    let mut rels_root = rels_opt.unwrap_or_else(default_relationships_root);
    let mut existing_ids = rels_root
        .elements()
        .filter_map(|child| child.attr(None, "Id").map(str::to_owned))
        .collect();
    let before_converted = summary.converted;
    let mut converted_rids = HashSet::new();
    let mut next_ole_rid = 1usize;
    let mut next_img_rid = 1usize;

    let mut items = Vec::new();
    collect_upgrade_items(&doc_root, &mut Vec::new(), &mut items);

    for chunk in items.chunks(UPGRADE_CHUNK_SIZE) {
        if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
            return Err(Error::Cancelled);
        }

        let part_paths: Vec<String> = {
            let chunk_refs: Vec<(&UpgradeWorkRef, &Element)> = chunk
                .iter()
                .filter_map(|w| element_at(&doc_root, &w.path).map(|el| (w, el)))
                .collect();
            collect_upgrade_chunk_part_paths(&chunk_refs, &rel_map)
                .into_iter()
                .map(str::to_owned)
                .collect()
        };
        let chunk_cache = pkg.load_transient_batch(part_paths.iter().map(String::as_str))?;

        let prepared: Vec<Result<UpgradePrepared>> = {
            let chunk_refs: Vec<(&UpgradeWorkRef, &Element)> = chunk
                .iter()
                .filter_map(|w| element_at(&doc_root, &w.path).map(|el| (w, el)))
                .collect();
            let lookup = PartLookup {
                pkg,
                cache: &chunk_cache,
            };
            if chunk_refs.len() <= 1 {
                chunk_refs
                    .iter()
                    .map(|(w, el)| {
                        prepare_upgrade_item(w.kind, el, &lookup, &rel_map, opts.font_pt, cancel)
                    })
                    .collect()
            } else {
                chunk_refs
                    .par_iter()
                    .map(|(w, el)| {
                        prepare_upgrade_item(w.kind, el, &lookup, &rel_map, opts.font_pt, cancel)
                    })
                    .collect()
            }
        };
        drop(chunk_cache);

        if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
            return Err(Error::Cancelled);
        }

        for (item, res) in chunk.iter().zip(prepared) {
            let prep = res?;
            match prep {
                UpgradePrepared::NotAnEquation => {}
                UpgradePrepared::Skipped {
                    reason,
                    extra_context,
                } => {
                    summary.found += 1;
                    let eq_num = summary.found;
                    summary.skipped += 1;
                    if let Some(ctx) = extra_context {
                        log::warn!(
                            "Equation {eq_num}: {reason} is not supported in MathType ({ctx}), skipping"
                        );
                    } else {
                        log::warn!(
                            "Equation {eq_num}: {reason} is not supported in MathType, skipping"
                        );
                    }
                }
                UpgradePrepared::Failed { err } => {
                    summary.found += 1;
                    let eq_num = summary.found;
                    summary.errors += 1;
                    log::error!("Equation {eq_num}: {err}");
                }
                UpgradePrepared::Ready {
                    ole_bin,
                    preview,
                    rids_to_retire,
                    log_label,
                } => {
                    summary.found += 1;
                    let eq_num = summary.found;
                    let mut ctx = UpgradeAssembleCtx {
                        pkg,
                        rels_root: &mut rels_root,
                        ct_root,
                        ct_overrides,
                        existing_ids: &mut existing_ids,
                        next_ole_rid: &mut next_ole_rid,
                        next_img_rid: &mut next_img_rid,
                        ole_index,
                        img_index,
                    };
                    let modern_run = assemble_modern_ole_run(ole_bin, preview, eq_num, &mut ctx);
                    if let Some(slot_el) = element_at_mut(&mut doc_root, &item.path) {
                        *slot_el = modern_run;
                    }
                    converted_rids.extend(rids_to_retire);
                    summary.converted += 1;
                    log::info!("Equation {eq_num}: {log_label}");
                }
            }
        }
    }

    sanitize_document_baselines(&mut doc_root);
    if summary.converted == before_converted {
        return Ok(());
    }

    pkg.set(xml_path, xml::to_bytes(&doc_root));
    let dead_rids = obsolete_relationship_ids(&doc_root, converted_rids);
    for rid in &dead_rids {
        if let Some(target_zip_path) = rel_map.get(rid) {
            candidate_files_to_remove.insert(target_zip_path.clone());
        }
    }
    if !dead_rids.is_empty() {
        rels_root.children.retain(|node| match node {
            Node::Element(element) if element.local == "Relationship" => element
                .attr(None, "Id")
                .is_none_or(|id| !dead_rids.contains(id)),
            _ => true,
        });
    }
    pkg.set(&rels_path, xml::to_bytes(&rels_root));
    Ok(())
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

fn next_unique_id_from(existing: &mut HashSet<String>, prefix: &str, cursor: &mut usize) -> String {
    loop {
        let id = format!("{prefix}{}", *cursor);
        *cursor += 1;
        if existing.insert(id.clone()) {
            return id;
        }
    }
}

#[cfg(test)]
fn next_unique_id(existing: &mut HashSet<String>, prefix: &str) -> String {
    let mut cursor = 1;
    next_unique_id_from(existing, prefix, &mut cursor)
}

fn convert_ole_rid_to_mathml(
    rid: &str,
    image_ids: &[String],
    pkg: &PartLookup<'_>,
    rel_map: &HashMap<String, String>,
) -> Result<MathNode> {
    if let Some(math) = super::to_omml::preferred_metafile_math(image_ids, rid, pkg, rel_map) {
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

fn mtef_from_metafile(path: &str, bytes: &[u8]) -> Option<Vec<u8>> {
    if path.to_ascii_lowercase().ends_with(".wmf") {
        extract_mtef_from_wmf(bytes).or_else(|| extract_mtef_from_emf(bytes))
    } else if path.to_ascii_lowercase().ends_with(".emf") {
        extract_mtef_from_emf(bytes).or_else(|| extract_mtef_from_wmf(bytes))
    } else {
        None
    }
}

struct UpgradeAssembleCtx<'a> {
    pkg: &'a mut Package,
    rels_root: &'a mut Element,
    ct_root: &'a mut Element,
    ct_overrides: &'a mut HashSet<String>,
    existing_ids: &'a mut HashSet<String>,
    next_ole_rid: &'a mut usize,
    next_img_rid: &'a mut usize,
    ole_index: &'a mut usize,
    img_index: &'a mut usize,
}

#[derive(Clone, Copy)]
enum UpgradeWorkKind {
    AlternateContent,
    OleRun,
    MetafileRun,
}

struct UpgradeWorkRef {
    path: Vec<usize>,
    kind: UpgradeWorkKind,
}

enum UpgradePrepared {
    NotAnEquation,
    Skipped {
        reason: &'static str,
        extra_context: Option<String>,
    },
    Failed {
        err: Error,
    },
    Ready {
        ole_bin: Vec<u8>,
        preview: crate::render::emf::EmfPreview,
        rids_to_retire: Vec<String>,
        log_label: String,
    },
}

fn collect_upgrade_items(el: &Element, path: &mut Vec<usize>, out: &mut Vec<UpgradeWorkRef>) {
    for (i, child) in el.children.iter().enumerate() {
        let Node::Element(c) = child else {
            continue;
        };
        path.push(i);
        if (c.is(NS_MC, "AlternateContent") || c.local == "AlternateContent")
            && !find_mathtype_ole_in_subtree(c).is_empty()
        {
            out.push(UpgradeWorkRef {
                path: path.clone(),
                kind: UpgradeWorkKind::AlternateContent,
            });
            path.pop();
            continue;
        }
        if c.is(NS_W, "r") {
            if !find_mathtype_ole_in_subtree(c).is_empty() {
                out.push(UpgradeWorkRef {
                    path: path.clone(),
                    kind: UpgradeWorkKind::OleRun,
                });
                path.pop();
                continue;
            }
            if !collect_imagedata_rids(c).is_empty() {
                out.push(UpgradeWorkRef {
                    path: path.clone(),
                    kind: UpgradeWorkKind::MetafileRun,
                });
                path.pop();
                continue;
            }
        }
        collect_upgrade_items(c, path, out);
        path.pop();
    }
}

fn collect_upgrade_chunk_part_paths<'a>(
    items: &[(&UpgradeWorkRef, &Element)],
    rel_map: &'a HashMap<String, String>,
) -> Vec<&'a str> {
    let mut paths = Vec::new();
    for (work, el) in items {
        match work.kind {
            UpgradeWorkKind::AlternateContent | UpgradeWorkKind::OleRun => {
                for ole in find_mathtype_ole_in_subtree(el) {
                    if let Some(rid) = get_r_attr(ole, "id") {
                        if let Some(p) = rel_map.get(rid) {
                            paths.push(p.as_str());
                        }
                    }
                }
                for rid in collect_imagedata_rids(el) {
                    if let Some(p) = rel_map.get(&rid) {
                        paths.push(p.as_str());
                    }
                }
            }
            UpgradeWorkKind::MetafileRun => {
                for rid in collect_imagedata_rids(el) {
                    if let Some(p) = rel_map.get(&rid) {
                        let lower = p.to_ascii_lowercase();
                        if lower.ends_with(".wmf") || lower.ends_with(".emf") {
                            paths.push(p.as_str());
                        }
                    }
                }
            }
        }
    }
    paths
}

fn compute_modern_ole_payload(
    mathml: &MathNode,
    font_pt: f32,
) -> Result<(Vec<u8>, crate::render::emf::EmfPreview)> {
    let eq_bytes = mathml_to_mtef_bytes(mathml)?;
    if eq_bytes.is_empty() {
        return Err(Error::InvalidMtef("empty equation content".into()));
    }
    let native_stream = make_equation_native(&eq_bytes);
    let ole_bin = make_ole(&native_stream)?;
    let complete_mtef = crate::mtef::build_mtef(&eq_bytes);
    let pv = render_emf_with_mtef(mathml, font_pt, Some(&complete_mtef))?;
    Ok((ole_bin, pv))
}

fn prepare_upgrade_item(
    kind: UpgradeWorkKind,
    el: &Element,
    pkg: &PartLookup<'_>,
    rel_map: &HashMap<String, String>,
    font_pt: f32,
    cancel: Option<&AtomicBool>,
) -> Result<UpgradePrepared> {
    if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
        return Err(Error::Cancelled);
    }

    match kind {
        UpgradeWorkKind::AlternateContent => {
            let ole_nodes = find_mathtype_ole_in_subtree(el);
            let ole_rid = ole_nodes[0]
                .attr(Some(NS_R), "id")
                .or_else(|| ole_nodes[0].attr(None, "id"));
            let img_rids = collect_imagedata_rids(el);

            let mathml_opt = match ole_rid {
                Some(rid) => convert_ole_rid_to_mathml(rid, &img_rids, pkg, rel_map).ok(),
                None => None,
            }
            .or_else(|| {
                el.find(NS_MC, "Fallback")
                    .and_then(|fb| fb.find(NS_M, "oMath"))
                    .and_then(|omath| crate::omml::decode::omml_to_mathml(omath).ok())
            });

            if let Some(ref mml) = mathml_opt {
                if let Some(reason) = crate::mtef::check_mathtype_unsupported(mml) {
                    return Ok(UpgradePrepared::Skipped {
                        reason,
                        extra_context: None,
                    });
                }
            }

            let payload_res = match mathml_opt {
                Some(mml) => compute_modern_ole_payload(&mml, font_pt),
                None => Err(Error::InvalidMtef(
                    "failed to decode AlternateContent formula".into(),
                )),
            };

            match payload_res {
                Ok((ole_bin, preview)) => {
                    let mut rids_to_retire = img_rids;
                    if let Some(rid) = ole_rid {
                        rids_to_retire.push(rid.to_owned());
                    }
                    Ok(UpgradePrepared::Ready {
                        ole_bin,
                        preview,
                        rids_to_retire,
                        log_label: "upgraded AlternateContent → Equation.DSMT4 OLE".into(),
                    })
                }
                Err(err) => Ok(UpgradePrepared::Failed { err }),
            }
        }
        UpgradeWorkKind::OleRun => {
            let oles = find_mathtype_ole_in_subtree(el);
            let ole_rid = oles.first().and_then(|o| get_r_attr(o, "id"));
            let img_rids = collect_imagedata_rids(el);

            let mathml_res = match ole_rid {
                Some(rid) => convert_ole_rid_to_mathml(rid, &img_rids, pkg, rel_map),
                None => Err(Error::InvalidOle("OLEObject missing r:id".into())),
            };
            let mml = match mathml_res {
                Ok(mml) => mml,
                Err(err) => return Ok(UpgradePrepared::Failed { err }),
            };
            if let Some(reason) = crate::mtef::check_mathtype_unsupported(&mml) {
                return Ok(UpgradePrepared::Skipped {
                    reason,
                    extra_context: None,
                });
            }
            match compute_modern_ole_payload(&mml, font_pt) {
                Ok((ole_bin, preview)) => {
                    let mut rids_to_retire = img_rids;
                    if let Some(rid) = ole_rid {
                        rids_to_retire.push(rid.to_owned());
                    }
                    Ok(UpgradePrepared::Ready {
                        ole_bin,
                        preview,
                        rids_to_retire,
                        log_label: "upgraded OLE → Equation.DSMT4".into(),
                    })
                }
                Err(err) => Ok(UpgradePrepared::Failed { err }),
            }
        }
        UpgradeWorkKind::MetafileRun => {
            let img_rids = collect_imagedata_rids(el);
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
                let Some(mtef_bytes) = mtef_from_metafile(target_path, img_bytes) else {
                    continue;
                };

                let kind_str = if is_wmf { "WMF" } else { "EMF" };
                let mathml_opt = mtef_to_mathml(&mtef_bytes);
                if let Some(ref mml) = mathml_opt {
                    if let Some(reason) = crate::mtef::check_mathtype_unsupported(mml) {
                        return Ok(UpgradePrepared::Skipped {
                            reason,
                            extra_context: Some(format!("{kind_str} {rid}")),
                        });
                    }
                }
                let payload_res = match mathml_opt {
                    Some(mml) => compute_modern_ole_payload(&mml, font_pt),
                    None => Err(Error::InvalidMtef(format!(
                        "failed to decode {kind_str} MTEF in {target_path}"
                    ))),
                };
                return match payload_res {
                    Ok((ole_bin, preview)) => Ok(UpgradePrepared::Ready {
                        ole_bin,
                        preview,
                        rids_to_retire: vec![rid.clone()],
                        log_label: format!(
                            "upgraded standalone {kind_str} ({rid}) → Equation.DSMT4 OLE"
                        ),
                    }),
                    Err(err) => Ok(UpgradePrepared::Failed { err }),
                };
            }
            Ok(UpgradePrepared::NotAnEquation)
        }
    }
}

fn assemble_modern_ole_run(
    ole_bin: Vec<u8>,
    pv: crate::render::emf::EmfPreview,
    eq_num: usize,
    ctx: &mut UpgradeAssembleCtx<'_>,
) -> Element {
    let mut bin_name = format!("word/embeddings/oleObject{}.bin", *ctx.ole_index);
    while ctx.pkg.contains(&bin_name) {
        *ctx.ole_index += 1;
        bin_name = format!("word/embeddings/oleObject{}.bin", *ctx.ole_index);
    }
    *ctx.ole_index += 1;
    ctx.pkg.set(&bin_name, ole_bin);
    let bin_target = format!("embeddings/oleObject{}.bin", *ctx.ole_index - 1);

    let mut img_name = format!("word/media/equation{}.emf", *ctx.img_index);
    while ctx.pkg.contains(&img_name) {
        *ctx.img_index += 1;
        img_name = format!("word/media/equation{}.emf", *ctx.img_index);
    }
    *ctx.img_index += 1;
    ctx.pkg.set(&img_name, pv.emf);
    let img_target = format!("media/equation{}.emf", *ctx.img_index - 1);

    let ole_rid = next_unique_id_from(ctx.existing_ids, "rIdOle", ctx.next_ole_rid);
    add_relationship(ctx.rels_root, &ole_rid, REL_OLEOBJECT, &bin_target);

    let img_rid = next_unique_id_from(ctx.existing_ids, "rIdImg", ctx.next_img_rid);
    add_relationship(ctx.rels_root, &img_rid, REL_IMAGE, &img_target);

    ensure_content_type_indexed(
        ctx.ct_root,
        ctx.ct_overrides,
        &format!("/{bin_name}"),
        CT_OLEOBJECT,
    );
    ensure_default_extension(ctx.ct_root, "emf", "image/x-emf");

    let shape_id = format!("_x0000_i{}", 1024 + eq_num);
    let obj_id = format!(
        "_{}",
        1_000_000_000u128 + uuid::Uuid::new_v4().as_u128() % 9_000_000_000u128
    );
    make_object_run(
        &ole_rid,
        &img_rid,
        &shape_id,
        &obj_id,
        pv.width_pt,
        pv.height_pt,
        pv.depth_pt,
    )
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
        let opts = zip::write::SimpleFileOptions::default();
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
            let opts = zip::write::SimpleFileOptions::default();
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

    #[test]
    fn test_upgrade_to_mathtype_skips_unsupported_constructs() {
        let mut node =
            crate::service::parse_latex_to_mathml(r"\enclose{verticalstrike}{x}").unwrap();
        // Inject a fake MTEF stream containing tmSTRIKE with verticalstrike? No, MTEF lacks verticalstrike;
        // build an OLE with AlternateContent fallback or EMF+MathML? Wait: MTEF cannot represent verticalstrike,
        // so test AlternateContent where Fallback is OMML with strikeV!
        let _ = &mut node;
        let doc_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math"
            xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"
            xmlns:o="urn:schemas-microsoft-com:office:office"
            xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <w:body><w:p>
    <mc:AlternateContent>
      <mc:Choice Requires="wpg"><w:r><w:object><o:OLEObject Type="Embed" ProgID="Equation.3" r:id="rIdMissing"/></w:object></w:r></mc:Choice>
      <mc:Fallback><m:oMath><m:borderBox><m:borderBoxPr><m:hideTop m:val="1"/><m:hideBot m:val="1"/><m:hideLeft m:val="1"/><m:hideRight m:val="1"/><m:strikeV m:val="1"/></m:borderBoxPr><m:e><m:r><m:t>x</m:t></m:r></m:e></m:borderBox></m:oMath></mc:Fallback>
    </mc:AlternateContent>
  </w:p></w:body>
</w:document>"#;
        let ct_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#;

        let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let opts = zip::write::SimpleFileOptions::default();
        w.start_file("[Content_Types].xml", opts).unwrap();
        w.write_all(ct_xml.as_bytes()).unwrap();
        w.start_file("word/document.xml", opts).unwrap();
        w.write_all(doc_xml.as_bytes()).unwrap();
        let src_docx = w.finish().unwrap().into_inner();

        let (out_docx, summary) = upgrade_legacy_mathtype_bytes(&src_docx).unwrap();
        assert_eq!(summary.found, 1);
        assert_eq!(summary.converted, 0);
        assert_eq!(summary.skipped, 1);
        assert_eq!(summary.errors, 0);

        let pkg = Package::read(&out_docx).unwrap();
        assert!(!pkg.contains("word/embeddings/oleObject1.bin"));
    }

    #[test]
    fn test_chunked_upgrade_over_200_equations_matches_bytes_output() {
        let mut body = String::new();
        for _ in 0..200 {
            body.push_str("<w:p><m:oMath><m:r><m:t>x</m:t></m:r><m:r><m:t>+</m:t></m:r><m:r><m:t>2</m:t></m:r></m:oMath></w:p>");
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

        let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let opts = zip::write::SimpleFileOptions::default();
        w.start_file("[Content_Types].xml", opts).unwrap();
        w.write_all(ct_xml.as_bytes()).unwrap();
        w.start_file("word/document.xml", opts).unwrap();
        w.write_all(doc_xml.as_bytes()).unwrap();
        let omml_src = w.finish().unwrap().into_inner();

        let (mt_src, _) = crate::docx::convert_bytes(&omml_src).unwrap();
        let (bytes_upg, sum_bytes) = upgrade_legacy_mathtype_bytes(&mt_src).unwrap();
        assert_eq!(sum_bytes.found, 200);
        assert_eq!(sum_bytes.converted, 200);

        let dir = std::env::temp_dir().join(format!("eqntide_chunked_upg_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let in_path = dir.join("mt.docx");
        let out_path = dir.join("upg.docx");
        std::fs::write(&in_path, &mt_src).unwrap();

        let sum_file = upgrade_legacy_mathtype_docx(&in_path, &out_path).unwrap();
        assert_eq!(sum_file, sum_bytes);

        let pkg_bytes = Package::read(&bytes_upg).unwrap();
        let pkg_file = Package::read(&std::fs::read(&out_path).unwrap()).unwrap();
        assert_eq!(pkg_bytes.part_names(), pkg_file.part_names());
        for i in 1..=200 {
            let bin = format!("word/embeddings/oleObject{i}.bin");
            let img = format!("word/media/equation{i}.emf");
            assert_eq!(pkg_bytes.get(&bin), pkg_file.get(&bin), "mismatch {bin}");
            assert_eq!(pkg_bytes.get(&img), pkg_file.get(&img), "mismatch {img}");
        }

        let _ = std::fs::remove_dir_all(&dir);
    }
}
