# wps-mathtype-ole-insertion Specification

## Purpose

Defines reliable selection, replacement, and insertion behavior when the WPS Writer add-in edits, toggles, or imports MathType OLE + EMF (`Equation.DSMT4`) Flat OPC packages.

## Requirements

### Requirement: MathType OLE editing and toggling preserve the full equation field range
When the WPS add-in targets an existing MathType OLE `InlineShape` (whose document range spans multiple characters for the underlying `{ EMBED Equation.* }` field) for editing or LaTeX toggling, it SHALL preserve the complete `InlineShape` range (trimming only any accidental trailing paragraph mark) rather than clamping it to 1 character, plus any adjacent `U+200C` baseline-reset anchor.

#### Scenario: Editing a 26-character MathType OLE object preserves its full span
- **WHEN** the user triggers Edit Equation on an existing MathType OLE `InlineShape` whose `Range` is `[285..311]`
- **THEN** the pinned edit range spans the full `[285..311]` object (plus any trailing `U+200C`) rather than clamping to `[285..286]`, so replacement cleanly removes the entire prior OLE field

#### Scenario: Toggling a multi-character MathType OLE object to LaTeX preserves its full span
- **WHEN** the user toggles a MathType OLE `InlineShape` whose `Range` is `[285..311]` to LaTeX text
- **THEN** the entire `[285..311]` OLE object is replaced by the `$...$` LaTeX text without leaving orphan field characters behind

### Requirement: MathType OLE + EMF replacement and insertion succeed across WPS range behaviors
When the WPS add-in receives a MathType OLE + EMF Flat OPC package (containing `Equation.` `ProgID`) for either inserting a new equation or replacing an existing equation/image selection, it SHALL safely replace the target range, clean up only verified imported trailing paragraph breaks within document bounds, and locate the inserted inline shape using document-level fallback and bounded start-offset tolerance so post-insert sizing and baseline alignment succeed.

#### Scenario: Replacing an existing MathType image in WPS when sub-range shape lookup is empty or offset
- **WHEN** the user edits an existing MathType equation picture in WPS, saves in EqnTide Desktop (`Ctrl+S`), and WPS imports the returned `Equation.DSMT4` Flat OPC XML with `range.InlineShapes` empty or the new shape starting within `+1`..`+5` characters of `insertPos`
- **THEN** the add-in locates the inserted inline shape via `doc.InlineShapes` fallback and offset tolerance, applies size and baseline positioning, places the cursor after the equation, and does not report an insertion failure

#### Scenario: Importing a MathType package at or near the end of a WPS document
- **WHEN** the target range is at the end of the document or `insertedEnd` reaches `doc.Range().End`
- **THEN** trailing paragraph-break trimming guards `insertedEnd <= newEnd` and `insertedEnd > insertPos` without throwing an out-of-bounds range error

### Requirement: Post-insert shape lookup failure does not mask successful XML insertion, and real errors report diagnostics
If XML package import succeeds (`InsertXML` completes without throwing) and the package content was inserted into the document, inability to locate the resulting `InlineShape` solely for optional sizing/baseline alignment SHALL NOT cause the add-in to report a false fatal insertion error; genuine `InsertXML` or range errors SHALL be logged and surfaced with error details.

#### Scenario: InsertXML succeeds even when WPS does not expose the newly inserted InlineShape handle immediately
- **WHEN** `InsertXML` inserts the MathType Flat OPC package without error but WPS exposes no matching `InlineShape` handle for baseline adjustment
- **THEN** the add-in logs a diagnostic warning via `wpsLog` and completes without showing a false `"Failed to insert MathType OLE + EMF object."` failure alert

#### Scenario: InsertXML throws an error on WPS
- **WHEN** `InsertXML` itself throws a runtime error while importing a MathType Flat OPC package
- **THEN** the add-in logs the underlying error message via `wpsLog` and alerts the user with the failure reason instead of hiding it
