# Proposal

## Why

Editing or inserting a MathType OLE + EMF equation in the WPS Office JS add-in (`wps-addin/main.js`) fails for every formula with `"Failed to insert MathType OLE + EMF object."` even though EqnTide Desktop and the local bridge successfully convert and package the equation into `Equation.DSMT4` Flat OPC XML. Live WPS bridge logs proved two compounding regressions introduced after switching from standalone EMF `AddPicture` (`bcfddb9`) to `Equation.DSMT4` OLE + EMF packages (`54c87a2`):
1. `EditEquation` blindly clamps any `InlineShape.Range` longer than 1 character down to 1 character (`[285..311]` → `[285..286]`), leaving 25 characters of the existing `{ EMBED Equation.DSMT4 }` OLE field unreplaced when `InsertXML` runs (`ToggleLatex` has the same `rEnd = rStart + 1` clamp).
2. `insertMathTypeAtDocRange` returns `null` at `if (!pic) return null;` whenever `range.InlineShapes` is empty or `candidate.Range.Start !== insertPos`, without falling back to `doc.InlineShapes`, without start-offset tolerance, and without separating `InsertXML` success from optional baseline formatting.

Word (`word-template/EqnTide.bas`) was verified and is unaffected because it preserves the full `InlineShape.Range.Duplicate` and calls `target.InsertXML` directly without sub-range `InlineShapes` lookup.

## What Changes

- Preserve the full `InlineShape.Range` when editing or toggling a MathType OLE/equation object in `wps-addin/main.js` (`EditEquation` and `ToggleLatex`) instead of clamping multi-character OLE field ranges (`[start..end]`) to 1 character (`[start..start+1]`), while still ensuring trailing paragraph marks (`\r`/`\n`) are not swallowed.
- Update `insertMathTypeAtDocRange` (and `InsertMathTypeEquation` / `wps-addin/taskpane.js` `insertUpgradedFlatOpcAtRange` where shared insertion behavior applies) so replacing an existing MathType `InlineShape`/OLE field or inserting a new MathType OLE + EMF package cleanly deletes/replaces `targetRange`, clamps `insertedEnd` against `newEnd` before trailing-break cleanup, and falls back from `range.InlineShapes` to `doc.InlineShapes` with bounded start-offset tolerance (matching `InsertOmmlToSelection`).
- Distinguish actual `InsertXML` failures from best-effort post-insert shape lookup / baseline formatting, and surface actionable error details via `wpsLog` and alerts instead of collapsing all failures into a generic alert.
- Expand `tests/wps/test_wps_insertion.js` to cover multi-character OLE field ranges (`[285..311]`) in `EditEquation`/`ToggleLatex`, empty `range.InlineShapes` with `doc.InlineShapes` fallback, `+1` start offsets, and `insertedEnd` at document end.

## Capabilities

### New Capabilities
- `wps-mathtype-ole-insertion`: Reliable selection, replacement, and insertion of MathType OLE + EMF (`Equation.DSMT4`) Flat OPC packages in the WPS Writer JS add-in, including full OLE field range preservation, range bounds, shape lookup fallbacks, and error reporting.

### Modified Capabilities
<!-- No existing spec requirements are changing. -->

## Impact

- **WPS Add-in**: `wps-addin/main.js` (`EditEquation`, `insertMathTypeAtDocRange`, `ToggleLatex`, `InsertMathTypeEquation`, `HandleSessionResult`) and `wps-addin/taskpane.js` (`insertUpgradedFlatOpcAtRange`)
- **Tests**: `tests/wps/test_wps_insertion.js`
- **Word Add-in / Core Rust**: No changes required (`word-template/EqnTide.bas` already keeps full `InlineShape.Range.Duplicate` and inserts cleanly; Rust `mathml_to_flat_opc` and bridge `/session/*` already succeed).
