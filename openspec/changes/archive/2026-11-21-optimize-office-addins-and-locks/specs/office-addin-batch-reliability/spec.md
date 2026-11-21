# Spec Delta

## Purpose

Defines reliable, responsive batch LaTeX toggling, Flat OPC payload compaction, selection and font-size validation, macOS bridge discovery, and Windows file-lock-safe disk upgrades across the Microsoft Word VBA template and WPS Office JavaScript add-in.

## ADDED Requirements

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
