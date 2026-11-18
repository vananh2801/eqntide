# Proposal

## Why

The CLI already supports recursive directory batch conversion that preserves subfolder hierarchies, but the desktop **Batch DOCX** tab only accepts individual `.docx` files and ignores dropped folders. Meanwhile, in the **Settings** tab, theme switching still uses a single `"Switch to Light/Dark mode"` toggle button rather than matching the selectable option-button pattern used by the controls below it.

## What Changes

- Add folder support to the desktop **Batch DOCX** tab via an **Add folder…** picker button and drag-and-drop folder handling.
- Recursively discover `.docx` files inside added folders (skipping Word lock files `~$*` and hidden entries `.*`) and convert them while preserving relative subfolder hierarchy inside a sibling timestamped folder `<folder>_<target>_<time>/` when overwrite is disabled.
- Update **Open output folder** in **Batch DOCX** to open the generated folder-batch output directory when applicable.
- Replace the theme toggle button in **Settings** (`Appearance`) with two selectable **Light** and **Dark** option buttons styled consistently with the **Startup focus** and canvas preset selectors below.

## Capabilities

### New Capabilities

- `desktop-batch-conversion`: Recursive folder and multi-file `.docx` batch conversion in the desktop application, preserving subfolder hierarchy in `<folder>_<target>_<time>/` sibling output directories when not overwriting in place.

### Modified Capabilities

- `desktop-ui-theme`: Require the Settings **Appearance** theme selector to use a `Light` / `Dark` selectable button group consistent with other option selectors in Settings.

## Impact

- Affected code: `src/docx/` & `src/cli/paths.rs` (sharing recursive `.docx` discovery), `desktop/src/batch.rs`, `desktop/src/app.rs` (`add_files`, `batch_ui`, `settings_ui`, and tests), and `desktop/Cargo.toml` (if `time` crate is declared directly from the existing workspace lockfile).
- Existing Settings tests checking `"Switch to Light mode"` will be updated to target the `"Light"` / `"Dark"` selector buttons.
