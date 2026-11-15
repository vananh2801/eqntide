//! Command-line interface for `eqntide-cli`.

use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;

use clap::Parser;

mod conversion;
mod dispatch;
mod paths;

use conversion::convert_single_file;
use dispatch::run_conversion;
use paths::{collect_docx_files_recursive, default_dir_output, default_file_output, is_docx_file};

/// Convert equations in a .docx file or directory between OMML and MathType OLE objects.
#[derive(Parser, Debug)]
#[command(
    name = "eqntide-cli",
    version,
    about = "Convert equations in a .docx file or directory (recursively) between OMML and MathType OLE objects."
)]
pub(super) struct Args {
    /// Input .docx file or directory containing .docx files (scanned recursively)
    #[arg(required_unless_present = "server")]
    input: Option<PathBuf>,

    /// Start the local loopback HTTP bridge server (127.0.0.1:17890) for Office/WPS Add-ins and the native desktop
    #[arg(long, short = 's')]
    server: bool,

    /// Host IP address to bind the bridge server to (default: 127.0.0.1)
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Port to bind the bridge server to (default: 17890)
    #[arg(long, default_value_t = 17890)]
    port: u16,

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
pub(super) enum PreviewArg {
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

fn log_level(verbose: bool, quiet: bool) -> log::LevelFilter {
    if quiet {
        log::LevelFilter::Error
    } else if verbose {
        log::LevelFilter::Info
    } else {
        log::LevelFilter::Warn
    }
}

fn configure_logging(verbose: bool, quiet: bool) {
    // set_logger fails if already installed (e.g. repeated calls in tests); fine.
    let _ = log::set_logger(&LOGGER);
    log::set_max_level(log_level(verbose, quiet));
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

    // ── Server Mode ───────────────────────────────────────────────────────────
    if args.server {
        let addr = format!("{}:{}", args.host, args.port);
        let _ = writeln!(out, "Starting local bridge server on http://{addr} ...");
        match crate::bridge::start_bridge_server(&addr) {
            Ok(handle) => {
                let _ = writeln!(
                    out,
                    "Bridge server running on http://{addr}. Press Ctrl+C to stop."
                );
                while handle.addr().port() != 0 {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }
                return 0;
            }
            Err(e) => {
                let _ = writeln!(err, "Error: failed to start bridge server: {e}");
                return 1;
            }
        }
    }

    run_conversion(&args, out, err)
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
        let temp_root =
            std::env::temp_dir().join(format!("omml_conv_cli_test_{}", std::process::id()));
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
                "eqntide-cli",
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
        assert!(!mathtype_dir
            .join("chapter1")
            .join("~$lesson1.docx")
            .exists());

        // 2. Reverse batch conversion (--to-omml) with default output directory (<dir>_omml)
        out.clear();
        err.clear();
        let code_rev = run_with_io(
            ["eqntide-cli", mathtype_dir.to_str().unwrap(), "--to-omml"],
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
        let temp_dir =
            std::env::temp_dir().join(format!("omml_cli_upgrade_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let src_file = temp_dir.join("input.docx");
        let dst_file = temp_dir.join("upgraded.docx");

        let node = crate::mathml::MathNode::with_children(
            "math",
            vec![crate::mathml::MathNode::leaf("mi", "a")],
        );
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
                "eqntide-cli",
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

    #[test]
    fn test_cli_skips_unsupported_equations_without_failing() {
        let temp_dir =
            std::env::temp_dir().join(format!("eqntide_cli_skip_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        // 1. OMML -> MathType skipping strikeV
        let src_omml = temp_dir.join("omml_in.docx");
        let dst_mt = temp_dir.join("mt_out.docx");
        let docx_bytes = make_minimal_docx(
            r#"<m:oMath><m:r><m:t>x</m:t></m:r><m:r><m:t>+</m:t></m:r><m:r><m:t>1</m:t></m:r></m:oMath><m:oMath><m:borderBox><m:borderBoxPr><m:hideTop m:val="1"/><m:hideBot m:val="1"/><m:hideLeft m:val="1"/><m:hideRight m:val="1"/><m:strikeV m:val="1"/></m:borderBoxPr><m:e><m:r><m:t>y</m:t></m:r></m:e></m:borderBox></m:oMath>"#,
        );
        std::fs::write(&src_omml, docx_bytes).unwrap();

        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = run_with_io(
            [
                "eqntide-cli",
                src_omml.to_str().unwrap(),
                "-o",
                dst_mt.to_str().unwrap(),
            ],
            &mut out,
            &mut err,
        );
        assert_eq!(code, 0, "stderr: {}", String::from_utf8_lossy(&err));
        assert!(String::from_utf8_lossy(&out).contains("Converted 1/2 equation(s)"));
        assert!(String::from_utf8_lossy(&err).contains("1 skipped"));

        // 2. MathType -> OMML skipping longdiv
        let longdiv = crate::service::parse_latex_to_mathml(r"\enclose{longdiv}{144}").unwrap();
        let eq_bytes = crate::mtef::encode::mathml_to_mtef_bytes(&longdiv).unwrap();
        let mtef = crate::mtef::build_mtef(&eq_bytes);
        let preview = crate::render::emf::render_emf_with_mtef(&longdiv, 12.0, Some(&mtef)).unwrap();
        let src_mt = temp_dir.join("mt_in.docx");
        let dst_omml = temp_dir.join("omml_out.docx");
        {
            let mut w = zip::ZipWriter::new(std::fs::File::create(&src_mt).unwrap());
            let opts = zip::write::FileOptions::default();
            w.start_file("[Content_Types].xml", opts).unwrap();
            IoWrite::write_all(&mut w, br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
  <Default Extension="emf" ContentType="image/x-emf"/>
</Types>"#).unwrap();
            w.start_file("word/document.xml", opts).unwrap();
            IoWrite::write_all(&mut w, br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
            xmlns:v="urn:schemas-microsoft-com:vml">
  <w:body><w:p><w:r><w:pict><v:shape id="_x0000_i1025" style="width:10pt;height:10pt"><v:imagedata r:id="rId1"/></v:shape></w:pict></w:r></w:p></w:body>
</w:document>"#).unwrap();
            w.start_file("word/_rels/document.xml.rels", opts).unwrap();
            IoWrite::write_all(&mut w, br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/equation1.emf"/>
</Relationships>"#).unwrap();
            w.start_file("word/media/equation1.emf", opts).unwrap();
            IoWrite::write_all(&mut w, &preview.emf).unwrap();
            w.finish().unwrap();
        }
        out.clear();
        err.clear();
        let code_rev = run_with_io(
            [
                "eqntide-cli",
                src_mt.to_str().unwrap(),
                "-o",
                dst_omml.to_str().unwrap(),
                "--to-omml",
            ],
            &mut out,
            &mut err,
        );
        assert_eq!(code_rev, 0, "stderr: {}", String::from_utf8_lossy(&err));
        assert!(String::from_utf8_lossy(&out).contains("Converted 0/1 equation(s)"));
        assert!(String::from_utf8_lossy(&err).contains("1 skipped"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
