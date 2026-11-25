# Proposal

## Why

In WPS Writer on both Linux and Windows, inserting or replacing a MathType OLE + EMF equation via `Range.InsertXML` (during **Edit Equation**, single **Toggle TeX**, or partial/batch **Toggle TeX**) can cause existing MathType equations outside the target range to disappear from the viewport until the document is saved, closed, and reopened. We need a structured investigation and test plan to isolate whether the trigger is in EqnTide's Flat OPC XML structure or strictly inside WPS Writer's OLE/VML render cache, and to verify whether a non-scrolling refresh workaround can keep existing equations visible.

## What Changes

- Run controlled diagnostic experiments on live WPS Writer (Linux and Windows) comparing:
  1. Self-reinsertion of WPS-exported `WordOpenXML` vs. EqnTide's `mathml_to_flat_opc` output (specifically testing `<v:shapetype id="_x0000_t75">` inclusion and `ObjectID` format).
  2. Non-destructive post-insert repaint/invalidation mechanisms across `EditEquation`, single `ToggleLatex`, and batch `runLatexToMathTypeBatch` (e.g. touching existing `InlineShapes`, `ScreenRefresh`, or view/scroll-preserving invalidation) without jumping the viewport to page 1.
- If a reliable XML adjustment or host repaint workaround is verified without regressions, apply it to Flat OPC packaging (`src/docx/ole_xml.rs` / `src/docx/flat_opc.rs`) and/or `wps-addin/main.js`; otherwise document the confirmed WPS host boundary in `KNOWN_ISSUES.md`.

## Capabilities

### New Capabilities

### Modified Capabilities

- `wps-mathtype-ole-insertion`: Verify and, if feasible via XML or non-scrolling host refresh, preserve visible rendering of existing MathType equations outside the target range after MathType OLE + EMF insertion, editing, and LaTeX toggling in WPS Writer (Linux and Windows).

## Impact

- Investigation & tests target `wps-addin/main.js` (`insertMathTypeAtDocRange`, `refreshToggleScreen`, `InsertMathTypeEquation`), `src/docx/ole_xml.rs`, `src/docx/flat_opc.rs`, and `tests/wps/test_wps_insertion.js`.
- `KNOWN_ISSUES.md` will be updated with the confirmed root cause or narrowed scope after live WPS testing.
