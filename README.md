# EqnTide

A standalone, high-performance **bidirectional math equation converter and editor suite** for Microsoft Word `.docx` documents between **OMML** (Office Math Markup Language, Word's native equation format), **MathType** (`Equation.DSMT4` / `Equation.3` OLE objects & WMF/EMF previews), and **LaTeX** — without requiring Microsoft Word, MathType, or Windows. Written in pure Rust as a self-contained library, CLI tool (`eqntide-cli`), and Desktop application (`EqnTide`).

- **Forward (`OMML → MathType`)**: Converts Word native OMML equations into embedded MathType 5.0 OLE objects (`Equation.DSMT4`) with crisp vector EMF (or PNG) previews. Double-clicking any converted equation in Microsoft Word or WPS Office opens it directly in **MathType** for full visual editing.
- **Reverse (`MathType → OMML`)**: Extracts MTEF v3 (Equation Editor 3.x) and MTEF v5 (MathType 4–7) binary streams from embedded OLE `.bin` containers or WMF (`MTra`) preview images, decodes them into MathML, and replaces the VML `<w:object>` elements with native Word `<m:oMath>` XML.
- **Interactive Visual Editor (`EqnTide Desktop & Word/WPS Add-in`)**: Visual equation editing with the native Rust desktop preview or legacy MathLive UI, LaTeX source editing, customizable fonts, baseline alignment, and single-click insertion/replacement into Word or WPS Office documents.

---

## Repository Components

| Component | Location | Description |
| --- | --- | --- |
| Rust library and CLI | `src/`, root `Cargo.toml` | Pure Rust conversion engine and the `eqntide-cli` command-line application. |
| Native Rust desktop (preview) | `desktop/`, `crates/eqntide-editor/` | egui/eframe shell, structural equation editing, and the shared STIX renderer. No desktop WebView or JavaScript. |
| Legacy desktop (comparison/rollback) | `src-tauri/`, `src-ui/` | Tauri v2 and vendored MathLive UI, retained until native platform and Office qualification is complete. |
| Microsoft Word add-in | `word-addin/` | Static Office.js manifest, task pane, commands, and assets. It communicates with the local EqnTide bridge. |
| WPS Office add-in | `wps-addin/` | Static WPS JavaScript add-in, ribbon definition, task pane, and assets. It communicates with the same bridge. |

The Word and WPS add-ins are not separate compiled applications. They are served or copied as static files and require either the desktop application or `eqntide-cli --server` to provide the local bridge at `http://127.0.0.1:17890`.

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
The Word/WPS add-ins remain JavaScript/VBA integrations; they communicate through
the same local bridge. The desktop itself does not load a WebView or MathLive.

Build with the current stable Rust toolchain:

```bash
cargo run -p eqntide-desktop
cargo build --workspace --release --locked
```

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
backend plus `zenity` for file dialogs. Windows uses the MSVC toolchain; macOS
uses Xcode Command Line Tools. Native UI libraries still use OS APIs and graphics
drivers; a Rust UI does not imply a binary without system dependencies.

Click the equation canvas to type, use **Ctrl+F** for a fraction, **Ctrl+R** for a
root, **Ctrl+H/L/J** for scripts, **Tab/Shift+Tab** for placeholders, and
**Ctrl+Z/Y** for undo/redo (Command on macOS where applicable). Templates wrap a
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

DEB requires `dpkg-deb`; RPM requires `rpmbuild`. Packages include the add-ins,
Word template and font licenses. The portable Linux package includes
`install-user.sh` to register the URI handler. Windows release CI stages the
portable ZIP and builds `scripts/installer.nsi`, which registers the current-user
URI handler. macOS packages include an app bundle, but macOS URL dispatch and
Office integration still need platform qualification.

This remains a **native preview**. The legacy Tauri implementation is retained
for comparison and rollback until the manual platform/Office matrix passes.
See [implementation plan](plan.md) and [validation status](desktop/VALIDATION.md).

### Legacy Tauri prerequisites

- A current stable [Rust toolchain](https://rustup.rs/) with Cargo.
- **Windows**: 64-bit Windows 10/11 and Microsoft Visual Studio C++ Build Tools with the Windows SDK.
- **macOS**: macOS 10.15 or later and Xcode Command Line Tools (`xcode-select --install`).
- **Linux**: GTK3, WebKit2GTK 4.1, and the other Tauri system libraries.

  On Ubuntu/Debian:

  ```bash
  sudo apt-get update
  sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev \
    librsvg2-dev patchelf libgtk-3-dev
  ```

  On Fedora:

  ```bash
  sudo dnf install -y webkit2gtk4.1-devel libappindicator-gtk3-devel \
    librsvg2-devel gtk3-devel gcc gcc-c++ make pkgconf-pkg-config rpm-build
  ```

Node.js is not required for a normal build. The desktop frontend is loaded directly from `src-ui/`, and its MathLive runtime is already vendored in `src-ui/vendor/mathlive/`.

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

- Linux/macOS: `target/release/eqntide-cli`
- Windows: `target\release\eqntide-cli.exe`

Verify the build with:

```bash
./target/release/eqntide-cli --version
```

On Windows PowerShell, use `./target/release/eqntide-cli.exe --version`.

### Build the Legacy Tauri v2 Desktop Application

Install the Tauri v2 CLI once:

```bash
cargo install tauri-cli --version "^2.0.0" --locked
```

For development with automatic rebuilds:

```bash
cd src-tauri
cargo tauri dev
```

For a release executable and the platform installer/bundles configured in `src-tauri/tauri.conf.json`:

```bash
cd src-tauri
cargo tauri build
```

Build outputs are placed under `src-tauri/target/release/`:

- Raw executable: `eqntide-desktop` on Linux/macOS or `eqntide-desktop.exe` on Windows.
- Installers/packages: `src-tauri/target/release/bundle/` (for example NSIS on Windows, DEB/AppImage on Linux, and app/DMG bundles on macOS when supported by the host platform).

If only the raw desktop executable is needed, without generating a Tauri installer, run this from the repository root:

```bash
cargo build --release --manifest-path src-tauri/Cargo.toml
```

Tauri bundles are platform-specific and should normally be built on the target operating system.

### Linux: Complete Build and Installation Procedure

The following steps install the desktop application, the WPS add-in, and a background bridge that starts at **graphical login**. Run commands as your normal desktop user; use `sudo` only for system packages. Fedora RPM packaging is verified in this repository. The Debian/Ubuntu commands are provided as an alternative and have not been verified here.

#### 1. Install build dependencies and Rust

Fedora:

```bash
sudo dnf install -y git curl gcc gcc-c++ make pkgconf-pkg-config rpm-build \
  webkit2gtk4.1-devel libappindicator-gtk3-devel librsvg2-devel gtk3-devel
```

Ubuntu/Debian (a release providing WebKit2GTK 4.1):

```bash
sudo apt-get update
sudo apt-get install -y git curl build-essential pkg-config libssl-dev \
  libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libgtk-3-dev
```

Install stable Rust through [rustup](https://rustup.rs/) if Cargo is not already installed. Open a new terminal afterward and verify:

```bash
rustc --version
cargo --version
```

Use the distribution's development packages rather than a custom sysroot. If a previous shell configuration sets `PKG_CONFIG_PATH` to `~/.local/sysroot`, use `env -u PKG_CONFIG_PATH` before the build command to use the installed system packages instead. This works in both Bash and Fish.

#### 2. Build the CLI and install the Tauri CLI

Clone the repository if needed, then run from its root:

```bash
git clone https://github.com/vananh2801/eqntide.git
cd eqntide
cargo build --release --bin eqntide-cli
./target/release/eqntide-cli --version
cargo install tauri-cli --version "^2.0.0" --locked
```

The CLI binary is `target/release/eqntide-cli`. The background service uses this binary, so building only the desktop application is insufficient.

The frontend already includes MathLive under `src-ui/vendor/`; do **not** run `npm install` for a normal build. `src-ui/node_modules/` must not be present because Tauri rejects it inside `frontendDist`. If it exists from earlier work, move that directory outside `src-ui/` before continuing.

#### 3. Build and install the desktop package

Fedora, starting from the repository root:

```bash
cd src-tauri
cargo tauri build --bundles rpm
cd ..
sudo dnf install ./src-tauri/target/release/bundle/rpm/*.rpm
```

Ubuntu/Debian, starting from the repository root:

```bash
cd src-tauri
cargo tauri build --bundles deb
cd ..
sudo apt-get install ./src-tauri/target/release/bundle/deb/*.deb
```

These commands install `/usr/bin/eqntide-desktop` and an EqnTide application-menu entry. `cargo build --release --manifest-path src-tauri/Cargo.toml` alone only builds a raw executable; it does not install the application or create menu entries.

For a rebuilt Fedora package with the same version already installed, use:

```bash
sudo dnf reinstall ./src-tauri/target/release/bundle/rpm/*.rpm
```

#### 4. Register the WPS add-in

Completely exit WPS before running this command from the repository root:

```bash
bash scripts/install-addin.sh
```

This copies the add-in into both `~/.local/share/Kingsoft/wps/jsaddons/` and `~/.local/share/Kingsoft/wps_intl/jsaddons/`, removes legacy `MathEditor` add-in directories, and registers EqnTide. This script **does not install the background service**; perform the next step separately.

#### 5. Install and enable the background bridge

From the repository root, without `sudo`:

```bash
bash scripts/install-bridge-service.sh
```

The script installs `~/.config/systemd/user/eqntide.service`, disables the old `matheditor.service`, and enables and starts the new service immediately. On future graphical logins, systemd starts the bridge automatically. This is a user-session service, not a boot service before login.

The service runs `target/release/eqntide-cli --server` from this checkout and sets `EQNTIDE_DESKTOP_BIN=/usr/bin/eqntide-desktop`. Keep the repository and release binary at their installed paths. If you move the checkout, build the CLI and rerun the service installer from the new location.

Do not start a second CLI server manually while this service owns port `17890`. The bridge stays running in the background and launches EqnTide Desktop when an add-in opens an editing session.

#### 6. Verify the installation

```bash
systemctl --user is-enabled eqntide.service
systemctl --user status eqntide.service --no-pager
curl http://127.0.0.1:17890/health
```

Expected results: `enabled`, `active (running)`, and JSON containing `"status":"ok"` and `"server":"eqntide-bridge"`. Reopen WPS Writer, select **EqnTide → Insert Equation**, and confirm that the desktop editor opens. The background bridge should already be available even before opening the editor.

Useful maintenance commands:

```bash
# Read recent bridge errors.
journalctl --user -u eqntide.service -n 80 --no-pager
# Restart after rebuilding the CLI.
systemctl --user restart eqntide.service
# Disable automatic startup and stop the bridge.
systemctl --user disable --now eqntide.service
# Remove the WPS add-in (does not stop/remove the bridge service).
bash scripts/install-addin.sh --uninstall
```

If the bridge is healthy but the editor does not appear, check `systemctl --user cat eqntide.service` for `EQNTIDE_DESKTOP_BIN` and verify `/usr/bin/eqntide-desktop` exists. On Fedora/KDE, the installed `/usr/share/applications/EqnTide.desktop` must contain `Exec=eqntide-desktop %u` and `MimeType=x-scheme-handler/eqntide`; reinstall the latest RPM if these are missing.

### Prepare and Install the Word/WPS Add-ins

There is no npm, Cargo, or bundling step for either add-in. Keep each directory intact:

```text
word-addin/   # Microsoft Word Office.js add-in
wps-addin/    # WPS Office JavaScript add-in
```

When making a portable distribution, copy both directories beside `eqntide-cli[.exe]` so the bridge can serve `/word-addin/*` and `/wps-addin/*`. When running from a source checkout, start the bridge from the repository root:

```bash
./target/release/eqntide-cli --server
```

The desktop application starts the same bridge automatically. Confirm it is available before opening the add-in:

```bash
curl http://127.0.0.1:17890/health
```

#### Windows: Word and WPS Office

Run the PowerShell registration script from the repository root, then restart Word or WPS Office:

```powershell
.\scripts\install-addin.ps1
```

The script registers `word-addin/manifest.xml` in the current user's Word developer catalog and copies `wps-addin/` into the WPS `jsaddons` directory. To remove both registrations:

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

#### macOS: Word and WPS Office

```bash
./scripts/install-addin.sh
```

The script sideloads the Word manifest into Word's `wef` directory and installs the WPS add-in when a WPS container is present. Restart the Office application after registration.

### Manual Add-in Sideloading

Use these steps when the registration scripts are unavailable or when you need to inspect the installation manually. Start the EqnTide desktop application or CLI bridge before opening an add-in.

#### Microsoft Word on Windows

Register the manifest in the current user's Word developer catalog from PowerShell, replacing the example path with the absolute path to this checkout:

```powershell
$manifest = "C:\path\to\eqntide\word-addin\manifest.xml"
New-Item -Path "HKCU:\Software\Microsoft\Office\16.0\WEF\Developer" -Force
Set-ItemProperty -Path "HKCU:\Software\Microsoft\Office\16.0\WEF\Developer" `
  -Name "EqnTide" -Value $manifest
```

Restart Word, then open **Insert → My Add-ins → Developer Add-ins** and select **EqnTide**.

#### Microsoft Word on macOS

```bash
mkdir -p ~/Library/Containers/com.microsoft.Word/Data/Documents/wef
cp word-addin/manifest.xml \
  ~/Library/Containers/com.microsoft.Word/Data/Documents/wef/manifest-eqntide.xml
```

Restart Word, then activate EqnTide from **Insert → Add-ins → Developer Add-ins**.

#### Microsoft Word on the Web

Open a document in Word on the Web, choose **Insert → Add-ins → More Add-ins → Manage My Add-ins → Upload My Add-in**, and upload `word-addin/manifest.xml`. The browser must be able to access the bridge URLs in the manifest; organizational Office policies may disable developer add-in uploads or local HTTP endpoints.

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
- **Port 17890 is already occupied:** Stop the other process using the port before starting EqnTide. Although the CLI supports `--port`, the bundled Word/WPS manifests and scripts are configured for port `17890`; using another port requires updating those add-in URLs as well.
- **The add-in files return 404:** Run the CLI bridge from the repository root, or place `word-addin/` and `wps-addin/` beside the distributed executable.
- **The ribbon does not appear:** Fully close all Word/WPS processes, verify that the manifest or `jsaddons` files were installed for the current user, and reopen the application.
- **Linux WebKit/GStreamer warnings:** EqnTide disables MathLive audio effects; remaining media-framework warnings are normally harmless unless the editor fails to render.
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
