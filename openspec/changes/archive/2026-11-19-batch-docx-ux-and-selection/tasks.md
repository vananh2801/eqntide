# Tasks

## 1. Batch Data Model & Subset Runner

- [x] 1.1 Extend batch item/row state in `desktop/src/batch.rs` and `desktop/src/app.rs` with `BatchSelectionMode` (`FlatList` default, `FolderTree`), independent `flat_selected` and `tree_selected` flags (defaulting to `true` on add), and per-row `last_output` metadata (`file_dir` and optional `root_output_dir`), and verify existing `desktop/src/batch.rs` tests pass alongside unit coverage for independent mode flags.
- [x] 1.2 Update `batch::run_items` and `Event::Batch` handling to accept `(usize, BatchItem)` pairs for only the selected subset, preserve `source_root` when a standalone item is later added via its parent folder, track `batch_run_total` accurately, and verify with unit tests covering partial-subset conversion and multi-folder output metadata.

## 2. Toolbar Layout & Consistent Disabled Button States

- [x] 2.1 Reorganize `App::batch_ui` in `desktop/src/app.rs` into clear **Input** (`Add DOCX files…`, `Add folder…`, `Clear list`) and **Options & Actions** (`Target`, `Overwrite`, `Convert`, `Stop after current file`) groups, removing the top-level `Open output folder` toolbar button and verifying full-width card layout tests still pass.
- [x] 2.2 Fix `primary_button_with` disabled styling (eliminate double-dimming) and enforce consistent enable/disable predicates (`Clear list` when `!batch_running && !files.is_empty()`, `Convert` when `!batch_running && selected_count > 0`, `Stop after current file` when `batch_running`), verifying disabled appearance and interaction tests in `desktop/src/app.rs`.

## 3. Dual-Mode Selection UI (Flat List & Folder Tree)

- [x] 3.1 Implement the selection-mode toggle (`Flat list` default / `Folder tree`) and `Select all` / `Deselect all` controls without per-row delete buttons (keeping `Clear list` as the sole removal action), and verify mode-switching preservation tests confirm independent selection states are retained.
- [x] 3.2 Implement virtualized `Folder tree` row generation and rendering (`ScrollArea::show_rows`) grouped by `source_root`, nested subfolders, and an individual-files group with collapsible headers and tri-state parent checkboxes (`checked`, `indeterminate`, `unchecked`), and verify tree toggle and subfolder selection behavior via UI tests.

## 4. Per-Row Output Folder Actions & Integration Verification

- [x] 4.1 Render per-row compact folder-opening buttons on successfully converted rows (`Open output folder` targeting `file_dir` for all items, plus `Open root output folder` targeting `root_output_dir` only when `source_root` is present) and update/add batch UI tests verifying standalone vs nested folder output actions across multiple added sources.
- [x] 4.2 Run the desktop test suite (`cargo test -p eqntide-desktop` or workspace test command) and verify all Batch DOCX layout, selection-mode, subset-conversion, and per-row output-folder tests pass without regressions.
