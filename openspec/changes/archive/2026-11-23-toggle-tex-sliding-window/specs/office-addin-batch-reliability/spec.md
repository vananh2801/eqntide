## MODIFIED Requirements

### Requirement: Bounded-concurrency batch conversion and responsive DOM replacement
The Word VBA template and WPS Office add-in SHALL bound concurrent `/convert/snippet` requests (`<= 8` in WPS), release request XML payloads promptly, apply batch DOM replacements in chunks with UI yields, and report aggregate batch counts while isolating HTTP `422` unsupported formulas.

#### Scenario: Large multi-equation batch in WPS caps HTTP concurrency and yields DOM updates
- **WHEN** a user runs batch `Toggle LaTeX` or in-memory taskpane upgrade over a selection or document containing dozens of formulas or equations in WPS
- **THEN** at most `8` `/convert/snippet` requests are in flight concurrently, request XML buffers are released upon dispatch, DOM replacements execute in reverse-offset chunks with event-loop yields (`setTimeout`), and all valid targets are converted without triggering bridge 503 overload

#### Scenario: Batch conversion reports summary counts and isolates unsupported formulas
- **WHEN** a batch conversion encounters formulas rejected by the bridge with HTTP `422` alongside valid formulas
- **THEN** valid formulas still convert, the user receives a summary with `Found`, `Converted`, and `Skipped/Errors` counts, and a fatal non-422 failure when zero items convert surfaces an explicit error alert

#### Scenario: WPS starts replacing before all conversions finish
- **WHEN** a ribbon Toggle TeX batch in WPS contains more than eight formulas and bridge responses arrive in either conversion direction
- **THEN** the add-in keeps at most eight items in flight or awaiting replacement combined, and submits further items as capacity becomes available
- **AND** the add-in begins replacing as soon as the next item in reverse document order is ready, even while later items are still converting
- **AND** replacements run serially, with UI yields between groups, and the add-in releases each item's payload and result as soon as they are no longer needed

#### Scenario: Out-of-order results and unsupported formulas preserve document content
- **WHEN** a later result arrives before an earlier item in replacement order, or one formula fails conversion
- **THEN** the add-in applies each successful result at most once to its verified target in reverse document order, leaves failed conversions untouched, and proceeds with other valid targets
- **AND** the summary counts each eligible target once
