//! Output naming and recursive DOCX input discovery for the CLI.

use std::path::{Path, PathBuf};

use super::UpgradeTargetArg;

fn output_suffix(to_omml: bool, upgrade: bool, upgrade_target: UpgradeTargetArg) -> &'static str {
    if upgrade {
        match upgrade_target {
            UpgradeTargetArg::Omml => "upgraded_omml",
            UpgradeTargetArg::Mathtype => "upgraded_mathtype",
        }
    } else if to_omml {
        "omml"
    } else {
        "mathtype"
    }
}

pub(super) fn default_file_output(
    input: &Path,
    to_omml: bool,
    upgrade: bool,
    upgrade_target: UpgradeTargetArg,
) -> PathBuf {
    let stem = input
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    input.with_file_name(format!(
        "{stem}_{}.docx",
        output_suffix(to_omml, upgrade, upgrade_target)
    ))
}

pub(super) fn default_dir_output(
    input: &Path,
    to_omml: bool,
    upgrade: bool,
    upgrade_target: UpgradeTargetArg,
) -> PathBuf {
    let resolved = if input.file_name().is_some() {
        input.to_path_buf()
    } else {
        input.canonicalize().unwrap_or_else(|_| input.to_path_buf())
    };
    let name = resolved
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "output".to_owned());
    resolved.with_file_name(format!(
        "{name}_{}",
        output_suffix(to_omml, upgrade, upgrade_target)
    ))
}

pub(super) fn is_docx_file(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("docx"))
}

fn is_ignored_name(path: &Path) -> bool {
    path.file_name()
        .map(|name| name.to_string_lossy())
        .is_some_and(|name| name.starts_with("~$") || name.starts_with('.'))
}

pub(super) fn collect_docx_files_recursive(
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
