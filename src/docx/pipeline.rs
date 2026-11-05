//! Entry points: convert a `.docx` held in memory or on disk.

use std::collections::{HashSet};
use std::path::Path;

use crate::error::{Error, Result};





use crate::xml::{self};

use super::consts::*;
use super::job::*;
use super::options::*;

use super::package::*;

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

    let doc_root = xml::parse(
        pkg.get(DOC_XML_PATH)
            .ok_or_else(|| Error::MissingPart(DOC_XML_PATH.into()))?,
    )?;
    let rels_root = xml::parse(pkg.get(RELS_PATH).unwrap_or(EMPTY_RELS))?;
    let ct_root = xml::parse(
        pkg.get(CT_PATH)
            .ok_or_else(|| Error::MissingPart(CT_PATH.into()))?,
    )?;

    // Collect all m:oMath elements up front (paths stay valid because each
    // replacement is a 1-for-1 swap in the same child slot).
    let mut paths = Vec::new();
    collect_omath_paths(&doc_root, &mut Vec::new(), &mut paths);
    log::info!("Found {} equation(s)", paths.len());

    // Track which rIds are already used.
    let existing_ids: HashSet<String> = rels_root
        .elements()
        .filter_map(|c| c.attr(None, "Id").map(str::to_owned))
        .collect();

    // Scan existing embeddings to avoid name collisions.
    let mut ole_index = 1;
    while pkg.contains(&format!("word/embeddings/oleObject{ole_index}.bin")) {
        ole_index += 1;
    }

    let mut job = Job {
        doc_root,
        rels_root,
        ct_root,
        pkg,
        existing_ids,
        ole_index,
        img_index: 1,
        opts: *opts,
        summary: ConversionSummary {
            found: paths.len(),
            ..Default::default()
        },
    };

    for (i, path) in paths.iter().enumerate() {
        let eq_num = i + 1;
        // Per-equation error isolation: one bad equation never aborts the file.
        if let Err(e) = job.convert_one(eq_num, path) {
            job.summary.errors += 1;
            log::error!("Equation {eq_num}: {e}");
        }
    }

    // Serialise the modified XML back and write the output package.
    let Job {
        doc_root,
        rels_root,
        ct_root,
        mut pkg,
        summary,
        ..
    } = job;
    pkg.set(DOC_XML_PATH, xml::to_bytes(&doc_root));
    pkg.set(RELS_PATH, xml::to_bytes(&rels_root));
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
    std::fs::write(output_path.as_ref(), output)?;
    log::info!("Saved → {}", output_path.as_ref().display());
    Ok(summary)
}
