# Native desktop validation and cutover gate

The native implementation is a preview. The Tauri app remains available for
comparison and rollback; no release has been published by the implementation task.

## Automated coverage

- Workspace engine/CLI tests, structured editor transactions, Unicode edits,
  template selection wrapping, scripts, fixed arity, matrix resizing, stable
  import attributes/unknown nodes, undo/redo, and import depth limits.
- Every button from the original desktop palette is represented in native data
  and is parsed by the shared Rust LaTeX service.
- Renderer hit geometry shares the image layout; existing PNG/EMF behavior and
  STIX baseline tests remain covered.
- App dirty-session queue, concurrent startup requests, failed imports, failed
  session save, and unapplied LaTeX drafts have regression tests.
- Workspace Clippy and cargo-deny license/source/bans checks.

## Local smoke checks

On Linux, the application was built and launched on an isolated Xvfb display
with OpenGL software rendering. A nested fraction/root imported through a URI
rendered correctly. Mouse focus, Ctrl+A, text input and Ctrl+H created `x²` with
an aligned caret and synchronized LaTeX source. A second invocation forwarded
`eqntide://open?tab=batch` and exited, selecting Batch in the existing instance.
This verifies an X11 smoke path, not an Office or Wayland qualification.

A release Linux build and portable/RPM packaging were exercised locally. A Windows
MSVC cross-target `cargo check` also passed on Linux; it is not a Windows runtime
or installer test. The legacy Tauri command tests passed independently. Final
workspace checks and artifact sizes are reported in the task completion message.
Do not extrapolate release behavior on other OSes from a Linux build.

## Manual parity matrix still required before cutover

| Scenario | Linux X11 | Linux Wayland | Windows | macOS |
| --- | --- | --- | --- | --- |
| Window/render/basic editing | Isolated smoke checked | Pending | Pending | Pending |
| Vietnamese IME and text selection | Pending actual IME | Pending | Pending | Pending |
| Clipboard PNG/Flat OPC, large formula, DPI changes | Pending | Pending | Pending | Pending |
| URI cold/warm launch and installed assets | Development IPC checked | Pending | Pending | Pending |
| Word OMML/MathType insert and replace | N/A | N/A | Pending | Pending |
| WPS OMML/MathType insert and replace | Pending | Pending | Pending | Platform-dependent |
| Install/upgrade/uninstall and default URI handler | RPM/portable built, installation pending | Pending | Pending | Pending |

Also exercise nested fractions/scripts, accents/stretchy delimiters, matrix/cases,
structural mouse selection, dirty close/save/discard, expired sessions, occupied
bridge port, and closing during conversion. The worker stops between files;
closing while a batch is active requires stopping and waiting first.

## Measurements

Native release executable size can be read from the generated artifact. Compare
Tauri/native cold startup, idle RSS, installed size and input latency on the same
hardware, OS, fonts and build profile. No before/after performance improvement is
claimed until those measurements exist. Rendering uses the existing font/layout
engine and caches an equation texture by revision/font; event delivery wakes the
UI, so an idle window does not poll or force continuous repainting.

## Cutover

Keep native packages labeled preview until the matrix passes. Preserve old
releases. Remove the Tauri/frontend directories only after verifying no add-in
uses their assets and the native editor meets the formula corpus and Office
workflows. The CLI/library default build and HTTP contracts remain available
throughout migration.
