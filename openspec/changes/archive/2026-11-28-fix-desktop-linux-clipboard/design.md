# Design

## Context

See `proposal.md` for motivation. Both `eframe = "0.37"` and `egui-winit = "0.37"` in `desktop/Cargo.toml` specify `default-features = false` so the desktop crate controls optional platform features explicitly. When `egui-winit` was added for `wgpu_runner.rs` and later updated for `"links"`, the `"clipboard"` feature was omitted on both crates.

In `egui-winit 0.37.0`:
- `clipboard` enables `arboard`, `bytemuck`, and (on Linux/BSD) `smithay-clipboard`.
- `wayland` already sets `arboard?/wayland-data-control`, which activates `arboard`'s Wayland data-control support once `arboard` is enabled by `clipboard`.
- `egui_winit::State::new` receives `event_loop` (`&dyn HasDisplayHandle`), initializes `smithay-clipboard` when `RawDisplayHandle::Wayland` is present, and initializes `arboard` for X11/Windows/macOS (and as read fallback on Wayland when `smithay-clipboard` returns `NotFound` or an error).
- Both `wgpu_runner.rs` (`Wgpu` / `Software`) and `eframe::run_native` (`Glow`) rely on `egui_winit::State` for `OutputCommand::CopyText`, `clipboard_text()`, and `Ctrl+C`/`Ctrl+X`/`Ctrl+V` translation.

## Goals / Non-Goals

**Goals:**
- Enable OS clipboard read/write across Linux Wayland, Linux X11, Windows, and macOS in all desktop renderer paths (`Wgpu`, `Software`, `Glow`).
- Keep `cargo-deny` (`advisories`, `licenses`, `bans`, `sources`) clean across all four desktop target triples.
- Add automated regression guards so future edits to `desktop/Cargo.toml` cannot silently drop `"clipboard"` again.

**Non-Goals:**
- Implementing an out-of-process clipboard persistence daemon after EqnTide exits on Linux (standard X11/Wayland clients rely on the desktop environment's clipboard manager once closed).
- Adding image copy/paste support to the Equation Canvas or changing canvas LaTeX/MathML paste parsing rules in `desktop/src/app/keyboard.rs`.

## Decisions

1. **Enable `"clipboard"` on both `eframe` and `egui-winit` in `desktop/Cargo.toml`**
   - *Rationale*: `wgpu_runner.rs` instantiates `egui_winit::State` directly, while the `--glow` fallback runs through `eframe::run_native`. Enabling `"clipboard"` on both keeps feature resolution consistent across all renderer paths and mirrors how `"links"` was wired in `desktop/Cargo.toml`.
   - *Alternative considered*: Calling `arboard` directly from `App` / `wgpu_runner.rs` instead of enabling `egui-winit/clipboard`. Rejected because `egui::TextEdit` (used by `LaTeX source`, the `\` command popup, and Settings search) already goes through `egui_winit::State` (`OutputCommand::CopyText` and `Event::Paste`), and `arboard` alone without `smithay-clipboard` fails on Wayland compositors that do not expose `wlr-data-control` (such as GNOME Mutter).

2. **Add feature-wiring and clipboard event regression tests**
   - *Rationale*: Headless unit tests in `desktop/src/app/tests/` cannot reliably mutate a real OS clipboard in CI, so we pair behavioral tests of Equation Canvas / `LaTeX source` `Event::Copy`/`Cut`/`Paste` handling with a manifest + `Cargo.lock` wiring guard (following the pattern in `desktop/src/app/tests/update_tests.rs`).
   - *Alternative considered*: Live OS clipboard round-trip integration test in `cargo test`. Rejected as flaky in headless Linux CI environments without a running Wayland/X11 display server.

## Risks / Trade-offs

- **New transitive crates from `smithay-clipboard 0.7.2` / `arboard 3.7.0`** -> Run `cargo deny --all-features --workspace --locked check advisories licenses bans sources` across all configured desktop targets and inspect any flagged transitive dependency before finalizing.
- **Locked CI builds (`--locked`)** -> Regenerate `Cargo.lock` alongside `desktop/Cargo.toml` and verify `--locked` workspace commands pass.
- **Wayland / X11 runtime behavior** -> Manually smoke-test copying and pasting both directions between `eqntide-desktop` and an external application under Wayland (and `WINIT_UNIX_BACKEND=x11` if available).
