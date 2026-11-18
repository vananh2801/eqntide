# Design

## Context

See `proposal.md` for motivation. The CLI already implements recursive directory discovery (`collect_docx_files_recursive` in `src/cli/paths.rs`) and relative-path hierarchy preservation (`run_directory` in `src/cli/dispatch.rs`), whereas the desktop app (`desktop/src/app.rs`, `desktop/src/batch.rs`) currently tracks only `Vec<PathBuf>` of standalone `.docx` files and uses a single toggle button for Light/Dark theme in `settings_ui`.

## Goals / Non-Goals

**Goals:**
- Reuse the CLI's recursive `.docx` scanning rules (ignoring `~$*` and `.*`, sorted order) for desktop folder selection and drag-and-drop.
- Preserve exact subfolder hierarchy inside `<folder>_<target>_<time>/` when converting folders with overwrite disabled, while keeping single-file behavior (`<stem>_<target>.docx`) and overwrite-in-place unchanged.
- Align the Settings **Theme** (`Light` / `Dark`) control with the `Startup focus` selectable button group (`with_selectable_style` + `standard_button`).

**Non-Goals:**
- Changing CLI directory output naming (CLI keeps `<dir>_<suffix>` without timestamp).
- Adding a custom output directory picker to Batch DOCX (sibling `<folder>_<target>_<time>/` is automatic).
- Exposing a `System` theme button in Settings UI (the user requested `Light` and `Dark` buttons; `ThemeMode::System` remains valid for initial settings load/sanitization).

## Decisions

- **Share recursive `.docx` scanner in `eqntide::docx`:**
  - *Decision*: Extract `collect_docx_files_recursive` (and its `is_docx_file` / ignored-entry filter) from `src/cli/paths.rs` into a shared helper under `src/docx/` re-exported by `eqntide::docx`, and call it from both `src/cli/paths.rs` and `desktop/src/app.rs` / `desktop/src/batch.rs`.
  - *Rationale*: Guarantees identical handling of `~$*` lock files, hidden dotfiles/dotdirs, and deterministic sorting between CLI and desktop without duplicating traversal code.
  - *Alternative considered*: Re-implementing `read_dir` recursion inside `desktop/src/batch.rs`. Rejected to avoid logic duplication and drift.

- **Queue item representation (`BatchItem`) & per-run `<folder>_<target>_<time>/` naming:**
  - *Decision*: Replace `Vec<PathBuf>` in `App::files` with a `BatchItem` struct holding `input: PathBuf` and `source_root: Option<PathBuf>`. Standalone `.docx` files have `source_root: None`; files discovered from an added/dropped folder store `source_root: Some(folder_root)`. When starting a batch run, generate one local-time (falling back to UTC if local offset is unavailable on Linux) timestamp string `YYYYMMDD_HHMMSS` (`%Y%m%d_%H%M%S`) and compute each unique folder root's sibling output directory `<folder_name>_<target>_<time>/` once for the batch run.
  - *Rationale*: Grouping by `(source_root, target, timestamp)` guarantees every file from the same folder during one `Convert` run lands in the same `<folder>_<target>_<time>/` directory with its relative path (`input.strip_prefix(root)`) preserved, while allowing repeated runs without `"Output already exists"` collisions. Storing the last created output directory on `App` lets **Open output folder** open `<folder>_<target>_<time>/` directly.
  - *Timestamp crate*: `time` `0.3.55` is already present in `Cargo.lock` (used by `zip`), MIT/Apache-2.0 licensed under `deny.toml`; declaring `time` in `desktop/Cargo.toml` with `local-offset` + `formatting` (or a small formatter using `OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc())`) avoids introducing new lockfile dependencies or `unsafe` code (`#![forbid(unsafe_code)]`).
  - *Alternative considered*: Writing directly to `<folder>_<target>/` without `<time>`. Rejected per user requirement (`<ten_folder>_<target>_<time>/`).

- **Batch conversion destination helper in `desktop/src/batch.rs`:**
  - *Decision*: Generalize `batch::convert_one` (or add `output_for_item` + `convert_item`) so that `overwrite == true` replaces `input`, `source_root == None` writes `<stem>_<target>.docx` beside `input`, and `source_root == Some(root)` writes `<output_dir>/<relative>` after `create_dir_all(parent)` using the existing atomic temp-file + `persist`/`persist_noclobber` commit pattern.
  - *Rationale*: Preserves atomic safe commit (original files are untouched on conversion error) across nested subdirectories.

- **Batch UI ergonomics:**
  - *Decision*: Add **Add folder…** (`rfd::FileDialog::new().pick_folder()`) next to **Add DOCX files…**, update `add_files` to inspect each path (`is_dir()` -> recursive collect with `source_root: Some(dir)`, `.docx` file -> `source_root: None`) while deduplicating by `input` path, and display relative paths (e.g. `folder_name/sub/file.docx`) or full path with full-path hover tooltip in the file list.

- **Settings Light / Dark option buttons:**
  - *Decision*: In `settings_ui` (`Appearance` card), replace the `"Current theme: ..."` label and `"Switch to ... mode"` button with a `"Theme"` row rendering `[Light]` and `[Dark]` via `Self::with_selectable_style(ui, selected, |ui| Self::standard_button(ui, label))`, matching the `Startup focus` pattern (`"Equation canvas"` / `"LaTeX source"`). Clicking a button sets `ui.ctx().set_theme(...)`, updates `self.persisted.theme`, and marks `self.settings_dirty = true`.

## Risks / Trade-offs

- **Local offset lookup on multi-threaded Linux** → `time::OffsetDateTime::now_local()` refuses when `localtime_r` is unsafe in multi-threaded Linux processes and returns `Err`; falling back to `now_utc()` avoids panics, or obtaining local offset before spawning background threads (at startup / UI thread) allows local time cleanly. Wait: we can capture/cache local `UtcOffset` on startup before worker threads spawn, or handle fallback gracefully so timestamps never fail.
- **Empty folders or folders with only `~$` temp files** → `add_files` will find zero valid `.docx` files; set `self.status` to a clear message (e.g. `"No .docx files found in selected folder."`) instead of silently doing nothing.
- **Existing tests in `desktop/src/app.rs`** → Tests referencing `"Switch to Light mode"` (`sidebar_settings_and_about_tabs_work_and_toggle_theme`, `user_ui_edits_persist_atomically_and_coalesce_pointer_drag`) and `a.files` (`Vec<PathBuf>` in `batch_cards_and_empty_state_span_full_central_panel_width`) will be updated to match the new `"Light"` / `"Dark"` buttons and `BatchItem`.
