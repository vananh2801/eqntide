# Proposal

## Why

In the desktop **About** tab, clicking **Open release v...**, **Download v...**, **Release notes**, **Open Releases**, or the **GitHub** repository hyperlink produces no visible action. Both `eframe` and `egui-winit` in `desktop/Cargo.toml` set `default-features = false` without enabling the `"links"` feature, so `egui-winit` compiles `open_url_in_browser` as a no-op warning instead of invoking the system browser.

## What Changes

- Enable the `"links"` feature on `eframe` and `egui-winit` in `desktop/Cargo.toml` and update `Cargo.lock` so `egui_winit::State::handle_platform_output_with_event_loop` opens `OutputCommand::OpenUrl` requests in the user's system browser.
- Verify that the `webbrowser` dependency graph pulled by `"links"` satisfies `cargo-deny` (`advisories`, `licenses`, `bans`, `sources`) across all configured desktop targets.
- Expand automated verification in `eqntide-desktop` to cover the About **GitHub** repository hyperlink and verify `egui-winit` browser-link support (`cfg` / feature wiring) so silent no-op regressions are caught.

## Capabilities

### New Capabilities

<!-- None -->

### Modified Capabilities

- `desktop-update-check`: Clarify that About browser actions—including **Open release** / **Download**, **Release notes**, **Open Releases**, and the About **GitHub** repository hyperlink—must actually dispatch `OpenUrl` to an enabled system browser handler rather than silently no-oping when `egui-winit` runs with minimal features.

## Impact

- Affected files: `desktop/Cargo.toml`, `Cargo.lock`, `desktop/src/app/tests/update_tests.rs` (and `deny.toml` only if `cargo-deny` flags any new license/advisory exception).
- Affected components: `eqntide-desktop` `wgpu`/software runner (`wgpu_runner.rs`) and `eframe` Glow fallback (`main.rs`), both of which rely on `egui-winit` for `OutputCommand::OpenUrl`.
- Dependencies: Adds `webbrowser` (transitive via `egui-winit/links`).
