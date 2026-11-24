# Tasks

## 1. Dependency Gate & Fallback Decision

- [x] 1.1 Add `hdrshot-egui-backend` (`0.2`) and `softbuffer` (`0.4`) to `desktop/Cargo.toml` and update `Cargo.lock`; verify `cargo deny --all-features --workspace --locked check advisories licenses bans sources` passes across all workspace targets (if it fails, revert the new dependencies and pivot to removing the ineffective VM throttling workarounds in Section 3 instead of Section 2).

## 2. Software Renderer Integration (when Dependency Gate 1.1 passes)

- [x] 2.1 Implement the `SoftwarePainter` backend in `desktop/src/` wrapping `softbuffer::Context`/`Surface` and `hdrshot_egui_backend::EguiSoftwareRender` (`ColorFieldOrder::Bgra`) while preserving `#![forbid(unsafe_code)]`; verify `cargo check -p eqntide-desktop` and `cargo check -p eqntide-desktop --target x86_64-pc-windows-msvc` succeed.
- [x] 2.2 Probe graphics adapter capabilities in `desktop/src/main.rs` before creating a `wgpu` swapchain so `DeviceType::Cpu` / software adapters and `--software` / `EQNTIDE_RENDERER=software` route to `SoftwarePainter`, while hardware `wgpu` uses `PresentMode::AutoNoVsync` (unless overridden) and `desired_maximum_frame_latency = Some(1)`; add unit tests for renderer selection precedence and verify `cargo test -p eqntide-desktop` passes.
- [x] 2.3 Generalize `desktop/src/wgpu_runner.rs` to drive both `Wgpu` and `Software` painters with `egui_winit`, AccessKit, HiDPI `pixels_per_point`, resize/minimize/occlusion guards, and unchanged-shape skip; verify interactive rendering under `EQNTIDE_RENDERER=software` in local X11/Xvfb smoke check.

## 3. Remove Ineffective VM Throttling Workarounds & Update Docs

- [x] 3.1 Remove `HOVER_SETTLE_DELAY`, `software_frame_cap` / `frame_cap_for_profile`, `passive_hover_rect` suppression, and `RequestRepaint` delay clamping from `desktop/src/wgpu_runner.rs` and `desktop/src/app.rs` (keeping zero-cost optimizations such as `PaletteAtlas`, `last_shapes == shapes` skip, and `WindowEvent::Moved` ignore); update affected unit tests in `desktop/src/app.rs` and verify `cargo test --workspace` passes.
- [x] 3.2 Update `KNOWN_ISSUES.md` (and `desktop/VALIDATION.md` if needed) to reflect either the software-renderer resolution or, if Task 1.1 triggered the fallback, the retained DX12 WARP limitation without throttling delays; verify `openspec validate desktop-software-renderer` and `cargo clippy --workspace --all-targets -- -D warnings` pass.

## 4. Final Workspace Verification

- [x] 4.1 Run full workspace checks (`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo deny --all-features --workspace --locked check advisories licenses bans sources`, and `cargo check -p eqntide-desktop --target x86_64-pc-windows-msvc` if target installed) and verify zero errors.
