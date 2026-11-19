# desktop-batch-conversion Specification

## Purpose

Enables the desktop application's Batch DOCX tab to recursively discover and convert `.docx` files across folders and subfolders while preserving the original directory hierarchy.

## Requirements

### Requirement: Folder selection and recursive DOCX discovery
The desktop **Batch DOCX** tab SHALL accept folders via the **Add folder…** picker, recursively queuing all `.docx` files inside while skipping Word temporary lock files (`~$*`) and hidden entries (`.*`), and SHALL display the queued list frame only when at least one file is queued.

#### Scenario: Picking or dropping a folder queues nested DOCX files and skips ignored entries
- **WHEN** the user picks a folder containing root `.docx` files, nested subfolders with `.docx` files, and a `~$temp.docx` lock file
- **THEN** all non-hidden `.docx` files across the folder and its subfolders are queued in deterministic order while `~$temp.docx` and hidden entries are excluded

#### Scenario: Adding individual files continues to work alongside folders
- **WHEN** the user adds individual `.docx` files via **Add DOCX files…**
- **THEN** those individual files are queued without duplicates alongside any folder-derived entries

### Requirement: Sibling timestamped folder output preserving hierarchy
When overwrite is disabled, files queued from a folder SHALL be written into a sibling directory named `<folder>_<target>_<time>/` preserving each file's relative path inside the source folder, while standalone files continue saving beside each input.

#### Scenario: Folder batch conversion mirrors subfolder structure into sibling timestamped directory
- **WHEN** the user converts a queued folder `docs` with target `omml` and overwrite disabled
- **THEN** the converted documents are written under a sibling directory named `docs_omml_<time>/` with each file at its original relative path and missing subdirectories created automatically

#### Scenario: Overwrite mode updates folder files in place
- **WHEN** the user enables overwrite and converts a queued folder
- **THEN** each converted `.docx` file replaces its original source file in place rather than creating `<folder>_<target>_<time>/`

### Requirement: Structured batch toolbar and consistent control states
The desktop **Batch DOCX** tab SHALL organize its top controls into an **Input** group (`Add DOCX files…`, `Add folder…`, `Clear list`) and an **Options & Actions** group (target selector, overwrite toggle, `Convert`, `Stop after current file`), without a global `Open output folder` toolbar button, and SHALL disable unavailable actions with consistent visual opacity.

#### Scenario: Empty queue shows consistent disabled states
- **WHEN** the **Batch DOCX** tab has no queued `.docx` files
- **THEN** `Clear list`, `Convert`, and `Stop after current file` are disabled with consistent disabled styling, while `Add DOCX files…` and `Add folder…` remain enabled

#### Scenario: Multiple additive additions across different locations
- **WHEN** the user invokes `Add DOCX files…` or `Add folder…` multiple times across different directories or drives
- **THEN** newly discovered `.docx` files are appended without duplicates alongside existing entries while retaining each folder item's own `source_root` for sibling output generation

#### Scenario: Active run enables stop and disables editing controls
- **WHEN** a batch conversion is running
- **THEN** `Stop after current file` is enabled and `Add DOCX files…`, `Add folder…`, `Clear list`, target selection, overwrite toggle, and `Convert` are disabled until the run finishes

### Requirement: Dual-mode non-destructive file selection
The desktop **Batch DOCX** tab SHALL let users switch between **Folder tree** (default) and **File tree** selection modes, maintaining independent checkbox selection states for each mode where newly added files default to checked in both modes and individual rows are excluded by unchecking rather than deleted.

#### Scenario: Default state after adding files or folders
- **WHEN** the user adds files or folders to the **Batch DOCX** queue
- **THEN** the active mode defaults to **Folder tree** and all added files are checked in both **Folder tree** and **File tree** selection states

#### Scenario: Switching selection modes preserves per-mode customizations
- **WHEN** the user toggles checkboxes in **Folder tree**, switches to **File tree**, unchecks files, and switches back to **Folder tree**
- **THEN** **Folder tree** retains its earlier checked/unchecked state and **File tree** retains its own checked/unchecked state

#### Scenario: Folder tree groups subfolders with tri-state parent checkboxes
- **WHEN** the user views queued folder items in **Folder tree** mode
- **THEN** items are organized under their root folder and nested subfolders with expand/collapse toggles and tri-state parent checkboxes (`checked`, `indeterminate`, `unchecked`) that toggle all descendant files, while standalone files appear in a dedicated individual-files group

#### Scenario: Only Clear list removes queued files
- **WHEN** the user unchecks items in either mode and later clicks **Clear list**
- **THEN** unchecking keeps all rows visible in the queue for re-selection, whereas **Clear list** removes all queued items, logs, and recorded outputs

### Requirement: Selected-only conversion execution
Clicking **Convert** SHALL convert only the `.docx` files checked in the currently active selection mode, tracking progress over that selected subset and updating only those rows' status and output actions.

#### Scenario: Converting a subset of checked files
- **WHEN** the user unchecks several files in the active mode and clicks **Convert**
- **THEN** only the checked files in the active mode are converted, the progress bar advances against the checked file count, unselected files are not modified or converted, and **Convert** is disabled whenever zero files are checked in the active mode

### Requirement: Per-row output and root folder actions
After conversion succeeds, **Batch DOCX** SHALL display context-appropriate **`Open folder`** buttons without a global toolbar button: **File tree** rows open each file's immediate output directory, while **Folder tree** places **`Open folder`** on root and subfolder rows (for their corresponding output directory) and on standalone file rows, omitting redundant buttons on folder-derived file rows and the individual-files header.

#### Scenario: File tree shows one Open folder button per converted row
- **WHEN** `docs/chapter1/lesson1.docx` and `other/note.docx` finish converting while **File tree** mode is active
- **THEN** each row displays a single **`Open folder`** action targeting `docs_omml_<time>/chapter1/` and `other/` respectively

#### Scenario: Folder tree places Open folder on folder rows and standalone files
- **WHEN** `docs/intro.docx`, `docs/chapter1/lesson1.docx`, and standalone `other/note.docx` finish converting and the user views **Folder tree** mode
- **THEN** the `docs` root row shows **`Open folder`** targeting `docs_omml_<time>/`, the `chapter1` subfolder row shows **`Open folder`** targeting `docs_omml_<time>/chapter1/`, `intro.docx` and `lesson1.docx` show no button, and standalone `note.docx` shows **`Open folder`** targeting `other/`
