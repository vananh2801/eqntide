# ROADMAP: Cross-Platform MathType Alternative (Desktop App & Office/WPS Add-in)

> **License Objective:** This project and all its subcomponents MUST strictly maintain the **MIT License**.  
> Every library, asset, font, and dependency introduced across all phases must be compatible with MIT (e.g., MIT, Apache-2.0, BSD-2/3-Clause, SIL OFL 1.1). **GPL, LGPL, AGPL, and proprietary licenses are strictly prohibited.**

---

## 1. Executive Summary & Core Requirements

The goal of this project is to create an open-source, modern, cross-platform alternative to MathType that runs seamlessly on **Windows, macOS, and Linux**, with native-like integration into **Microsoft Word** and **WPS Office**.

### Key Differentiators & Mandatory Requirements
1. **Familiar MathType UX:**
   - Dedicated Ribbon Tab in Word and WPS Office.
   - Modal editor window with classic math palettes, virtual keyboard, and inline LaTeX autocomplete.
   - Prompt-on-close (`[X]` button): *"Do you want to insert/update this equation in the document? [Yes] [No] [Cancel]"*.
   - Bidirectional editing: Select existing equation in document ➔ click **Edit** (or double-click) ➔ loads formula in editor for modification.
2. **100% Full Bidirectional Compatibility with MathType Documents (`.docx`):**
   - **Forward Compatibility (Legacy MathType ➔ New Editor):**
     - Able to open and edit any `.docx` file created with MathType 6.x / Equation Editor 3.0.
     - Automatically parse and extract MTEF binary data from legacy OLE Compound File Binary (CFB) objects (`word/embeddings/oleObject*.bin`) and WMF comment streams without requiring Windows COM or MathType installed.
   - **Reverse Compatibility (New Editor ➔ Legacy MathType on other machines):**
     - When saving or exporting, write genuine **MathType OLE objects** (with MathType CLSID `{0002CE03-0000-0000-C000-000000000046}`, ProgID `Equation.DSMT4`, valid MTEF 5.0 stream, and vector EMF/WMF preview).
     - **Result:** A recipient on another computer who only has MathType installed can double-click the equation in Word, and MathType will open it for editing without any error or corruption.
   - **Batch Conversion:** Built-in tool to batch convert entire documents or folders between OMML (`<m:oMath>`) and MathType OLE back and forth.
3. **Hybrid / Dual-Storage Mode (Simultaneous OMML + `.bin` OLE Embedding):**
   - When inserting an equation into Word, the system supports saving **BOTH** formats simultaneously inside an `<mc:AlternateContent>` block:
     - `<mc:Choice Requires="mtype">`: Contains the `<w:object>` pointing to `word/embeddings/oleObject*.bin` (genuine MathType MTEF OLE container).
     - `<mc:Fallback>`: Contains the native Word OMML equation (`<m:oMath>`).
   - **Advantage:**
     - On computers with MathType: MathType detects the embedded `.bin` object and allows native double-click editing.
     - On computers without MathType (or on Word Online, mobile, Mac, WPS): Word displays and formats the native OMML equation beautifully, sharp at any zoom level, without missing font or broken image warnings.
4. **Lightweight & High Performance:** Powered by **Rust** and **Tauri v2** (< 20MB installer, < 60MB RAM footprint, instant cold-start < 200ms).

---

### High-Level Architecture Diagram

```
┌────────────────────────────────────────────────────────────────────────┐
│                   Microsoft Word / WPS Office                          │
│                                                                        │
│   [Ribbon Tab: Math Editor]                                            │
│   ├── [Insert Equation] ──▶ Calls localhost bridge                     │
│   ├── [Edit Equation]   ──▶ Extracts OMML or MathType OLE ──▶ Bridge   │
│   └── [Batch Convert]   ──▶ Converts document OMML ⇄ MathType OLE     │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ HTTP / WebSocket (localhost:17890)
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│               Local Tauri v2 Application (Desktop Agent)               │
│                                                                        │
│   ┌────────────────────────────────────────────────────────────────┐   │
│   │  Frontend UI (Webview)                                         │   │
│   │  - MathLive (WYSIWYG equation editor, LaTeX shortcuts, TeX font)│   │
│   │  - MathType-inspired Palettes (Fractions, Radicals, Matrices)   │   │
│   │  - "Save to Document?" confirmation on close (X)               │   │
│   │  - Storage Mode: [Hybrid: OMML + .bin] [MathType OLE] [OMML]   │   │
│   └───────────────────────────────┬────────────────────────────────┘   │
│                                   │ Tauri IPC (invoke)                 │
│   ┌───────────────────────────────▼────────────────────────────────┐   │
│   │  Rust Backend (Core Engine)                                    │   │
│   │  - Embedded Local API Server (Axum / tiny_http)                │   │
│   │  - System Tray & Single-Instance manager                       │   │
│   │  - Dual-Storage Packager (<mc:AlternateContent> OLE + OMML)    │   │
│   │  - Pure-Rust OLE CFB packager/extractor (`oleObject*.bin`)     │   │
│   │  - Vector EMF/WMF preview generator (STIX Two Math font)       │   │
│   │  - System Clipboard integration                                │   │
│   └────────────────────────────────────────────────────────────────┘   │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Licensing & Compliance Guidelines (Mandatory for AI Agents)

Every AI agent implementing tasks from this roadmap must strictly adhere to these compliance rules:

1. **Permitted Licenses:**
   - **MIT License** (Preferred)
   - **Apache License 2.0**
   - **BSD-2-Clause / BSD-3-Clause**
   - **SIL Open Font License 1.1 (OFL-1.1)** (For math fonts)
   - **CC0 / Public Domain**
2. **Strictly Forbidden:**
   - **GPL v2/v3, LGPL v2.1/v3, AGPL v3** (Strict copyleft or library copyleft is incompatible with MIT distribution).
   - Proprietary SDKs or binary blobs without explicit permissive redistribution licenses.
3. **Approved Stack Candidates:**
   - **Desktop Framework:** Tauri v2 (`crates.io/crates/tauri`) - MIT / Apache-2.0.
   - **Math Editor Core:** MathLive (`@cortex-js/mathlive`) - MIT License (Author: Arno Gourdol).
   - **Icons:** Lucide Icons (`lucide-react` / `lucide-static`) - ISC / MIT.
   - **Local Server:** `axum` / `tiny_http` - MIT.
   - **Fonts:** STIX Two Math, KaTeX fonts - SIL OFL 1.1.

---

## 3. Phased Implementation Roadmap

### Phase 1: Core Engine Adaptation & Local Communication Bridge
**Goal:** Expose the existing Rust `omml-converter` library to a lightweight local HTTP/IPC server that runs in the background.

- [ ] **Task 1.1: Local Bridge Server in Rust**
  - Integrate a minimal, zero-overhead HTTP server (`tiny_http` or `axum`) inside the Rust backend.
  - Listen on a loopback port (default: `127.0.0.1:17890`) with CORS restricted to local origins and Office/WPS add-in origins.
  - Implement endpoints:
    - `GET /health`: Health check, active version, and engine status.
    - `POST /open`: Command to focus/show the editor window. Takes payload `{ "data": "...", "format": "omml" | "mtef_base64" | "mathml", "session_id": "..." }`.
    - `POST /convert`: Convert between MathML, OMML, MTEF binary, and LaTeX via `omml-converter`.
    - `POST /submit`: Channel where the editor submits the finalized formula back to the pending Office/WPS session.
    - `POST /batch-convert`: Trigger full `.docx` document batch conversion between OMML and MathType OLE.
- [ ] **Task 1.2: Session & State Handling**
  - Implement a pending transaction state: When Word/WPS requests "Insert" or "Edit", assign a `session_id`.
  - Hold the HTTP long-poll or WebSocket connection until the user confirms or cancels the editor window.

---

### Phase 2: MathType OLE, OMML & Hybrid Storage Engine
**Goal:** Implement full bidirectional compatibility with legacy MathType equations and dual-storage embedding.

- [ ] **Task 2.1: Legacy MathType OLE Ingestion (Read & Parse)**
  - Support extracting MTEF 3.0 and MTEF 5.0 streams from legacy `.docx` objects:
    - OLE CFB storage streams: `\x01Ole`, `\x01CompObj`, `\x03ObjInfo`, `Equation Native`.
    - WMF/EMF comment records containing embedded MTEF headers.
  - Decode extracted MTEF into clean MathML, ready for editing inside MathLive.
- [ ] **Task 2.2: MathType OLE Package Generation (Write & Export)**
  - When the user selects "Export as MathType OLE":
    - Encode MathML ➔ MTEF 5.0 binary.
    - Generate CFB container (`oleObject*.bin`) with genuine MathType root CLSID (`{0002CE03-0000-0000-C000-000000000046}`) and ProgID (`Equation.DSMT4`).
    - Render sharp preview picture (vector EMF using STIX Two Math font outline engine).
    - Wrap into standard Word OpenXML `<w:object>` markup.
- [ ] **Task 2.3: Native Word OMML Generation (Standard Mode)**
  - Encode MathML ➔ Word OMML (`<m:oMath>`) with clean typography runs (`<m:r>`) and math fonts (Cambria Math / STIX Two Math).
  - Both Word (2007 - 365) and WPS Office natively render and edit without any plugin.
- [ ] **Task 2.4: Hybrid / Dual-Storage Mode Implementation (`mc:AlternateContent`)**
  - Implement simultaneous embedding:
    - Build `<mc:AlternateContent>` tree according to `src/docx/ole_xml.rs`.
    - Put the MathType OLE `<w:object>` referencing `word/embeddings/oleObject*.bin` into `<mc:Choice Requires="mtype">`.
    - Put the native `<m:oMath>` into `<mc:Fallback>`.
    - Store both the `.bin` OLE object and the native OMML in the document package.
    - Verify that Word displays the OMML equation directly, while MathType on another machine recognizes and edits the `.bin` package.

---

### Phase 3: Tauri v2 Desktop App & MathType-Style UI
**Goal:** Build the desktop editor window using Tauri v2, MathLive, and a modern, customizable MathType toolbar.

- [ ] **Task 3.1: Tauri v2 Project Setup**
  - Initialize Tauri v2 in the workspace (`src-tauri/` and `src-ui/`).
  - Configure single-instance plugin (`@tauri-apps/plugin-single-instance`).
  - Configure system tray support: App stays resident in system tray for instant (< 200ms) popup.
- [ ] **Task 3.2: MathLive Integration & Equation Canvas**
  - Embed `<math-field>` from MathLive.
  - Customize rendering settings: Configure TeX/Cambria Math fonts, font size, cursor blink, and selection styling.
  - Support dual-input: Point-and-click palettes + inline LaTeX autocomplete (`/`, `\sqrt`, `\int`, etc.).
- [ ] **Task 3.3: MathType Palette Toolbar**
  - Design categorized symbol palettes:
    1. **Fractions & Radicals:** Standard fraction, slanted, square root, n-th root.
    2. **Subscripts & Superscripts:** Top, bottom, pre-subscripts.
    3. **Integrals & Summations:** Definite/indefinite integrals, $\sum$, $\prod$, $\bigcup$, $\bigcap$.
    4. **Brackets & Fences:** Parentheses, square brackets, braces, angle brackets, absolute value bars with auto-expansion.
    5. **Matrices & Tables:** Presets ($2\times2$, $3\times3$) and custom dimension grid picker.
    6. **Greek & Math Symbols:** Lowercase/uppercase Greek, arrows, relational operators, set theory symbols.
- [ ] **Task 3.4: Output Mode Selector & "Save to Document" Flow**
  - Storage Mode selector in Settings / Toolbar:
    - **Hybrid (Recommended):** Saves both `.bin` OLE and native OMML via `mc:AlternateContent`.
    - **Native Word OMML:** Cleanest document footprint, best for modern Word & WPS.
    - **MathType OLE Only:** Legacy compatibility mode.
  - Intercept the window close event (`onCloseRequested`):
    - If equation modified: Show confirmation modal:
      *"Do you want to insert/update this equation in the document? [Yes] [No] [Cancel]"*.
    - **Yes:** Invoke Rust command to generate target format based on selected mode, return data to bridge, and hide window.
    - **No:** Discard changes and hide window.
    - **Cancel:** Return to editing.
  - Provide dedicated **"Insert into Document"** primary button and shortcut (`Ctrl + S` / `Cmd + S`).

---

### Phase 4: Microsoft Word Add-in (Office Web Add-in)
**Goal:** Create a clean, cross-platform Office Add-in that adds a dedicated Ribbon Tab in Word and controls the local editor.

- [ ] **Task 4.1: Office Add-in Manifest (`manifest.xml`)**
  - Define custom Ribbon Tab: `<Tab id="MathEditorTab" label="Math Editor">`.
  - Define custom Action Controls:
    - Button: **Insert Equation** (Large icon, launches empty editor).
    - Button: **Edit Equation** (Large icon, extracts selection).
    - Button: **Convert to OMML** (Batch converts all MathType OLE in docx to Word OMML).
    - Button: **Convert to MathType** (Batch converts all OMML in docx to MathType OLE).
    - Button: **Status** (Checks if local desktop agent is running).
- [ ] **Task 4.2: Insert Workflow Execution (`commands.js`)**
  - Ping `http://127.0.0.1:17890/health`. If not running, prompt user to launch desktop app.
  - Send request to `http://127.0.0.1:17890/open` with `mode: "insert"`.
  - Receive generated OOXML package and insert into Word:
    ```javascript
    context.document.setSelectedDataAsync(ooxmlString, {
      coercionType: Office.CoercionType.Ooxml
    });
    ```
- [ ] **Task 4.3: Edit Workflow Execution (OMML, MathType OLE & Hybrid)**
  - When **Edit Equation** is clicked:
    1. Retrieve selected range OOXML via `context.document.getSelection().getOoxml()`.
    2. Check if selection contains:
       - Case A: `<mc:AlternateContent>` ➔ Extract `<m:oMath>` or `oleObject*.bin`.
       - Case B: `<m:oMath>` ➔ Send OMML to bridge.
       - Case C: `<w:object>` (MathType OLE) ➔ Extract binary relationship, send OLE binary to bridge.
    3. Bridge decodes into MathML and opens editor with existing formula.
    4. On save: Replace selection in place with updated equation package.

---

### Phase 5: WPS Office JS Add-in
**Goal:** Provide full parity for WPS Office on Windows, Linux, and macOS using WPS JavaScript Add-in API.

- [ ] **Task 5.1: WPS Add-in Manifest & Ribbon UI**
  - Create WPS add-in directory structure (`ribbon.xml`, `main.js`).
  - Configure Ribbon tab for WPS Writer matching Word's appearance.
- [ ] **Task 5.2: WPS Document Manipulation & Compatibility**
  - Implement insertion logic using WPS KSO API:
    ```javascript
    wps.Application.Selection.Range.InsertXML(ooxmlPayload);
    ```
  - Implement selection extraction for native OMML, MathType OLE shapes, and `<mc:AlternateContent>` in WPS.
- [ ] **Task 5.3: Linux & macOS Validation for WPS**
  - Validate add-in registration and localhost socket permissions on Linux (Ubuntu, Debian, Fedora) and macOS.

---

### Phase 6: Standalone Desktop Features & Batch Converter GUI
**Goal:** Ensure the app is also a best-in-class standalone equation tool and document converter.

- [ ] **Task 6.1: Document Batch Converter Window**
  - Visual GUI interface for the existing CLI:
    - Drag & drop `.docx` files or entire folders.
    - Convert all MathType equations ➔ Native Word OMML.
    - Convert all Word OMML ➔ MathType OLE objects (with optional OMML fallback).
    - Real-time progress bar, summary table (converted / found / errors), and one-click "Open Output Folder".
- [ ] **Task 6.2: Global Hotkey (Instant Math Snippet)**
  - Register global hotkey (e.g., `Ctrl + Alt + M` / `Cmd + Alt + M`).
  - Pressing hotkey summons editor anywhere (browser, LaTeX document, chat).
- [ ] **Task 6.3: Multi-Format Copy & Export**
  - Copy as **Word OMML** (places raw OpenXML on clipboard for instant paste into any Office app).
  - Copy as **MathType OLE** (CFB binary on clipboard).
  - Copy as **MathML** (W3C standard).
  - Copy as **LaTeX** (ready for Overleaf, Markdown, typst).
  - Copy as **Vector SVG / EMF** or **High-DPI PNG**.

---

### Phase 7: Testing, CI/CD & Distribution Packaging
**Goal:** Automated cross-platform builds and easy, one-click installation for end users.

- [ ] **Task 7.1: Cross-Platform Packaging**
  - **Windows:** `.msi` and lightweight `.exe` installer. Auto-registers Word and WPS add-ins during install.
  - **macOS:** Universal `.dmg` (Apple Silicon & Intel).
  - **Linux:** `.deb`, `.rpm`, and `.AppImage`.
- [ ] **Task 7.2: Add-in Sideloading & Installation Scripts**
  - Automate manifest registration:
    - Windows: Office Add-in shared folder catalog registry script.
    - macOS: `~/Library/Containers/com.microsoft.Word/Data/Documents/wef/` manifest linker.
    - WPS: `%APPDATA%\Kingsoft\wps\jsaddons\` or `~/.local/share/Kingsoft/wps/jsaddons/` auto-installer.
- [ ] **Task 7.3: Automated License Audit**
  - Add `cargo-deny` and `license-checker` to GitHub Actions CI to ensure no non-MIT/OFL dependencies enter the codebase.

---

## 4. Immediate Next Steps for Implementation

1. **Sprint 1 (Local Bridge & Core Integration):** Set up the local HTTP server module in `src/` to handle `/open`, `/convert`, and `/submit`.
2. **Sprint 2 (Tauri v2 Workspace Initialization):** Create the Tauri app shell and hook up MathLive with the Hybrid Storage toggle.
3. **Sprint 3 (Word Add-in Manifest & Testing):** Write the minimal Word Add-in to verify the complete Insert & Edit roundtrip with both OMML and `.bin` OLE objects.
