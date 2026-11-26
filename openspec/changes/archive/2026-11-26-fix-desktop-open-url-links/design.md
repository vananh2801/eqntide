# Design

## Context

See [proposal.md](proposal.md) for motivation.

In `desktop/src/app/about_ui.rs`, both `ui.hyperlink(env!("CARGO_PKG_REPOSITORY"))` and the update buttons (**Open release v...**, **Download v...**, **Release notes**, **Open Releases**) call `ui.ctx().open_url(...)`, emitting `egui::OutputCommand::OpenUrl`. Both the primary `wgpu`/software event loop (`desktop/src/wgpu_runner.rs`) and the `eframe` Glow fallback (`desktop/src/main.rs`) forward `platform_output` to `egui_winit::State::handle_platform_output_with_event_loop`.

In `egui-winit` 0.37.0, `open_url_in_browser` is compiled with `#[cfg(feature = "webbrowser")]` (enabled by `egui-winit`'s `"links"` feature). Because `desktop/Cargo.toml` sets `default-features = false` on both `eframe` and `egui-winit` without `"links"`, `open_url_in_browser` currently compiles to a `log::warn!` no-op.

Existing tests in `desktop/src/app/tests/update_tests.rs` run `egui::Context` headless and assert only on `FullOutput.platform_output.commands`, which cannot catch a disabled `egui-winit` `"links"` feature.

## Goals / Non-Goals

**Goals:**
- Enable functional browser URL opening for every `OutputCommand::OpenUrl` trigger in `eqntide-desktop` (all About update buttons and the `GitHub:` hyperlink) across `wgpu`, `software`, and `glow` backends.
- Preserve `#![forbid(unsafe_code)]` and satisfy `cargo-deny` (`advisories`, `licenses`, `bans`, `sources`) across all desktop target triples.
- Add test coverage verifying both the About **GitHub** hyperlink `OpenUrl` emission and compile-time/feature wiring of `egui-winit`'s browser-link support.

**Non-Goals:**
- Adding custom in-app error banners for browser launch failures (upstream `egui-winit` logs launch errors via `log::warn!`).
- Changing the URL allowlist (`update::is_trusted_release_url`) or auto-updater behavior.

## Decisions

### Decision 1: Enable `"links"` on `eframe` and `egui-winit` in `desktop/Cargo.toml`
- **Choice:** Add `"links"` to the `features` list of both `eframe` and `egui-winit` in `desktop/Cargo.toml`, and update `Cargo.lock`.
- **Rationale:** Matches upstream `egui-winit` defaults for desktop link handling, automatically fixes both `wgpu_runner` and `eframe` Glow, covers `ui.hyperlink` as well as `ui.ctx().open_url`, and delegates Windows/macOS/Linux browser invocation quirks to `webbrowser` without adding `unsafe` code.
- **Alternative considered:** Intercepting `OutputCommand::OpenUrl` in `wgpu_runner.rs` and spawning OS commands directly (similar to `runtime::open_folder`). Rejected because it leaves `eframe` Glow broken unless duplicated or bypassed, and Windows URL launching via `cmd /c start` or `explorer` introduces quoting/escaping pitfalls that `webbrowser` already handles.

### Decision 2: Verify `OpenUrl` and `links` feature wiring in tests
- **Choice:** Extend `desktop/src/app/tests/update_tests.rs` to assert that clicking the **GitHub** hyperlink in About emits `OutputCommand::OpenUrl` with `env!("CARGO_PKG_REPOSITORY")`, and verify via Cargo feature/metadata or a targeted unit test that `egui-winit` has `"links"`/`webbrowser` enabled in `eqntide-desktop` builds.
- **Rationale:** Headless `egui::Context` tests alone missed the disabled `egui-winit` feature; keeping an explicit assertion prevents future `default-features = false` edits from silently breaking URL opening again.

## Risks / Trade-offs

- **New transitive dependency graph (`webbrowser`)** → Run `cargo deny --all-features --workspace --locked check advisories licenses bans sources` after updating `Cargo.lock` to confirm all licenses and advisories pass `deny.toml` across Linux, Windows, and macOS targets.
- **Locked CI builds (`--locked`)** → Regenerate `Cargo.lock` alongside `desktop/Cargo.toml` and verify all workspace `--locked` checks pass.
