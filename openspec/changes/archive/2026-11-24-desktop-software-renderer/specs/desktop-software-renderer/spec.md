# desktop-software-renderer Specification

## Purpose

Defines automatic graphics-adapter selection, direct CPU 2D software rendering fallback for GPU-less and virtualized environments, and responsive input/repaint scheduling for the native desktop editor.

## ADDED Requirements

### Requirement: Automatic software renderer selection for CPU-only adapters
The desktop application SHALL inspect available graphics adapters (or explicit `--software` / `--wgpu` / `--glow` flags and `EQNTIDE_RENDERER` overrides) at startup and SHALL select the direct CPU 2D software rendering backend whenever no hardware-accelerated `wgpu` adapter is available or the selected adapter reports `DeviceType::Cpu` (such as Windows `Microsoft Basic Render Driver` / WARP or Linux `llvmpipe`/`lavapipe`), unless an explicit override forces `wgpu` or `glow`.

#### Scenario: Windows VM without GPU acceleration routes to CPU software backend
- **WHEN** `eqntide-desktop` starts on Windows without an explicit renderer override and `wgpu` only exposes `Microsoft Basic Render Driver` (`DeviceType::Cpu`)
- **THEN** the application MUST initialize the CPU 2D software renderer instead of creating a DX12 WARP swapchain

#### Scenario: Hardware GPU or explicit override uses hardware-accelerated backend
- **WHEN** `eqntide-desktop` detects a hardware GPU adapter (`DiscreteGpu`, `IntegratedGpu`, or hardware-backed `VirtualGpu`) or receives `--wgpu` / `EQNTIDE_RENDERER=wgpu`
- **THEN** the application MUST use the `wgpu` renderer configured with low-latency presentation (`AutoNoVsync` unless overridden, `desired_maximum_frame_latency = Some(1)`)

#### Scenario: Explicit software override forces CPU 2D renderer
- **WHEN** `eqntide-desktop` is launched with `--software` or `EQNTIDE_RENDERER=software`
- **THEN** the application MUST launch using the CPU 2D software renderer regardless of host GPU hardware

### Requirement: Direct CPU 2D framebuffer rendering preserves desktop parity
The CPU 2D software backend SHALL rasterize `egui` tessellated primitives and texture updates directly into an in-memory pixel buffer and present via native 2D window surface blitting (`softbuffer`) while preserving `#![forbid(unsafe_code)]`, HiDPI `pixels_per_point` scaling, AccessKit tree updates, IME input, equation canvas rendering, and window resize/occlusion handling.

#### Scenario: Window resize and DPI scaling update software surface cleanly
- **WHEN** the window is resized or its scale factor changes while running on the CPU software renderer
- **THEN** the backing pixel buffer MUST resize to match non-zero physical surface dimensions and render crisp UI at the current `pixels_per_point` without crashing on zero-sized or minimized states

#### Scenario: Idle and unchanged frames skip redundant CPU rasterization
- **WHEN** no `egui` shapes, texture deltas, surface dimensions, scale factor, or clear color have changed (including window-drag `Moved` events and redundant OS exposes)
- **THEN** the runner MUST skip re-tessellating and re-rasterizing the UI until an input or state change occurs

### Requirement: Responsive unthrottled input and repaint scheduling
The desktop event loop SHALL process pointer motion, hover transitions, keyboard typing, scrolling, and `egui` multi-pass `RequestRepaint` events without artificial settle delays or frame-rate caps that freeze hover feedback or delay typing responses.

#### Scenario: Hovering and typing schedule immediate UI updates without artificial settle delays
- **WHEN** the user moves the pointer across palette/sidebar controls or types in the equation canvas or LaTeX source editor
- **THEN** the runner MUST deliver events and process `RequestRepaint` without imposing `HOVER_SETTLE_DELAY` or `frame_cap` delays

### Requirement: Dependency policy compliance gate with fallback removal of ineffective throttling
Any newly introduced software-rendering crates SHALL pass `cargo deny --all-features --workspace --locked check advisories licenses bans sources` across all configured workspace targets without weakening `deny.toml` rules; if compliance fails during implementation, the software-backend integration SHALL be aborted and the ineffective VM throttling workarounds SHALL still be removed cleanly.

#### Scenario: Dependency audit passes before retaining software backend
- **WHEN** `hdrshot-egui-backend` and `softbuffer` are added to `desktop/Cargo.toml` and resolved in `Cargo.lock`
- **THEN** `cargo deny --all-features --workspace --locked check advisories licenses bans sources` MUST exit zero with no unapproved licenses, wildcard versions, or unignored `unmaintained`/`unsound` advisories
