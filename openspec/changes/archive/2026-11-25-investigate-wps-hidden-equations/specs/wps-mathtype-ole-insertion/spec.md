## ADDED Requirements

### Requirement: MathType OLE insertion and toggling preserve visibility of existing equations without resetting viewport scroll
When the WPS add-in inserts, edits, or toggles a MathType OLE + EMF (`Equation.DSMT4`) equation via `Range.InsertXML` in WPS Writer (Linux or Windows), existing MathType equations outside the modified range SHALL remain visibly rendered in the document without requiring a save-close-reopen cycle, and any post-insert screen refresh SHALL NOT reset the user's viewport scroll position or zoom level.

#### Scenario: Editing or single-toggling one equation keeps other equations visible
- **WHEN** a WPS Writer document contains multiple MathType OLE equations and the user edits one equation (`EditEquation`) or toggles one LaTeX expression (`ToggleLatex`) into MathType
- **THEN** the modified equation is inserted as a valid MathType OLE + EMF object and all existing MathType equations outside that range remain visible in the viewport without saving and reopening

#### Scenario: Partial batch Toggle TeX keeps outside equations visible and preserves scroll position
- **WHEN** the user selects a partial range of LaTeX formulas in a document that also contains existing MathType equations outside the selection and runs `ToggleLatex`
- **THEN** all selected formulas convert to MathType OLE + EMF, existing MathType equations outside the selection remain visibly rendered, and the viewport remains at its scroll position rather than jumping to page 1
