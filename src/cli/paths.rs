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

pub(super) use crate::docx::{collect_docx_files_recursive, is_docx_file};
