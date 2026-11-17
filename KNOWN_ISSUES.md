# Known Issues

## Windows Virtual Machines (Software Rendering / Basic Render Driver)

- **UI lag & elevated CPU usage on Windows VMs without GPU acceleration:**
  - **Environment:** Windows guest virtual machines without GPU passthrough or hardware 3D acceleration enabled (where graphics fall back to CPU-based `Microsoft Basic Render Driver` / DirectX 12 WARP rasterizer).
  - **Symptom:** Hovering, typing, or moving/dragging the window may experience noticeable lag and increased CPU consumption.
  - **Status:** Bare-metal physical machines (both Linux and Windows with dedicated/integrated hardware GPUs) run smoothly with low CPU overhead. Optimization for pure software CPU rendering in virtualized environments is deferred for future investigation.

## XML behavior retained intentionally

- A CRLF produced by numeric XML entities (`&#13;&#10;`) is not normalized to
  LF by the XML parser, matching the W3C XML 1.0 rule for character references.
