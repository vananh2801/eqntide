# Spec Delta

## Purpose

Enables the desktop application's Batch DOCX tab to recursively discover and convert `.docx` files across folders and subfolders while preserving the original directory hierarchy.

## ADDED Requirements

### Requirement: Folder selection and recursive DOCX discovery
The desktop **Batch DOCX** tab SHALL accept folders via both an **Add folder…** picker and drag-and-drop, recursively queuing all `.docx` files inside while skipping Word temporary lock files (`~$*`) and hidden entries (`.*`).

#### Scenario: Picking or dropping a folder queues nested DOCX files and skips ignored entries
- **WHEN** the user picks or drops a folder containing root `.docx` files, nested subfolders with `.docx` files, and a `~$temp.docx` lock file
- **THEN** all non-hidden `.docx` files across the folder and its subfolders are queued in deterministic order while `~$temp.docx` and hidden entries are excluded

#### Scenario: Adding individual files continues to work alongside folders
- **WHEN** the user adds individual `.docx` files via **Add DOCX files…** or drag-and-drop
- **THEN** those individual files are queued without duplicates alongside any folder-derived entries

### Requirement: Sibling timestamped folder output preserving hierarchy
When overwrite is disabled, files queued from a folder SHALL be written into a sibling directory named `<folder>_<target>_<time>/` preserving each file's relative path inside the source folder, while standalone files continue saving beside each input.

#### Scenario: Folder batch conversion mirrors subfolder structure into sibling timestamped directory
- **WHEN** the user converts a queued folder `docs` with target `omml` and overwrite disabled
- **THEN** the converted documents are written under a sibling directory named `docs_omml_<time>/` with each file at its original relative path and missing subdirectories created automatically

#### Scenario: Overwrite mode updates folder files in place
- **WHEN** the user enables overwrite and converts a queued folder
- **THEN** each converted `.docx` file replaces its original source file in place rather than creating `<folder>_<target>_<time>/`

### Requirement: Open output folder targets generated directory
The **Open output folder** action in **Batch DOCX** SHALL open the generated `<folder>_<target>_<time>/` output directory when a folder batch produced one, falling back to the first queued item's folder otherwise.

#### Scenario: Opening output folder after a non-overwrite folder batch
- **WHEN** a non-overwrite folder batch conversion creates `<folder>_<target>_<time>/` and the user clicks **Open output folder**
- **THEN** the file manager opens `<folder>_<target>_<time>/`
