# Proposal

## Why

Copying or cutting content from `eqntide-desktop` cannot be pasted into external applications on Linux, and copying text from external applications cannot be pasted into `eqntide-desktop`. Both `eframe` and `egui-winit` in `desktop/Cargo.toml` specify `default-features = false` without enabling the `"clipboard"` feature, so `egui_winit::clipboard::Clipboard` compiles out both `smithay-clipboard` (Wayland) and `arboard` (X11 / Windows / macOS) and falls back to an in-process `String` buffer.

## What Changes

- Enable the `"clipboard"` feature on `eframe` and `egui-winit` in `desktop/Cargo.toml` and update `Cargo.lock` so `egui_winit::State` connects to the OS clipboard on Linux Wayland (`smithay-clipboard`) and Linux X11 (`arboard`), as well as Windows and macOS, across both `wgpu`/software (`wgpu_runner.rs`) and Glow (`eframe::run_native`) backends.
- Verify that the newly activated clipboard dependency graph (`smithay-clipboard`, `arboard`, and their transitive crates) satisfies `cargo-deny` (`advisories`, `licenses`, `bans`, `sources`) across all configured desktop targets.
- Add automated regression tests in `eqntide-desktop` verifying that `desktop/Cargo.toml` enables `"clipboard"` on `eframe` and `egui-winit`, `Cargo.lock` resolves `arboard` and `smithay-clipboard` under `egui-winit`, and desktop Copy/Cut/Paste flows emit `OutputCommand::CopyText` and apply `Event::Paste` as expected.

## Capabilities

### New Capabilities

<!-- None -->

### Modified Capabilities

- `desktop-canvas-editing`: Require Equation Canvas and desktop editor clipboard operations (`Copy`, `Cut`, `Paste`) to exchange text with the active operating-system clipboard (including Linux Wayland and X11) rather than an in-process fallback buffer.

## Impact

- Affected files: `desktop/Cargo.toml`, `Cargo.lock`, `desktop/src/app/tests/` (and `deny.toml` only if `cargo-deny` flags an approved transitive exception).
- Affected components: `eqntide-desktop` `wgpu`/software runner (`desktop/src/wgpu_runner.rs`), Glow fallback (`desktop/src/main.rs`), Equation Canvas (`desktop/src/app/keyboard.rs`), `LaTeX source` editor (`desktop/src/app/canvas.rs`), and inline `\` command popup (`egui::TextEdit`).
- Dependencies: Activates `egui-winit/clipboard` (`arboard` + `smithay-clipboard`, plus `arboard/wayland-data-control` via `egui-winit/wayland`).
