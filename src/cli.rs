//! Command-line interface for `omml-converter`.

use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};

use clap::Parser;

use crate::docx::{convert_with, EmbedMode, Options, PreviewFormat};

/// Convert OMML equations in a .docx to MathType OLE objects.
#[derive(Parser, Debug)]
#[command(
    name = "omml-converter",
    version,
    about = "Convert OMML equations in a .docx to MathType OLE objects."
)]
struct Args {
    /// Input .docx file
    input: PathBuf,

    /// Output .docx path (default: <input>_mathtype.docx)
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,

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

fn default_output(input: &Path) -> PathBuf {
    let stem = input
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    input.with_file_name(format!("{stem}_mathtype.docx"))
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
        let _ = writeln!(err, "Error: file not found: {}", inp.display());
        return 1;
    }
    let is_docx = inp
        .extension()
        .map(|e| e.eq_ignore_ascii_case("docx"))
        .unwrap_or(false);
    if !is_docx {
        let _ = writeln!(err, "Error: expected a .docx file, got: {}", inp.display());
        return 1;
    }

    let out_path = args.output.clone().unwrap_or_else(|| default_output(inp));

    let opts = Options {
        mode: if args.omml_fallback { EmbedMode::OmmlFallback } else { EmbedMode::Preview },
        font_pt: args.font_size,
        preview: match args.preview {
             PreviewArg::Emf => PreviewFormat::Emf,
             PreviewArg::Png => PreviewFormat::Png,
        },
    };
    let summary = match convert_with(inp, &out_path, &opts) {
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
    0
}
