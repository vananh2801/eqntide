# Design

## Context

See `proposal.md` for motivation and live WPS log evidence. Currently, `wps-addin/main.js` handles MathType OLE + EMF packages in `EditEquation` -> `HandleSessionResult` -> `InsertMathTypeEquation` -> `insertMathTypeAtDocRange`, while `wps-addin/taskpane.js` handles upgrades via `insertUpgradedFlatOpcAtRange` and OMML insertion in `wps-addin/main.js` uses `InsertOmmlToSelection`. Word (`word-template/EqnTide.bas`) was inspected and does not share these bugs.

Live WPS logs revealed three concrete flaws in `wps-addin/main.js`:
1. **`EditEquation` (`main.js#L761-L765`) and `ToggleLatex` (`main.js#L1433-L1435`) clamp `InlineShape.Range` to 1 char (`tEnd = tStart + 1`)**: That heuristic assumed standalone `AddPicture` images (`1` char), whereas an embedded MathType OLE object (`{ EMBED Equation.DSMT4 }`) spans ~26 chars (`[285..311]`). Clamping to `[285..286]` targets only the opening character of the OLE field.
2. **Sub-range `InlineShapes` lookup and strict start equality (`main.js#L849-L857`)**: `insertMathTypeAtDocRange` only scans `range.InlineShapes` (never falling back to `doc.InlineShapes` as `InsertFlatOpcToSelection` and `InsertOmmlToSelection` do) and demands `candidate.Range.Start === insertPos`. Live logs showed `InsertXML` did not throw, yet `pic` was `null` at line 857.
3. **Unclamped `insertedEnd` bounds and hidden errors (`main.js#L840-L857, L1749`)**: `insertedEnd` is not clamped/checked against `newEnd` before calling `doc.Range(insertedEnd, ...)`, and `if (!pic) return null;` / `catch (err)` collapse into a generic `"Failed to insert MathType OLE + EMF object."` alert without surfacing details.

## Goals / Non-Goals

**Goals:**
- Preserve the full `InlineShape.Range` (excluding only accidental trailing `\r`/`\n`, plus adjacent `U+200C`) in `EditEquation` and `ToggleLatex` when targeting multi-character OLE equations.
- Make `insertMathTypeAtDocRange` resilient to WPS `InlineShape` replacement, range-bound edge cases, missing sub-range `InlineShapes`, and `+1`..`+5` character start offsets.
- Align `wps-addin/taskpane.js` (`insertUpgradedFlatOpcAtRange`) with the same `doc.InlineShapes` fallback and offset tolerance where applicable.
- Log and surface concrete error reasons when `InsertXML` fails, while keeping optional post-insert baseline/size formatting best-effort (mirroring `InsertOmmlToSelection` and `taskpane.js`).
- Verify all edge cases in `tests/wps/test_wps_insertion.js`.

**Non-Goals:**
- Changing Rust `mathml_to_flat_opc`, MTEF encoding, or `Equation.DSMT4` OLE/EMF binary structure.
- Modifying `word-template/EqnTide.bas` (Word already preserves `Selection.InlineShapes(1).Range.Duplicate` and completes `InsertFlatOpcInline` without sub-range `InlineShapes` lookup).

## Decisions

1. **Preserve `targetRange` span in `EditEquation` and `ToggleLatex` (trim only trailing `\r`/`\n`)**
   - *Choice*: Remove the `if (isEquationFound && (tEnd - tStart > 1)) { tEnd = tStart + 1; }` clamp in `EditEquation` (and `rEnd = rStart + 1` in `ToggleLatex`) when `targetRange` came from `InlineShapes` / `findEquationTarget`. Instead, trim only trailing `\r`/`\n` (matching `taskpane.js` and `EqnTide.bas`) so an entire 26-char OLE field (`[285..311]`) is replaced while never swallowing a paragraph break.
   - *Rationale*: In `EditEquation`, `findEquationTarget` already sets `targetRange = shape.Range` when an `InlineShape` is found; clamping that range to 1 char corrupts OLE field replacement.
   - *Alternative considered*: Clamping only when `tEnd - tStart > 1 && !hasOleField` — fragile because WPS exposes OLE objects via `InlineShape.Range` of ~26 chars without a reliable JS property distinguishing them across all versions, whereas trimming trailing `\r`/`\n` handles both 1-char EMF pictures and 26-char OLE fields safely.

2. **Delete/clean old `InlineShape` range safely before `InsertXML` when replacing**
   - *Choice*: Trim trailing `\r`/`\n` from `targetRange`. If `targetRange.End > targetRange.Start`, clean up the replaced range before/around `InsertXML` (deleting any `InlineShapes` inside `targetRange` / `targetRange.Delete()` and re-anchoring at `doc.Range(insertPos, insertPos)` with `replaced = 0`, or falling back cleanly if `InsertXML` on a non-empty range fails) so `InsertXML` inserts cleanly at `insertPos` without leaving fragments of the prior OLE field.
   - *Rationale*: Replacing an active 26-char OLE field in WPS can shift offsets or retain orphaned field state if the prior range/shape isn't cleared first.

3. **Guard `insertedEnd` bounds and trailing-break cleanup identically to `InsertOmmlToSelection`**
   - *Choice*: Compute `insertedEnd = insertPos + newEnd - originalEnd + replaced`, verify `insertedEnd > insertPos && insertedEnd <= newEnd && typeof suffix === "string"` before reading `doc.Range(insertedEnd, Math.min(insertedEnd + suffix.length, newEnd)).Text`, guard `insertedEnd > insertPos` inside the break-trimming loop, and clamp `insertedEnd = Math.min(insertedEnd, newEnd)` before constructing `doc.Range(insertPos, insertedEnd)`.
   - *Rationale*: Prevents out-of-range exceptions at document end or when WPS normalizes characters during `InsertXML`.

4. **Fallback from `range.InlineShapes` to `doc.InlineShapes` with bounded start-offset matching**
   - *Choice*: Check `range.InlineShapes` first for a shape whose `Range.Start` is within `[insertPos, insertPos + 5]` (preferring smallest `Math.abs(Start - insertPos)`), and if none is found, scan `doc.InlineShapes` with the same criterion.
   - *Rationale*: Combines `InsertOmmlToSelection`'s `range` -> `doc` fallback and the pre-`54c87a2` `InsertFlatOpcToSelection` nearest-shape scan (`diff <= 5`).

5. **Separate `InsertXML` failure from optional shape-lookup/formatting failure**
   - *Choice*: Only fail `InsertMathTypeEquation` (and return `false` / show error alert with `err` message) when `packageXml` is invalid, `targetRange` is missing, or `InsertXML` itself throws. If `InsertXML` succeeds without finding `pic`, log a diagnostic warning via `wpsLog` and treat post-insert sizing/baseline positioning as best-effort so the user is not falsely alerted after the equation was already inserted.

## Risks / Trade-offs

- **[Risk] If `targetRange` in `EditEquation` came from a fallback larger than the equation (e.g., `Words.Item(1)` containing adjacent text)** → Mitigation: Ensure `findEquationTarget` only falls back to `word.InlineShapes.Item(1).Range` (never `word.Range`), which it already does, and trim trailing `\r`/`\n` before pinning `g_editRangeStart..g_editRangeEnd`.
- **[Risk] Deleting `targetRange` before `InsertXML` invalidates `targetRange` reference in WPS** → Mitigation: Re-create `targetRange = doc.Range(insertPos, insertPos)` after deletion and reset `replaced = 0`.
