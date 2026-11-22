## MODIFIED Requirements

### Requirement: WPS batch Toggle TeX progress

The WPS add-in SHALL show the processed and total equation counts at the left edge of a temporary pane directly above the Writer status bar during long batch toggles in both directions. The Toggle TeX button label SHALL remain fixed. Batch replacement SHALL retain bounded conversion concurrency and event-loop yields.

#### Scenario: Large batch reports progress during insertion
- **WHEN** WPS Writer inserts MathType objects for a selection with hundreds or thousands of formulas
- **THEN** the lower-left progress pane shows the insertion phase and updates the processed count against the total during insertion
- **AND** at most eight snippet conversion requests are in flight and document replacements yield to the WPS event loop between groups
- **AND** the final result reports the counts found, converted, and skipped or failed

#### Scenario: Large batch reports progress in both directions
- **WHEN** a batch converts LaTeX to MathType or MathType to LaTeX
- **THEN** the lower-left progress pane shows progress during bridge conversion and document replacement with counts for the current phase
- **AND** the Toggle TeX button label stays fixed throughout the operation

#### Scenario: Status clears after completion or failure
- **WHEN** a batch finishes or exits early because of an error
- **THEN** the add-in clears its progress message and hides the progress pane
