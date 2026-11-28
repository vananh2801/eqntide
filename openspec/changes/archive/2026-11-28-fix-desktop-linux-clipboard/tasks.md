# Tasks

## 1. Enable OS Clipboard Feature in Desktop Manifest

- [x] 1.1 Add `"clipboard"` to the `features` list of both `eframe` and `egui-winit` in `desktop/Cargo.toml`, update `Cargo.lock`, and verify `cargo tree -e features -p eqntide-desktop -i egui-winit --locked` shows `clipboard`, `arboard`, and `smithay-clipboard` enabled.
- [x] 1.2 Run `cargo deny --all-features --workspace --locked check advisories licenses bans sources` and verify `smithay-clipboard`, `arboard`, and their transitive dependencies pass `deny.toml` across all configured desktop targets.

## 2. Regression Tests for Clipboard Wiring and Canvas Events

- [x] 2.1 Add a manifest and lockfile wiring regression test in `desktop/src/app/tests/` (`clipboard_tests.rs`) verifying `desktop/Cargo.toml` enables `"clipboard"` on `eframe` and `egui-winit` and `Cargo.lock` resolves `arboard` and `smithay-clipboard` under `egui-winit`, and verify it passes with `cargo test -p eqntide-desktop --locked`.
- [x] 2.2 Add Equation Canvas and `LaTeX source` clipboard tests in `desktop/src/app/tests/` (`clipboard_tests.rs`) covering `Event::Copy` (`OutputCommand::CopyText`), `Event::Cut`, and `Event::Paste` (plain text, LaTeX, and MathML), and verify `cargo test -p eqntide-desktop --locked clipboard_tests` and `canvas_input_tests` pass.

## 3. Workspace Pre-Commit Checks and Linux Manual Smoke Test

- [x] 3.1 Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --all-targets --locked`, and `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`, confirming zero warnings or failures.
- [x] 3.2 Smoke-test `eqntide-desktop` on Linux (Wayland and `WINIT_UNIX_BACKEND=x11` if available) by copying an equation/LaTeX string from EqnTide into an external app and copying text/LaTeX from an external app into EqnTide.
