# EqnTide

A standalone, high-performance **bidirectional math equation converter and editor suite** for Microsoft Word `.docx` documents between **OMML** (Office Math Markup Language, Word's native equation format), **MathType** (`Equation.DSMT4` / `Equation.3` OLE objects & WMF/EMF previews), and **LaTeX** — without requiring Microsoft Word, MathType, or Windows. Written in pure Rust as a self-contained library, CLI tool (`eqntide-cli`), and Desktop application (`EqnTide`).

- **Forward (`OMML → MathType`)**: Converts Word native OMML equations into embedded MathType 5.0 OLE objects (`Equation.DSMT4`) with crisp vector EMF (or PNG) previews. Double-clicking any converted equation in Microsoft Word or WPS Office opens it directly in **MathType** for full visual editing.
- **Reverse (`MathType → OMML`)**: Extracts MTEF v3 (Equation Editor 3.x) and MTEF v5 (MathType 4–7) binary streams from embedded OLE `.bin` containers or WMF (`MTra`) preview images, decodes them into MathML, and replaces the VML `<w:object>` elements with native Word `<m:oMath>` XML.
- **Interactive Visual Editor (`EqnTide Desktop & Word/WPS Add-in`)**: Visual equation editing with the native Rust desktop preview, LaTeX source editing, customizable fonts, baseline alignment, and single-click insertion/replacement into Word or WPS Office documents.

---

## Repository Components

| Component | Location | Description |
| --- | --- | --- |
| Rust library and CLI | `src/`, root `Cargo.toml` | Pure Rust conversion engine and the `eqntide-cli` command-line application. |
| Native Rust desktop (preview) | `desktop/`, `crates/eqntide-editor/` | egui/eframe shell, structural equation editing, and the shared STIX renderer. No desktop WebView or JavaScript. |
| Microsoft Word VBA template | `word-template/EqnTide.dotm` | Global VBA template loaded from Word STARTUP; opens native editing sessions through the local bridge. |
| WPS Office add-in | `wps-addin/` | Static WPS JavaScript add-in, ribbon definition, task pane, and assets. It communicates with the same bridge. |

Word uses only the `.dotm` VBA template; there is no Word web add-in or browser-based Word integration. WPS retains its JavaScript add-in. Both communicate with the local bridge at `http://127.0.0.1:17890`, provided by the background CLI or desktop.

---

## Key Features

- **Bidirectional End-to-End Conversion Pipelines**:
  - **OMML → MathType**: `.docx → OMML → MathML AST → MTEF 5.0 → OLE 2.0 Compound File Binary + EMF/PNG Preview → .docx`
  - **MathType → OMML**: `.docx (OLE .bin / WMF MTra) → MTEF v3/v5 → MathML AST → OMML (<m:oMath>) → .docx`
  - Processes equations across the entire document package: `word/document.xml`, `word/header*.xml`, `word/footer*.xml`, `word/footnotes.xml`, and `word/endnotes.xml`.
  - **Batch Folder Conversion**: Supports converting an entire directory of `.docx` files recursively while preserving the original subfolder hierarchy and filenames.
- **Comprehensive MathType Template & OMML Element Coverage**:
  - **Fences & Intervals**: Parentheses, brackets, curly braces (including single-sided system braces `{` / `}`), angle brackets, open brackets, absolute values, double vertical bars, floor/ceiling, Dirac notation (`⟨ψ|`, `|ψ⟩`, `⟨ϕ|ψ⟩`), and open/half-open intervals (`(a, b]`, `[a, b)`).
  - **Radicals & Fractions**: Square roots, nth roots, regular bar fractions, skewed / diagonal, and inline slash fractions (`a/b`).
  - **Scripts & Limits**: Subscripts, superscripts, combined sub-superscripts, pre-subscripts/pre-superscripts (`m:sPre` / `tmSUP`), and limit operators (`\lim`, `\max`, `\min`, `\sup`, `\inf`, `\det`, `\gcd`) with under/over bar support (`tvSUBAR`, `tvDUBAR`), `<m:limLoc>` (`undOvr` / `subSup`) preservation, and `FULL` size restoration.
  - **Large Operators & Integrals**:
    - Summations (`∑`), products (`∏`), coproducts (`∐`), unions (`⋃`, `∪`), intersections (`⋂`, `∩`) with `undOvr` or `subSup` limit placement.
    - Single, double, and triple integrals ($\int$, $\iint$, $\iiint$).
    - Contour and loop integrals ($\oint$, $\oiint$, $\oiiint$, $\ointclockwise$, $\varointctrclockwise$) mapped to MathType's native composite glyph structures (loop circles in `MT Extra` paired with symbol stems).
  - **Embellishments, Accents & Over/Under Bars**:
    - Expandable vectors (`\vec`, `\overrightarrow`, `\overleftarrow`, `\overleftrightarrow`, under-arrows) mapped to `tmVEC` and `<m:acc>`.
    - Hats (`\hat`, `\widehat`), tildes (`\tilde`, `\widetilde`), arcs (`\overparen`, `\wideparen`, `\underparen`), single/double overbars (`\overline`), single/double underbars (`\underline`), horizontal braces/brackets (`\overbrace`, `\underbrace`, `\overbracket`, `\underbracket`), joint status, horizontal/slanted strike-throughs, primes (`′`, `″`, `‴`), dot accents (`\dot`, `\ddot`, `\dddot`, `\ddddot`), and Unicode combining mark decomposition.
  - **Extensible Arrows**: Left, right, and bidirectional extensible arrows with upper and lower label slots (`A \xrightarrow{\text{yields}} B`, `A \xleftrightarrow[y]{x} B`).
  - **Matrices, Piecewise Cases & Alignment Arrays**: Multiline equation arrays (`m:eqArr`), piecewise systems, and matrices (`m:m` / `tmMATRIX`) with horizontal/vertical alignment and row/column partition lines.
  - **Boxes & Long Division**: Full and partial border boxes (`top`, `bottom`, `left`, `right`, `actuarial`, strike-throughs via `<m:borderBox>` / `tmBOX`) and elementary long division (`tmLDIV` / `menclose notation="longdiv"`) with optional quotient slot.
  - **Full MathType Typeface & Character Classification**: Accurate bidirectional mapping across `TF_TEXT`, `TF_FUNCTION`, `TF_VARIABLE`, `TF_LCGREEK`, `TF_UCGREEK`, `TF_SYMBOL`, `TF_VECTOR`, `TF_MTEXTRA`, `TF_NUMBER`, and `TF_EXPAND`.
- **Crisp Vector & Glyphs-Assembled Previews**:
  - Built-in EMF vector (and optional PNG raster) preview renderer powered by bundled STIX Two Math fonts.
  - Uses proportional glyph piece assembly for tall delimiters (parentheses, brackets, curly braces), vector arrows, and horizontally scaled wide accents (`\widehat`, `\wideparen`) so equations look crisp at any zoom level before MathType is opened.
- **OLE 2.0 & WMF Compatibility**:
  - Generates standard OLE Storage streams (`\x01CompObj`, `\x01Ole`, `\x03ObjInfo`, and `Equation Native` with `EQNOLEFILEHDR`), verified against MathType 5/6/7.
  - Extracts `Equation Native` / `agn` streams from OLE Compound Files and `MTra` comment records from WMF previews.

---

## Installation

### Native Rust Desktop (Preview)

The new desktop uses **egui + eframe** with the existing Rust conversion engine.
It supports structural equation editing (caret, selection, templates, undo/redo),
live LaTeX preview, Office editing sessions, and the four DOCX batch directions.
Word uses the VBA `.dotm` template and WPS uses its JavaScript add-in; they communicate through
the same local bridge. The desktop itself does not load a WebView or MathLive.

On Linux, build with the current stable Rust toolchain:

```bash
cargo run -p eqntide-desktop
cargo build --workspace --release --locked
```

On Windows, use an explicit MSVC target so static CRT flags apply to the target
executables independently of host build scripts and procedural macros:

```powershell
cargo build --workspace --release --locked --target x86_64-pc-windows-msvc
python scripts/package_native.py --target x86_64-pc-windows-msvc
```

Windows executables are under `target/x86_64-pc-windows-msvc/release/`.

The executables are `target/release/eqntide-desktop` and
`target/release/eqntide-cli` (add `.exe` on Windows). Running `cargo build` without
`--workspace` continues to build only the core/CLI package.

Native Linux build prerequisites, without WebKit:

```bash
# Ubuntu / Debian
sudo apt-get install build-essential pkg-config libx11-dev libxi-dev \
  libxrandr-dev libxkbcommon-dev libgl1-mesa-dev libwayland-dev
# Fedora
sudo dnf install gcc gcc-c++ pkgconf-pkg-config libX11-devel libXi-devel \
  libXrandr-devel libxkbcommon-devel mesa-libGL-devel wayland-devel
```

At runtime, provide OpenGL, X11 or Wayland, and a working XDG Desktop Portal
backend plus `zenity` for file dialogs. Windows uses the MSVC toolchain.
macOS support is temporarily suspended because no test hardware is available;
CI and releases target Linux and Windows only. Native UI libraries still use OS APIs and graphics
drivers; a Rust UI does not imply a binary without system dependencies.

Click the equation canvas to type, use **Ctrl+F** for a fraction, **Ctrl+R** for a
root, **Ctrl+H/L/J** for scripts, **Tab/Shift+Tab** for placeholders, and
**Ctrl+Z/Y** for undo/redo. Templates wrap a
selection. Matrix controls add/remove rows and columns. Imported MathML is the
source of truth: opening and saving it does not round-trip through LaTeX.
Unsupported elements are retained as atomic blocks. Typing in the LaTeX source
updates the canvas automatically after a short pause (180 ms); no Apply button is
needed. Source formatting and the text cursor stay intact. Invalid or incomplete
LaTeX keeps the last valid formula visible and shows an inline error; revert to
the canvas to discard that draft. Each successful source update can be undone,
including restoring the attributes of an imported equation. The shared LaTeX parser supports the existing palette, including cases and multiline tables;
it is not a complete TeX engine, and invalid drafts leave the equation intact.
Integral limits entered through LaTeX/palettes use the editor's above/below layout;
imported Office `limLoc` settings retain their requested side or stacked placement.

The fixed toolbar's **Lưu và trở về tài liệu (Ctrl+S)** button saves an Office
session back to Word/WPS and closes the editor after handing off the result.
**Ctrl+S** works from both the canvas and the LaTeX source field. Saving waits
until the source is valid and its preview is up to date; failures retain the
equation for retry. Standalone editing shows **Lưu MathML (Ctrl+S)** instead.

The desktop reuses a healthy CLI/service bridge or hosts one if port 17890 is
free. Closing exits the desktop after save/discard handling; an external bridge
continues running. Single-instance IPC is separate from that port. URI examples:

```bash
eqntide-desktop 'eqntide://open?tab=batch'
eqntide-desktop 'eqntide://open?session_id=SESSION_ID'
```

Batch outputs use a suffix by default and existing copies are not overwritten.
The explicit **Overwrite original documents** option enables replacement; the
worker first completes a temporary sibling file before committing it. Stop waits
for the current file to finish. Clipboard actions include LaTeX, MathML, Flat OPC,
and PNG. EMF is available through **Export EMF** rather than an unverified native
clipboard format.

Create portable or Linux distribution packages from the release build:

```bash
python3 scripts/package_native.py --target x86_64-unknown-linux-gnu
python3 scripts/package_native.py --target x86_64-unknown-linux-gnu --format deb
python3 scripts/package_native.py --target x86_64-unknown-linux-gnu --format rpm
```

All distribution files use one output root, grouped by platform and the version
read from the root `Cargo.toml`:

```text
dist/
  linux/0.3.0/
    eqntide-native-preview-0.3.0-x86_64-unknown-linux-gnu.tar.gz
    eqntide-native-preview-0.3.0-1.x86_64.rpm
    eqntide-native-preview_0.3.0_amd64.deb
  windows/0.3.0/
    eqntide-native-preview-0.3.0-x86_64-pc-windows-msvc.zip
    EqnTide-0.3.0-Setup.exe
    README-WINDOWS.txt
    installer-config.nsh
    windows-stage/
```

Only requested formats are generated. A version bump creates a new directory;
rebuilding the same version replaces its artifacts. `--output /some/path` changes
the root while retaining `<platform>/<version>/`. `target/release/` remains
Cargo's compiler output, separate from distributable packages. Old Windows installers have been removed; only freshly generated packages
should be distributed.

Windows packaging generates NSIS inputs inside its version directory. Compile
Setup from the repository root after packaging the Windows binaries:

```powershell
$version = python scripts/package_native.py --print-version
$packageDir = (Resolve-Path "dist/windows/$version").Path
makensis "/DPACKAGE_DIR=$packageDir" scripts/installer.nsi
```

The generated configuration sets the installer's displayed version and output
filename. The Windows release workflow runs these steps automatically.

DEB requires `dpkg-deb`; RPM requires `rpmbuild`. Packages include the add-ins,
Word template and font licenses. The portable Linux package includes
`install-user.sh` to register the URI handler. Windows release CI stages the
portable ZIP and builds `scripts/installer.nsi`, which registers the current-user
URI handler. macOS packaging is disabled.

The Rust desktop is the only desktop implementation. This remains a
**native preview** until the manual platform/Office matrix passes.
See [implementation plan](plan.md) and [validation status](desktop/VALIDATION.md).

### Native desktop integration and add-in setup

Close the old EqnTide desktop and Office before upgrading. Native Linux packages
replace the legacy `eqn-tide` package and retain its desktop-file URI association;
the Windows installer asks you to close a locked legacy executable. Package
installation configures the login bridge and restarts it with the installed binaries.

Packaging requires Python 3.11 or later; Linux URI/add-in setup helpers require Python 3; the desktop/CLI binaries do not.

All current launchers, package scripts and release jobs target `eqntide-desktop`
and `eqntide-cli`. The old Tauri desktop and web frontend have been removed;
previous versions remain available in Git history.

- **Windows installer:** installs the native binaries, registers `eqntide://`,
  and runs the shared PowerShell installer for the Word `.dotm` template
  and WPS add-in. It also enables the hidden login bridge. Close Office first. The
  Start Menu offers a manual bridge launcher.
- **Windows portable:** extract the complete ZIP and run `install-addins.bat` to
  register the URI and add-ins. `start-server.bat` starts the optional external
  bridge using the sibling native desktop. Add-ins can also use the bridge hosted
  by the desktop itself.
- **Linux portable:** keep the extracted folder in a stable location, run
  `./install-user.sh` for URI registration, then
  `bash scripts/install-addin.sh` for WPS. Optional login bridge:
  `bash scripts/install-bridge-service.sh`. Preview its unit without installing
  using `--print-unit`. The service resolves binaries from the installed bundle,
  the repository release directory, or PATH; overrides are
  `EQNTIDE_CLI_BIN` and `EQNTIDE_DESKTOP_BIN`.
- **Linux DEB/RPM:** executables/resources live under `/usr/lib/eqntide` with
  launchers in `/usr/bin`. Package hooks configure WPS add-ins and the login
  bridge for logged-in users, with setup at next login as fallback. Repair
  scripts are available under `/usr/lib/eqntide/scripts/`.

Word's rebuilt VBA template now uses `/session/open` → native editing →
`/session/:id/wait`, inserts the returned Flat OPC into the captured range and
leaves the document untouched on cancellation. Its batch command opens the
native Batch tab. The legacy `/convert` bridge route remains an alias for old
installed templates. Rebuild the `.dotm` after source changes with:

```bash
python -m pip install -r word-template/requirements.txt
python word-template/build_dotm.py
```

WPS launchers use the registered native URI, including an `xdg-open` fallback on
Linux. Add-in installers merge their WPS registrations rather than replacing
other add-ins' configuration. The bridge serves packaged resources beside its
executable before development fallbacks, so installed bundles work outside the
repository. Windows Office and installer runtime qualification remains in
[the validation matrix](desktop/VALIDATION.md).

### Build prerequisites

Use stable Rust. Windows builds require Visual Studio C++ Build Tools and the
Windows SDK. Linux dependencies are listed above; WebKit and the Tauri CLI are
not required. Node.js is needed only for JavaScript syntax checks in CI.

### Clone the Repository

```bash
git clone https://github.com/vananh2801/eqntide.git
cd eqntide
```

### Build the Pure Rust CLI

Run this command from the repository root:

```bash
cargo build --release --bin eqntide-cli
```

The release binary is written to:

- Linux: `target/release/eqntide-cli`
- Windows: `target\release\eqntide-cli.exe`

Verify the build with:

```bash
./target/release/eqntide-cli --version
```

On Windows PowerShell, use `./target/release/eqntide-cli.exe --version`.

### Linux: Complete Build and Installation Procedure

Run builds as your desktop user and use `sudo` only for system dependencies and
package installation. Close EqnTide and WPS before upgrading.

#### 1. Install build dependencies

Fedora:

```bash
sudo dnf install git gcc gcc-c++ make pkgconf-pkg-config rpm-build python3 \
  libX11-devel libXi-devel libXrandr-devel libxkbcommon-devel \
  mesa-libGL-devel wayland-devel
```

Ubuntu/Debian:

```bash
sudo apt-get update
sudo apt-get install git build-essential pkg-config python3 dpkg-dev \
  libx11-dev libxi-dev libxrandr-dev libxkbcommon-dev libgl1-mesa-dev libwayland-dev
```

Install stable Rust through [rustup](https://rustup.rs/) if Cargo is unavailable.

#### 2. Build the desktop and CLI

From the repository root:

```bash
cargo build --workspace --release --locked
```

#### 3. Package and install

Fedora:

```bash
python3 scripts/package_native.py --target x86_64-unknown-linux-gnu --format rpm
sudo dnf install ./dist/linux/0.3.0/eqntide-native-preview-0.3.0-1.x86_64.rpm
```

Ubuntu/Debian (not locally qualified):

```bash
python3 scripts/package_native.py --target x86_64-unknown-linux-gnu --format deb
sudo apt-get install ./dist/linux/0.3.0/eqntide-native-preview_0.3.0_amd64.deb
```

For an already installed RPM with the same version, use `sudo dnf reinstall`
instead. Build on the oldest distribution version you intend to support and
verify the generated package on each supported version.

Packages install both binaries/resources under `/usr/lib/eqntide` and launchers
under `/usr/bin`. Package hooks configure the WPS add-in and enable the per-user
background bridge. Setup is deferred to next graphical login if no user session
is available; no separate source checkout or manual service installation is needed.

#### 4. Verify and maintain

```bash
systemctl --user is-enabled eqntide.service
systemctl --user status eqntide.service --no-pager
curl http://127.0.0.1:17890/health
```

Expect `enabled`, `active (running)` and a JSON response with `"status":"ok"`.
Restart WPS and select **EqnTide → Insert Equation** to test native editing.

```bash
journalctl --user -u eqntide.service -n 80 --no-pager
systemctl --user restart eqntide.service
```

For source/portable installs or repairs, use the add-in/service helpers described
above. The service runs at graphical login; do not start another bridge while
it owns port `17890`.

### Prepare and Install the Word Template and WPS Add-in

Distribute the prebuilt Word template and the complete WPS directory:

```text
word-template/EqnTide.dotm  # Microsoft Word VBA template
wps-addin/                 # WPS Office JavaScript add-in
```

Keep these paths beside `eqntide-cli[.exe]` in portable distributions. The bridge serves `/wps-addin/*`; Word calls the conversion/session API directly from VBA. When running from a source checkout, start the bridge from the repository root:

```bash
./target/release/eqntide-cli --server
```

The desktop application starts the same bridge automatically. Confirm it is available before opening the add-in:

```bash
curl http://127.0.0.1:17890/health
```

Packaged installers register bundled add-ins and enable the background bridge
automatically. Linux uses a per-user `eqntide.service` (starts at graphical login,
restarts on failure); Windows uses a hidden per-user Startup supervisor (starts
at login and restarts the CLI bridge if it exits). Installation starts the bridge
immediately when a user session is available. Upgrades restart it; uninstall
removes startup configuration and stops the owned bridge. Linux package removal
also cleans tracked offline profiles. The server remains bound to localhost;
no administrator service or boot-before-login server is installed.

Packaged installers register bundled add-ins automatically. Windows Setup installs
Word/WPS add-ins for the installing user. Linux RPM/DEB installs WPS add-ins for
logged-in users, dropping root privileges before touching their profiles; when
no user session is available, setup runs at the next desktop login through XDG
autostart. Add-in files are copied again only when the bundled add-in changes or
its files are missing; each login also ensures the background service is enabled. Restart Word/WPS after installation. Word macro security or organization
policies may still require enabling the VBA template. Source/portable setups can use
the manual commands below.

#### Windows: Word and WPS Office

Run the PowerShell registration script from the repository root, then restart Word or WPS Office:

```powershell
.\scripts\install-addin.ps1
```

The script copies `word-template/EqnTide.dotm` into the current user's Word STARTUP directory and installs the WPS JavaScript add-in. It also removes EqnTide's old Word web catalog registration and leftover web assets when upgrading. It does not change other add-ins or shared WebView settings. To remove the template and WPS registration:

```powershell
.\scripts\install-addin.ps1 -Uninstall
```

#### Linux: WPS Office

For the complete desktop and background-service setup, follow **Linux: Complete Build and Installation Procedure** above. The command below installs only the add-in, not the service:

```bash
./scripts/install-addin.sh
```

This installs the static files under the current user's Kingsoft/WPS `jsaddons` directories. Uninstall them with:

```bash
./scripts/install-addin.sh --uninstall
```

### Manual Add-in Sideloading

Use these steps when the registration scripts are unavailable or when you need to inspect the installation manually. Start the EqnTide desktop application or CLI bridge before opening an add-in.

#### Microsoft Word on Windows: VBA template only

Completely close Word, then copy the template into the default user STARTUP folder:

```powershell
$startup = Join-Path $env:APPDATA 'Microsoft\Word\STARTUP'
New-Item -ItemType Directory -Path $startup -Force | Out-Null
Copy-Item '.\word-template\EqnTide.dotm' $startup -Force
```

If Word uses a custom STARTUP location, copy the template there instead. Reopen
Word and check the **EqnTide** ribbon. The VBA template provides Insert/Edit,
LaTeX conversion and opening the native batch tab. Word macro policy must allow
loading the template; it does not require an Office.js catalog or web runtime.
Compatibility must still be qualified on the actual Word versions in use.
Word on the Web is not supported by this VBA integration.

#### WPS Office on Windows or Linux

Copy the complete `wps-addin/` directory into the WPS `jsaddons` directory as `EqnTide_1.0.0`:

- Windows: `%APPDATA%\Kingsoft\wps\jsaddons\EqnTide_1.0.0\`
- Linux: `~/.local/share/Kingsoft/wps/jsaddons/EqnTide_1.0.0/`
- Linux international builds may use `~/.local/share/Kingsoft/wps_intl/jsaddons/EqnTide_1.0.0/`.

Create `publish.xml` in the parent `jsaddons` directory:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<jsplugins>
  <jsplugin name="EqnTide" type="wps" version="1.0.0"
    enable="enable_dev" url="EqnTide_1.0.0/" />
</jsplugins>
```

Create `authaddin.json` in the same directory:

```json
{
  "wps": {
    "namelist": "EqnTide",
    "EqnTide": {
      "name": "EqnTide",
      "enable": true,
      "isload": true,
      "path": "EqnTide_1.0.0/",
      "md5": "",
      "mode": 2
    }
  }
}
```

Completely exit and restart WPS Writer. The **EqnTide** tab should then appear in the ribbon.

### Installation Troubleshooting

- **The add-in reports `Bridge: Offline`:** Start the desktop application or run `eqntide-cli --server`, then open `http://127.0.0.1:17890/health`. A working bridge returns JSON containing `"status":"ok"` and `"server":"eqntide-bridge"`.
- **Port 17890 is already occupied:** Stop the other process using the port before starting EqnTide. Although the CLI supports `--port`, the Word VBA source and WPS scripts are configured for port `17890`; using another port requires updating those add-in URLs as well.
- **The add-in files return 404:** Run the CLI bridge from the repository root, or place `wps-addin/` beside the distributed executable. Word does not load web assets.
- **The ribbon does not appear:** Fully close all Word/WPS processes, verify the Word STARTUP template and macro settings, or the WPS `jsaddons` files for the current user, and reopen the application.
- **Network and privacy:** Conversion runs locally and the bridge binds to `127.0.0.1`; EqnTide does not require a conversion cloud service or telemetry endpoint.

---

## CLI Usage

### 1. Convert Word OMML → MathType (`default`)

```bash
# Basic single-file conversion (writes <input>_mathtype.docx)
eqntide-cli input.docx

# Specify custom output file or output directory
eqntide-cli input.docx -o output.docx
eqntide-cli input.docx -o output_dir/

# Batch convert an entire directory recursively (preserves subfolder structure & filenames)
# Default output directory: <input_dir>_mathtype/
eqntide-cli ./my_documents
eqntide-cli ./my_documents -o ./my_documents_mathtype

# Set preview font size (default: 12 pt, matching MathType default)
eqntide-cli input.docx --font-size 13

# Use PNG raster preview instead of EMF vector
eqntide-cli input.docx --preview png

# Keep original OMML visible (MathType OLE objects stored in background as fallback)
eqntide-cli input.docx --omml-fallback
```

### 2. Convert MathType → Word OMML (`-r` / `--to-omml`)

```bash
# Convert a single file (writes <input>_omml.docx)
eqntide-cli input.docx --to-omml

# Short flag with custom output path
eqntide-cli input.docx -r -o output_omml.docx

# Batch convert a folder of MathType .docx files to OMML (preserves subfolder structure)
# Default output directory: <input_dir>_omml/
eqntide-cli ./mathtype_docs -r
eqntide-cli ./mathtype_docs -r -o ./omml_docs

# Apply a custom math font to all converted OMML runs
eqntide-cli input.docx -r --math-font "Cambria Math"
```

### CLI Options

| Flag / Option | Description |
| --- | --- |
| `<INPUT>` | Path to the source `.docx` file or directory (scanned recursively for `.docx` files). |
| `-o, --output <PATH>` | Destination `.docx` file or directory path (default: `<input>_mathtype[.docx]` or `<input>_omml[.docx]`). |
| `-r, --to-omml` | Reverse direction: convert embedded MathType equations (`.bin` / `.wmf`) to Word native OMML (`<m:oMath>`). |
| `--math-font <FONT>` | Optional font name applied to converted OMML runs (`<w:rFonts>`) when using `--to-omml`. |
| `--preview <FORMAT>` | Picture format for OLE previews in OMML → MathType mode: `emf` (vector, default) or `png` (raster). |
| `--font-size <PT>` | Font size (in points) used to render previews in OMML → MathType mode (default: `12`). |
| `--omml-fallback` | Retains OMML visible in Word using `mc:AlternateContent`, storing MathType objects in the background. |
| `-v, --verbose` | Logs per-equation conversion progress and details. |
| `-q, --quiet` | Suppresses info messages; only prints errors. |
| `-h, --help` | Prints help information. |
| `-V, --version` | Prints version. |

---

## Library Usage

Add `eqntide` to your `Cargo.toml`:

```toml
[dependencies]
eqntide = "0.3.0"
```

```rust
use eqntide::{
    convert, convert_bytes, convert_to_omml, convert_to_omml_with,
    ConversionSummary, ToOmmlOptions,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Forward: Convert Word OMML -> MathType OLE on disk
    let summary: ConversionSummary = convert("document.docx", "document_mathtype.docx")?;
    println!("OMML -> MathType: converted {}/{} equations.", summary.converted, summary.found);

    // 2. Reverse: Convert MathType OLE/WMF -> Word OMML on disk
    let rev_summary: ConversionSummary = convert_to_omml("document_mathtype.docx", "document_omml.docx")?;
    println!("MathType -> OMML: converted {}/{} equations.", rev_summary.converted, rev_summary.found);

    // 3. Reverse with custom math font option
    let opts = ToOmmlOptions {
        math_font: Some("Cambria Math".to_string()),
    };
    convert_to_omml_with("document_mathtype.docx", "document_cambria.docx", &opts)?;

    // 4. In-memory bytes conversion (ideal for web services, desktop apps, or WASM)
    let docx_bytes = std::fs::read("document.docx")?;
    let (mathtype_docx_bytes, _) = convert_bytes(&docx_bytes)?;

    Ok(())
}
```

### Low-Level Pipeline APIs

Both conversion directions expose their intermediate AST and binary codecs:

- **OMML ↔ MathML**:
  - `omml::omml_to_mathml`: Decodes an `<m:oMath>` / `<m:oMathPara>` XML string into a `MathNode` AST.
  - `omml::mathml_to_omml`: Encodes a `MathNode` AST into an `<m:oMath>` XML string.
  - `omml::mathml_to_omml_xml`: Converts a MathML XML string directly into an `<m:oMath>` XML string.
  - `omml::apply_math_font`: Injects `<w:rPr><w:rFonts .../></w:rPr>` into every `<m:r>` in an OMML fragment.
- **MathML ↔ MTEF ↔ OLE**:
  - `mtef::encode::mathml_to_mtef_bytes`: Encodes a `MathNode` AST into MTEF 5.0 binary records.
  - `mtef::make_equation_native`: Wraps MTEF bytes with the 28-byte `EQNOLEFILEHDR` header.
  - `ole::make_ole`: Packages `Equation Native` bytes into an OLE 2.0 Compound File Binary container.
  - `ole::extract_mtef_from_ole` & `ole::extract_mtef_from_wmf`: Extracts raw MTEF streams from OLE `.bin` containers or WMF (`MTra`) images.
  - `mtef::mtef_to_mathml` & `mtef::ole_to_mathml`: Decodes MTEF v3/v5 binary streams into a `MathNode` AST.

---

## Architecture

```text
                    ┌──────────────────────────────────────────┐
                    │           .docx (Word Document)          │
                    └───────────┬──────────────────▲───────────┘
                                │                  │
        ┌───────────────────────┴──────┐    ┌──────┴───────────────────────┐
        │   Forward: OMML → MathType   │    │   Reverse: MathType → OMML   │
        └───────────────┬──────────────┘    └──────────────▲───────────────┘
                        ▼                                  │
         [src/omml/decode.rs]               [src/docx/pipeline.rs]
         Parse <m:oMath> → MathNode         Replace <w:object> with <m:oMath>
                        │                                  ▲
                        ▼                                  │
         ┌──────────────┴──────────────┐    [src/omml/encode.rs]
         │     MathNode (MathML AST)   ├────Encode MathNode → <m:oMath> XML
         └───┬─────────────────────┬───┘                   ▲
             │                     │                       │
             ▼                     ▼                       │
   [src/render/mod.rs]   [src/mtef/encode.rs]   [src/mtef/decode.rs]
   Layout & Glyph Engine Encode MathNode →      Decode MTEF v3/v5 → MathNode
   (EMF / PNG Preview)   MTEF 5.0 + OLE .bin                ▲
             │                     │                       │
             └──────────┬──────────┘        [src/ole.rs]
                        ▼                   Extract MTEF from OLE .bin / WMF
         [src/docx/ole_xml.rs]
         Embed OLE .bin + Preview in .docx
```

---

## References & Credits

This project is an independent bidirectional conversion engine and library that draws architectural design, format specifications, mathematical typography rules, and reference test cases from the following open-source projects and standards:

1. **[omml-converter](https://github.com/bohdanbodnarenko/omml-converter)** (by Bohdan Bodnarenko):
   - Referenced for the forward (`OMML → MathType`) `.docx` packaging workflow, OLE Compound File Binary container generation, and initial MTEF 5.0 binary generation.
2. **[mathtypejx](https://github.com/a917470154/mathtypejx)** (by a917470154) & **[mathtype_to_mathml](https://github.com/jure/mathtype_to_mathml)** (by Jure Triglav):
   - Referenced for the reverse (`MathType → MathML → OMML`) pipeline: MTEF v3 and v5 binary stream decoding, WMF `MTra` extraction, template selector/variation bitmask tables, sub/superscript base-mover logic, and MathType character set mappings.
3. **[Design Science MathType MTEF v3 & v5 Specifications](http://www.dessci.com/en/support/mathtype/tech/mtef5.htm)**:
   - Technical documentation for the MathType Equation Format (MTEF) v3 and v5 binary records.
4. **[ECMA-376 (Office Open XML File Formats, §22.1 OMML)](https://www.ecma-international.org/publications-and-standards/standards/ecma-376/) / ISO/IEC 29500**:
   - Standard reference for Office Math Markup Language (OMML) XML schema and semantics.
5. **[Microsoft OpenType Specification: The `MATH` Table](https://learn.microsoft.com/en-us/typography/opentype/spec/math)**:
   - Specification for OpenType math constants (`MathConstants`), glyph metrics (`MathGlyphInfo`), and extensible delimiter construction (`MathVariants`).
6. **Donald E. Knuth, *The TeXbook*, Appendix G: "Generating Boxes from Formulas" (Addison-Wesley, 1984)**:
   - Algorithmic foundations for mathematical typesetting: horizontal/vertical spacing rules, fraction positioning (Rule 15), radical placement, and operator alignments (see [TYPESETTING_ROADMAP.md](TYPESETTING_ROADMAP.md)).
7. **[W3C MathML Core Specification](https://w3c.github.io/mathml-core/)**:
   - Standard reference for MathML mathematical layout and token element formatting.
8. **[STIX Fonts Project](https://github.com/stipub/stixfonts)** (by The STI Pub Companies):
   - Embedded fonts in `assets/fonts/` (`STIXTwoText`, `STIXTwoMath`) used for standalone vector (EMF) and raster (PNG) equation preview rendering and OpenType `MATH` table metrics, licensed under the [SIL Open Font License 1.1](assets/fonts/stixfonts-2.13b171/OFL.txt).

Detailed attribution and copyright information are documented in [NOTICE](NOTICE).

---

## License

- Source code is licensed under the [MIT License](LICENSE).
- Bundled STIX fonts (`assets/fonts/stixfonts-2.13b171/`) are licensed under the [SIL Open Font License 1.1](assets/fonts/stixfonts-2.13b171/OFL.txt).
- Comprehensive copyright and third-party notices are available in [NOTICE](NOTICE).

Background bridge diagnostics after native installation:

```bash
systemctl --user status eqntide.service
journalctl --user -u eqntide.service -n 50
```

On Windows, bridge logs are under `%LOCALAPPDATA%\EqnTide\logs`. Run
`scripts\install-bridge-startup.ps1` to repair startup or pass `-Uninstall` to
stop the owned bridge and remove its Startup entry. On Linux, the equivalent
manual opt-out is `scripts/install-bridge-service.sh --uninstall`; remove or
disable the `eqntide-addins.desktop` XDG autostart entry too if retaining the
package but opting out of automatic session setup.

### GitHub build workflows

CI runs on matching pull requests to `main` or manual dispatch; ordinary pushes
do not trigger it. Linux and
Windows jobs run workspace tests, Clippy, packaging checks, WPS JavaScript syntax
and Windows PowerShell syntax checks. After checks pass, the reusable
`build-native.yml` workflow rebuilds the Word template, compiles release binaries,
smokes their version commands, packages Linux portable/DEB/RPM and Windows ZIP/NSIS,
and uploads artifacts containing the `linux/<version>/` or `windows/<version>/`
layout. The tag/manual release workflow uses the same package build; only `v*`
tags publish the downloaded packages to GitHub Releases.

For local Windows cross-builds on Linux with cargo-xwin/MSVC SDK and NSIS already
configured:

```bash
cargo xwin build --workspace --release --locked --target x86_64-pc-windows-msvc
python3 scripts/package_native.py --target x86_64-pc-windows-msvc
makensis -DPACKAGE_DIR="$(pwd)/dist/windows/$(python3 scripts/package_native.py --print-version)" scripts/installer.nsi
```

Cross-compilation and NSIS generation do not replace runtime testing in Windows
and Word.

Windows x64 MSVC builds use the static CRT flags in `.cargo/config.toml`, so the
portable/Setup executables do not need a separately installed VC++ runtime.
