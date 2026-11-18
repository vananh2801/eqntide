# Tasks

## 1. Shared Recursive DOCX Discovery

- [x] 1.1 Expose a shared recursive `.docx` discovery helper in `eqntide::docx` (`src/docx/`) that skips `~$*` and `.*` entries and sorts paths deterministically, update `src/cli/paths.rs` to call it, and verify CLI directory tests pass via `cargo test --lib cli::tests`.

## 2. Desktop Folder Batch Conversion

- [x] 2.1 Add `BatchItem` (with optional `source_root`) and `<folder>_<target>_<time>/` output path resolution (`YYYYMMDD_HHMMSS`, declaring `time` in `desktop/Cargo.toml` if needed) in `desktop/src/batch.rs`, preserving subfolder hierarchy when `overwrite` is `false` and replacing in place when `overwrite` is `true`; add unit tests in `desktop/src/batch.rs` and verify via `cargo test -p eqntide-desktop batch::tests`.
- [x] 2.2 Update `desktop/src/app.rs` (`add_files`, `batch_ui`, and `Open output folder`) to add an **Add folder…** button, support dropping folders recursively, display queued folder items cleanly, target the generated `<folder>_<target>_<time>/` folder on **Open output folder**, and verify with desktop unit tests covering dropped/added folders and output-folder routing via `cargo test -p eqntide-desktop`.

## 3. Settings Light / Dark Option Buttons

- [x] 3.1 Replace the single theme toggle button in `settings_ui` (`desktop/src/app.rs`) with selectable **Light** and **Dark** buttons using `with_selectable_style` + `standard_button` matching **Startup focus**, update and add Settings theme selection/persistence tests, and verify via `cargo test -p eqntide-desktop`.

## 4. Regression Verification

- [x] 4.1 Run `cargo test --lib` and `cargo test -p eqntide-desktop` to verify all CLI and desktop tests pass cleanly without warnings.
