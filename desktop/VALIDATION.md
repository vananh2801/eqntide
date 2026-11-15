# Native desktop validation and cutover gate

The native implementation is a preview and the only desktop in this repository.
The legacy Tauri app/frontend were removed at the user's request; Git history
preserves them. Removal does not qualify the pending Office/platform scenarios.

## Automated coverage

- Canvas command completion covers typed `\lim_{x\rightarrow+\infty}`,
  nested fraction/root arguments, keyboard selection and mouse acceptance of
  the real popup. `\dfrac` completion inserts editable fraction slots; Toggle
  tests cover bare/delimited WPS detection and OMML/MathType conversion packages.
  Checks cover source synchronization, one-step undo, imported
  attributes, incomplete/unknown drafts, cancellation and save blocking.

- Sketch-based layout: fixed footer/save and LaTeX visibility at 1040×560,
  1100×650 and 1440×900 in light/dark themes for standalone/Office sessions.
  Checks cover initial and settled layout, canvas/source overlap and aligned
  horizontal edges. Templates reserve three rows at every size; format/font/undo/
  redo controls and Save stay on one footer row at the window's enforced minimum.
  Synthetic placeholder decoration uses the preview's geometry
  without adding placeholder attributes to the editor's formula or exports.
- Startup resize tests cover native geometry and Wayland's missing inner-rect
  geometry, already-large/maximized windows and smaller monitors. All 49 Symbols
  fit at the minimum width at 100–200% DPI; temporarily narrower viewports keep
  at most three rows using horizontal overflow.
- Canvas scrolling covers short/long/tall formulas and returning to a short
  formula at 100%, 125%, 150% and 200% DPI. Content bounds exclude empty canvas
  space to avoid rounding-induced horizontal overflow; corners match text inputs.
- Invalid LaTeX retains an undimmed white canvas and reports its error only in
  the bottom status bar. Sidebar Batch DOCX and its back button retain unsaved
  equation/source contents.
- Every visual palette thumbnail renders nonempty math from its insertion
  template, including symbols and structural placeholders. Cached white alpha
  masks are tinted with the current theme's text color.

- Workspace engine/CLI tests, structured editor transactions, Unicode edits,
  template selection wrapping, scripts, fixed arity, matrix resizing, stable
  import attributes/unknown nodes, undo/redo, and import depth limits.
- Every button from the original desktop palette is represented in native data
  and is parsed by the shared Rust LaTeX service.
- Renderer hit geometry shares the image layout; existing PNG/EMF behavior and
  STIX baseline tests remain covered.
- Real Upgrade regression samples cover headerless MTEF in old EMF previews:
  repair is accepted only when it reproduces the paired OLE payload exactly.
  Rerunning `bug_upgrade_before.docx` preserved the LaTeX representation of all
  4,466 source OLE equations and produced identical MTEF in every OLE/EMF pair.
  The wrapped two-row cases table was rendered and its full-height brace checked;
  a layout regression test compares direct and wrapped tables.
- OMML token styles are explicit: default digits, operators, text and function
  names are upright, single-letter variables italic. Regression tests cover
  fraction contents and preservation of explicit mathvariant overrides.
- App dirty-session queue, concurrent startup requests, failed imports, failed
  session save, and unapplied LaTeX drafts have regression tests.
- Duplicate Office opens are ignored during loading, active editing and queuing.
  Tests cover global Ctrl+S, source edits in the save frame, Office save/close,
  and save-button visibility at the minimum size, including Office save labels.
- Unchanged equations close/switch without a confirmation dialog; edits, source
  drafts, font-size changes and running batches retain their confirmation.
- Bridge session tests cover changing the output mode on insert and edit:
  OMML results omit the EMF path so WPS uses native equation XML; MathType
  results retain OLE plus the EMF preview, with identical MTEF in both parts.
- WPS desktop-session and Toggle LaTeX insertions use the complete MathType
  package, never AddPicture. Mock tests cover replacement, baseline positioning
  and imported paragraph cleanup. Package tests cover EMF-first reading, OLE
  fallback, dual-MTEF equality and unique part/shape/relationship identifiers.
  The user confirmed that the resulting file opens in MathType. WPS
  scrolling/reopen behavior and repeated MathType editing still need runtime
  checks after changing from standalone EMF to OLE+EMF insertion.
- `npm ci --prefix scripts && npm test --prefix scripts` exercises the add-in with mocked document
  ranges: OMML insert/replacement imports Flat OPC (raw XML is literal text in WPS),
  preserves surrounding text and existing paragraph marks, removes a verified
  extra trailing break using document-length deltas (independent of OMath.Range),
  selects the insertion end and clears picture positioning. Cases cover CR/LF/CRLF,
  unavailable OMath APIs and OMath ranges including the paragraph mark.
  Toggle tests reproduce Range.Text preserving the OMML container and verify
  replacing the actual oMath node with a w:r/w:t node in the paragraph package
  in cursor, selected-single and batch paths, without trusting Remove APIs.
  XML assertions use `scripts/fixtures/bug_toggle_v1.xml` minimized from the
  inspected DOCX sample, preserve other equations and
  bookmarks, and check that imported spacers are removed while original spaces stay.
  EMF positioning retains the WPS calibration from commit `9165468`: rendered
  depth plus 0.045 times the font size, rounded to tenths of a point. These mocks do not qualify actual WPS paragraph layout.
- Live LaTeX preview is debounced by 180 ms and parsed on a worker. Tests cover
  source preservation, stale-result rejection, invalid drafts, canvas changes while a
  parse is in flight, clearing the source, and undo to imported attributes.
- Workspace Clippy and cargo-deny license/source/bans checks.
- WPS ribbon tests cover always-on equation-tab activation (ignoring old disabled settings), recognition
  of tagged pictures, MathType OLE and embedded EMF comments, ordinary picture
  exclusions, stale selections/document switches and recursive event protection.
  Switching uses WPS RibbonUI.ActivateTab after a deferred selection event;
  actual contextual-tab timing on WPS Linux/Windows still requires manual checks.
- Bridge-launched desktop processes are waited on by background reaper threads.
  A Linux test launches and reaps eight real child processes and verifies their
  `/proc` entries disappear, covering zombie accumulation after repeated closes.
- Word's rebuilt DOTM contains the always-on WindowSelectionChange event sink
  and Ribbon onLoad callback. Install `word-template/test-requirements.txt` then
  run `python -m unittest discover -s scripts -p test_word_ribbon.py` to verify
  the event class's non-procedural VBA metadata, packaged source and Ribbon wiring
  with an independent CFB/VBA reader. These checks do not compile VBA or test
  contextual-tab timing in Word; Windows runtime validation remains pending.

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
or installer test. Final
workspace checks and artifact sizes are reported in the task completion message.
Do not extrapolate release behavior on other OSes from a Linux build.

## Manual parity matrix still required before cutover

| Scenario | Linux X11 | Linux Wayland | Windows |
| --- | --- | --- | --- |
| Window/render/basic editing | Isolated smoke checked | Pending | Pending |
| Vietnamese IME and text selection | Pending actual IME | Pending | Pending |
| Clipboard LaTeX, large formula, DPI changes | Pending | Pending | Pending |
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

Keep native packages labeled preview until the matrix passes. The legacy
Tauri/frontend directories have been removed; add-ins use independent assets.
Qualify the native editor against the formula corpus and Office workflows.
The CLI/library default build and HTTP contracts remain available.

Integral placement regression checks cover LaTeX-to-under/over normalization,
PNG/hit geometry above and below the integral, and explicit Office `subSup` /
`undOvr` overrides without rewriting imported MathML.

## Native integration migration

Native packages include the Word VBA template installer, WPS add-in installer,
URI registration and bridge startup helpers. Installed assets take
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

After removing the legacy desktop/frontend, 130 Rust workspace tests and 11
Python integration tests passed. Workspace Clippy, formatting and diff checks
also passed. The native desktop version smoke command remains available without
legacy files. Historical desktop URI associations and the VBA conversion alias
are retained for installed clients; Tauri-specific origin exceptions are removed.

Word integration now ships only `word-template/EqnTide.dotm`. The Word Office.js
frontend and HTTP asset routes have been removed. Windows installation removes
EqnTide's old web catalog registration and leftover assets on upgrade, then copies
the VBA template to Word STARTUP. Shared WebView exemptions and unrelated Office
add-ins are left untouched. WPS and session/conversion endpoints remain available.
Actual VBA compatibility and macro policy still need testing on each Word version.

Word-template-only migration checks: 130 Rust workspace tests, 12 Python
integration tests, Clippy and syntax checks passed. Tests verify Word web routes
return 404, WPS assets remain served, the packaged DOTM retains VBA and ribbon
parts, and re-staging Windows packages removes stale Word web assets. NSIS
compiled with mock executables. Word's installed VBA template was not changed;
actual Word loading and the PowerShell upgrade cleanup require Windows testing.
Release binaries, portable archive and RPM were rebuilt for this migration.
Both package inventories contain the Word DOTM and WPS payload, with no Word
web directory; the release desktop version smoke check passed.

Distribution output now uses `dist/<windows|linux>/<Cargo version>/`, including
Windows staging, generated NSIS version config and Setup output. The old root
`release/` directory is retired; its guide lives in `docs/`. The unversioned old Windows binaries have been
removed before rebuilding the native Windows distribution.
Thirteen Python tests passed, including multiple versions/platforms under a
custom output root without changing previous artifacts. NSIS compiled into the
version directory using mock executables. Linux portable/RPM were regenerated
under `dist/linux/0.3.0/`; package installation paths remain unchanged.

CI runs on matching pull requests to `main` or manual dispatch and uses the
shared native-package workflow after tests/license checks. Ordinary pushes do
not trigger CI. Both CI and release builds use the same
Word-template rebuild, locked Cargo release build, host executable smoke commands,
versioned packaging and artifact upload. Windows installer scripts are parsed
with Windows PowerShell in CI, and Linux-only symlink layout tests are skipped
on Windows. All three workflows passed actionlint locally.

The obsolete Windows Setup/ZIP were deleted and a real x64 MSVC release was
cross-compiled locally with cargo-xwin. The local LLVM wrapper was broken; the
build used Rust's bundled LLD via a temporary PATH override. Windows target CRT
linking is now static in `.cargo/config.toml`; PE imports confirm neither binary
requires VCRUNTIME140.dll. The cross SDK emits missing CRT debug-PDB warnings,
which do not prevent the release build. Real NSIS Setup and ZIP were generated
under `dist/windows/0.3.0/`. ZIP CRCs and executable hashes match the fresh build,
with DOTM/WPS assets and no Word web payload. Windows execution/install/uninstall
was not exercised locally (no Windows/Wine); GitHub's Windows build now performs
executable smoke checks. Workflow edits were linted, not yet run remotely.
