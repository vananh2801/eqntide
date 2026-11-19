use crate::{Event, EventSender as Sender};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{SystemTime, UNIX_EPOCH},
};

pub const TARGETS: &[(&str, &str)] = &[
    ("omml", "MathType → OMML"),
    ("mathtype", "OMML → MathType"),
    ("upgrade_omml", "Legacy → modern OMML"),
    ("upgrade_mathtype", "Legacy → modern MathType"),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatchItem {
    pub input: PathBuf,
    pub source_root: Option<PathBuf>,
}

impl BatchItem {
    pub fn file(input: PathBuf) -> Self {
        Self {
            input,
            source_root: None,
        }
    }

    pub fn from_folder(input: PathBuf, root: PathBuf) -> Self {
        Self {
            input,
            source_root: Some(root),
        }
    }

    pub fn display_label(&self) -> String {
        if let Some(root) = &self.source_root {
            if let Ok(rel) = self.input.strip_prefix(root) {
                if let Some(root_name) = root.file_name() {
                    return Path::new(root_name).join(rel).display().to_string();
                }
                return rel.display().to_string();
            }
        }
        self.input.display().to_string()
    }
}

pub fn format_timestamp(time: SystemTime) -> String {
    let secs = time
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = (secs / 86_400) as i64;
    let day_secs = secs % 86_400;
    let hour = day_secs / 3_600;
    let minute = (day_secs % 3_600) / 60;
    let second = day_secs % 60;

    // Civil-from-days algorithm (Howard Hinnant) in UTC.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = y + i64::from(month <= 2);

    format!("{year:04}{month:02}{day:02}_{hour:02}{minute:02}{second:02}")
}

pub fn timestamp_now() -> String {
    format_timestamp(SystemTime::now())
}

pub fn folder_output_dir(root: &Path, target: &str, timestamp: &str) -> PathBuf {
    let resolved = if root.file_name().is_some() {
        root.to_path_buf()
    } else {
        root.canonicalize().unwrap_or_else(|_| root.to_path_buf())
    };
    let name = resolved
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| "output".to_owned());
    resolved.with_file_name(format!("{name}_{target}_{timestamp}"))
}

pub fn output_path(input: &Path, target: &str) -> PathBuf {
    let mut name = input.file_stem().unwrap_or_default().to_os_string();
    name.push(format!("_{target}.docx"));
    input.with_file_name(name)
}

pub fn item_output_path(
    item: &BatchItem,
    target: &str,
    overwrite: bool,
    timestamp: &str,
) -> PathBuf {
    if overwrite {
        return item.input.clone();
    }
    if let Some(root) = &item.source_root {
        let out_dir = folder_output_dir(root, target, timestamp);
        let relative = item
            .input
            .strip_prefix(root)
            .unwrap_or_else(|_| Path::new(item.input.file_name().unwrap_or_default()));
        return out_dir.join(relative);
    }
    output_path(&item.input, target)
}

/// Convert into a temporary sibling before committing, preserving inputs on failure.
pub fn convert_item(
    item: &BatchItem,
    target: &str,
    font_pt: f32,
    overwrite: bool,
    timestamp: &str,
) -> Result<eqntide::bridge::ConvertDocumentResponse, String> {
    let output = item_output_path(item, target, overwrite, timestamp);
    if !overwrite && output.exists() {
        return Err(format!("Output already exists: {}", output.display()));
    }
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temp = tempfile::Builder::new()
        .prefix(".eqntide-")
        .suffix(".docx")
        .tempfile_in(parent)
        .map_err(|e| e.to_string())?;
    let mut response =
        eqntide::service::convert_document(&eqntide::bridge::ConvertDocumentRequest {
            input_path: item.input.to_string_lossy().into_owned(),
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

#[allow(dead_code)]
pub fn convert_one(
    input: &Path,
    target: &str,
    font_pt: f32,
    overwrite: bool,
) -> Result<eqntide::bridge::ConvertDocumentResponse, String> {
    convert_item(
        &BatchItem::file(input.to_path_buf()),
        target,
        font_pt,
        overwrite,
        "",
    )
}

pub fn run_items(
    items: Vec<BatchItem>,
    target: String,
    font_pt: f32,
    overwrite: bool,
    timestamp: String,
    stop: Arc<AtomicBool>,
    tx: Sender<Event>,
) {
    std::thread::spawn(move || {
        for (i, item) in items.iter().enumerate() {
            if stop.load(Ordering::Relaxed) {
                break;
            }
            let _ = tx.send(Event::Batch(
                i,
                convert_item(item, &target, font_pt, overwrite, &timestamp),
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
        let input = Path::new("folder_α/exercise_β.docx");
        assert_eq!(
            output_path(input, "omml"),
            Path::new("folder_α/exercise_β_omml.docx")
        );
    }
    #[test]
    fn failed_conversion_never_replaces_original_or_leaves_temporary_files() {
        let directory = tempfile::tempdir().unwrap();
        let input = directory.path().join("exposé_α.docx");
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

    #[test]
    fn format_timestamp_produces_yyyymmdd_hhmmss() {
        // 2026-11-18 14:58:09 UTC = 1795013889 seconds since Unix epoch.
        let ts = UNIX_EPOCH + std::time::Duration::from_secs(1_795_013_889);
        assert_eq!(format_timestamp(ts), "20261118_145809");
    }

    #[test]
    fn folder_batch_preserves_nested_hierarchy_in_sibling_timestamped_dir() {
        use std::io::{Cursor, Write};
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let opts = zip::write::FileOptions::default();
        zip.start_file("[Content_Types].xml", opts).unwrap();
        zip.write_all(br#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="xml" ContentType="application/xml"/></Types>"#).unwrap();
        zip.start_file("word/document.xml", opts).unwrap();
        zip.write_all(br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math"><w:body><w:p><m:oMath><m:r><m:t>x</m:t></m:r></m:oMath></w:p></w:body></w:document>"#).unwrap();
        let original = zip.finish().unwrap().into_inner();

        let temp_root = tempfile::tempdir().unwrap();
        let input_root = temp_root.path().join("docs");
        let nested = input_root.join("chapter1").join("section_a");
        std::fs::create_dir_all(&nested).unwrap();

        let root_doc = input_root.join("intro.docx");
        let nested_doc = nested.join("exercises.docx");
        std::fs::write(&root_doc, &original).unwrap();
        std::fs::write(&nested_doc, &original).unwrap();

        let items = [
            BatchItem::from_folder(root_doc.clone(), input_root.clone()),
            BatchItem::from_folder(nested_doc.clone(), input_root.clone()),
        ];
        assert_eq!(
            items[0].display_label(),
            Path::new("docs").join("intro.docx").display().to_string()
        );
        assert_eq!(
            items[1].display_label(),
            Path::new("docs")
                .join("chapter1")
                .join("section_a")
                .join("exercises.docx")
                .display()
                .to_string()
        );

        let ts = "20261118_145809";
        let expected_out_root = temp_root.path().join("docs_mathtype_20261118_145809");
        assert_eq!(
            folder_output_dir(&input_root, "mathtype", ts),
            expected_out_root
        );

        for item in &items {
            let res = convert_item(item, "mathtype", 12.0, false, ts).unwrap();
            assert_eq!(res.converted, 1);
        }
        assert!(expected_out_root.join("intro.docx").is_file());
        assert!(expected_out_root
            .join("chapter1")
            .join("section_a")
            .join("exercises.docx")
            .is_file());
        assert_eq!(std::fs::read(&root_doc).unwrap(), original);
        assert_eq!(std::fs::read(&nested_doc).unwrap(), original);

        // Overwrite replaces original files in place without creating a sibling directory.
        let res_ow = convert_item(&items[1], "mathtype", 12.0, true, ts).unwrap();
        assert_eq!(res_ow.output_path, nested_doc.to_string_lossy());
        assert_ne!(std::fs::read(&nested_doc).unwrap(), original);
    }
}
