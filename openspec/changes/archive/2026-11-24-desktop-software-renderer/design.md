# Design

## Context

See `proposal.md` for motivation. Currently `desktop/src/main.rs` defaults Windows to `eframe::Renderer::Wgpu` (because Windows VMs without 3D guest drivers only expose OpenGL 1.1, which fails `egui_glow`'s OpenGL 2.0+ check) and routes `Wgpu` into `desktop/src/wgpu_runner.rs`. On a VM without GPU acceleration, `wgpu` selects DX12 `Microsoft Basic Render Driver` (WARP). Every UI change (`last_shapes != shapes`) forces `egui_wgpu::winit::Painter` to execute a full-window DX12 render pass on CPU via WARP, while `wgpu_runner.rs` adds `HOVER_SETTLE_DELAY` (45 ms), `frame_cap` (66 ms), and `RequestRepaint` clamping (+33–66 ms) on top of `PresentMode::AutoVsync` (`desired_maximum_frame_latency = Some(2)`).

All workspace code in `desktop/src/main.rs` enforces `#![forbid(unsafe_code)]`, and `deny.toml` enforces `unmaintained = "all"`, `unsound = "all"`, and strict permissive licenses across Linux, Windows, and macOS target triples.

## Goals / Non-Goals

**Goals:**
- Bypass DX12 WARP (`Microsoft Basic Render Driver`) and Linux `llvmpipe`/`lavapipe` completely by routing `DeviceType::Cpu` environments to a pure-CPU 2D `egui` rasterizer (`hdrshot-egui-backend`) + native 2D surface presenter (`softbuffer`, which uses Win32 GDI `BitBlt` / `DIBSection` on Windows).
- Eliminate artificial input and repaint delays (`HOVER_SETTLE_DELAY`, `software_frame_cap`, `RequestRepaint` delay clamping) and reduce `wgpu` swapchain latency (`AutoNoVsync`, `desired_maximum_frame_latency = Some(1)`).
- Enforce an early `cargo-deny` verification gate right after updating `Cargo.toml`/`Cargo.lock`, with a pre-agreed fallback to removing the ineffective VM throttling workarounds if `cargo-deny` fails.

**Non-Goals:**
- Modifying `egui` itself to support partial/dirty-rect layout invalidation (beyond skipping unchanged frames and letting the CPU 2D rasterizer execute efficiently).
- Replacing `wgpu` on machines with real hardware GPUs or 3D-accelerated `VirtualGpu` drivers.
- Writing custom `unsafe` Win32/X11/Wayland FFI code inside `eqntide-desktop` (`#![forbid(unsafe_code)]` stays intact).

## Decisions

### 1. Renderer Abstraction in `desktop/src/wgpu_runner.rs` (renamed/generalized runner)
Rather than duplicating the `winit` + `egui_winit` + AccessKit + single-instance event loop, generalize the custom `winit` runner to abstract the paint backend behind an enum (`Wgpu(egui_wgpu::winit::Painter)` vs `Software(SoftwarePainter)`):
- **Probe before creating a swapchain:** At startup (before constructing a `wgpu` surface or after probing with `compatible_surface: None`), inspect adapter info (`device_type == DeviceType::Cpu` or known software adapter names). Wait: on Windows headless/surface-less probes, `Instance::request_adapter(&RequestAdapterOptions { compatible_surface: None, .. })` enumerates the same primary DX12 adapter (`Microsoft Basic Render Driver` when no hardware DX12 GPU exists).
- **Alternative considered:** Probing after `Painter::new` + `set_window` and tearing down `Painter` if `DeviceType::Cpu`. Creating and immediately destroying a DX12 WARP swapchain adds startup latency and can leave DWM swapchain state on the HWND; probing before `Painter::new` avoids initializing WARP altogether (with a runtime fallback if `set_window` ever resolves to a CPU adapter).
- **Alternative considered:** Using `tiny-skia` instead of `hdrshot-egui-backend`. Rejected because `tiny-skia` does not natively rasterize `epaint::Mesh` (textured/vertex-colored triangles used throughout `egui`) without significant custom triangulation/texturing code. `hdrshot-egui-backend` (`v0.2.0`) is a direct `egui 0.37` port of `egui_software_backend` designed specifically for `egui` + `softbuffer`.

### 2. Removing Ineffective VM Throttling Workarounds
Remove `HOVER_SETTLE_DELAY`, `software_frame_cap` / `frame_cap_for_profile`, `passive_hover_rect` cursor suppression, and `RequestRepaint` clamping (`cap / 2` and `next_throttled_frame`) from the event loop:
- **Rationale:** On the CPU 2D software backend, delaying hover by 45 ms and capping frames at 15 FPS creates visible input freeze rather than responsiveness. On hardware GPUs, throttling is unused anyway.
- Keep genuinely useful zero-cost optimizations from `app.rs` (`PaletteAtlas`, texture handle reuse, `warning_cache`, `show_rows` batch virtualization, `last_shapes == shapes` unchanged-frame skip, `WindowEvent::Moved` ignore, `DeviceEvents::Never`, and disabling heavy shadows/animations/cursor-blink on `Software` if desired as a style preference—though without `frame_cap` delays).

### 3. Hardware `wgpu` Surface Latency Settings
Set `PresentMode::AutoNoVsync` (unless overridden) and `desired_maximum_frame_latency = Some(1)` in `desktop/src/main.rs` so hardware `wgpu` and `--wgpu` overrides do not queue 2–3 stale frames during resize/drag/typing.

### 4. Early `cargo-deny` Gate & Fallback Branch
Immediately after adding `hdrshot-egui-backend` (`0.2.0`) and `softbuffer` (`0.4`) to `desktop/Cargo.toml` and updating `Cargo.lock`, run:
```bash
cargo deny --all-features --workspace --locked check advisories licenses bans sources
```
- **If it passes:** Proceed with `SoftwarePainter` integration, cross-compile check (`cargo check --target x86_64-pc-windows-msvc`), and desktop tests.
- **If it fails (and cannot be resolved cleanly without weakening `deny.toml`):** Revert the new crates from `Cargo.toml`/`Cargo.lock`, remove the ineffective VM throttling code (`HOVER_SETTLE_DELAY`, `software_frame_cap`, `RequestRepaint` clamping, redundant `GpuProfile` throttling), keep `AutoNoVsync` + `desired_maximum_frame_latency = Some(1)` for `wgpu`, and document the DX12 WARP limitation in `KNOWN_ISSUES.md`.

## Risks / Trade-offs

- **[Risk] `hdrshot-egui-backend` visual or HiDPI differences (gamma/blending/thin strokes vs `egui_wgpu`)** → Mitigation: Verify palette icons, equation canvas texture, caret snapping, and Light/Dark theme contrast on both `Software` and `Wgpu` backends; keep `raster_scale` / `pixels_per_point` handling explicit and testable via `EQNTIDE_RENDERER=software` on Linux X11/Xvfb as well as Windows.
- **[Risk] `softbuffer` pixel format (`0RGB` `u32` buffer) vs `hdrshot-egui-backend` color order (`Bgra` / `Rgba`) and `#![forbid(unsafe_code)]`** → Mitigation: `hdrshot-egui-backend` writes directly into `&mut [u32]` / `BufferMutRef` with `ColorFieldOrder::Bgra` (matching `softbuffer`'s `0x00RRGGBB` layout on little-endian x86_64/aarch64) without needing unsafe slice casts in `desktop`.
- **[Risk] `softbuffer` feature flags pulling unnecessary Unix deps or triggering `cargo-deny`** → Mitigation: Test `cargo deny` with full workspace targets right away; if Unix `softbuffer` features aren't needed or cause issues, evaluate `cfg(target_os = "windows")` vs cross-platform support within the `cargo-deny` gate.
- **[Risk] Windows DWM itself still runs on WARP during full-window drags in GPU-less VMs** → Mitigation: `softbuffer` uses GDI `BitBlt` rather than a DX12 flip-model swapchain (`DXGI_SWAP_EFFECT_FLIP_DISCARD`), avoiding DX12 swapchain DWM composition overhead, though host/guest VM DWM overhead cannot be eliminated entirely from user space.
