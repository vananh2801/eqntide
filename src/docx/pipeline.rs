//! Entry points: convert a `.docx` held in memory or on disk.

use std::collections::HashSet;
use std::path::Path;

use crate::error::{Error, Result};
use crate::xml;

use super::consts::*;
use super::job::*;
use super::options::*;
use super::package::*;

fn discover_omml_parts(pkg: &Package) -> Vec<String> {
    let mut parts = Vec::new();
    if pkg.contains(DOC_XML_PATH) {
        parts.push(DOC_XML_PATH.to_owned());
    }
    for name in pkg.part_names() {
        if name == DOC_XML_PATH {
            continue;
        }
        if let Some(rest) = name.strip_prefix("word/") {
            if !rest.contains('/')
                && rest.ends_with(".xml")
                && (rest.starts_with("header")
                    || rest.starts_with("footer")
                    || rest == "footnotes.xml"
                    || rest == "endnotes.xml")
            {
                parts.push(name);
            }
        }
    }
    parts
}

fn rels_path_for(xml_path: &str) -> String {
    match xml_path.rsplit_once('/') {
        Some((dir, file)) => format!("{dir}/_rels/{file}.rels"),
        None => format!("_rels/{xml_path}.rels"),
    }
}

// ── Main conversion pipeline ──────────────────────────────────────────────────

/// Convert every OMML equation in a `.docx` held in memory.
///
/// Returns the new `.docx` bytes and a [`ConversionSummary`]. The output is
/// always produced, even when some equations fail to convert (failed equations
/// are left as their original OMML).
pub fn convert_bytes(input: &[u8]) -> Result<(Vec<u8>, ConversionSummary)> {
    convert_bytes_with(input, &Options::default())
}

/// Like [`convert_bytes`], with explicit [`Options`].
pub fn convert_bytes_with(input: &[u8], opts: &Options) -> Result<(Vec<u8>, ConversionSummary)> {
    let pkg = Package::read(input)?;
    if !pkg.contains(DOC_XML_PATH) {
        return Err(Error::MissingPart(DOC_XML_PATH.into()));
    }

    let ct_root = xml::parse(
        pkg.get(CT_PATH)
            .ok_or_else(|| Error::MissingPart(CT_PATH.into()))?,
    )?;

    // Scan existing embeddings to avoid name collisions.
    let mut ole_index = 1;
    while pkg.contains(&format!("word/embeddings/oleObject{ole_index}.bin")) {
        ole_index += 1;
    }

    let parts = discover_omml_parts(&pkg);
    let dummy_root = xml::parse(b"<root/>")?;

    let mut job = Job {
        doc_root: dummy_root.clone(),
        rels_root: dummy_root,
        ct_root,
        pkg,
        existing_ids: HashSet::new(),
        ole_index,
        img_index: 1,
        opts: *opts,
        summary: ConversionSummary::default(),
    };

    let mut global_eq_num = 0usize;

    for xml_path in parts {
        let Some(xml_bytes) = job.pkg.get(&xml_path) else {
            continue;
        };
        let doc_root = xml::parse(xml_bytes)?;
        let mut paths = Vec::new();
        collect_omath_paths(&doc_root, &mut Vec::new(), &mut paths);
        if paths.is_empty() && xml_path != DOC_XML_PATH {
            continue;
        }

        let rels_path = rels_path_for(&xml_path);
        let rels_root = xml::parse(job.pkg.get(&rels_path).unwrap_or(EMPTY_RELS))?;
        let existing_ids: HashSet<String> = rels_root
            .elements()
            .filter_map(|c| c.attr(None, "Id").map(str::to_owned))
            .collect();

        job.doc_root = doc_root;
        job.rels_root = rels_root;
        job.existing_ids = existing_ids;
        job.summary.found += paths.len();

        for path in &paths {
            global_eq_num += 1;
            // Per-equation error isolation: one bad equation never aborts the file.
            if let Err(e) = job.convert_one(global_eq_num, path) {
                job.summary.errors += 1;
                log::error!("Equation {global_eq_num} ({xml_path}): {e}");
            }
        }

        super::ole_xml::sanitize_document_baselines(&mut job.doc_root);
        job.pkg.set(&xml_path, xml::to_bytes(&job.doc_root));
        job.pkg.set(&rels_path, xml::to_bytes(&job.rels_root));
    }

    log::info!("Found {} equation(s)", job.summary.found);

    let Job {
        ct_root,
        mut pkg,
        summary,
        ..
    } = job;
    pkg.set(CT_PATH, xml::to_bytes(&ct_root));

    Ok((pkg.write()?, summary))
}

/// Convert every OMML equation in `input_path` and write `output_path`.
///
/// Returns a [`ConversionSummary`] describing how many equations were found,
/// converted, skipped and failed. The output `.docx` is always written, even
/// when some equations fail to convert (failed equations are left as their
/// original OMML).
pub fn convert(input_path: impl AsRef<Path>, output_path: impl AsRef<Path>) -> Result<ConversionSummary> {
    convert_with(input_path, output_path, &Options::default())
}

/// Like [`convert`], with explicit [`Options`].
pub fn convert_with(
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    opts: &Options,
) -> Result<ConversionSummary> {
    let input = std::fs::read(input_path.as_ref())?;
    let (output, summary) = convert_bytes_with(&input, opts)?;
    let out_path = output_path.as_ref();
    if let Some(parent) = out_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(out_path, output)?;
    log::info!("Saved → {}", out_path.display());
    Ok(summary)
}
