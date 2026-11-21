use crate::{Event, EventSender as Sender};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    io::{BufRead, BufReader, Write},
    path::{Component, Path, PathBuf},
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BatchSelectionMode {
    FlatList,
    #[default]
    FolderTree,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatchRowOutput {
    pub file_dir: PathBuf,
    pub root_output_dir: Option<PathBuf>,
}

impl BatchRowOutput {
    pub fn from_conversion(
        item: &BatchItem,
        output_path: &Path,
        target: &str,
        overwrite: bool,
        timestamp: &str,
    ) -> Self {
        let file_dir = output_path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        let root_output_dir = item.source_root.as_deref().map(|root| {
            if overwrite {
                root.to_path_buf()
            } else {
                folder_output_dir(root, target, timestamp)
            }
        });
        Self {
            file_dir,
            root_output_dir,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatchRowState {
    pub flat_selected: bool,
    pub tree_selected: bool,
    pub last_output: Option<BatchRowOutput>,
}

impl Default for BatchRowState {
    fn default() -> Self {
        Self {
            flat_selected: true,
            tree_selected: true,
            last_output: None,
        }
    }
}

impl BatchRowState {
    pub fn is_selected(&self, mode: BatchSelectionMode) -> bool {
        match mode {
            BatchSelectionMode::FlatList => self.flat_selected,
            BatchSelectionMode::FolderTree => self.tree_selected,
        }
    }

    pub fn set_selected(&mut self, mode: BatchSelectionMode, selected: bool) {
        match mode {
            BatchSelectionMode::FlatList => self.flat_selected = selected,
            BatchSelectionMode::FolderTree => self.tree_selected = selected,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TreeRow {
    Group {
        key: String,
        label: String,
        hover_path: Option<String>,
        depth: usize,
        collapsed: bool,
        descendant_indices: Vec<usize>,
    },
    File {
        item_index: usize,
        label: String,
        depth: usize,
    },
}

impl TreeRow {
    pub fn depth(&self) -> usize {
        match self {
            Self::Group { depth, .. } | Self::File { depth, .. } => *depth,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TreeGuide {
    /// For each ancestor column `0..depth.saturating_sub(1)`, whether a vertical trunk continues past this row.
    pub ancestor_continues: Vec<bool>,
    /// Whether this row (`depth > 0`) is the last sibling under its parent.
    pub is_last_sibling: bool,
}

/// Compute per-row tree connector topology (`|-`, `└-`, and continuing ancestor trunks).
pub fn compute_tree_guides(rows: &[TreeRow]) -> Vec<TreeGuide> {
    let mut guides = vec![TreeGuide::default(); rows.len()];
    let mut active_depths: HashSet<usize> = HashSet::new();

    for (idx, row) in rows.iter().enumerate().rev() {
        let depth = row.depth();
        active_depths.retain(|&d| d <= depth);
        if depth > 0 {
            let is_last_sibling = !active_depths.contains(&depth);
            let ancestor_continues = (1..depth).map(|d| active_depths.contains(&d)).collect();
            guides[idx] = TreeGuide {
                ancestor_continues,
                is_last_sibling,
            };
        }
        active_depths.insert(depth);
    }

    guides
}

struct DirNode {
    key: String,
    label: String,
    hover_path: Option<String>,
    children: BTreeMap<String, DirNode>,
    direct_files: Vec<(String, usize)>,
    descendants: Vec<usize>,
}

impl DirNode {
    fn new(key: String, label: String, hover_path: Option<String>) -> Self {
        Self {
            key,
            label,
            hover_path,
            children: BTreeMap::new(),
            direct_files: Vec::new(),
            descendants: Vec::new(),
        }
    }

    fn emit(&mut self, depth: usize, collapsed: &HashSet<String>, out: &mut Vec<TreeRow>) {
        let is_collapsed = collapsed.contains(&self.key);
        out.push(TreeRow::Group {
            key: self.key.clone(),
            label: self.label.clone(),
            hover_path: self.hover_path.clone(),
            depth,
            collapsed: is_collapsed,
            descendant_indices: self.descendants.clone(),
        });
        if is_collapsed {
            return;
        }
        for child in self.children.values_mut() {
            child.emit(depth + 1, collapsed, out);
        }
        self.direct_files
            .sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        for (filename, item_index) in &self.direct_files {
            out.push(TreeRow::File {
                item_index: *item_index,
                label: filename.clone(),
                depth: depth + 1,
            });
        }
    }
}

/// Build flattened visible rows for the Folder Tree view while preserving deterministic order.
pub fn build_tree_rows(items: &[BatchItem], collapsed: &HashSet<String>) -> Vec<TreeRow> {
    let mut root_order: Vec<PathBuf> = Vec::new();
    let mut standalone_indices: Vec<usize> = Vec::new();
    for item in items {
        if let Some(root) = &item.source_root {
            if !root_order.contains(root) {
                root_order.push(root.clone());
            }
        }
    }
    for (idx, item) in items.iter().enumerate() {
        if item.source_root.is_none() {
            standalone_indices.push(idx);
        }
    }

    let mut out = Vec::new();
    for root in root_order {
        let root_label = root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| root.display().to_string());
        let root_key = format!("root:{}", root.display());
        let mut root_node = DirNode::new(root_key, root_label, Some(root.display().to_string()));

        for (idx, item) in items.iter().enumerate() {
            if item.source_root.as_ref() != Some(&root) {
                continue;
            }
            root_node.descendants.push(idx);
            let rel = item
                .input
                .strip_prefix(&root)
                .unwrap_or(item.input.as_path());
            let mut cur = &mut root_node;
            if let Some(parent) = rel.parent() {
                for comp in parent.components() {
                    if let Component::Normal(seg) = comp {
                        let name = seg.to_string_lossy().into_owned();
                        let child_key = format!("{}/{}", cur.key, name);
                        let child_hover = cur
                            .hover_path
                            .as_ref()
                            .map(|p| Path::new(p).join(&name).display().to_string());
                        cur = cur
                            .children
                            .entry(name.clone())
                            .or_insert_with(|| DirNode::new(child_key, name, child_hover));
                        cur.descendants.push(idx);
                    }
                }
            }
            let filename = rel
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| item.display_label());
            cur.direct_files.push((filename, idx));
        }

        root_node.emit(0, collapsed, &mut out);
    }

    if !standalone_indices.is_empty() {
        let key = "group:individual".to_owned();
        let is_collapsed = collapsed.contains(&key);
        out.push(TreeRow::Group {
            key,
            label: "Individual files".to_owned(),
            hover_path: None,
            depth: 0,
            collapsed: is_collapsed,
            descendant_indices: standalone_indices.clone(),
        });
        if !is_collapsed {
            for idx in standalone_indices {
                out.push(TreeRow::File {
                    item_index: idx,
                    label: items[idx].display_label(),
                    depth: 1,
                });
            }
        }
    }

    out
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
    match convert_item_with_cancel(item, target, font_pt, overwrite, timestamp, None) {
        BatchItemOutcome::Finished(res) => res,
        BatchItemOutcome::Cancelled => Err("document conversion cancelled".into()),
    }
}

enum BatchItemOutcome {
    Finished(Result<eqntide::bridge::ConvertDocumentResponse, String>),
    Cancelled,
}

fn convert_item_with_cancel(
    item: &BatchItem,
    target: &str,
    font_pt: f32,
    overwrite: bool,
    timestamp: &str,
    stop: Option<&AtomicBool>,
) -> BatchItemOutcome {
    if stop.is_some_and(|s| s.load(Ordering::Relaxed)) {
        return BatchItemOutcome::Cancelled;
    }
    let output = item_output_path(item, target, overwrite, timestamp);
    if !overwrite && output.exists() {
        return BatchItemOutcome::Finished(Err(format!(
            "Output already exists: {}",
            output.display()
        )));
    }
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    if let Err(e) = std::fs::create_dir_all(parent) {
        return BatchItemOutcome::Finished(Err(e.to_string()));
    }
    let temp = match tempfile::Builder::new()
        .prefix(".eqntide-")
        .suffix(".docx")
        .tempfile_in(parent)
    {
        Ok(t) => t,
        Err(e) => return BatchItemOutcome::Finished(Err(e.to_string())),
    };
    let mut response = match eqntide::service::convert_document_with_cancel(
        &eqntide::bridge::ConvertDocumentRequest {
            input_path: item.input.to_string_lossy().into_owned(),
            output_path: Some(temp.path().to_string_lossy().into_owned()),
            target: target.into(),
            preview: "emf".into(),
            font_pt,
        },
        stop,
    ) {
        Ok(r) => r,
        Err(e) if e.status == 499 => return BatchItemOutcome::Cancelled,
        Err(e) => return BatchItemOutcome::Finished(Err(e.to_string())),
    };
    if stop.is_some_and(|s| s.load(Ordering::Relaxed)) {
        return BatchItemOutcome::Cancelled;
    }
    if overwrite {
        if let Err(e) = temp.persist(&output) {
            return BatchItemOutcome::Finished(Err(e.to_string()));
        }
    } else if let Err(e) = temp.persist_noclobber(&output) {
        return BatchItemOutcome::Finished(Err(e.to_string()));
    }
    response.output_path = output.to_string_lossy().into_owned();
    BatchItemOutcome::Finished(Ok(response))
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

#[derive(Serialize, Deserialize)]
struct WorkerRequest {
    items: Vec<(usize, BatchItem)>,
    target: String,
    font_pt: f32,
    overwrite: bool,
    timestamp: String,
}

#[derive(Serialize, Deserialize)]
enum WorkerEvent {
    Item {
        orig_idx: usize,
        result: Result<eqntide::bridge::ConvertDocumentResponse, String>,
    },
}

fn process_items_sequential(
    items: &[(usize, BatchItem)],
    target: &str,
    font_pt: f32,
    overwrite: bool,
    timestamp: &str,
    stop: &AtomicBool,
    mut on_item: impl FnMut(usize, Result<eqntide::bridge::ConvertDocumentResponse, String>),
) {
    for (orig_idx, item) in items {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        if let BatchItemOutcome::Finished(res) =
            convert_item_with_cancel(item, target, font_pt, overwrite, timestamp, Some(stop))
        {
            on_item(*orig_idx, res);
        }
    }
}

fn execute_worker_request<W: Write>(req: &WorkerRequest, mut writer: W, stop: &AtomicBool) {
    process_items_sequential(
        &req.items,
        &req.target,
        req.font_pt,
        req.overwrite,
        &req.timestamp,
        stop,
        |orig_idx, result| {
            if let Ok(line) = serde_json::to_string(&WorkerEvent::Item { orig_idx, result }) {
                let _ = writeln!(writer, "{line}");
                let _ = writer.flush();
            }
        },
    );
}

#[cfg(test)]
fn run_worker_lines<R: BufRead, W: Write>(
    mut reader: R,
    writer: W,
    stop: &AtomicBool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut first_line = String::new();
    let n = reader.read_line(&mut first_line)?;
    if n == 0 || first_line.trim().is_empty() {
        return Ok(());
    }
    let req: WorkerRequest = serde_json::from_str(&first_line)?;
    execute_worker_request(&req, writer, stop);
    Ok(())
}

/// Entry point for `eqntide-desktop --batch-worker`: processes one file at a time
/// (multi-threaded inside each file) and exits so the OS reclaims 100% of memory.
pub fn run_worker_stdio() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = std::io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let mut first_line = String::new();
    let n = reader.read_line(&mut first_line)?;
    if n == 0 || first_line.trim().is_empty() {
        return Ok(());
    }
    let req: WorkerRequest = serde_json::from_str(&first_line)?;
    let stop = Arc::new(AtomicBool::new(false));
    let stop_reader = Arc::clone(&stop);

    // Watch stdin for explicit "stop" or EOF (parent closed pipe / clicked Stop)
    drop(reader);
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        let mut reader = BufReader::new(stdin.lock());
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => {
                    stop_reader.store(true, Ordering::Relaxed);
                    break;
                }
                Ok(_) if line.trim().eq_ignore_ascii_case("stop") => {
                    stop_reader.store(true, Ordering::Relaxed);
                    break;
                }
                Ok(_) => {}
            }
        }
    });

    let stdout = std::io::stdout();
    let writer = std::io::BufWriter::new(stdout.lock());
    execute_worker_request(&req, writer, &stop);
    Ok(())
}

#[cfg(not(test))]
fn run_via_worker_subprocess(
    items: &[(usize, BatchItem)],
    target: &str,
    font_pt: f32,
    overwrite: bool,
    timestamp: &str,
    stop: &Arc<AtomicBool>,
    tx: &Sender<Event>,
) -> bool {
    use std::process::{Command, Stdio};
    use std::sync::Mutex;
    use std::time::Duration;

    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let Ok(mut child) = Command::new(exe)
        .arg("--batch-worker")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return false;
    };

    let Some(mut child_stdin) = child.stdin.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return false;
    };
    let Some(child_stdout) = child.stdout.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return false;
    };

    let req = WorkerRequest {
        items: items.to_vec(),
        target: target.to_owned(),
        font_pt,
        overwrite,
        timestamp: timestamp.to_owned(),
    };
    let Ok(payload) = serde_json::to_string(&req) else {
        let _ = child.kill();
        let _ = child.wait();
        return false;
    };
    if writeln!(child_stdin, "{payload}").is_err() || child_stdin.flush().is_err() {
        let _ = child.kill();
        let _ = child.wait();
        return false;
    }

    let stdin_guard = Arc::new(Mutex::new(Some(child_stdin)));
    let worker_done = Arc::new(AtomicBool::new(false));
    let stop_flag = Arc::clone(stop);
    let done_flag = Arc::clone(&worker_done);
    let stdin_watcher = Arc::clone(&stdin_guard);
    let watcher = std::thread::spawn(move || {
        while !done_flag.load(Ordering::Relaxed) {
            if stop_flag.load(Ordering::Relaxed) {
                if let Ok(mut guard) = stdin_watcher.lock() {
                    if let Some(mut pipe) = guard.take() {
                        let _ = writeln!(pipe, "stop");
                        let _ = pipe.flush();
                    }
                }
                break;
            }
            std::thread::sleep(Duration::from_millis(40));
        }
    });

    let reader = BufReader::new(child_stdout);
    for line in reader.lines().map_while(Result::ok) {
        if let Ok(WorkerEvent::Item { orig_idx, result }) = serde_json::from_str(&line) {
            let _ = tx.send(Event::Batch(orig_idx, result));
        }
    }

    worker_done.store(true, Ordering::Relaxed);
    if let Ok(mut guard) = stdin_guard.lock() {
        guard.take();
    }
    let _ = watcher.join();
    let _ = child.wait();
    true
}

pub fn run_items(
    items: Vec<(usize, BatchItem)>,
    target: String,
    font_pt: f32,
    overwrite: bool,
    timestamp: String,
    stop: Arc<AtomicBool>,
    tx: Sender<Event>,
) {
    std::thread::spawn(move || {
        #[cfg(not(test))]
        let ran_worker =
            run_via_worker_subprocess(&items, &target, font_pt, overwrite, &timestamp, &stop, &tx);
        #[cfg(test)]
        let ran_worker = false;

        if !ran_worker {
            process_items_sequential(
                &items,
                &target,
                font_pt,
                overwrite,
                &timestamp,
                &stop,
                |orig_idx, res| {
                    let _ = tx.send(Event::Batch(orig_idx, res));
                },
            );
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

    #[test]
    fn row_state_tracks_independent_selection_modes_defaulting_to_flat_all_selected() {
        assert_eq!(
            BatchSelectionMode::default(),
            BatchSelectionMode::FolderTree
        );
        let mut row = BatchRowState::default();
        assert!(row.is_selected(BatchSelectionMode::FlatList));
        assert!(row.is_selected(BatchSelectionMode::FolderTree));

        row.set_selected(BatchSelectionMode::FlatList, false);
        assert!(!row.is_selected(BatchSelectionMode::FlatList));
        assert!(row.is_selected(BatchSelectionMode::FolderTree));

        row.set_selected(BatchSelectionMode::FolderTree, false);
        row.set_selected(BatchSelectionMode::FlatList, true);
        assert!(row.is_selected(BatchSelectionMode::FlatList));
        assert!(!row.is_selected(BatchSelectionMode::FolderTree));
    }

    #[test]
    fn row_output_metadata_tracks_file_dir_and_optional_folder_root() {
        let root = PathBuf::from("/tmp/docs_a");
        let nested = root.join("ch1").join("lesson.docx");
        let folder_item = BatchItem::from_folder(nested.clone(), root.clone());
        let ts = "20261119_194500";

        let out_path = item_output_path(&folder_item, "omml", false, ts);
        let meta = BatchRowOutput::from_conversion(&folder_item, &out_path, "omml", false, ts);
        assert_eq!(
            meta.file_dir,
            PathBuf::from("/tmp/docs_a_omml_20261119_194500/ch1")
        );
        assert_eq!(
            meta.root_output_dir,
            Some(PathBuf::from("/tmp/docs_a_omml_20261119_194500"))
        );

        let overwrite_meta =
            BatchRowOutput::from_conversion(&folder_item, &nested, "omml", true, ts);
        assert_eq!(overwrite_meta.file_dir, root.join("ch1"));
        assert_eq!(overwrite_meta.root_output_dir, Some(root));

        let solo_input = PathBuf::from("/tmp/other/note.docx");
        let solo_item = BatchItem::file(solo_input.clone());
        let solo_out = item_output_path(&solo_item, "omml", false, ts);
        let solo_meta = BatchRowOutput::from_conversion(&solo_item, &solo_out, "omml", false, ts);
        assert_eq!(solo_meta.file_dir, PathBuf::from("/tmp/other"));
        assert_eq!(solo_meta.root_output_dir, None);
    }

    #[test]
    fn build_tree_rows_groups_nested_folders_and_individual_files_with_collapse() {
        let root_a = PathBuf::from("/drive_a/docs_a");
        let root_b = PathBuf::from("/drive_b/docs_b");
        let items = vec![
            BatchItem::from_folder(root_a.join("ch1").join("l1.docx"), root_a.clone()),
            BatchItem::from_folder(root_a.join("intro.docx"), root_a.clone()),
            BatchItem::from_folder(root_b.join("b1.docx"), root_b.clone()),
            BatchItem::file(PathBuf::from("/tmp/solo.docx")),
        ];

        let expanded = build_tree_rows(&items, &HashSet::new());
        assert_eq!(
            expanded,
            vec![
                TreeRow::Group {
                    key: "root:/drive_a/docs_a".into(),
                    label: "docs_a".into(),
                    hover_path: Some("/drive_a/docs_a".into()),
                    depth: 0,
                    collapsed: false,
                    descendant_indices: vec![0, 1],
                },
                TreeRow::Group {
                    key: "root:/drive_a/docs_a/ch1".into(),
                    label: "ch1".into(),
                    hover_path: Some(root_a.join("ch1").display().to_string()),
                    depth: 1,
                    collapsed: false,
                    descendant_indices: vec![0],
                },
                TreeRow::File {
                    item_index: 0,
                    label: "l1.docx".into(),
                    depth: 2,
                },
                TreeRow::File {
                    item_index: 1,
                    label: "intro.docx".into(),
                    depth: 1,
                },
                TreeRow::Group {
                    key: "root:/drive_b/docs_b".into(),
                    label: "docs_b".into(),
                    hover_path: Some("/drive_b/docs_b".into()),
                    depth: 0,
                    collapsed: false,
                    descendant_indices: vec![2],
                },
                TreeRow::File {
                    item_index: 2,
                    label: "b1.docx".into(),
                    depth: 1,
                },
                TreeRow::Group {
                    key: "group:individual".into(),
                    label: "Individual files".into(),
                    hover_path: None,
                    depth: 0,
                    collapsed: false,
                    descendant_indices: vec![3],
                },
                TreeRow::File {
                    item_index: 3,
                    label: "/tmp/solo.docx".into(),
                    depth: 1,
                },
            ]
        );

        let mut collapsed = HashSet::new();
        collapsed.insert("root:/drive_a/docs_a/ch1".to_owned());
        collapsed.insert("group:individual".to_owned());
        let rows = build_tree_rows(&items, &collapsed);
        assert!(!rows.iter().any(|r| matches!(
            r,
            TreeRow::File {
                item_index: 0 | 3,
                ..
            }
        )));

        let guides = compute_tree_guides(&expanded);
        assert_eq!(guides.len(), 8);
        // docs_a/ch1 (idx 1, depth 1) has sibling intro.docx below -> not last
        assert!(!guides[1].is_last_sibling);
        // l1.docx (idx 2, depth 2) is last in ch1, and ancestor depth 1 (ch1) continues to intro.docx
        assert!(guides[2].is_last_sibling);
        assert_eq!(guides[2].ancestor_continues, vec![true]);
        // intro.docx (idx 3, depth 1) is last child of docs_a
        assert!(guides[3].is_last_sibling);
        // docs_b/b1.docx (idx 5, depth 1) is last child of docs_b
        assert!(guides[5].is_last_sibling);
    }

    #[test]
    fn run_items_converts_multiple_files_concurrently_and_honors_stop() {
        use std::io::{Cursor, Write};
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let opts = zip::write::FileOptions::default();
        zip.start_file("[Content_Types].xml", opts).unwrap();
        zip.write_all(br#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="xml" ContentType="application/xml"/></Types>"#).unwrap();
        zip.start_file("word/document.xml", opts).unwrap();
        zip.write_all(br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math"><w:body><w:p><m:oMath><m:r><m:t>x</m:t></m:r></m:oMath></w:p></w:body></w:document>"#).unwrap();
        let original = zip.finish().unwrap().into_inner();

        let dir = tempfile::tempdir().unwrap();
        let mut items = Vec::new();
        for i in 0..6 {
            let p = dir.path().join(format!("doc_{i}.docx"));
            std::fs::write(&p, &original).unwrap();
            items.push((i, BatchItem::file(p)));
        }

        // 1. Concurrent full run
        let (raw_tx, rx) = std::sync::mpsc::channel();
        let tx = crate::EventSender::new(raw_tx);
        let stop = Arc::new(AtomicBool::new(false));
        run_items(
            items.clone(),
            "mathtype".into(),
            12.0,
            false,
            String::new(),
            stop,
            tx,
        );

        let mut completed = Vec::new();
        let mut done_seen = false;
        for ev in rx {
            match ev {
                Event::Batch(idx, res) => {
                    assert!(res.is_ok(), "item {idx} failed: {res:?}");
                    completed.push(idx);
                }
                Event::BatchDone => {
                    done_seen = true;
                    break;
                }
                _ => {}
            }
        }
        assert!(done_seen);
        completed.sort_unstable();
        assert_eq!(completed, vec![0, 1, 2, 3, 4, 5]);
        for (_, item) in &items {
            assert!(output_path(&item.input, "mathtype").is_file());
        }

        // 2. Cooperative stop skips unstarted files and leaves no temp files
        let dir_stop = tempfile::tempdir().unwrap();
        let mut stop_items = Vec::new();
        for i in 0..4 {
            let p = dir_stop.path().join(format!("stop_{i}.docx"));
            std::fs::write(&p, &original).unwrap();
            stop_items.push((i, BatchItem::file(p)));
        }
        let (raw_tx2, rx2) = std::sync::mpsc::channel();
        let tx2 = crate::EventSender::new(raw_tx2);
        let stop_flag = Arc::new(AtomicBool::new(true));
        run_items(
            stop_items.clone(),
            "mathtype".into(),
            12.0,
            false,
            String::new(),
            stop_flag,
            tx2,
        );
        let mut stopped_events = Vec::new();
        let mut done2 = false;
        for ev in rx2 {
            match ev {
                Event::Batch(idx, _) => stopped_events.push(idx),
                Event::BatchDone => {
                    done2 = true;
                    break;
                }
                _ => {}
            }
        }
        assert!(done2);
        assert!(stopped_events.is_empty());
        assert_eq!(std::fs::read_dir(dir_stop.path()).unwrap().count(), 4);

        // 3. Headless --batch-worker JSON stdio protocol
        let dir_worker = tempfile::tempdir().unwrap();
        let worker_doc = dir_worker.path().join("w0.docx");
        std::fs::write(&worker_doc, &original).unwrap();
        let req = WorkerRequest {
            items: vec![(7, BatchItem::file(worker_doc.clone()))],
            target: "mathtype".into(),
            font_pt: 12.0,
            overwrite: false,
            timestamp: String::new(),
        };
        let mut in_bytes = serde_json::to_vec(&req).unwrap();
        in_bytes.push(b'\n');
        let mut out_bytes = Vec::new();
        run_worker_lines(
            std::io::Cursor::new(in_bytes),
            &mut out_bytes,
            &AtomicBool::new(false),
        )
        .unwrap();
        let out_line = String::from_utf8(out_bytes).unwrap();
        let ev: WorkerEvent = serde_json::from_str(out_line.trim()).unwrap();
        match ev {
            WorkerEvent::Item { orig_idx, result } => {
                assert_eq!(orig_idx, 7);
                assert_eq!(result.unwrap().converted, 1);
            }
        }
        assert!(output_path(&worker_doc, "mathtype").is_file());
    }
}
