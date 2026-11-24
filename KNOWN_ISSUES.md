# Known Issues

## WPS Writer on Linux can hide existing equations after Toggle TeX

When converting a partial selection from LaTeX to MathType, WPS Writer can
temporarily stop displaying existing MathType equations outside the selection.
The equations remain in the document and display again after saving, closing,
and reopening it. The issue was not observed when toggling from MathType to LaTeX.

To restore the display, save and reopen the document. The cause has not been
confirmed, and further investigation is deferred.

## Windows Virtual Machines (Software Rendering / Basic Render Driver)

- **Automatic CPU 2D software renderer fallback on GPU-less VMs:**
  - **Environment:** Windows guest virtual machines (and headless/software Linux setups) without GPU passthrough or 3D acceleration, where `wgpu` resolves to a software/CPU adapter such as `Microsoft Basic Render Driver` (DirectX 12 WARP) or `llvmpipe`/`lavapipe`.
  - **Resolution:** EqnTide probes `wgpu` adapters at startup and automatically routes software/CPU adapters to a CPU 2D rasterizer (`egui_software_backend` + `softbuffer`) instead of DX12 WARP, eliminating shader/swapchain overhead during hovering and typing without artificial input throttling. You can override backend selection with `--software` / `EQNTIDE_RENDERER=software` or `--wgpu` / `EQNTIDE_RENDERER=wgpu` (and `--vsync` / `EQNTIDE_PRESENT_MODE=vsync` when running hardware `wgpu`).
  - **Remaining caveat:** Window dragging/compositing performance still depends on the guest OS compositor (DWM) and VM display driver bandwidth.

## XML behavior retained intentionally

- A CRLF produced by numeric XML entities (`&#13;&#10;`) is not normalized to
  LF by the XML parser, matching the W3C XML 1.0 rule for character references.
