//! Command-line interface for `omml-converter`.

use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};

use clap::Parser;

use crate::docx::{
    convert_to_omml_with, convert_with, upgrade_legacy_mathtype_docx_with, ConversionSummary,
    EmbedMode, Options, PreviewFormat, ToOmmlOptions, UpgradeOptions, UpgradeTarget,
};
use crate::error::Result as ConvResult;

/// Convert equations in a .docx file or directory between OMML and MathType OLE objects.
#[derive(Parser, Debug)]
#[command(
    name = "omml-converter",
    version,
    about = "Convert equations in a .docx file or directory (recursively) between OMML and MathType OLE objects."
)]
struct Args {
    /// Input .docx file or directory containing .docx files (scanned recursively)
    input: PathBuf,

    /// Output .docx file or directory path (default: <input>_mathtype[.docx] or <input>_omml[.docx])
    #[arg(short, long, value_name = "PATH")]
    output: Option<PathBuf>,

    /// Convert MathType OLE / WMF equations in the .docx to Word OMML (<m:oMath>)
    #[arg(short = 'r', long = "to-omml")]
    to_omml: bool,

    /// Upgrade and re-typeset all MathType equations (legacy Equation.3/WMF and existing MathType) with this repo's typesetting engine
    #[arg(short = 'u', long = "upgrade", conflicts_with = "to_omml")]
    upgrade: bool,

    /// Target format for --upgrade: 'mathtype' (Modern MathType 7 DSMT4 OLE + vector EMF, default) or 'omml' (native Word OMML)
    #[arg(long, value_enum, default_value_t = UpgradeTargetArg::Mathtype)]
    upgrade_target: UpgradeTargetArg,

    /// Optional font name (e.g. "Cambria Math") to apply to OMML runs when using --to-omml
    #[arg(long, value_name = "FONT")]
    math_font: Option<String>,

    /// Show per-equation progress
    #[arg(short, long, conflicts_with = "quiet")]
    verbose: bool,

    /// Only show errors
    #[arg(short, long)]
    quiet: bool,

    /// Keep the original OMML visible and store the MathType objects unseen
    /// (the pre-0.2 behaviour); by default each equation becomes a visible
    /// MathType object with a rendered preview picture
    #[arg(long)]
    omml_fallback: bool,

    /// Picture format of the equation previews: emf is vector (sharp at any
    /// zoom), png is raster. Ignored with --omml-fallback
    #[arg(long, value_enum, default_value_t = PreviewArg::Emf)]
    preview: PreviewArg,

    /// Font size (points) used to draw the preview pictures
    #[arg(long, value_name = "PT", default_value_t = 12.0, value_parser = parse_font_size)]
    font_size: f32,
}

/// `--upgrade-target` values.
#[derive(Clone, Copy, Debug, clap::ValueEnum, Default)]
pub enum UpgradeTargetArg {
    #[default]
    Mathtype,
    Omml,
}

/// `--preview` values.
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum PreviewArg {
    Emf,
    Png,
}

fn parse_font_size(s: &str) -> Result<f32, String> {
    let v: f32 = s.parse().map_err(|_| format!("'{s}' is not a number"))?;
    if (4.0..=72.0).contains(&v) {
        Ok(v)
    } else {
        Err("font size must be between 4 and 72 points".into())
    }
}

struct StderrLogger;

impl log::Log for StderrLogger {
    fn enabled(&self, _: &log::Metadata<'_>) -> bool {
        true
    }
    fn log(&self, record: &log::Record<'_>) {
        if self.enabled(record.metadata()) {
            eprintln!("{}", record.args());
        }
    }
    fn flush(&self) {}
}

static LOGGER: StderrLogger = StderrLogger;

fn configure_logging(verbose: bool, quiet: bool) {
    // set_logger fails if already installed (e.g. repeated calls in tests); fine.
    let _ = log::set_logger(&LOGGER);
    log::set_max_level(if quiet {
        log::LevelFilter::Error
    } else if verbose {
        log::LevelFilter::Info
    } else {
        log::LevelFilter::Warn
    });
}

fn default_file_output(input: &Path, to_omml: bool, upgrade: bool, upgrade_target: UpgradeTargetArg) -> PathBuf {
    let stem = input
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let suffix = if upgrade {
        match upgrade_target {
            UpgradeTargetArg::Omml => "upgraded_omml",
            UpgradeTargetArg::Mathtype => "upgraded_mathtype",
        }
    } else if to_omml {
        "omml"
    } else {
        "mathtype"
    };
    input.with_file_name(format!("{stem}_{suffix}.docx"))
}

fn default_dir_output(input: &Path, to_omml: bool, upgrade: bool, upgrade_target: UpgradeTargetArg) -> PathBuf {
    let suffix = if upgrade {
        match upgrade_target {
            UpgradeTargetArg::Omml => "upgraded_omml",
            UpgradeTargetArg::Mathtype => "upgraded_mathtype",
        }
    } else if to_omml {
        "omml"
    } else {
        "mathtype"
    };
    // Resolve trailing slashes or `.` if needed
    let resolved = if input.file_name().is_some() {
        input.to_path_buf()
    } else {
        input.canonicalize().unwrap_or_else(|_| input.to_path_buf())
    };
    let name = resolved
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "output".to_owned());
    resolved.with_file_name(format!("{name}_{suffix}"))
}

fn is_docx_file(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("docx"))
}

fn is_ignored_name(path: &Path) -> bool {
    path.file_name()
        .map(|n| n.to_string_lossy())
        .is_some_and(|name| name.starts_with("~$") || name.starts_with('.'))
}

fn collect_docx_files_recursive(
    dir: &Path,
    exclude_dir: Option<&Path>,
    out: &mut Vec<PathBuf>,
) -> std::io::Result<()> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    entries.sort();

    for path in entries {
        if is_ignored_name(&path) {
            continue;
        }
        if let Some(ex) = exclude_dir {
            if path == ex {
                continue;
            }
            if let (Ok(canon_p), Ok(canon_ex)) = (path.canonicalize(), ex.canonicalize()) {
                if canon_p == canon_ex {
                    continue;
                }
            }
        }
        if path.is_dir() {
            collect_docx_files_recursive(&path, exclude_dir, out)?;
        } else if path.is_file() && is_docx_file(&path) {
            out.push(path);
        }
    }
    Ok(())
}

fn convert_single_file(src: &Path, dst: &Path, args: &Args) -> ConvResult<ConversionSummary> {
    if let Some(parent) = dst.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    if args.upgrade {
        let opts = UpgradeOptions {
            target: match args.upgrade_target {
                UpgradeTargetArg::Omml => UpgradeTarget::ModernOmml,
                UpgradeTargetArg::Mathtype => UpgradeTarget::ModernMathType,
            },
            font_pt: args.font_size,
            math_font: args.math_font.clone(),
        };
        upgrade_legacy_mathtype_docx_with(src, dst, &opts)
    } else if args.to_omml {
        let opts = ToOmmlOptions {
            math_font: args.math_font.clone(),
        };
        convert_to_omml_with(src, dst, &opts)
    } else {
        let opts = Options {
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
        convert_with(src, dst, &opts)
    }
}

/// Run the CLI with the process's real stdout / stderr. Returns the exit code.
pub fn run<I, T>(args: I) -> i32
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    run_with_io(args, &mut std::io::stdout(), &mut std::io::stderr())
}

/// Like [`run`], with explicit output streams (used by the tests).
///
/// Exit codes: `0` success, `1` error or any failed equation, `2` bad usage.
pub fn run_with_io<I, T>(args: I, out: &mut dyn Write, err: &mut dyn Write) -> i32
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let args = match Args::try_parse_from(args) {
        Ok(a) => a,
        Err(e) => {
            let text = e.render().to_string();
            if e.use_stderr() {
                let _ = write!(err, "{text}");
            } else {
                let _ = write!(out, "{text}");
            }
            return e.exit_code();
        }
    };

    configure_logging(args.verbose, args.quiet);

    let inp = &args.input;
    if !inp.exists() {
        let _ = writeln!(err, "Error: path not found: {}", inp.display());
        return 1;
    }

    // ── Single-file mode ─────────────────────────────────────────────────────
    if inp.is_file() {
        if !is_docx_file(inp) {
            let _ = writeln!(err, "Error: expected a .docx file, got: {}", inp.display());
            return 1;
        }

        let out_path = match &args.output {
            Some(o) => {
                let s = o.as_os_str().to_string_lossy();
                if o.is_dir() || s.ends_with('/') || s.ends_with('\\') {
                    o.join(inp.file_name().unwrap_or_default())
                } else {
                    o.clone()
                }
            }
            None => default_file_output(inp, args.to_omml, args.upgrade, args.upgrade_target),
        };

        let summary = match convert_single_file(inp, &out_path, &args) {
            Ok(s) => s,
            Err(e) => {
                let _ = writeln!(err, "Error: {e}");
                return 1;
            }
        };

        let _ = writeln!(
            out,
            "Converted {}/{} equation(s) → {}",
            summary.converted,
            summary.found,
            out_path.display()
        );
        if summary.skipped > 0 {
            let _ = writeln!(
                err,
                "  {} skipped (empty or unsupported content)",
                summary.skipped
            );
        }
        if summary.errors > 0 {
            let _ = writeln!(err, "  {} failed — see messages above", summary.errors);
            return 1;
        }
        return 0;
    }

    // ── Directory batch mode (recursive, preserving folder hierarchy) ────────
    if inp.is_dir() {
        let out_dir = args
            .output
            .clone()
            .unwrap_or_else(|| default_dir_output(inp, args.to_omml, args.upgrade, args.upgrade_target));

        // Avoid scanning `out_dir` if it is a subdirectory of `inp` (unless in-place `out_dir == inp`).
        let same_dir = *inp == out_dir
            || (inp.canonicalize().ok().is_some()
                && inp.canonicalize().ok() == out_dir.canonicalize().ok());
        let exclude_dir = if same_dir {
            None
        } else {
            Some(out_dir.as_path())
        };

        let mut docx_files = Vec::new();
        if let Err(e) = collect_docx_files_recursive(inp, exclude_dir, &mut docx_files) {
            let _ = writeln!(
                err,
                "Error: failed to scan directory {}: {e}",
                inp.display()
            );
            return 1;
        }

        if docx_files.is_empty() {
            let _ = writeln!(
                err,
                "Error: no .docx files found in directory: {}",
                inp.display()
            );
            return 1;
        }

        let total_files = docx_files.len();
        let mut files_succeeded = 0usize;
        let mut files_failed = 0usize;
        let mut total_found = 0usize;
        let mut total_converted = 0usize;
        let mut total_skipped = 0usize;
        let mut total_errors = 0usize;

        for (idx, src_file) in docx_files.iter().enumerate() {
            let rel = src_file.strip_prefix(inp).unwrap_or(src_file.as_path());
            let dst_file = out_dir.join(rel);

            if !args.quiet {
                let _ = writeln!(
                    out,
                    "[{}/{}] Processing: {}",
                    idx + 1,
                    total_files,
                    rel.display()
                );
            }

            match convert_single_file(src_file, &dst_file, &args) {
                Ok(summary) => {
                    total_found += summary.found;
                    total_converted += summary.converted;
                    total_skipped += summary.skipped;
                    total_errors += summary.errors;

                    if summary.errors == 0 {
                        files_succeeded += 1;
                    } else {
                        files_failed += 1;
                    }

                    if !args.quiet {
                        let _ = writeln!(
                            out,
                            "  Converted {}/{} equation(s) → {}",
                            summary.converted,
                            summary.found,
                            dst_file.display()
                        );
                    }
                    if summary.skipped > 0 {
                        let _ = writeln!(
                            err,
                            "  {} skipped (empty or unsupported content)",
                            summary.skipped
                        );
                    }
                    if summary.errors > 0 {
                        let _ = writeln!(
                            err,
                            "  {} failed in {} — see messages above",
                            summary.errors,
                            rel.display()
                        );
                    }
                }
                Err(e) => {
                    files_failed += 1;
                    let _ = writeln!(err, "  Error ({}): {e}", rel.display());
                }
            }
        }

        let _ = writeln!(
            out,
            "Batch complete: {}/{} file(s) succeeded, {}/{} equation(s) converted → {}",
            files_succeeded,
            total_files,
            total_converted,
            total_found,
            out_dir.display()
        );
        if total_skipped > 0 {
            let _ = writeln!(err, "  {total_skipped} total equation(s) skipped");
        }
        if files_failed > 0 || total_errors > 0 {
            return 1;
        }
        return 0;
    }

    let _ = writeln!(
        err,
        "Error: unsupported input path: {}",
        inp.display()
    );
    1
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as IoWrite;

    fn make_minimal_docx(omml_body: &str) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buf);
            let opts = zip::write::FileOptions::default();

            zip.start_file("[Content_Types].xml", opts).unwrap();
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#,
            )
            .unwrap();

            zip.start_file("word/document.xml", opts).unwrap();
            let doc_xml = format!(
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math">
  <w:body>
    <w:p>{omml_body}</w:p>
  </w:body>
</w:document>"#
            );
            zip.write_all(doc_xml.as_bytes()).unwrap();
            zip.finish().unwrap();
        }
        buf.into_inner()
    }

    #[test]
    fn test_cli_directory_batch_preserves_hierarchy_both_directions() {
        let temp_root = std::env::temp_dir().join(format!(
            "omml_conv_cli_test_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&temp_root);

        let input_dir = temp_root.join("input_docs");
        let sub_a = input_dir.join("chapter1");
        let sub_b = input_dir.join("chapter2").join("section_a");
        std::fs::create_dir_all(&sub_a).unwrap();
        std::fs::create_dir_all(&sub_b).unwrap();

        let docx_bytes = make_minimal_docx(
            r#"<m:oMath><m:r><m:t>x</m:t></m:r><m:r><m:t>+</m:t></m:r><m:r><m:t>y</m:t></m:r></m:oMath>"#,
        );

        let file_root = input_dir.join("intro.docx");
        let file_sub_a = sub_a.join("lesson1.docx");
        let file_sub_b = sub_b.join("exercises.docx");
        let lock_file = sub_a.join("~$lesson1.docx");

        std::fs::write(&file_root, &docx_bytes).unwrap();
        std::fs::write(&file_sub_a, &docx_bytes).unwrap();
        std::fs::write(&file_sub_b, &docx_bytes).unwrap();
        std::fs::write(&lock_file, b"corrupted lock file").unwrap();

        // 1. Forward batch conversion: input_dir -> mathtype_dir
        let mathtype_dir = temp_root.join("output_mathtype");
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = run_with_io(
            [
                "omml-converter",
                input_dir.to_str().unwrap(),
                "-o",
                mathtype_dir.to_str().unwrap(),
            ],
            &mut out,
            &mut err,
        );
        assert_eq!(code, 0, "stderr: {}", String::from_utf8_lossy(&err));

        // Verify relative hierarchy is preserved and lock file is ignored
        assert!(mathtype_dir.join("intro.docx").is_file());
        assert!(mathtype_dir.join("chapter1").join("lesson1.docx").is_file());
        assert!(mathtype_dir
            .join("chapter2")
            .join("section_a")
            .join("exercises.docx")
            .is_file());
        assert!(!mathtype_dir.join("chapter1").join("~$lesson1.docx").exists());

        // 2. Reverse batch conversion (--to-omml) with default output directory (<dir>_omml)
        out.clear();
        err.clear();
        let code_rev = run_with_io(
            [
                "omml-converter",
                mathtype_dir.to_str().unwrap(),
                "--to-omml",
            ],
            &mut out,
            &mut err,
        );
        assert_eq!(code_rev, 0, "stderr: {}", String::from_utf8_lossy(&err));

        let default_omml_dir = temp_root.join("output_mathtype_omml");
        assert!(default_omml_dir.join("intro.docx").is_file());
        assert!(default_omml_dir
            .join("chapter1")
            .join("lesson1.docx")
            .is_file());
        assert!(default_omml_dir
            .join("chapter2")
            .join("section_a")
            .join("exercises.docx")
            .is_file());

        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn test_cli_upgrade_single_file() {
        let temp_dir = std::env::temp_dir().join(format!("omml_cli_upgrade_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let src_file = temp_dir.join("input.docx");
        let dst_file = temp_dir.join("upgraded.docx");

        let node = crate::mathml::MathNode::with_children("math", vec![crate::mathml::MathNode::leaf("mi", "a")]);
        let preview = crate::render::emf::render_emf(&node, 12.0).unwrap();

        let doc_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
            xmlns:v="urn:schemas-microsoft-com:vml">
  <w:body>
    <w:p>
      <w:r>
        <w:pict>
          <v:shape id="_x0000_i1025" style="width:10pt;height:10pt">
            <v:imagedata r:id="rId1"/>
          </v:shape>
        </w:pict>
      </w:r>
    </w:p>
  </w:body>
</w:document>"#;

        let rels_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image1.emf"/>
</Relationships>"#;

        let ct_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
  <Default Extension="emf" ContentType="image/x-emf"/>
</Types>"#;

        let mut w = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let opts = zip::write::FileOptions::default();
        w.start_file("[Content_Types].xml", opts).unwrap();
        std::io::Write::write_all(&mut w, ct_xml.as_bytes()).unwrap();
        w.start_file("word/document.xml", opts).unwrap();
        std::io::Write::write_all(&mut w, doc_xml.as_bytes()).unwrap();
        w.start_file("word/_rels/document.xml.rels", opts).unwrap();
        std::io::Write::write_all(&mut w, rels_xml.as_bytes()).unwrap();
        w.start_file("word/media/image1.emf", opts).unwrap();
        std::io::Write::write_all(&mut w, &preview.emf).unwrap();
        let bytes = w.finish().unwrap().into_inner();
        std::fs::write(&src_file, bytes).unwrap();

        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = run_with_io(
            [
                "omml-converter",
                src_file.to_str().unwrap(),
                "-o",
                dst_file.to_str().unwrap(),
                "--upgrade",
                "--upgrade-target",
                "mathtype",
            ],
            &mut out,
            &mut err,
        );
        assert_eq!(code, 0, "stderr: {}", String::from_utf8_lossy(&err));
        assert!(dst_file.exists());
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
