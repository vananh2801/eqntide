//! Entry points for converting OMML equations inside a `.docx` to MathType OLE objects.
//!
//! ```text
//! input `.docx` (path or &[u8])
//!   │
//!   ▼
//! Package::open / Package::read
//!   │
//!   ▼
//! convert_package_with_cancel
//!   ├── discover_omml_parts (document.xml, headers, footers, footnotes, endnotes)
//!   ├── Job::convert_part_equations  (OMML ─▶ MathML ─▶ MTEF ─▶ OLE + EMF/PNG)
//!   ├── sanitize_document_baselines
//!   └── write updated XML, rels, and [Content_Types].xml back to Package
//!   │
//!   ▼
//! Package::write / Package::write_to_file_atomic
//! ```

use std::collections::HashSet;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::error::{Error, Result};
use crate::xml;

use super::consts::*;
use super::job::*;
use super::options::*;
use super::package::*;

// ── Public Entry Points ──────────────────────────────────────────────────────

/// Convert every OMML equation in `input_path` and write `output_path`.
///
/// Returns a [`ConversionSummary`] describing how many equations were found,
/// converted, skipped and failed. The output `.docx` is always written, even
/// when some equations fail to convert (failed equations are left as their
/// original OMML).
pub fn convert(
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
) -> Result<ConversionSummary> {
    convert_with(input_path, output_path, &Options::default())
}

/// Like [`convert`], with explicit [`Options`].
pub fn convert_with(
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    options: &Options,
) -> Result<ConversionSummary> {
    convert_with_cancel(input_path, output_path, options, None)
}

/// Like [`convert_with`], supporting cooperative cancellation before writing output.
pub fn convert_with_cancel(
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    options: &Options,
    cancel: Option<&AtomicBool>,
) -> Result<ConversionSummary> {
    if cancel.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
        return Err(Error::Cancelled);
    }
    let package = Package::open(input_path.as_ref())?;
    let (mut package, summary) = convert_package_with_cancel(package, options, cancel)?;
    package.write_to_file_atomic(output_path.as_ref(), cancel)?;
    log::info!("Saved → {}", output_path.as_ref().display());
    Ok(summary)
}

/// Convert every OMML equation in a `.docx` held in memory.
///
/// Returns the new `.docx` bytes and a [`ConversionSummary`]. The output is
/// always produced, even when some equations fail to convert (failed equations
/// are left as their original OMML).
pub fn convert_bytes(input: &[u8]) -> Result<(Vec<u8>, ConversionSummary)> {
    convert_bytes_with(input, &Options::default())
}

/// Like [`convert_bytes`], with explicit [`Options`].
pub fn convert_bytes_with(input: &[u8], options: &Options) -> Result<(Vec<u8>, ConversionSummary)> {
    convert_bytes_with_cancel(input, options, None)
}

/// Like [`convert_bytes_with`], supporting cooperative cancellation.
pub fn convert_bytes_with_cancel(
    input: &[u8],
    options: &Options,
    cancel: Option<&AtomicBool>,
) -> Result<(Vec<u8>, ConversionSummary)> {
    if cancel.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
        return Err(Error::Cancelled);
    }
    let package = Package::read(input)?;
    let (package, summary) = convert_package_with_cancel(package, options, cancel)?;
    Ok((package.write()?, summary))
}

// ── Package Conversion Pipeline ──────────────────────────────────────────────

fn convert_package_with_cancel(
    mut package: Package,
    options: &Options,
    cancel: Option<&AtomicBool>,
) -> Result<(Package, ConversionSummary)> {
    if !package.contains(DOC_XML_PATH) {
        return Err(Error::MissingPart(DOC_XML_PATH.into()));
    }

    let ct_root = package
        .load_xml(CT_PATH)?
        .ok_or_else(|| Error::MissingPart(CT_PATH.into()))?;
    let ct_overrides = collect_override_part_names(&ct_root);

    // Scan existing embeddings to avoid name collisions.
    let mut ole_index = 1;
    while package.contains(&format!("word/embeddings/oleObject{ole_index}.bin")) {
        ole_index += 1;
    }

    let parts = discover_omml_parts(&package);

    let mut job = Job {
        doc_root: xml::Element::new(None, None, "root"),
        rels_root: xml::Element::new(None, None, "root"),
        ct_root,
        ct_overrides,
        pkg: package,
        existing_ids: HashSet::new(),
        next_ole_rid: 1,
        next_img_rid: 1,
        ole_index,
        img_index: 1,
        opts: *options,
        summary: ConversionSummary::default(),
    };

    let mut global_eq_num = 0usize;

    for xml_path in parts {
        if cancel.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            return Err(Error::Cancelled);
        }
        let Some(doc_root) = job.pkg.load_xml(&xml_path)? else {
            continue;
        };
        let mut paths = Vec::new();
        collect_omath_paths(&doc_root, &mut Vec::new(), &mut paths);
        if paths.is_empty() && xml_path != DOC_XML_PATH {
            continue;
        }

        let rels_path = rels_path_for(&xml_path);
        let rels_root = match job.pkg.load_xml(&rels_path)? {
            Some(root) => root,
            None => xml::parse(EMPTY_RELS)?,
        };
        let existing_ids: HashSet<String> = rels_root
            .elements()
            .filter_map(|child| child.attr(None, "Id").map(str::to_owned))
            .collect();

        job.doc_root = doc_root;
        job.rels_root = rels_root;
        job.existing_ids = existing_ids;
        job.next_ole_rid = 1;
        job.next_img_rid = 1;
        job.summary.found += paths.len();

        job.convert_part_equations(&xml_path, &paths, &mut global_eq_num, cancel)?;

        super::ole_xml::sanitize_document_baselines(&mut job.doc_root);
        job.pkg.set(&xml_path, xml::to_bytes(&job.doc_root));
        job.pkg.set(&rels_path, xml::to_bytes(&job.rels_root));

        // Release finished XML DOMs immediately before loading the next part.
        job.doc_root = xml::Element::new(None, None, "root");
        job.rels_root = xml::Element::new(None, None, "root");
    }

    log::info!("Found {} equation(s)", job.summary.found);

    let Job {
        ct_root,
        mut pkg,
        summary,
        ..
    } = job;
    pkg.set(CT_PATH, xml::to_bytes(&ct_root));

    Ok((pkg, summary))
}

fn discover_omml_parts(package: &Package) -> Vec<String> {
    let mut parts = Vec::new();
    if package.contains(DOC_XML_PATH) {
        parts.push(DOC_XML_PATH.to_owned());
    }
    for name in package.part_names() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};

    #[test]
    fn test_omml_to_mathtype_skips_unsupported_omml() {
        let doc_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math">
  <w:body>
    <w:p>
      <m:oMath><m:r><m:t>x</m:t></m:r><m:r><m:t>+</m:t></m:r><m:r><m:t>1</m:t></m:r></m:oMath>
      <m:oMath><m:borderBox><m:borderBoxPr><m:hideTop m:val="1"/><m:hideBot m:val="1"/><m:hideLeft m:val="1"/><m:hideRight m:val="1"/><m:strikeV m:val="1"/></m:borderBoxPr><m:e><m:r><m:t>y</m:t></m:r></m:e></m:borderBox></m:oMath>
    </w:p>
  </w:body>
</w:document>"#;
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
        let src_docx = zip_writer.finish().unwrap().into_inner();

        let (out_docx, summary) = convert_bytes(&src_docx).unwrap();
        assert_eq!(summary.found, 2);
        assert_eq!(summary.converted, 1);
        assert_eq!(summary.skipped, 1);
        assert_eq!(summary.errors, 0);

        let package = Package::read(&out_docx).unwrap();
        assert!(package.contains("word/embeddings/oleObject1.bin"));
        assert!(!package.contains("word/embeddings/oleObject2.bin"));
        let doc_el = xml::parse(package.get("word/document.xml").unwrap()).unwrap();
        assert_eq!(
            doc_el
                .iter()
                .into_iter()
                .filter(|element| element.is(crate::omml::decode::NS_M, "oMath"))
                .count(),
            1
        );
    }

    #[test]
    fn test_parallel_omml_to_mathtype_deterministic_order_and_error_isolation() {
        let valid_eq = r#"<m:oMath><m:r><m:t>x</m:t></m:r><m:r><m:t>+</m:t></m:r><m:r><m:t>1</m:t></m:r></m:oMath>"#;
        let unsupported_eq = r#"<m:oMath><m:borderBox><m:borderBoxPr><m:hideTop m:val="1"/><m:hideBot m:val="1"/><m:hideLeft m:val="1"/><m:hideRight m:val="1"/><m:strikeV m:val="1"/></m:borderBoxPr><m:e><m:r><m:t>y</m:t></m:r></m:e></m:borderBox></m:oMath>"#;
        // 300 wide glyphs at 12pt exceed MAX_WIDTH_PX (20_000) in PNG preview rendering,
        // triggering Error::PreviewTooLarge for just this equation.
        let bad_eq = format!(
            "<m:oMath><m:r><m:t>{}</m:t></m:r></m:oMath>",
            "W".repeat(300)
        );

        let mut body = String::new();
        for _ in 0..5 {
            body.push_str("<w:p>");
            body.push_str(valid_eq);
            body.push_str("</w:p>");
        }
        body.push_str("<w:p>");
        body.push_str(unsupported_eq);
        body.push_str("</w:p>");
        body.push_str("<w:p>");
        body.push_str(&bad_eq);
        body.push_str("</w:p>");
        for _ in 0..5 {
            body.push_str("<w:p>");
            body.push_str(valid_eq);
            body.push_str("</w:p>");
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
        let src_docx = zip_writer.finish().unwrap().into_inner();

        let png_opts = Options {
            preview: PreviewFormat::Png,
            ..Options::default()
        };
        let (out_a, sum_a) = convert_bytes_with(&src_docx, &png_opts).unwrap();
        let (out_b, sum_b) = convert_bytes_with(&src_docx, &png_opts).unwrap();

        assert_eq!(sum_a.found, 12);
        assert_eq!(sum_a.converted, 10);
        assert_eq!(sum_a.skipped, 1);
        assert_eq!(sum_a.errors, 1);
        assert_eq!(sum_a, sum_b);

        let pkg_a = Package::read(&out_a).unwrap();
        let pkg_b = Package::read(&out_b).unwrap();
        for index in 1..=10 {
            let bin = format!("word/embeddings/oleObject{index}.bin");
            let img = format!("word/media/equation{index}.png");
            assert!(pkg_a.contains(&bin), "missing {bin}");
            assert!(pkg_a.contains(&img), "missing {img}");
            assert_eq!(pkg_a.get(&bin), pkg_b.get(&bin));
        }
        assert!(!pkg_a.contains("word/embeddings/oleObject11.bin"));
        assert_eq!(
            pkg_a.get("word/_rels/document.xml.rels"),
            pkg_b.get("word/_rels/document.xml.rels")
        );
    }

    #[test]
    fn test_convert_with_cancel_writes_no_output_file() {
        let dir = std::env::temp_dir().join(format!("eqntide_cancel_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let in_path = dir.join("in.docx");
        let out_path = dir.join("out.docx");

        let mut zip_writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let file_opts = zip::write::SimpleFileOptions::default();
        zip_writer
            .start_file("[Content_Types].xml", file_opts)
            .unwrap();
        zip_writer.write_all(br#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="xml" ContentType="application/xml"/></Types>"#).unwrap();
        zip_writer
            .start_file("word/document.xml", file_opts)
            .unwrap();
        zip_writer.write_all(br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math"><w:body><w:p><m:oMath><m:r><m:t>x</m:t></m:r></m:oMath></w:p></w:body></w:document>"#).unwrap();
        std::fs::write(&in_path, zip_writer.finish().unwrap().into_inner()).unwrap();

        let cancel = AtomicBool::new(true);
        let err = convert_with_cancel(&in_path, &out_path, &Options::default(), Some(&cancel))
            .unwrap_err();
        assert!(matches!(err, Error::Cancelled));
        assert!(!out_path.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_chunked_omml_to_mathtype_over_300_equations_is_deterministic() {
        let valid_eq = r#"<m:oMath><m:r><m:t>x</m:t></m:r><m:r><m:t>+</m:t></m:r><m:r><m:t>1</m:t></m:r></m:oMath>"#;
        let skipped_eq = r#"<m:oMath><m:borderBox><m:borderBoxPr><m:hideTop m:val="1"/><m:hideBot m:val="1"/><m:hideLeft m:val="1"/><m:hideRight m:val="1"/><m:strikeV m:val="1"/></m:borderBoxPr><m:e><m:r><m:t>y</m:t></m:r></m:e></m:borderBox></m:oMath>"#;

        let mut body = String::new();
        for idx in 0..300 {
            body.push_str("<w:p>");
            if idx == 150 {
                body.push_str(skipped_eq);
            } else {
                body.push_str(valid_eq);
            }
            body.push_str("</w:p>");
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
        let header = b"<w:hdr xmlns:w='http://schemas.openxmlformats.org/wordprocessingml/2006/main'><w:p/></w:hdr>";
        zip_writer
            .start_file(
                "word/header1.xml",
                file_opts.compression_method(zip::CompressionMethod::Stored),
            )
            .unwrap();
        zip_writer.write_all(header).unwrap();
        zip_writer
            .start_file("word/media/untouched.png", file_opts)
            .unwrap();
        zip_writer.write_all(b"keep-media").unwrap();
        let src_docx = zip_writer.finish().unwrap().into_inner();

        let (bytes_out, sum_bytes) = convert_bytes(&src_docx).unwrap();
        assert_eq!(sum_bytes.found, 300);
        assert_eq!(sum_bytes.converted, 299);
        assert_eq!(sum_bytes.skipped, 1);
        assert_eq!(sum_bytes.errors, 0);

        let dir = std::env::temp_dir().join(format!("eqntide_chunked_o2m_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let in_path = dir.join("in.docx");
        let out_path = dir.join("out.docx");
        std::fs::write(&in_path, &src_docx).unwrap();

        let sum_file = convert(&in_path, &out_path).unwrap();
        assert_eq!(sum_file, sum_bytes);

        let pkg_bytes = Package::read(&bytes_out).unwrap();
        let pkg_file = Package::read(&std::fs::read(&out_path).unwrap()).unwrap();
        let mut zip_archive =
            zip::ZipArchive::new(std::fs::File::open(&out_path).unwrap()).unwrap();
        assert_eq!(
            zip_archive
                .by_name("word/header1.xml")
                .unwrap()
                .compression(),
            zip::CompressionMethod::Stored
        );
        assert_eq!(pkg_file.get("word/header1.xml"), Some(header.as_slice()));
        drop(zip_archive);
        assert_eq!(pkg_bytes.part_names(), pkg_file.part_names());
        assert_eq!(
            pkg_file.get("word/media/untouched.png").unwrap(),
            b"keep-media"
        );
        for index in 1..=299 {
            let bin = format!("word/embeddings/oleObject{index}.bin");
            let img = format!("word/media/equation{index}.emf");
            assert_eq!(pkg_bytes.get(&bin), pkg_file.get(&bin), "mismatch {bin}");
            assert_eq!(pkg_bytes.get(&img), pkg_file.get(&img), "mismatch {img}");
        }
        assert!(!pkg_file.contains("word/embeddings/oleObject300.bin"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
