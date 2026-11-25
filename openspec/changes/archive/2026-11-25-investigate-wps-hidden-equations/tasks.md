# Tasks

## 1. Live WPS XML Isolation (Linux & Windows)

- [x] 1.1 Prepare a multi-page test `.docx` containing at least 10 existing MathType OLE (`Equation.DSMT4`) equations across multiple pages alongside plain LaTeX expressions (`$...$`), and confirm baseline reproduction of hidden outside equations on Edit Equation, single Toggle TeX, and partial batch Toggle TeX in WPS Writer (Linux and Windows).
- [x] 1.2 Test WPS self-exported `WordOpenXML` re-insertion: extract `range.WordOpenXML` from an existing MathType equation in WPS and insert it at another range via `InsertXML` (with unique `rId`s) to determine whether WPS's own XML still hides outside equations; record the result.
- [x] 1.3 Run A/B tests on EqnTide's Flat OPC payload in `src/docx/ole_xml.rs` / `src/docx/flat_opc.rs`—specifically omitting repeated `<v:shapetype id="_x0000_t75">` and switching `ObjectID` to a Word-style 32-bit `_[0-9]{10}` identifier—and verify in WPS (both blank and existing multi-equation documents) and Microsoft Word whether either change prevents hidden equations; keep `cargo test` passing if retained.

## 2. Non-Scrolling Refresh & Workaround Testing

- [x] 2.1 Test post-insert refresh candidates on WPS Writer (Linux and Windows) across `InsertMathTypeEquation`, single `ToggleLatex`, and batch `runLatexToMathTypeBatch`: compare `app.ScreenRefresh()`, bounded `InlineShapes` property touches (e.g. `Width`/`Range` invalidation), and scroll-restoring view/layout refresh; verify both equation visibility restoration and that the viewport does not jump to page 1.
- [x] 2.2 If a reliable XML change or scroll-safe refresh workaround is confirmed, implement it cleanly in `wps-addin/main.js` (and/or `src/docx/ole_xml.rs`) with regression coverage in `tests/wps/test_wps_insertion.js`, and verify `node tests/wps/test_wps_insertion.js`, `node tests/wps/test_wps_ribbon.js`, and `cargo test` pass.
- [x] 2.3 Verify on a large multi-page benchmark document in WPS Writer that any applied workaround preserves insertion speed, `Equation.DSMT4` editability, and viewport scroll position, and update `KNOWN_ISSUES.md` with the final confirmed root cause or resolution.
