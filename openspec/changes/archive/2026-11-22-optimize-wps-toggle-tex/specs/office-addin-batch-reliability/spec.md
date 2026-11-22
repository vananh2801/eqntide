# Office Add-in Batch Reliability Delta

## ADDED Requirements

### Requirement: WPS batch MathType insertion performance
WPS Writer for Linux SHALL reduce the median insertion time for the reference LaTeX-to-MathType batch by at least 50% against the current add-in. Insertion time SHALL be measured from immediately before the first document replacement until the final replacement and formatting finish.

#### Scenario: Compare insertion times on the reference document
- **GIVEN** the reference selection contains 4,336 LaTeX formulas and the current add-in converts 4,334 and skips 2
- **WHEN** each build runs at least three times on the same machine and WPS version, starting from an unmodified copy of the same document each time
- **THEN** the optimized build's median insertion time is at most half the baseline median, and the optimized build converts the same 4,334 formulas and skips the same 2
- **AND** bridge conversion and document insertion timings are reported separately

### Requirement: WPS batch Toggle TeX progress
The WPS add-in SHALL show progress during long LaTeX-to-MathType batch toggles and retain the bounded conversion concurrency and event-loop yields required for batch replacement.

#### Scenario: Large batch reports progress during insertion
- **WHEN** WPS Writer inserts MathType objects for a selection with hundreds or thousands of formulas
- **THEN** the add-in shows the insertion phase and updates the processed count against the total during insertion
- **AND** at most eight snippet conversion requests are in flight and document replacements yield to the WPS event loop between groups
- **AND** the final result reports the counts found, converted, and skipped or failed

### Requirement: WPS batch MathType replacement fidelity
WPS batch LaTeX-to-MathType toggling SHALL preserve text, formatting, and document structure outside the verified formula ranges. Each converted formula SHALL remain an editable MathType OLE object with an EMF preview, size, and baseline appropriate to the original text.

#### Scenario: Replace formulas near paragraph, table, and object boundaries
- **WHEN** a selection contains adjacent formulas, mixed font sizes, or formulas next to a paragraph end, table cell boundary, bookmark, or existing OLE object
- **THEN** each successful replacement affects only its verified formula range
- **AND** surrounding content, formatting, paragraph breaks, cell boundaries, bookmarks, and existing objects are preserved

#### Scenario: Skip a formula without losing its source
- **WHEN** a formula is unsupported or an insertion fails
- **THEN** its original LaTeX remains in the document and the formula is counted as skipped or failed
- **AND** other verified formulas remain eligible for conversion
