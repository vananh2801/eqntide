# Design

## Context

See [proposal.md](./proposal.md) for motivation. Current constraints in the Word VBA (`word-template/EqnTide.bas` / `EqnTide.dotm`) and WPS JS (`wps-addin/main.js`, `wps-addin/taskpane.js`) clients:
- Office COM / WPS JS automation calls (`Range.Text`, `Range.WordOpenXML`, `InlineShapes.Item(i)`, `OMaths.Item(i)`) cross IPC/COM boundaries on every property read. Calling them inside nested loops or per-character scans turns 100+ equation documents into $O(N^2)$ stalls.
- Word's and WPS's `WordOpenXML` serialization of a single equation includes the host document's full `/word/styles.xml`, `/word/numbering.xml`, `/word/fonttable.xml`, and theme parts (often 30–150 KB per snippet), inflating JSON payload serialization, bridge HTTP parsing, and VBA `JsonEscape` / `ExtractJsonField` overhead.
- On Windows, both Microsoft Word and WPS Writer hold exclusive write-deny OS locks on open `.docx` files, causing `POST /convert/document` (which writes atomically and renames over `input_path`) to fail with sharing-violation errors if the file remains open. Moreover, closing the *only* open document in WPS Writer terminates the document window and unloads the JS add-in / taskpane WebView unless another document stays open.

## Goals / Non-Goals

**Goals:**
- Reduce batch latency and eliminate UI/bridge hangs across 30–500+ equation documents in both Word VBA and WPS JS.
- Prevent false bare-LaTeX conversions on `Ctrl+A` prose selections and prevent `wdUndefined` (`9999999`) `font_pt` errors.
- Make `/convert/document` disk upgrades reliable on Windows (Word + WPS) without unloading the WPS taskpane or stranding users with closed files on error.
- Keep all existing bridge endpoints (`/convert/snippet`, `/convert/document`) and single-equation insertion behavior intact.

**Non-Goals:**
- Modifying the Rust bridge HTTP server or `.docx` conversion pipelines (covered by `optimize-conversion-memory`).
- Adding new Ribbon UI buttons or changing icon assets (`office-addin-icons`).
- Changing single-object MathType OLE range/fallback semantics (`wps-mathtype-ole-insertion`).

## Decisions

### 1. Client-Side Flat OPC Compaction & Shape Pre-Classification
- **Decision**: Implement `CompactFlatOpc` (Word VBA) and `compactFlatOpcXml` (WPS `main.js` & `taskpane.js`) using string part scanning (`<pkg:part ...>...</pkg:part>`) when `xml.length >= 16384` to strip non-equation parts (`/word/styles.xml`, `/word/styleswitheffects.xml`, `/word/numbering.xml`, `/word/fonttable.xml`, `/word/settings.xml`, `/word/websettings.xml`, `/word/theme/*`, `/docProps/*`, `/customXml/*`). Pair this with `QuickClassifyShape` / `quickClassifyShape` to classify `InlineShape` objects by `AlternativeText`, `Type`, and `OLEFormat.ProgID` before reading `WordOpenXML`.
- **Rationale**: String slicing over `<pkg:part>` avoids slow `DOMParser` / `MSXML` roundtrips for compaction and cuts snippet payloads from ~100 KB down to ~2–5 KB. In Word VBA, `JsonEscape` uses `Replace$` and `ExtractJsonField` uses a dynamic `chunks()` array + `Join(chunks, "")` to eliminate $O(N^2)$ string reallocation; `m_CachedBridgeToken` and `m_SnippetHttp` (with `setTimeouts` and 1 retry) avoid re-opening the token file and WinHTTP object per equation.
- **Alternative considered**: Stripping parts server-side in Rust after upload. Rejected because the main bottleneck on Office clients is VBA/JS JSON escaping and HTTP body transmission *before* Rust receives the request.

### 2. Bounded HTTP Concurrency (`8`) and Chunked DOM / COM Execution
- **Decision**:
  - In WPS (`main.js` and `taskpane.js`), replace unbounded `for`-loops of `ajaxPost` calls with `runBoundedQueue(..., 8, ...)` (`BATCH_HTTP_CONCURRENCY = 8`, `TASKPANE_HTTP_CONCURRENCY = 8`), nulling `item.xml` as soon as the request is sent. Apply sorted reverse-order DOM replacements via `applyItemsInChunks(..., 10, ...)` (`setTimeout(step, 0)` between 10-item chunks) and null `convData` after each replacement.
  - Use `cumShift` offset tracking + chunked `512`-char window scanning (`ResolveMatchRange` / `ScanRangeForMatch` in VBA; `findExactRangeInBounds` in WPS) and binary search on `InlineShapes` / `OMaths` when `Count > 16` (`findInsertedInlineShape`, `findInsertedShapeByPos`, `findInsertedOMathByPos`, `replaceEquationWithLatex`).
  - In Word VBA `ToggleLatexMacro`, bracket execution with `ScreenUpdating = False`, `Application.Options.Pagination = False`, and `Application.UndoRecord.StartCustomRecord`, iterate collections via `For Each` (avoiding $O(N^2)$ `Collection(i)` indexing), reuse `Duplicate` ranges (`tail.SetRange`), and call `YieldIfNeeded` (`DoEvents` every `0.15s`) + `UpdateProgressStatus` (`StatusBar` every `0.25s`).
- **Rationale**: The Rust bridge uses a bounded request pool; firing 200+ simultaneous XHR requests from WPS WebView triggers 503 rejections and memory spikes. `8` concurrent requests saturates the local bridge while chunked DOM replacement keeps WPS/Word responsive.
- **Alternative considered**: Sequential (`concurrency = 1`) XHR in WPS. Rejected because `8` concurrent requests substantially improves multi-equation throughput with minimal client memory overhead.

### 3. Selection Resolution Order, Delimiter Guards, and `font_pt` Sanitization
- **Decision**:
  - In `ToggleLatex` (WPS) and `ToggleLatexMacro` (Word), evaluate non-empty selections in strict order: (1) delimited LaTeX matches -> (2) existing OMML / MathType equation targets -> (3) bare-text LaTeX fallback only when neither delimited matches nor equations exist, and only if the selection passes `isLatexText` / clean-length (`<= 1000` chars, no multi-paragraph prose unless `\begin...\end`, no `\u0001`/`\u0007`/`\t`, and short algebraic prose guard).
  - Delimited matchers (`findAllLatexMatches`, `IsValidLatexMatch`) forbid `\r`/`\n`/`\u0007` inside `\(...\)`, reject matches `> 2000` chars, reject embedded object/cell markers and blank-line-spanning matches, and ignore empty inner expressions.
  - Extract `font_pt` via `isValidFontSizePt` (`1.0..200.0`), resolving per-match range font first and falling back to selection font or `12.0` so `wdUndefined` (`9999999`) never reaches `/convert/snippet`.
  - In `replaceEquationWithLatex` (WPS), strip trailing `\u0007` table cell end markers before `doc.Range(paragraph.Start, pEnd).InsertXML(xml)` and skip redundant `WordOpenXML` checks when `precheckedIsOmml === false`.
- **Rationale**: Prevents `Ctrl+A` on real exam documents (which often contain Vietnamese prose with `=` and `-` plus dozens of equations) from destroying the whole document into a single equation or failing with HTTP 400.

### 4. Windows File-Lock Release During `/convert/document`
- **Decision**:
  - Check `doc.ReadOnly` up front, call `doc.Save()`, and verify `doc.Saved` before proceeding.
  - In Word VBA (`UpgradeMathTypeMacro`), always close `doc` (`doc.Close 0`) and confirm it is absent from `Application.Documents` before `http.send payload`, then reopen `docPath` in both normal and `UpgradeError` paths.
  - In WPS (`UpgradeLegacyEquations` in `main.js` and `btnDiskUpgrade` in `taskpane.js`), gate pre-close on `isWindowsClient(filePath)` (Windows `navigator` or drive/UNC path): if `app.Documents.Count <= 1`, open a temporary blank document (`app.Documents.Add()`), close the target document (`doc.Close(false)`), run `/convert/document`, reopen via `reopenDocumentInApp`, and close the temporary blank document in both success and error callbacks. On Linux, keep the existing open-during-convert + post-alert hot-refresh flow.
- **Rationale**: Resolves Windows sharing violations on in-place `/convert/document` writes while preventing WPS Writer from tearing down the add-in/taskpane host when the sole document is closed.

### 5. macOS Bridge Discovery
- **Decision**: Add `homePath + "/Library/Application Support/com.eqntide.EqnTide/bridge.token"` to `getBridgeToken()` in `main.js` and `taskpane.js`, and add `/usr/bin/open` fallback for macOS in `ColdStartDesktopEditor()`.
- **Rationale**: Matches the `directories` / macOS bundle ID storage path used by the desktop bridge server on macOS.

## Risks / Trade-offs

- **WPS `app.Documents.Add()` failure or user closing the blank tab manually** -> Guarded with `try/catch` and explicit fallback instructions telling the user the saved `filePath` if `reopenDocumentInApp` fails.
- **Binary `.dotm` synchronization with `EqnTide.bas`** -> `word-template/EqnTide.dotm` embeds the compiled VBA stream; any future edit to `EqnTide.bas` must regenerate/sync `EqnTide.dotm` and pass `tests/python/test_word_ribbon.py` (requires `oletools`).
- **`CompactFlatOpc` / `compactFlatOpcXml` string slicing assumes standard `<pkg:part ...>` tags** -> Bounded by a `>= 16384` length threshold and falls back immediately to returning the original `xml` unmodified if any expected `<pkg:part` or `</pkg:part>` delimiter is missing.
