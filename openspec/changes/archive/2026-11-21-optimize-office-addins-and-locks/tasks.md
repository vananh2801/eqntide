# Tasks

## 1. Word VBA Template Batch Performance & File-Lock Release (`word-template/EqnTide.bas`, `EqnTide.dotm`)

- [x] 1.1 Add `CompactFlatOpc`, `QuickClassifyShape`, `IsEquationShape`, `ResolveMatchRange`, `ScanRangeForMatch`, `YieldIfNeeded`, and `UpdateProgressStatus` in `word-template/EqnTide.bas`, disable `ScreenUpdating`/`Pagination` with a single `UndoRecord`, iterate `OMaths`/`InlineShapes` via `For Each` + `cumShift`, reuse `Duplicate` ranges in `TrimTrailingRangeBreaks`/`InsertFlatOpcInline`, and cache `m_CachedBridgeToken`/`m_SnippetHttp` (`CallBridgeEx`) while replacing $O(N^2)$ string concatenation in `JsonEscape` (`Replace$`) and `ExtractJsonField` (`Join`); verify assertions in `tests/python/test_word_ribbon.py`.
- [x] 1.2 Update `UpgradeMathTypeMacro` in `word-template/EqnTide.bas` to reject `ReadOnly` documents, save and verify `doc.Saved`, close the document (`doc.Close 0`) before calling `/convert/document`, and reopen `docPath` in both normal and `UpgradeError` paths; rebuild/sync `word-template/EqnTide.dotm` and cover with `test_upgrade_releases_word_file_before_bridge_and_reopens_on_error` in `tests/python/test_word_ribbon.py`.
- [x] 1.3 Run `python3 -m unittest tests/python/test_word_ribbon.py` in an environment with `oletools` installed to verify the compiled VBA stream inside `word-template/EqnTide.dotm` passes all assertions.

## 2. WPS Add-in Bounded Concurrency, Compaction & Safe Selection (`wps-addin/main.js`, `wps-addin/taskpane.js`)

- [x] 2.1 Implement `compactFlatOpcXml` and `quickClassifyShape` across `wps-addin/main.js` and `wps-addin/taskpane.js`, and replace linear `InlineShapes`/`OMaths` lookups with binary search (`> 16` items) in `findInsertedInlineShape`, `findInsertedShapeByPos`, `findInsertedOMathByPos`, and `replaceEquationWithLatex`; verify via `node tests/wps/test_wps_insertion.js`.
- [x] 2.2 Implement `runBoundedQueue` (`<= 8` concurrent `/convert/snippet` requests, freeing request XML on dispatch) and `applyItemsInChunks` (`10`-item chunks yielding via `setTimeout`, freeing `convData`) for `batchLatexToMathType`, `batchEquationToLatex`, and taskpane `btnStartUpgrade`, plus `cumShift` and chunked `findExactRangeInBounds` scanning; verify via tests #9 and #11 in `node tests/wps/test_wps_insertion.js`.
- [x] 2.3 Harden `ToggleLatex`, `isLatexText`, `findAllLatexMatches`, `getDocumentFontSize`/`getTargetFontSize` (`1.0..200.0` clamping for `wdUndefined` `9999999`), table-cell `\u0007` break trimming in `replaceEquationWithLatex`, and batch summary / 422 reporting; verify via tests #7 and #10 in `node tests/wps/test_wps_insertion.js`.

## 3. WPS Windows Disk-Upgrade Lock Release & macOS Parity (`wps-addin/main.js`, `wps-addin/taskpane.js`)

- [x] 3.1 Update `UpgradeLegacyEquations` (`wps-addin/main.js`) and `btnDiskUpgrade` (`wps-addin/taskpane.js`) to reject `ReadOnly` documents, save & verify `doc.Saved`, open a temporary blank document (`app.Documents.Add()`) on Windows when closing the sole open document before `/convert/document`, reopen the target file and close the temp document on both success and failure, and retain Linux hot-refresh; verify via test #8 in `node tests/wps/test_wps_insertion.js`.
- [x] 3.2 Add macOS `bridge.token` path (`~/Library/Application Support/com.eqntide.EqnTide/bridge.token`) in `getBridgeToken()` (`main.js` and `taskpane.js`) and `/usr/bin/open` fallback in `ColdStartDesktopEditor()` (`main.js`).

## 4. Regression Suite Verification

- [x] 4.1 Run `node tests/wps/test_wps_insertion.js` and verify all WPS insertion, toggle, `Ctrl+A` priority, `font_pt` clamping, bounded concurrency, Flat OPC compaction, and Windows disk-upgrade lock-release tests pass.
