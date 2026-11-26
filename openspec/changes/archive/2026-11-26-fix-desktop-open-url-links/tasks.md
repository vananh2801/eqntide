# Tasks

## 1. Enable Browser Link Handling in Desktop Manifest

- [x] 1.1 Add `"links"` to the `features` list of both `eframe` and `egui-winit` in `desktop/Cargo.toml`, update `Cargo.lock` (`cargo update -p eqntide-desktop` or `cargo check -p eqntide-desktop`), and verify `cargo tree -e features -p eqntide-desktop -i egui-winit --locked` shows `links` and `webbrowser` enabled.
- [x] 1.2 Run `cargo deny --all-features --workspace --locked check advisories licenses bans sources` and confirm `webbrowser` and its transitive dependencies pass `deny.toml` across all desktop targets (updating `deny.toml` only if an approved transitive exception is required).

## 2. Automated Tests for About Links and Feature Wiring

- [x] 2.1 Extend `desktop/src/app/tests/update_tests.rs` to verify that clicking the **GitHub** repository hyperlink in the **About** tab emits `egui::OutputCommand::OpenUrl` with `env!("CARGO_PKG_REPOSITORY")`, alongside **Open release** / **Download**, **Release notes**, and **Open Releases**, and verify `cargo test -p eqntide-desktop --locked update_tests` passes.
- [x] 2.2 Add a regression test in `desktop/src/app/tests/update_tests.rs` (or `desktop/Cargo.toml` feature assertion) verifying `egui-winit` is resolved with the `links` / `webbrowser` feature for `eqntide-desktop` so `open_url_in_browser` is active, and verify it passes under `cargo test -p eqntide-desktop --locked`.

## 3. Workspace Pre-commit Verification

- [x] 3.1 Run the full pre-commit verification suite (`cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --all-targets --locked`, and `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`) and confirm zero warnings or failures, plus manually smoke-test clicking **GitHub** and **Open release** in the desktop **About** tab.
