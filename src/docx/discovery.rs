//! Shared recursive `.docx` file discovery helpers for CLI and desktop batching.

use std::path::{Path, PathBuf};

/// Returns `true` when `path` has a `.docx` extension (case-insensitive).
pub fn is_docx_file(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("docx"))
}

fn is_ignored_name(path: &Path) -> bool {
    path.file_name()
        .map(|name| name.to_string_lossy())
        .is_some_and(|name| name.starts_with("~$") || name.starts_with('.'))
}

fn is_excluded(path: &Path, exclude_dir: Option<&Path>) -> bool {
    let Some(exclude_dir) = exclude_dir else {
        return false;
    };
    path == exclude_dir
        || matches!(
            (path.canonicalize(), exclude_dir.canonicalize()),
            (Ok(path), Ok(exclude_dir)) if path == exclude_dir
        )
}

/// Recursively collects sorted `.docx` files under `directory`, skipping `~$*` and `.*` entries.
pub fn collect_docx_files_recursive(
    directory: &Path,
    exclude_dir: Option<&Path>,
    output: &mut Vec<PathBuf>,
) -> std::io::Result<()> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(directory)?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .collect();
    entries.sort();

    for path in entries {
        if is_ignored_name(&path) || is_excluded(&path, exclude_dir) {
            continue;
        }
        if path.is_dir() {
            collect_docx_files_recursive(&path, exclude_dir, output)?;
        } else if path.is_file() && is_docx_file(&path) {
            output.push(path);
        }
    }
    Ok(())
}
