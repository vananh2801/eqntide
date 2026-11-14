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
- Live LaTeX preview is debounced by 180 ms and parsed on a worker. Tests cover
  source preservation, stale-result rejection, invalid drafts, reverting while a
  parse is in flight, clearing the source, and undo to imported attributes.
- Workspace Clippy and cargo-deny license/source/bans checks.

## Local smoke checks

On Linux, the application was built and launched on an isolated Xvfb display
with OpenGL software rendering. A nested fraction/root imported through a URI
rendered correctly. Mouse focus, Ctrl+A, text input and Ctrl+H created `x²` with
an aligned caret and synchronized LaTeX source. A second invocation forwarded
`eqntide://open?tab=batch` and exited, selecting Batch in the existing instance.
A subsequent X11 source-input smoke check typed `\frac{x+1}{2}` and observed
an automatic fraction preview without Apply, then typed the incomplete
`\frac{` and verified that the previous formula remained with an inline error.
Clearing the source cleared the preview.
This verifies an X11 smoke path, not an Office or Wayland qualification.

A release Linux build and portable/RPM packaging were exercised locally. A Windows
MSVC cross-target `cargo check` also passed on Linux; it is not a Windows runtime
or installer test. The legacy Tauri command tests passed independently. Final
workspace checks and artifact sizes are reported in the task completion message.
Do not extrapolate release behavior on other OSes from a Linux build.

## Manual parity matrix still required before cutover

| Scenario | Linux X11 | Linux Wayland | Windows |
| --- | --- | --- | --- |
| Window/render/basic editing | Isolated smoke checked | Pending | Pending |
| Vietnamese IME and text selection | Pending actual IME | Pending | Pending |
| Clipboard PNG/Flat OPC, large formula, DPI changes | Pending | Pending | Pending |
| URI cold/warm launch and installed assets | Development IPC checked | Pending | Pending |
| Word OMML/MathType insert and replace | N/A | N/A | Pending |
| WPS OMML/MathType insert and replace | Pending | Pending | Pending |
| Install/upgrade/uninstall and default URI handler | RPM/portable built, installation pending | Pending | Pending |

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

Integral placement regression checks cover LaTeX-to-under/over normalization,
PNG/hit geometry above and below the integral, and explicit Office `subSup` /
`undOvr` overrides without rewriting imported MathML.

## Native integration migration

Native packages include shared Word/WPS add-in installers, the rebuilt Word VBA
template, URI registration and bridge startup helpers. Installed assets take
priority over repository fallbacks, and legacy `/convert` clients remain
compatible. WPS registration preserves unrelated add-ins. The VBA builder uses
a fresh source file to avoid duplicating modules on repeated builds.

Linux RPM/DEB setup drops root privileges before configuring user profiles,
starts/restarts the user bridge at installation and enables it at graphical
login. XDG autostart handles users without an available session during install.
Add-in files are cached by payload digest; service setup remains independent.
Final removal stops and removes managed services, including tracked offline
profiles, while upgrade preserves them until the new installation restarts them.
Windows Setup installs a hidden per-user Startup supervisor, stops owned
processes before replacement and removes startup on uninstall.

Local validation passed:

- 129 Rust tests, workspace Clippy with warnings denied, formatting and release build.
- 11 Python integration tests, covering installer inputs, WPS configuration
  preservation, isolated service install/reuse/remove, privilege dropping,
  add-in caching/repair and rejection of macOS packaging targets.
- JavaScript, shell and Python syntax checks; DEB configure/removal hook syntax
  and staging inspection using a mocked package builder.
- Portable and RPM packaging; RPM provides/obsoletes the legacy `eqn-tide`
  package and retains `EqnTide.desktop` for old URI associations. Runtime
  requirements use SONAME capabilities for Fedora's split Wayland packages,
  with automatic ELF requirements for the actual libc/libgcc ABI.
- NSIS compilation with mock executable payloads (syntax/staging verification).
- An extracted-style bundle outside the repository: packaged asset selection,
  legacy conversion, recorded session/batch URI launch, save and cancellation.
- Actual Linux service activation: the enabled service runs the installed
  `/usr/lib/eqntide/eqntide-cli` and `/health` returns `ok`. Native systemd
  verification caught invalid WorkingDirectory quoting; the regression check
  now verifies units with systemd-analyze when available.

The bridge tracks response writes; desktop shutdown gives Office clients a
brief grace period and drains pending responses, bounded by three seconds.
This still needs qualification in real Word/WPS sessions. Rebuilding DOTM does
not compile or execute the macros inside Word. Windows process/startup behavior,
Office host loading, complete package install/upgrade/remove transactions and
Wayland remain subject to the manual matrix. DEB generation was not exercised
locally because this host lacks `dpkg-deb`; release CI has the package builder.

macOS support is temporarily suspended because no test hardware is available.
GitHub CI/release builds only Linux and Windows. The native packager rejects
macOS targets before staging files; macOS app bundles and add-in installation
code have been removed. OS portability code/dependencies may remain in Rust,
but they do not represent a supported or tested macOS release.
