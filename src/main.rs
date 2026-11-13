//! CLI binary entry point.
//!
//! Command-line argument parsing and conversion orchestration live in
//! [`eqntide::cli`]. Keeping this binary to process setup makes the same
//! conversion pipeline available to the library and desktop application.

fn main() {
    std::process::exit(eqntide::cli::run(std::env::args_os()));
}
