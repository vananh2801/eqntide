//! File and directory conversion dispatch for the CLI.

use std::io::Write;

use super::{
    collect_docx_files_recursive, convert_single_file, default_dir_output, default_file_output,
    is_docx_file, Args,
};

fn build_job_pool(jobs: Option<usize>) -> Option<rayon::ThreadPool> {
    jobs.and_then(|n| rayon::ThreadPoolBuilder::new().num_threads(n).build().ok())
}

fn run_in_pool<R: Send>(pool: Option<&rayon::ThreadPool>, f: impl FnOnce() -> R + Send) -> R {
    match pool {
        Some(p) => p.install(f),
        None => f(),
    }
}

pub(super) fn run_conversion(args: &Args, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let Some(input) = args.input.as_deref() else {
        let _ = writeln!(err, "Error: missing input file or directory");
        return 2;
    };
    if !input.exists() {
        let _ = writeln!(err, "Error: path not found: {}", input.display());
        return 1;
    }
    if input.is_file() {
        return run_single_file(args, input, out, err);
    }
    if input.is_dir() {
        return run_directory(args, input, out, err);
    }
    let _ = writeln!(err, "Error: unsupported input path: {}", input.display());
    1
}

fn run_single_file(
    args: &Args,
    input: &std::path::Path,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    if !is_docx_file(input) {
        let _ = writeln!(
            err,
            "Error: expected a .docx file, got: {}",
            input.display()
        );
        return 1;
    }
    let output = match &args.output {
        Some(output) => {
            let value = output.as_os_str().to_string_lossy();
            if output.is_dir() || value.ends_with('/') || value.ends_with('\\') {
                output.join(input.file_name().unwrap_or_default())
            } else {
                output.clone()
            }
        }
        None => default_file_output(input, args.to_omml, args.upgrade, args.upgrade_target),
    };
    let pool = build_job_pool(args.jobs);
    let summary = match run_in_pool(pool.as_ref(), || convert_single_file(input, &output, args)) {
        Ok(summary) => summary,
        Err(error) => {
            let _ = writeln!(err, "Error: {error}");
            return 1;
        }
    };
    let _ = writeln!(
        out,
        "Converted {}/{} equation(s) → {}",
        summary.converted,
        summary.found,
        output.display()
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

fn run_directory(
    args: &Args,
    input: &std::path::Path,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let output_dir = args.output.clone().unwrap_or_else(|| {
        default_dir_output(input, args.to_omml, args.upgrade, args.upgrade_target)
    });
    let same_dir = input == output_dir
        || (input.canonicalize().ok().is_some()
            && input.canonicalize().ok() == output_dir.canonicalize().ok());
    let exclude_dir = (!same_dir).then_some(output_dir.as_path());
    let mut files = Vec::new();
    if let Err(error) = collect_docx_files_recursive(input, exclude_dir, &mut files) {
        let _ = writeln!(
            err,
            "Error: failed to scan directory {}: {error}",
            input.display()
        );
        return 1;
    }
    if files.is_empty() {
        let _ = writeln!(
            err,
            "Error: no .docx files found in directory: {}",
            input.display()
        );
        return 1;
    }
    let total_files = files.len();
    let pool = build_job_pool(args.jobs);
    let mut files_succeeded = 0usize;
    let mut files_failed = 0usize;
    let mut total_found = 0usize;
    let mut total_converted = 0usize;
    let mut total_skipped = 0usize;
    let mut total_errors = 0usize;

    for (index, source) in files.iter().enumerate() {
        let relative = source.strip_prefix(input).unwrap_or(source.as_path());
        let destination = output_dir.join(relative);
        if !args.quiet {
            let _ = writeln!(
                out,
                "[{}/{}] Processing: {}",
                index + 1,
                total_files,
                relative.display()
            );
        }
        let outcome = run_in_pool(pool.as_ref(), || {
            convert_single_file(source, &destination, args)
        });
        match outcome {
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
                        destination.display()
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
                        relative.display()
                    );
                }
            }
            Err(error) => {
                files_failed += 1;
                let _ = writeln!(err, "  Error ({}): {error}", relative.display());
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
        output_dir.display()
    );
    if total_skipped > 0 {
        let _ = writeln!(err, "  {total_skipped} total equation(s) skipped");
    }
    if files_failed > 0 || total_errors > 0 {
        1
    } else {
        0
    }
}
