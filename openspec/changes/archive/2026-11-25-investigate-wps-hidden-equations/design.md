# Design

## Context

See `proposal.md` for motivation. Across both WPS Writer for Linux and WPS Writer for Windows, every code path that calls `Range.InsertXML(packageXml)` with a `FlatOpcMode::ModernMathtypeOleEmf` package (`InsertMathTypeEquation`, single `ToggleLatex`, and batch `runLatexToMathTypeBatch`) can cause already-loaded MathType equations outside the target range to stop displaying until the document is saved, closed, and reopened.

Key codebase observations:
- `src/docx/flat_opc.rs` already assigns unique UUIDv4 identifiers to `rIdOle<uid>`, `rIdImage<uid>`, `shape_id`, `object_id`, `oleObject<uid>.bin`, and `equation<uid>.emf`.
- `src/docx/ole_xml.rs` embeds `<v:shapetype id="_x0000_t75" .../>` inside every `<w:object>` (which existing equations also reference via `type="#_x0000_t75"`) and uses a 128-bit decimal `ObjectID` (`_{u128}`).
- `wps-addin/main.js` currently calls `app.ScreenRefresh()` via `refreshToggleScreen(doc)` only for `ToggleLatex` (not for `InsertMathTypeEquation`), having removed `doc.Repaginate()` and `View.Zoom.Percentage` toggling in `245062a` because they jumped the viewport to page 1. Earlier commit `0dabcc9` also touched existing `InlineShapes` (`shp.Width = shp.Width`) to force layout invalidation.
- `Word` VBA (`word-template/EqnTide.bas`) calls `target.InsertXML flatOpc` with the same Flat OPC payload without hiding existing equations.

## Goals / Non-Goals

**Goals:**
- Isolate in live WPS Writer (Linux and Windows) whether the disappearing preview is caused by a specific XML construct in EqnTide's Flat OPC payload or is intrinsic to WPS's `InsertXML` OLE/VML cache.
- Test and compare lightweight, scroll-preserving host refresh/invalidation workarounds across all three entry points (`InsertMathTypeEquation`, single `ToggleLatex`, batch `ToggleLatex`).
- Keep MathType OLE + EMF (`Equation.DSMT4`) fidelity intact and keep existing unit tests (`node tests/wps/test_wps_insertion.js`) passing.

**Non-Goals:**
- Do not degrade MathType equations back to bare `InlineShapes.AddPicture` images without OLE (`Equation.DSMT4`).
- Do not reintroduce viewport-jumping `Repaginate()` or `Zoom.Percentage` toggles unless scroll position can be reliably preserved without flicker.
- Do not change Word VBA (`EqnTide.bas`) behavior.

## Decisions

### 1. Two-phase isolation matrix before committing to a fix
Run two targeted experiments directly on WPS Writer (Windows and Linux) using a multi-page `.docx` containing existing MathType equations above and below the edit site:

1. **XML A/B test (`InsertXML` payload isolation):**
   - **Control A (WPS self-export):** Extract `range.WordOpenXML` from an existing MathType equation in WPS, paste/insert it back into another range via `InsertXML`, and check if outside equations still disappear.
   - **Variant B (Omit duplicate `<v:shapetype id="_x0000_t75">`):** Test `make_object_run` without repeating `<v:shapetype id="_x0000_t75">` (or with a per-snippet / omitted shapetype when `_x0000_t75` already exists) to verify whether redefining `_x0000_t75` invalidates existing VML shape renderers.
   - **Variant C (32-bit `ObjectID`):** Test standard Word-style 32-bit `ObjectID` (`_[0-9]{10}`) vs. 128-bit `_{u128}`.
   - **Alternative considered:** Immediately changing `ole_xml.rs` blindly without A/B testing. Rejected because `ObjectID` or `shapetype` omission could break Word or MathType compatibility if it isn't actually the WPS trigger.

2. **Post-insert refresh / invalidation test (if WPS hides equations regardless of XML):**
   - Test `app.ScreenRefresh()` on `InsertMathTypeEquation` (currently missing it).
   - Test bounded/visible `InlineShapes` property touch (e.g., `shp.Width = shp.Width` or `shp.Range` access, or `ActiveWindow.ScrollIntoView` around a layout pass) and measure both visual restoration and scroll/performance impact on large documents (100+ and 4,000+ equations).
   - **Alternative considered:** Re-enabling `doc.Repaginate()` + `Zoom.Percentage` unconditionally. Rejected because `245062a` removed it specifically due to viewport jumping to page 1 and multi-second stalls on large documents.

### 2. Unify post-insert refresh behavior across Edit and Toggle paths
Whatever minimal refresh/XML fix passes live WPS verification on both Linux and Windows will be shared by `insertMathTypeAtDocRange` (covering `InsertMathTypeEquation`, single `ToggleLatex`, and batch `ToggleLatex`) so `EditEquation` is not left behind `ToggleLatex`.

## Risks / Trade-offs

- [WPS `InsertXML` drops preview cache internally regardless of XML shape] -> Verify via Control A (WPS self-exported `WordOpenXML`); if even WPS's own XML triggers it and no scroll-safe repaint restores it, document the exact confirmed behavior across Win/Linux and Edit/Toggle in `KNOWN_ISSUES.md` and avoid risky/slow COM hacks.
- [Touching all `doc.InlineShapes` in a 4,000-equation document could reintroduce O(N) UI stalls] -> Any shape-touch workaround must be benchmarked on large documents and bounded/skipped or applied only when outside shapes exist, without regressing the `245062a` insertion benchmark.
- [Omitting `<v:shapetype id="_x0000_t75">` could break rendering when inserting into a brand-new blank document that has no prior `_x0000_t75`] -> Only omit or deduplicate if confirmed safe on both blank documents and existing documents in both Word and WPS.
