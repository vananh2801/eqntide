# Proposal

## Why

In Microsoft Word (`word-template/EqnTide.bas` / `EqnTide.dotm`) and WPS Office (`wps-addin/main.js`, `wps-addin/taskpane.js`), batch `Toggle LaTeX` and `Upgrade` operations on multi-page documents suffered from three production issues: (1) quadratic COM/range scans, uncompacted Flat OPC XML payloads carrying `/word/styles.xml` and `/word/numbering.xml`, and unbounded concurrent bridge requests causing HTTP 503/UI freezes; (2) full-document (`Ctrl+A`) or mixed-formatting selections collapsing prose into one bare LaTeX block or sending `wdUndefined` (`9999999`) `font_pt` values; and (3) Windows file-locking failures when `/convert/document` writes in place while the active `.docx` remains open in Word or WPS.

## What Changes

- **Flat OPC payload compaction & fast shape classification**: Strip bloated non-equation Flat OPC parts (`/word/styles.xml`, `/word/styleswitheffects.xml`, `/word/numbering.xml`, `/word/fonttable.xml`, `/word/settings.xml`, `/word/websettings.xml`, `/word/theme/*`, `/docProps/*`, `/customXml/*`) in `CompactFlatOpc` (VBA) and `compactFlatOpcXml` (WPS), add `QuickClassifyShape` / `quickClassifyShape` to skip non-equation shapes without reading `WordOpenXML`, and reuse HTTP/token state in VBA (`m_CachedBridgeToken`, `m_SnippetHttp` with retry/timeouts) while removing $O(N^2)$ string concatenation in `JsonEscape` and `ExtractJsonField`.
- **Bounded concurrency, chunked DOM updates, and $O(N \log N)$ / $O(1)$ range tracking**: Cap concurrent `/convert/snippet` requests at `8` in WPS (`runBoundedQueue`) and release request XML payloads immediately on dispatch; yield DOM replacements in chunks of `10` (`applyItemsInChunks` via `setTimeout`) in WPS and disable `ScreenUpdating`/`Pagination` with single `UndoRecord` + periodic `DoEvents`/`StatusBar` progress in Word VBA; replace linear scans with `cumShift` offset tracking, chunked range resolution (`ScanRangeForMatch` / `findExactRangeInBounds`), and binary search for `InlineShapes`/`OMaths` collections (`> 16` items).
- **Safe `Ctrl+A` prose detection, delimiter bounds, and `font_pt` clamping**: Prioritize equation target detection over bare-text `isLatexText` fallback on non-empty selections, reject cross-blank-line or oversized (`> 2000` chars) delimiter spans and embedded-object/table-cell markers (`\u0001`, `\u0007`, `\t`), clamp `font_pt` to `1.0..200.0` (falling back to `12.0` for `wdUndefined` `9999999`), trim trailing `\u0007` table-cell breaks before paragraph `InsertXML`, and surface clear batch summaries (`Found`, `Converted`, `Skipped/Errors`) while skipping HTTP 422 unsupported formulas.
- **Windows file-lock release & safe reopen for disk upgrade**: Reject read-only documents early, save and verify `Saved`, and on Word (all platforms) and Windows WPS close the target document before calling `/convert/document` (creating a temporary blank document in WPS when closing the sole open window so the add-in/taskpane stays loaded), then reopen the upgraded file and close the temporary document—even on bridge errors—while preserving hot-refresh on Linux WPS.
- **macOS WPS bridge token & cold-start parity**: Add `~/Library/Application Support/com.eqntide.EqnTide/bridge.token` discovery and `/usr/bin/open` URI launch fallback in `wps-addin/`.

## Capabilities

### New Capabilities
- `office-addin-batch-reliability`: Reliable, bounded-concurrency batch LaTeX toggling, Flat OPC compaction, prose/font-size guards, macOS bridge discovery, and Windows file-lock-safe disk upgrades across Word VBA and WPS Office add-in clients.

### Modified Capabilities
<!-- None: existing `wps-mathtype-ole-insertion` single-object OLE span and post-insert diagnostics remain unchanged. -->

## Impact

- Affected files: `word-template/EqnTide.bas`, `word-template/EqnTide.dotm`, `wps-addin/main.js`, `wps-addin/taskpane.js`, `tests/python/test_word_ribbon.py`, `tests/wps/test_wps_insertion.js`.
- No Rust crate or bridge HTTP API schema changes required (`POST /convert/snippet` and `POST /convert/document` contracts remain unchanged).
