# EqnTide

A standalone, high-performance **bidirectional math equation converter and editor** for `.docx` documents between **OMML** (Word's native Office Math Markup Language), **MathType** (`Equation.DSMT4` / `Equation.3` OLE objects with EMF/WMF/PNG previews), and **LaTeX** — without requiring Microsoft Word, MathType, or Windows for conversion.

---

## Features

- **Forward (`OMML → MathType`)**: Converts Word `<m:oMath>` equations into embedded MathType 5.0 OLE objects (`Equation.DSMT4`) with crisp vector EMF (or PNG) previews using bundled STIX Two Math fonts.
- **Reverse (`MathType → OMML`)**: Extracts MTEF v3/v5 binary streams from OLE `.bin` containers or WMF/EMF previews and converts them into native Word `<m:oMath>` XML.
- **Legacy Upgrade (`--upgrade`)**: Repairs and re-typesets legacy `Equation.3` / headerless MTEF equations into modern MathType 7 (`Equation.DSMT4` + EMF) or native OMML.
- **Full Package & Batch Processing**: Converts `document.xml`, headers, footers, footnotes, and endnotes across single `.docx` files or entire folder trees recursively.
- **Native Rust Desktop & Office Add-ins**: Visual structural equation editor (`egui` + `eframe`, no WebView) with live LaTeX sync, Word VBA template (`EqnTide.dotm`), and WPS Office JavaScript add-in communicating via a local loopback bridge (`http://127.0.0.1:17890`).

## Architecture

```text
                    +------------------------------------------+
                    |           .docx (Word Document)          |
                    +-----------+------------------+-----------+
                                |                  ^
        +-----------------------v------+    +------+-----------------------+
        |   Forward: OMML -> MathType  |    |   Reverse: MathType -> OMML  |
        +---------------+--------------+    +--------------+---------------+
                        v                                  ^
         [src/omml/decode.rs]               [src/docx/pipeline.rs]
         Parse <m:oMath> -> MathNode        Replace <w:object> with <m:oMath>
                        |                                  ^
                        v                                  |
         +--------------+--------------+    [src/omml/encode.rs]
         |     MathNode (MathML AST)   +---> Encode MathNode -> <m:oMath> XML
         +--+-----------------------+--+                   ^
            |                       |                      |
            v                       v                      |
   [src/render/mod.rs]   [src/mtef/encode.rs]   [src/mtef/decode.rs]
   Layout & Glyph Engine Encode MathNode ->     Decode MTEF v3/v5 -> MathNode
   (EMF / PNG / SVG)     MTEF 5.0 + OLE .bin               ^
            |                       |                      |
            +-----------+-----------+       [src/ole.rs]   |
                        v                   Extract MTEF from OLE .bin / WMF
         [src/docx/ole_xml.rs]
         Embed OLE .bin + Preview in .docx
```

---

## Repository Structure

| Path | Description |
| --- | --- |
| `src/`, `Cargo.toml` | Core Rust conversion engine, MTEF/OMML/OLE codecs, STIX renderer, HTTP bridge, and `eqntide-cli`. |
| `crates/eqntide-editor/` | Shared structural equation editor state, caret/selection model, templates, and undo/redo engine. |
| `desktop/` | Native Rust desktop app (`eqntide-desktop`) built with `egui` / `eframe` and CPU software fallback. |
| `word-template/` | Microsoft Word global VBA template (`EqnTide.dotm`) and Python builder (`build_dotm.py`). |
| `wps-addin/` | WPS Office JavaScript add-in, custom Ribbon UI, upgrade taskpane, and icons. |
| `scripts/` | Native package builder (`package_native.py`), NSIS installer (`installer.nsi`), icon renderer, and add-in setup scripts. |
| `tests/` | Rust conversion matrix, Python packaging/DOTM tests, WPS Node.js tests, and `run_all.sh`. |

---

## Installation & Office Integration

> **Platform scope:** Official builds and CI target **Windows (x64)** and **Linux (x64)**. macOS support is currently suspended due to lack of test hardware. Word integration uses the `.dotm` VBA template only (no Word web add-in); WPS uses its JS add-in. Both talk to the local bridge on `http://127.0.0.1:17890`.

### Windows

- **Installer (`EqnTide-<version>-Setup.exe`, recommended):** Installs `eqntide-desktop.exe` and `eqntide-cli.exe`, registers the `eqntide://` URI handler, installs `EqnTide.dotm` into Word `STARTUP`, registers the WPS add-in, and configures the hidden per-user login bridge. Close Word and WPS before upgrading.
- **Portable (`eqntide-native-preview-<version>-x86_64-pc-windows-msvc.zip`):** Extract the archive and run `install-addins.bat` to register `eqntide://`, Word `.dotm`, and WPS add-ins (`scripts\install-addin.ps1 -Uninstall` removes them). Use `start-server.bat` if you want a standalone bridge outside the desktop app.

### Linux

- **DEB / RPM packages:** Install `eqntide-native-preview` (`sudo dnf install ./dist/linux/0.8.1/eqntide-native-preview-0.8.1-1.x86_64.rpm` or `sudo apt-get install ./dist/linux/0.8.1/eqntide-native-preview_0.8.1_amd64.deb`). Binaries and resources live under `/usr/lib/eqntide` with launchers in `/usr/bin`, and hooks configure the WPS add-in plus per-user `eqntide.service`.
- **Portable (`.tar.gz`):** Keep the extracted folder in a stable location and run:
  ```bash
  ./install-user.sh                  # Register eqntide:// desktop entry
  bash scripts/install-addin.sh      # Register WPS JS add-in (--uninstall to remove)
  bash scripts/install-bridge-service.sh  # Optional systemd user bridge (--print-unit to preview)
  ```

### Manual Add-in Sideloading

Start `eqntide-desktop` or `eqntide-cli --server` before opening an add-in:

- **Microsoft Word (Windows):** Close Word and copy `word-template/EqnTide.dotm` to `%APPDATA%\Microsoft\Word\STARTUP\` (macro policy must permit loading the template).
- **WPS Office (Windows / Linux):** Copy `wps-addin/` to `<jsaddons>/EqnTide_1.0.0/` (`%APPDATA%\Kingsoft\wps\jsaddons\` on Windows; `~/.local/share/Kingsoft/wps/jsaddons/` or `wps_intl` on Linux), add `publish.xml` pointing to `EqnTide_1.0.0/`, enable `EqnTide` in `authaddin.json`, and restart WPS Writer.

### Troubleshooting & Bridge Diagnostics

- **Verify local bridge:** Open or curl `http://127.0.0.1:17890/health` (expects JSON with `"status":"ok"`). Only one process can bind port `17890`.
- **Linux service logs:**
  ```bash
  systemctl --user status eqntide.service
  journalctl --user -u eqntide.service -n 50 --no-pager
  ```
- **Windows bridge logs:** Stored under `%LOCALAPPDATA%\EqnTide\logs`. Run `scripts\install-bridge-startup.ps1` to repair login startup (or `-Uninstall` to remove it).
- **Privacy:** All conversion and UI sessions run locally on `127.0.0.1`; no cloud service or telemetry is used.

---

## Desktop Editor Quick Guide

`eqntide-desktop` launches directly into the visual equation editor (or opens a session/tab via URI, e.g. `eqntide-desktop 'eqntide://open?tab=batch'`).

| Shortcut | Action |
| --- | --- |
| `Ctrl+F` / `Ctrl+R` | Insert fraction / radical (wraps selection) |
| `Ctrl+H` / `Ctrl+L` / `Ctrl+J` | Superscript / subscript / sub-superscript |
| `Tab` / `Shift+Tab` | Move between structural placeholders |
| `\` in canvas | Open native LaTeX command popup (`↑/↓`, `Tab`/`Enter`, `Esc`) |
| `Ctrl+Z` / `Ctrl+Y` | Undo / redo (includes source-sync and matrix edits) |
| `Ctrl+S` | Save & return equation to Word/WPS (or save MathML standalone) |

- **Editing & Live LaTeX:** Typing in the LaTeX source field updates the canvas after a `180 ms` debounce; invalid drafts keep the last valid formula visible and report the error in the status bar. Extended commands (`\dfrac`, `\tfrac`, `\sfrac`, `\atopfrac`, `\prescript`, `\overset`, `\underset`, `\limits`, `\nolimits`, `\enclose{notation}{body}`, `\boxed`, `\cancel`, `\bcancel`, `\xcancel`) are mapped in [src/catalog.rs](src/catalog.rs).
- **Output Modes (MathType vs. OMML):** With **MathType** selected, Word and WPS insert an `Equation.DSMT4` OLE object paired with a vector EMF preview (both carry identical MTEF streams). With **OMML** selected, native Office math XML is inserted.
- **Batch DOCX:** Converts folders safely using an output suffix by default; **Overwrite original documents** writes to a temporary sibling file before committing.
- **Graphics Backend Flags:** GPU-less VMs (`Basic Render Driver`, `llvmpipe`, `lavapipe`) automatically fall back to CPU 2D software rendering. Override via `--software` / `EQNTIDE_RENDERER=software`, `--wgpu` / `EQNTIDE_RENDERER=wgpu`, or `--vsync`.

---

## CLI & Library Usage

### CLI (`eqntide-cli`)

```bash
# Forward: OMML -> MathType (writes <input>_mathtype.docx or <dir>_mathtype/)
eqntide-cli input.docx
eqntide-cli ./my_documents -o ./my_documents_mathtype

# Reverse: MathType -> Word OMML
eqntide-cli input.docx -r --math-font "Cambria Math"

# Upgrade legacy Equation.3 / MTEF equations to modern MathType or OMML
eqntide-cli legacy.docx --upgrade --upgrade-target mathtype

# Run the standalone HTTP bridge for Word/WPS add-ins
eqntide-cli --server
```

| Flag / Option | Description |
| --- | --- |
| `<INPUT>` | Source `.docx` file or directory (scanned recursively). Required unless `--server` is set. |
| `-o, --output <PATH>` | Output `.docx` file or directory path. |
| `-r, --to-omml` | Convert embedded MathType OLE/WMF equations to native Word OMML (`<m:oMath>`). |
| `-u, --upgrade` | Re-typeset legacy and existing MathType equations (conflicts with `--to-omml`). |
| `--upgrade-target <FMT>` | Target for `--upgrade`: `mathtype` (default) or `omml`. |
| `--math-font <FONT>` | Font applied to converted OMML runs (`<w:rFonts>`) in `--to-omml` mode. |
| `--preview <FORMAT>` | Preview image format for MathType mode: `emf` (vector, default) or `png` (raster). |
| `--font-size <PT>` | Preview point size in MathType mode (`4`–`72`, default: `12`). |
| `--omml-fallback` | Keep OMML visible via `mc:AlternateContent` with MathType stored in background. |
| `-j, --jobs <N>` | Maximum parallel worker threads (default: all CPU cores). |
| `-s, --server` | Start local HTTP bridge (`--host 127.0.0.1`, `--port 17890`). |
| `-v, --verbose` / `-q, --quiet` | Increase progress logging or suppress info output. |

### Rust Library (`eqntide`)

Add `eqntide = "0.8.1"` to `Cargo.toml`:

```rust
use eqntide::{convert, convert_bytes, convert_to_omml_with, ToOmmlOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fwd = convert("document.docx", "document_mathtype.docx")?;
    println!("OMML -> MathType: {}/{}", fwd.converted, fwd.found);

    let opts = ToOmmlOptions { math_font: Some("Cambria Math".into()) };
    convert_to_omml_with("document_mathtype.docx", "document_omml.docx", &opts)?;

    let docx_bytes = std::fs::read("document.docx")?;
    let (out_bytes, _) = convert_bytes(&docx_bytes)?;
    std::fs::write("in_memory_out.docx", out_bytes)?;
    Ok(())
}
```

Low-level codecs are exposed under `eqntide::omml` (`omml_to_mathml`, `mathml_to_omml`, `mathml_to_omml_xml`, `apply_math_font`), `eqntide::mtef` (`encode::mathml_to_mtef_bytes`, `make_equation_native`, `mtef_to_mathml`, `ole_to_mathml`), and `eqntide::ole` (`make_ole`, `extract_mtef_from_ole`, `extract_mtef_from_wmf`).

---

## Building, Testing & Packaging

### Prerequisites

- **Rust:** Stable toolchain via [rustup](https://rustup.rs/).
- **Linux build/runtime libs** (no WebKit required):
  - *Fedora:* `sudo dnf install git gcc gcc-c++ make pkgconf-pkg-config rpm-build python3 libX11-devel libXi-devel libXrandr-devel libxkbcommon-devel mesa-libGL-devel wayland-devel`
  - *Ubuntu/Debian:* `sudo apt-get install git build-essential pkg-config python3 dpkg-dev libx11-dev libxi-dev libxrandr-dev libxkbcommon-dev libgl1-mesa-dev libwayland-dev`
  - *Runtime:* OpenGL, X11 or Wayland, XDG Desktop Portal, and `zenity`.
- **Windows:** Visual Studio C++ Build Tools + Windows SDK (`.cargo/config.toml` links static CRT for `x86_64-pc-windows-msvc`).
- **Packaging / DOTM rebuild:** Python 3.11+ (`dpkg-deb` for DEB, `rpmbuild` for RPM, NSIS for Windows `Setup.exe`).

### Build & Run Tests

```bash
cargo build --workspace --release --locked
./tests/run_all.sh --quick   # Rust workspace + conversion matrix + Python + WPS Node tests
```

### Package Distributions & Rebuild `.dotm`

```bash
# Rebuild Word VBA template after editing word-template/EqnTide.bas
python3 -m pip install -r word-template/requirements.txt
python3 word-template/build_dotm.py

# Linux packages -> dist/linux/<version>/
python3 scripts/package_native.py --target x86_64-unknown-linux-gnu --format all
```

```powershell
# Windows portable ZIP + NSIS Setup -> dist/windows/<version>/
cargo build --workspace --release --locked --target x86_64-pc-windows-msvc
python scripts/package_native.py --target x86_64-pc-windows-msvc
$version = python scripts/package_native.py --print-version
makensis "/DPACKAGE_DIR=$((Resolve-Path "dist/windows/$version").Path)" scripts/installer.nsi
```

GitHub Actions (`.github/workflows/ci.yml`, `build-native.yml`, `release.yml`) runs workspace tests, `clippy`, `cargo-deny`, JS/PowerShell/Python checks, and packages Linux/Windows artifacts on PRs to `main`, manual dispatch, or `v*` tags. On Linux with `cargo-xwin` and NSIS configured, you can also cross-build Windows binaries with `cargo xwin build --workspace --release --locked --target x86_64-pc-windows-msvc`.

---

## Documentation & Known Issues

- **[KNOWN_ISSUES.md](KNOWN_ISSUES.md)**: Documents host-specific caveats and implemented workarounds — including WPS Writer temporarily hiding outside equations after `Range.InsertXML` (mitigated via `View.ShowFieldCodes` refresh), automatic CPU 2D software renderer fallback on GPU-less Windows/Linux VMs, and intentional XML CRLF entity handling.
- **[tests/TESTING.md](tests/TESTING.md)**: Documents the 4-tier test suite (`run_all.sh --quick` / `--visual`), Rust conversion matrix, Python DOTM/packaging tests, mocked WPS JS tests, and white-background PNG visual layout exports under `target/test_exports/visual/`.

---

## References, Credits & License

EqnTide builds upon specifications and open-source reference work from:
1. [omml-converter](https://github.com/bohdanbodnarenko/omml-converter) (Bohdan Bodnarenko) — forward `.docx` packaging, OLE CFB, and MTEF 5.0 generation.
2. [mathtypejx](https://github.com/a917470154/mathtypejx) & [mathtype_to_mathml](https://github.com/jure/mathtype_to_mathml) (Jure Triglav) — MTEF v3/v5 decoding, WMF `MTra` extraction, and MathType character mappings.
3. Design Science MTEF v3 & v5 specifications, ECMA-376 (§22.1 OMML), Microsoft OpenType `MATH` table specification, Donald E. Knuth's *The TeXbook* (Appendix G), and W3C MathML Core.
4. [STIX Fonts](https://github.com/stipub/stixfonts), [Nunito](https://github.com/googlefonts/nunito), [JetBrains Mono](https://github.com/JetBrains/JetBrainsMono), and [Tabler Icons](https://github.com/tabler/tabler-icons).

### License

- **Source code:** [MIT License](LICENSE).
- **STIX Two Text & Math fonts (`assets/fonts/stixfonts-2.13b171/`):** [SIL Open Font License 1.1](assets/fonts/stixfonts-2.13b171/OFL.txt).
- **Nunito & JetBrains Mono fonts (`assets/fonts/Nunito/`, `assets/fonts/JetBrainsMono-2.304/`):** [SIL Open Font License 1.1](assets/fonts/Nunito/OFL.txt) and [JetBrains Mono OFL](assets/fonts/JetBrainsMono-2.304/OFL.txt).
- **Tabler Icons (`assets/icons/tabler-icons/`):** Copyright (c) 2020-2026 Paweł Kuna, licensed under the [MIT License](assets/icons/tabler-icons/LICENSE.txt).
- Full third-party copyright and license notices are listed in [NOTICE](NOTICE).
