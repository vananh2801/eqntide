//! Per-file conversion policy for the CLI.

use std::path::Path;

use crate::docx::{
    convert_to_omml_with, convert_with, upgrade_legacy_mathtype_docx_with, ConversionSummary,
    EmbedMode, Options, PreviewFormat, ToOmmlOptions, UpgradeOptions, UpgradeTarget,
};
use crate::error::Result as ConvResult;

use super::{Args, PreviewArg, UpgradeTargetArg};

pub(super) fn convert_single_file(
    source: &Path,
    destination: &Path,
    args: &Args,
) -> ConvResult<ConversionSummary> {
    if let Some(parent) = destination.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    if args.upgrade {
        let options = UpgradeOptions {
            target: match args.upgrade_target {
                UpgradeTargetArg::Omml => UpgradeTarget::ModernOmml,
                UpgradeTargetArg::Mathtype => UpgradeTarget::ModernMathType,
            },
            font_pt: args.font_size,
            math_font: args
                .math_font
                .clone()
                .or_else(|| UpgradeOptions::default().math_font),
        };
        upgrade_legacy_mathtype_docx_with(source, destination, &options)
    } else if args.to_omml {
        let options = ToOmmlOptions {
            math_font: args
                .math_font
                .clone()
                .or_else(|| ToOmmlOptions::default().math_font),
        };
        convert_to_omml_with(source, destination, &options)
    } else {
        let options = Options {
            mode: if args.omml_fallback {
                EmbedMode::OmmlFallback
            } else {
                EmbedMode::Preview
            },
            font_pt: args.font_size,
            preview: match args.preview {
                PreviewArg::Emf => PreviewFormat::Emf,
                PreviewArg::Png => PreviewFormat::Png,
            },
        };
        convert_with(source, destination, &options)
    }
}
