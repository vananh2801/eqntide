# Design

## Context

See `proposal.md` for motivation and scope. In `App::shell` (`desktop/src/app.rs`), switching to `self.tab` `1` (**Batch DOCX**), `2` (**Settings**), or `3` (**About**) hides the `158px` left sidebar (`app-sidebar`) and, in standalone mode, the bottom footer (`app-footer`), giving `CentralPanel` the entire window width (`1016px` inside horizontal margins at `1040x560`, or wider when resized). Because `egui::ScrollArea::vertical()` defaults to `auto_shrink = [true, true]` and the three views lack full-width card containers or clamp card width to `540px`, content hugs the left edge while leaving large blank regions on the right.

## Goals / Non-Goals

**Goals:**
- Make all secondary tabs (`batch_ui`, `settings_ui`, `about_ui`) visually span the full `CentralPanel` width at `1040x560` and larger window sizes.
- Keep card styling (`ThemeTokens::group_frame()` with `8px` surface radius), spacing (`theme::SPACE_*`), and existing navigation (`← Back to equation`) consistent with `desktop-ui-theme`.
- Keep interactive controls responsive and unclipped if the window is narrowed or scrolled vertically.

**Non-Goals:**
- Keeping the equation editor's left sidebar visible on secondary tabs (navigation remains `← Back to equation`).
- Introducing a centered narrow-column layout (`max-width`) that leaves empty horizontal gutters on both sides.
- Modifying `batch.rs` conversion logic or `settings.rs` persistence schema.

## Decisions

- **Full-width cards via `set_width(ui.available_width())` + `auto_shrink([false, false])`:**
  - *Decision*: Wrap card contents with `ui.set_width(ui.available_width())` (accounting for card inner margins by applying width inside or via a shared helper around `tokens.group_frame()`) and configure `egui::ScrollArea::vertical().auto_shrink([false, false])` on scrollable containers.
  - *Rationale*: `ui.set_min_width(...)` alone lets `ScrollArea` shrink horizontally to content width (`auto_shrink.x == true`) unless `auto_shrink([false, false])` is set, and `540.0` artificially capped `Settings` cards to half the window. Using a small shared card helper avoids duplicating frame + full-width boilerplate across all three tabs.
  - *Alternative considered*: Centered max-width container (~`760px`–`840px`). Rejected because the user specifically requested full-width alignment matching the window edges; also `Batch DOCX` file paths benefit from maximum horizontal space.

- **Balanced internal layout for wide `Settings` cards:**
  - *Decision*: In `settings_ui`, give `Canvas zoom`, `Appearance`, and `Startup focus` cards full width, arranging each setting row cleanly (for example, keeping descriptions/labels on the left or full-width wrapped `canvas_preset_picker` rows) and letting `egui::Slider` or control rows breathe across the card without drifting off-screen.
  - *Alternative considered*: Multi-column grid of cards. Rejected to preserve clear vertical scanning order and avoid awkward wrapping at `1040px`.

- **Card-framed `About` view:**
  - *Decision*: Wrap the `About` logo/summary block, `ui.separator()`, and `about-info` `Grid` inside a full-width `group_frame()` card (within a `ScrollArea::vertical().auto_shrink([false, false])` for short viewports).
  - *Rationale*: Keeps `ui.separator()` bounded inside the card so it aligns with the metadata block instead of cutting across empty background space.

- **Two-tier card structure in `Batch DOCX` (`batch_ui`):**
  - *Decision*: Group the batch toolbar (file selection, clear, conversion target `ComboBox`, overwrite checkbox, and `Convert` / `Stop after current file` / `Open output folder` actions) in a full-width `group_frame()` card, and render either a full-width drop-zone / empty-state card when `self.files.is_empty()` or the full-width `ProgressBar`, summary counts, and `ScrollArea::vertical().auto_shrink([false, false]).show_rows(...)` file list below it.
  - *Rationale*: Eliminates the top-left cluster appearance when no files are loaded and visually aligns the controls with the full-width file list when files are added.

## Risks / Trade-offs

- **Extremely wide monitors (e.g., maximized 1920px+)** → Card interiors have more horizontal whitespace between left-aligned controls and the right card border, though cards and borders remain cleanly aligned with the window edges. Mitigated by placing secondary actions or right-aligned helpers sensibly where appropriate and keeping `canvas_preset_picker` wrapped cleanly.
- **ScrollArea scrollbar reservation** → Changing `auto_shrink` to `[false, false]` ensures the scrollbar stays pinned to the right edge of the panel rather than beside narrow content, while `ui.available_width()` inside `ScrollArea` prevents horizontal overflow when the vertical scrollbar appears.
- **UI layout tests in `desktop/src/app.rs`** → Existing tests checking `Settings`, `About`, and `Batch DOCX` button labels and interactions should continue passing, and new geometric assertions should verify card/container widths span the `CentralPanel` width at `MIN_WINDOW_SIZE` (`1040x560`).
