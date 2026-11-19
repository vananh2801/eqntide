# Design

## Context

See `proposal.md` for motivation. In the current desktop app (`desktop/src/app.rs` and `desktop/src/batch.rs`):
- `App::batch_ui` packs input buttons, `Clear list`, target combo box, overwrite checkbox, `Convert`, `Stop after current file`, and `Open output folder` into a single card.
- `primary_button_with` sets `tokens.text_disabled` / `tokens.control_disabled_bg` and also wraps the inner button in `ui.add_enabled_ui(false, ...)`, which applies `egui`'s disabled alpha multiplier on top of disabled colors (causing `Convert` to look noticeably darker/dimmer than `Stop after current file`), while `Open output folder` is never disabled at all.
- `App::files: Vec<batch::BatchItem>` stores flat items without selection flags, `batch::run_items` assumes `0..items.len()` matches `App::batch_logs` indices, and `App::batch_output_dir: Option<PathBuf>` tracks only the first folder's output directory.
- Existing `desktop-ui-theme` rules require full-width cards, `30px` standard buttons / `24px` compact inline actions, and unified hover/disabled feedback.

## Goals / Non-Goals

**Goals:**
- Clear two-step visual hierarchy in the Batch DOCX top card (**Input** vs **Options & Actions**) with consistent disabled styling across all controls.
- Non-destructive dual-mode selection (**Flat list** default + **Folder tree**) where each mode keeps independent checked state and `Convert` executes only the active mode's checked items.
- Accurate per-row status, progress counting, and per-row output folder opening (immediate output file directory for all converted items + generated folder output root for folder-originated items) across multiple added files and folders from different paths.

**Non-Goals:**
- Changing the underlying DOCX conversion logic, sibling `<folder>_<target>_<time>/` naming, or `<stem>_<target>.docx` naming.
- Persisting queued batch files or per-file checkbox selections across app restarts (existing persisted settings remain `batch_target` and `batch_overwrite`).
- Reintroducing per-row delete (`x`) buttons or adding text/regex filter bars.

## Decisions

### 1. Toolbar grouping & unified disabled button appearance
- **Decision**: Split the top full-width card visually into an **Input** row (`Add DOCX files…`, `Add folder…`, and right-aligned/separated `Clear list`) and an **Options & Actions** row (`Target` combo box + `Overwrite original documents` checkbox, followed by `Convert` and `Stop after current file`). Remove the global `Open output folder` toolbar button.
- **Disabled styling**: Fix `primary_button_with` so disabled primary buttons do not double-dim (`add_enabled_ui(false, ...)` on top of `text_disabled`), or use consistent disabled visuals across `Convert`, `Stop after current file`, and `Clear list`.
- **Alternative**: Hiding `Stop after current file` when idle. Rejected to keep button positions stable and match the user's preference for a steady toolbar layout (Direction B).

### 2. Per-item dual selection state & mode switching
- **Decision**: Introduce `enum BatchSelectionMode { FlatList, FolderTree }` (default `FlatList`) on `App`, and store two independent selection flags on each queued row state (`BatchRowState` or fields alongside `BatchItem`):
  - `flat_selected: bool` (defaults to `true` on `push_batch_item`)
  - `tree_selected: bool` (defaults to `true` on `push_batch_item`)
- **Mode switching**: Switching between `FlatList` and `FolderTree` changes only `self.batch_selection_mode`, leaving both flags untouched. `Select all` / `Deselect all` mutate only the flag belonging to the active mode.
- **Convert**: Collects `(original_index, BatchItem)` for items where `item.is_selected(self.batch_selection_mode)` is `true`. `Convert` is enabled iff `!self.batch_running && selected_count > 0`.
- **Alternative**: Storing a single `selected: bool` shared across views. Rejected because toggling folders in Tree view would clobber careful manual selections made in Flat view.

### 3. Virtualized Folder Tree rendering
- **Decision**: Build a lightweight derived display list of `TreeRow` entries (root folder nodes, nested subfolder nodes, individual-files group node, and leaf file rows referring to `item_index: usize`) using a `HashSet<PathBuf>` of collapsed directory keys (`collapsed_tree_dirs`) so newly added folders start expanded.
- **Tri-state checkboxes**: Each directory/group node computes `(selected_count, total_count)` over its descendant `item_index` leaves (`all` -> checked, `none` -> unchecked, `partial` -> indeterminate). Clicking a directory checkbox sets `tree_selected` on all descendant leaves to `true` unless all are currently selected (in which case it sets them to `false`).
- **Scrolling**: Use `egui::ScrollArea::vertical().show_rows(ui, row_height, visible_rows.len(), ...)` with uniform `row_height` for both `FlatList` and `FolderTree` so large folders remain responsive and never hit layout O(N) slowdowns.
- **Alternative**: Recursive `egui::CollapsingHeader` widgets. Rejected because `CollapsingHeader` cannot participate in fixed-height `show_rows` virtualization and lacks built-in tri-state selection checkboxes.

### 4. Subset execution & per-row output folder tracking
- **Decision**: Update `batch::run_items` (or pass `Vec<(usize, BatchItem)>`) so `Event::Batch(orig_idx, result)` always identifies the original row in `self.files`.
- **Run state**: Store `batch_run_total: usize` alongside `batch_completed: usize` so `ProgressBar` shows `batch_completed / batch_run_total`. Before spawning the worker thread, reset logs/output records only for the selected `orig_idx` rows (`"Waiting"`), leaving unselected rows untouched (and displaying an `"Unchecked"` hint when never run).
- **Per-row output metadata**: On each row, store `last_output: Option<BatchRowOutput>` containing:
  - `file_dir: PathBuf` (`output_path.parent()`) — used by the per-row **Open output folder** compact button.
  - `root_output_dir: Option<PathBuf>` — `Some(folder_output_dir(root, target, &timestamp))` when `!overwrite && source_root.is_some()`, or `Some(root.clone())` when `overwrite && source_root.is_some()`, and `None` for individually added files (`source_root.is_none()`).
- **UI**: When `last_output` is `Some(...)` (meaning that row finished converting successfully), render compact (`24px`) buttons on the right of the row: `Open folder` (`file_dir`) and, when `root_output_dir.is_some()`, `Open root` (`root_output_dir`).

## Risks / Trade-offs

- **Adding a file individually first, then adding its parent folder later**: Currently `push_batch_item` deduplicates solely by `existing.input == item.input` and drops the folder entry. → Mitigation: If a newly added folder item matches an existing standalone item (`source_root == None`), upgrade its `source_root` to `Some(folder)` (or leave existing behavior clearly documented; upgrading is cleaner when users add a file then its folder).
- **Row width with two compact buttons**: Long file paths + two compact folder buttons in narrow windows (`1056x560`) could clip labels if not laid out with right-aligned button space first. → Mitigation: Use `egui::Layout::right_to_left` or reserve button width on the row header before rendering the truncated path label.
- **Existing tests**: `batch_tab_discovers_nested_folder_docx_files_and_targets_output_dir` currently asserts `a.batch_output_dir` and `a.resolved_batch_output_folder()`. → Mitigation: Update those unit tests to verify per-row `file_dir` and `root_output_dir` (and absence of `root_output_dir` on standalone files) alongside dual-mode selection and subset execution tests.
