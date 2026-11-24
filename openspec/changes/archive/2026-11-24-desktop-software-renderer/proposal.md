# Proposal

## Why

On Windows virtual machines without GPU passthrough or 3D acceleration, `eqntide-desktop` falls back to DirectX 12 WARP (`Microsoft Basic Render Driver`), causing severe hover, typing, and window-drag lag and elevated CPU usage. The existing VM workarounds in the codebase (`HOVER_SETTLE_DELAY`, `66 ms` software frame caps, and `RequestRepaint` delays) do not accelerate WARP's full-window 3D rasterization and instead add artificial input latency.

## What Changes

- Probe the graphics adapter before locking into a `wgpu` swapchain and route software-only environments (`DeviceType::Cpu` such as `Microsoft Basic Render Driver` or `llvmpipe`/`lavapipe`, plus explicit `--software` / `EQNTIDE_RENDERER=software` overrides) to a CPU 2D software renderer (`hdrshot-egui-backend`) presenting via `softbuffer` (Win32 GDI `BitBlt` on Windows).
- Remove the ineffective artificial input-throttling workarounds (`HOVER_SETTLE_DELAY`, `software_frame_cap` repaint delays, and `RequestRepaint` delay clamping) that freeze hover highlights and delay multi-pass typing/scroll updates, and use low-latency `wgpu` surface settings (`AutoNoVsync`, `desired_maximum_frame_latency = Some(1)`) on the hardware `wgpu` path.
- Enforce a strict `cargo-deny` dependency gate (`advisories`, `licenses`, `bans`, `sources`) on the new crates before implementation proceeds; if `cargo-deny` rejects the software backend dependencies, abort the software-backend integration and instead remove the ineffective VM throttling code while keeping `KNOWN_ISSUES.md` updated.

## Capabilities

### New Capabilities
- `desktop-software-renderer`: Automatic graphics-adapter selection and direct CPU 2D software rendering fallback for GPU-less and virtualized environments, paired with unthrottled input and repaint scheduling.

### Modified Capabilities
<!-- None: existing `desktop-ui-theme` and `workspace-dependency-policy` requirements remain unchanged. -->

## Impact

- **Dependencies:** `desktop/Cargo.toml`, `Cargo.lock` (evaluating `hdrshot-egui-backend` and `softbuffer` against `deny.toml` and `#![forbid(unsafe_code)]`).
- **Desktop runtime & UI loop:** `desktop/src/main.rs`, `desktop/src/wgpu_runner.rs`, `desktop/src/app.rs`.
- **Documentation:** `KNOWN_ISSUES.md` (update status or retain documented limitations depending on validation outcome).
