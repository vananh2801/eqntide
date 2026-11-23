# office-addin-batch-reliability Specification

## Purpose

Defines reliable, responsive batch LaTeX toggling, Flat OPC payload compaction, selection and font-size validation, macOS bridge discovery, and Windows file-lock-safe disk upgrades across the Microsoft Word VBA template and WPS Office JavaScript add-in.

## Requirements

### Requirement: Flat OPC compaction and fast shape classification
The Word VBA template and WPS Office add-in SHALL strip non-equation Flat OPC parts before sending equation XML to `/convert/snippet`, and SHALL skip reading XML for shapes whose host metadata identifies them as non-equation objects.

#### Scenario: Bloated Flat OPC parts are stripped before sending snippet requests
- **WHEN** Word or WPS extracts a large (`>= 16 KB`) Flat OPC package containing `/word/styles.xml`, `/word/numbering.xml`, or theme/property parts for equation conversion
- **THEN** those non-equation parts are stripped while `/word/document.xml`, relationships, OLE embeddings, and equation media remain intact

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

### Requirement: Safe full-document selection priority, delimiter bounds, and font-size clamping
When a user selects a range (including `Ctrl+A` across multi-page documents), the Word and WPS add-ins SHALL prioritize delimited LaTeX formulas and existing document equations over bare-text LaTeX interpretation, SHALL reject implausible or cross-paragraph delimiter matches, and SHALL sanitize mixed-formatting `font_pt` values (`wdUndefined` / out-of-range) before calling `/convert/snippet`.

#### Scenario: Full-document selection with prose and equations converts equations instead of collapsing prose
- **WHEN** a user presses `Ctrl+A` on a multi-paragraph document containing Vietnamese or English prose with `=` or `-` characters alongside OMML/MathType equations and invokes `Toggle LaTeX`
- **THEN** the add-in converts the equations in the selection rather than treating the entire document text as a single bare LaTeX formula

#### Scenario: Mixed-formatting selection clamps wdUndefined font size
- **WHEN** a selected range in WPS spans mixed font sizes such that `Font.Size` reports `9999999` (`wdUndefined`) or another non-finite/out-of-range value
- **THEN** the add-in clamps or falls back `font_pt` to a valid value in `1.0..200.0` (defaulting to `12.0`) so `/convert/snippet` does not reject the batch with HTTP `400`

#### Scenario: Stray delimiters across blank lines or oversized spans are ignored
- **WHEN** a document contains an unclosed `$$` or oversized span (`> 2000` characters) crossing blank lines, embedded object markers, or table cell markers
- **THEN** batch LaTeX matching rejects the implausible span instead of swallowing multiple paragraphs

### Requirement: Disk-based document upgrade releases Windows file locks safely before bridge mutation
When running a disk-based `/convert/document` upgrade on an active saved `.docx` file, the Word VBA macro and WPS add-in SHALL reject read-only documents, save pending edits, release host file locks on Windows before invoking `/convert/document`, and reopen the upgraded file afterward (including on conversion error).

#### Scenario: Windows WPS disk upgrade preserves add-in lifecycle with a temporary blank document
- **WHEN** a user triggers disk upgrade from the WPS Ribbon or taskpane on Windows while only the target `.docx` is open
- **THEN** WPS saves the document, opens a temporary blank document so WPS Writer and the taskpane remain alive, closes the target document to release its OS file lock, invokes `/convert/document`, reopens the target document, and closes the temporary blank document regardless of HTTP 200 or error status

#### Scenario: Word macro closes the active document before invoking bridge disk upgrade
- **WHEN** a user invokes `UpgradeMathTypeMacro` on a saved writable document in Microsoft Word
- **THEN** Word saves the document, closes it before posting `/convert/document`, and reopens the document afterward in both success and `UpgradeError` paths

#### Scenario: Linux WPS disk upgrade retains hot-refresh workflow
- **WHEN** a user triggers disk upgrade in WPS Writer on Linux
- **THEN** WPS saves the document, runs `/convert/document` while open, notifies the user, and closes/reopens the document to refresh displayed equations

### Requirement: macOS bridge token discovery and cold-start parity in WPS add-in
The WPS JavaScript add-in SHALL discover the per-user bridge authentication token from the macOS Application Support path in addition to Linux and Windows paths, and SHALL support launching the desktop editor via `/usr/bin/open` on macOS.

#### Scenario: WPS add-in on macOS locates the bridge token and launches the desktop URI
- **WHEN** the WPS add-in runs on macOS and reads the bridge token or cold-starts `eqntide://`
- **THEN** it checks `~/Library/Application Support/com.eqntide.EqnTide/bridge.token` and falls back to `/usr/bin/open` when `ShellExecute` targets a custom URI

### Requirement: WPS batch MathType insertion performance
WPS Writer for Linux SHALL reduce the median insertion time for the reference LaTeX-to-MathType batch by at least 50% against the current add-in. Insertion time SHALL be measured from immediately before the first document replacement until the final replacement and formatting finish.

#### Scenario: Compare insertion times on the reference document
- **GIVEN** the reference selection contains 4,336 LaTeX formulas and the current add-in converts 4,334 and skips 2
- **WHEN** each build runs at least three times on the same machine and WPS version, starting from an unmodified copy of the same document each time
- **THEN** the optimized build's median insertion time is at most half the baseline median, and the optimized build converts the same 4,334 formulas and skips the same 2
- **AND** bridge conversion and document insertion timings are reported separately

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

### Requirement: WPS batch counts reflect confirmed equation targets
WPS batch equation-to-LaTeX toggling SHALL exclude ordinary images and drawings from conversion requests and summary counts. A confirmed equation that fails conversion SHALL count as skipped or failed.

#### Scenario: Convert a document containing MathType and ordinary illustrations
- **WHEN** the user selects the reference DOCX containing 4,338 MathType OLE objects and ordinary illustrations and toggles to LaTeX
- **THEN** WPS reports 4,338 found, 4,338 converted, and zero skipped or failed
- **AND** ordinary illustrations remain in the document

#### Scenario: Ambiguous shape contains supported equation data
- **WHEN** a shape resembles a picture in WPS metadata but its associated content contains a supported equation payload
- **THEN** the add-in includes it in the equation batch

#### Scenario: A confirmed equation cannot be converted
- **WHEN** conversion fails for a confirmed equation
- **THEN** the equation remains in the document and is reported as skipped or failed

### Requirement: WPS Toggle LaTeX requires an equation selection

WPS Toggle LaTeX SHALL convert an equation to LaTeX only when WPS exposes the equation as selected or the highlighted range contains it. A caret immediately before or after an object SHALL leave the object unselected. Toggle LaTeX SHALL continue to support converting delimited LaTeX at the caret into an equation.

#### Scenario: The caret is next to an equation or illustration
- **WHEN** the user places the caret immediately before or after an equation or illustration without selecting the object and invokes Toggle LaTeX
- **THEN** the add-in sends no equation-to-LaTeX conversion request for the adjacent object
- **AND** the adjacent object remains in the document

#### Scenario: The user selects an equation
- **WHEN** the user clicks an equation that WPS exposes as selected or highlights a range containing the equation and invokes Toggle LaTeX
- **THEN** the add-in attempts to convert the selected equation to LaTeX

#### Scenario: The caret is inside delimited LaTeX
- **WHEN** the caret is inside a supported delimited LaTeX expression and the user invokes Toggle LaTeX
- **THEN** the add-in attempts to convert that expression into an equation

### Requirement: Word Toggle LaTeX preserves escaped dollars

Word Toggle LaTeX SHALL treat a dollar preceded by an odd number of consecutive backslashes as literal content when detecting formula delimiters. An even number of consecutive backslashes SHALL leave the dollar eligible to act as a delimiter.

#### Scenario: Literal dollar before a formula

- **WHEN** the user selects prose containing `Giá \$5, công thức $x+1$` and invokes Toggle LaTeX
- **THEN** Word converts only the actual formula and preserves the preceding prose

#### Scenario: Literal dollar inside a formula

- **WHEN** the user selects or places the caret inside a delimited formula containing `\$` and invokes Toggle LaTeX
- **THEN** Word recognizes the complete formula and passes the escaped dollar through to conversion

#### Scenario: Consecutive formulas and escaped dollars

- **WHEN** a selection contains multiple valid formulas separated by text with escaped dollars
- **THEN** Word converts each valid formula once and preserves the text between them
