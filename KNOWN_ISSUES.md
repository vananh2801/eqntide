# Known Issues

## WPS Writer can hide existing equations after MathType OLE insertion or Toggle TeX

When inserting or replacing a MathType OLE equation in WPS Writer on Linux or
Windows—including **Edit Equation**, single **Toggle TeX** (LaTeX to MathType),
and partial/batch **Toggle TeX**—WPS Writer can temporarily stop displaying
existing MathType equations outside the modified range. The equations remain in
the document (`Equation.DSMT4`) and are not deleted; the issue was not observed
when toggling from MathType to LaTeX.

- **Confirmed root cause (WPS Writer Linux):** Re-inserting WPS Writer's own
  self-exported `WordOpenXML` (with unique `rId`s) via `Range.InsertXML` still
  hides outside equations, and stripping `<v:shapetype id="_x0000_t75">` or
  switching `ObjectID` to a 32-bit identifier in EqnTide's Flat OPC payload does
  not prevent it. The disappearing preview is intrinsic to WPS Writer's
  `Range.InsertXML` OLE/VML render cache rather than EqnTide's XML structure.
- **Mitigation (`wps-addin/main.js` and `wps-addin/taskpane.js`, verified on WPS Writer Linux and Windows):**
  After MathType OLE insertion (`InsertMathTypeEquation`, single
  `ToggleLatex`, and once at the end of `batchLatexToMathType`, plus selection
  upgrade in `wps-addin/taskpane.js`), `refreshToggleScreen(doc, true)` toggles
  `ActiveWindow.View.ShowFieldCodes` (`!prev` then `prev`) under
  `app.ScreenUpdating = false` (preserving selection range and
  `VerticalPercentScrolled`, falling back to `doc.Fields.Update()` only when
  `View.ShowFieldCodes` is unavailable) when `doc.Fields.Count > 1`, followed by
  `app.ScreenRefresh()`. On Linux WPS benchmarks (254 equations / 7 pages and
  4,338 equations / 169 pages), this restores all hidden equations across all
  pages (~3 ms on 254 equations, ~1.4–2.6 s on 4,338 equations vs. ~8.5 s for
  `doc.Fields.Update()`) without mutating field values (`PAGE`, `SEQ`, `DATE`,
  `TOC`), jumping viewport scroll, or altering `Font.Position` /
  `Equation.DSMT4` editability.
- **Remaining caveats:** On very large multi-thousand-equation documents,
  forcing WPS Writer to re-render all OLE previews still causes a brief (~1.5 s)
  UI pause; on Windows while WPS is maximized, longer main-thread pauses can
  briefly affect Windows taskbar compositing until repaint completes. If an
  equation ever remains hidden on an untested WPS build, saving, closing, and
  reopening the document restores its display.

## Windows Virtual Machines (Software Rendering / Basic Render Driver)

- **Automatic CPU 2D software renderer fallback on GPU-less VMs:**
  - **Environment:** Windows guest virtual machines (and headless/software Linux setups) without GPU passthrough or 3D acceleration, where `wgpu` resolves to a software/CPU adapter such as `Microsoft Basic Render Driver` (DirectX 12 WARP) or `llvmpipe`/`lavapipe`.
  - **Resolution:** EqnTide probes `wgpu` adapters at startup and automatically routes software/CPU adapters to a CPU 2D rasterizer (`egui_software_backend` + `softbuffer`) instead of DX12 WARP, eliminating shader/swapchain overhead during hovering and typing without artificial input throttling. You can override backend selection with `--software` / `EQNTIDE_RENDERER=software` or `--wgpu` / `EQNTIDE_RENDERER=wgpu` (and `--vsync` / `EQNTIDE_PRESENT_MODE=vsync` when running hardware `wgpu`).
  - **Remaining caveat:** Window dragging/compositing performance still depends on the guest OS compositor (DWM) and VM display driver bandwidth.

## XML behavior retained intentionally

- A CRLF produced by numeric XML entities (`&#13;&#10;`) is not normalized to
  LF by the XML parser, matching the W3C XML 1.0 rule for character references.
