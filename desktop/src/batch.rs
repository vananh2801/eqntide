use crate::{Event, EventSender as Sender};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

pub const TARGETS: &[(&str, &str)] = &[
    ("omml", "MathType → OMML"),
    ("mathtype", "OMML → MathType"),
    ("upgrade_omml", "Legacy → modern OMML"),
    ("upgrade_mathtype", "Legacy → modern MathType"),
];

pub fn output_path(input: &Path, target: &str) -> PathBuf {
    let mut name = input.file_stem().unwrap_or_default().to_os_string();
    name.push(format!("_{target}.docx"));
    input.with_file_name(name)
}
/// Convert into a temporary sibling before committing, preserving inputs on failure.
pub fn convert_one(
    input: &Path,
    target: &str,
    font_pt: f32,
    overwrite: bool,
) -> Result<eqntide::bridge::ConvertDocumentResponse, String> {
    let output = if overwrite {
        input.to_path_buf()
    } else {
        output_path(input, target)
    };
    if !overwrite && output.exists() {
        return Err(format!("Output already exists: {}", output.display()));
    }
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let temp = tempfile::Builder::new()
        .prefix(".eqntide-")
        .suffix(".docx")
        .tempfile_in(parent)
        .map_err(|e| e.to_string())?;
    let mut response =
        eqntide::service::convert_document(&eqntide::bridge::ConvertDocumentRequest {
            input_path: input.to_string_lossy().into_owned(),
            output_path: Some(temp.path().to_string_lossy().into_owned()),
            target: target.into(),
            preview: "emf".into(),
            font_pt,
        })
        .map_err(|e| e.to_string())?;
    if overwrite {
        temp.persist(&output).map_err(|e| e.to_string())?;
    } else {
        temp.persist_noclobber(&output).map_err(|e| e.to_string())?;
    }
    response.output_path = output.to_string_lossy().into_owned();
    Ok(response)
}
pub fn run(
    files: Vec<PathBuf>,
    target: String,
    font_pt: f32,
    overwrite: bool,
    stop: Arc<AtomicBool>,
    tx: Sender<Event>,
) {
    std::thread::spawn(move || {
        for (i, input) in files.iter().enumerate() {
            if stop.load(Ordering::Relaxed) {
                break;
            }
            let _ = tx.send(Event::Batch(
                i,
                convert_one(input, &target, font_pt, overwrite),
            ));
        }
        let _ = tx.send(Event::BatchDone);
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode_output_is_distinct_and_keeps_directory() {
        let input = Path::new("thư mục/bài tập.docx");
        assert_eq!(
            output_path(input, "omml"),
            Path::new("thư mục/bài tập_omml.docx")
        );
    }
    #[test]
    fn failed_conversion_never_replaces_original_or_leaves_temporary_files() {
        let directory = tempfile::tempdir().unwrap();
        let input = directory.path().join("bài.docx");
        std::fs::write(&input, b"not a docx").unwrap();
        assert!(convert_one(&input, "omml", 12.0, true).is_err());
        assert_eq!(std::fs::read(&input).unwrap(), b"not a docx");
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    }
    #[test]
    fn existing_suffix_output_is_not_overwritten() {
        let directory = tempfile::tempdir().unwrap();
        let input = directory.path().join("input.docx");
        let output = output_path(&input, "omml");
        std::fs::write(&output, b"keep").unwrap();
        assert!(convert_one(&input, "omml", 12.0, false)
            .unwrap_err()
            .contains("already exists"));
        assert_eq!(std::fs::read(output).unwrap(), b"keep");
    }
    #[test]
    fn successful_batch_commit_preserves_input_then_can_replace_it() {
        use std::io::{Cursor, Write};
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let opts = zip::write::FileOptions::default();
        zip.start_file("[Content_Types].xml", opts).unwrap();
        zip.write_all(br#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="xml" ContentType="application/xml"/></Types>"#).unwrap();
        zip.start_file("word/document.xml", opts).unwrap();
        zip.write_all(br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math"><w:body><w:p><m:oMath><m:r><m:t>x</m:t></m:r></m:oMath></w:p></w:body></w:document>"#).unwrap();
        let original = zip.finish().unwrap().into_inner();
        let directory = tempfile::tempdir().unwrap();
        let input = directory.path().join("input.docx");
        std::fs::write(&input, &original).unwrap();
        let result = convert_one(&input, "mathtype", 12.0, false).unwrap();
        assert_eq!(result.converted, 1);
        assert_eq!(std::fs::read(&input).unwrap(), original);
        let result = convert_one(&input, "mathtype", 12.0, true).unwrap();
        assert_eq!(result.converted, 1);
        assert_ne!(std::fs::read(&input).unwrap(), original);
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 2);
    }
}
