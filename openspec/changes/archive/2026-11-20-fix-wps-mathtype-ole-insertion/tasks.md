# Tasks

## 1. Preserve Full OLE Equation Ranges in EditEquation and ToggleLatex

- [x] 1.1 Update `EditEquation` and `ToggleLatex` in `wps-addin/main.js` to stop clamping multi-character `InlineShape` ranges (such as 26-char `{ EMBED Equation.DSMT4 }` OLE fields like `[285..311]`) down to 1 character, trimming only trailing `\r`/`\n` before pinning `g_editRangeStart..g_editRangeEnd` (plus any adjacent `U+200C`). Add regression tests in `tests/wps/test_wps_insertion.js` covering multi-character OLE `InlineShape.Range` preservation and verify `npm test --prefix tests/wps` passes.

## 2. Harden WPS MathType OLE + EMF Range Replacement and Shape Lookup

- [x] 2.1 Update `insertMathTypeAtDocRange` and `InsertMathTypeEquation` in `wps-addin/main.js` to trim trailing breaks on `targetRange`, cleanly clear/replace non-collapsed existing `InlineShape`/OLE ranges before `InsertXML` (re-anchoring at `doc.Range(insertPos, insertPos)`), and guard `insertedEnd` bounds (`insertedEnd > insertPos && insertedEnd <= newEnd`) before suffix verification and break deletion; verify existing and new replacement mocks pass with `npm test --prefix tests/wps`.
- [x] 2.2 Add `doc.InlineShapes` fallback and bounded start-offset matching (`[insertPos, insertPos + 5]`, nearest start) to `insertMathTypeAtDocRange` in `wps-addin/main.js` and `insertUpgradedFlatOpcAtRange` in `wps-addin/taskpane.js`, and verify mocked WPS tests in `tests/wps/test_wps_insertion.js` pass when `range.InlineShapes` is empty or `Range.Start` is offset by `+1`.

## 3. Separate XML Import Errors from Optional Formatting & Surface Diagnostics

- [x] 3.1 Update `insertMathTypeAtDocRange`, `InsertMathTypeEquation`, and `HandleSessionResult` in `wps-addin/main.js` so a successful `InsertXML` with missing `InlineShape` logs a `wpsLog` warning without triggering a false fatal alert, while actual `InsertXML` / range errors log and display the concrete error detail; add unit tests in `tests/wps/test_wps_insertion.js` verifying both behaviors via `npm test --prefix tests/wps`.

## 4. End-to-End Verification

- [x] 4.1 Run the full WPS test suite (`npm test --prefix tests/wps`) and verify live WPS Writer round-trip (`Edit Equation` on an existing MathType OLE picture → `Ctrl+S` in EqnTide Desktop → confirm updated OLE + EMF replaces the whole `[285..311]` object cleanly with no alert, and `Toggle LaTeX` on the OLE object leaves no orphan field characters).
