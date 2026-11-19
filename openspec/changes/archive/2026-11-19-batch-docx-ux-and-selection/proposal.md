# Proposal

## Why

The desktop app's **Batch DOCX** tab currently packs input selection, list clearing, conversion options, and execution actions into one dense toolbar, making it unclear for first-time users what to do first. In addition, the action buttons render with three inconsistent opacity levels when no files are queued (`Convert` is double-dimmed, `Stop after current file` uses standard disabled opacity, and `Open output folder` stays fully opaque even though it does nothing), and users who add whole folders cannot choose which discovered `.docx` files to convert or exclude.

## What Changes

- Reorganize the **Batch DOCX** toolbar into clear workflow sections (**Input** vs **Options & Actions**) using consistent enabled/disabled button states, and remove the top-level **Open output folder** toolbar button.
- Support switching between two non-destructive selection modes — **Flat list** (default) and **Folder tree** — each preserving its own independent checkbox state so switching views never discards earlier selections; all newly added files start selected in both modes, and only **Clear list** removes items from the queue.
- Run conversion only on files currently checked in the active selection mode, tracking progress and per-item results accurately for partial runs across multiple added files and folders from different locations.
- Provide per-row folder-opening actions after a file completes conversion:
  - Open the immediate parent folder containing the converted output file (shown for every converted row).
  - Open the generated root output folder corresponding to the originally added folder (shown only for folder-derived items; hidden for individually added files).

## Capabilities

### New Capabilities

### Modified Capabilities

- `desktop-batch-conversion`: Reorganize the Batch DOCX toolbar and disabled states, replace the top-level Open output folder button with per-row output-file-folder and output-root-folder actions, and add dual-mode (Flat list / Folder tree) per-file selection with independent preserved state.

## Impact

- `desktop/src/app.rs`: `batch_ui`, `App` batch state (`BatchItem` selection state per mode, tree expansion state, subset execution tracking, per-row output paths), `primary_button_with` disabled styling consistency, and Batch DOCX UI tests.
- `desktop/src/batch.rs`: `BatchItem` / batch runner indexing so partial-selection runs map completion events back to the original queued rows and record each item's output path and folder output root.
- `openspec/specs/desktop-batch-conversion/spec.md`: Delta update to reflect per-row folder-opening actions and dual-mode selection.
