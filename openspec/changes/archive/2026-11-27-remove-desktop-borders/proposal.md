# Proposal

## Why

The desktop application currently draws 1px borders across surface cards, canvas frames, popups, and interactive controls. Removing surface and control borders in favor of layered Ayu neutral surface fills creates a cleaner, flatter, modern desktop workspace while preserving horizontal and vertical structural region dividers (`sidebar vline`, `footer hline`, `status` top `hline`, and section separators) plus functional in-content markings such as the Batch DOCX folder/file tree hierarchy lines and equation placeholder boxes.

## What Changes

- Preserve horizontal and vertical structural region dividers across the desktop shell while removing the four-sided outer box around the status bar:
  - Sidebar vertical divider (`vline`), footer horizontal divider (`hline`), and status bar top horizontal divider (`hline`) in the main editor layout.
  - `ui.separator()` section dividers in the footer, Batch DOCX toolbar card, and About card rendered via `draw_section_separator` so disabled widgets stay borderless.
- Remove outer borders on surface containers and popups:
  - Equation Canvas and synchronized LaTeX source box frames (`pal.border` stroke removed).
  - Settings, Batch DOCX, and About card frames (`group_frame()` stroke removed) and Settings background thumbnail border.
  - Modal dialogs, dropdowns, tooltips, and inline `\command` popup chrome borders (`window_stroke = NONE`, giving the `\command` input box a distinct surface fill).
- Remove control borders (`bg_stroke = NONE`) across inactive, hovered, open, and selected states (`ThemeTokens` global widget styles, `with_selectable_style`, `primary_button_with`, and the Batch DOCX progress bar track), shifting visual feedback and separation to layered background fills (`app_bg`, `panel_bg`, `card_bg`, `control_bg`, `control_hover_bg`, `selected_bg`).
- **Preserve functional content & indicator strokes**:
  - Keep Batch DOCX **Folder tree** and **File tree** internal horizontal row dividers and vertical/horizontal hierarchy branch connectors (`paint_batch_row_chrome`) intact.
  - Keep Equation Canvas structural dashed placeholder boxes (`□`), caret, and selection highlight intact.
  - Keep checkbox square and radio circle indicator outlines, slider thumb border, and the status bar bridge readiness circle stroke when disconnected, plus accessible keyboard-focus distinction in modals/inputs.
- Adjust Light and Dark mode neutral surface/control fill layering in `ThemeTokens` (`card_bg` vs `control_bg` in both modes, and `app_bg` vs `CanvasPalette::bg` in Dark mode) so borderless buttons, cards, modals, popups, and the Equation Canvas / LaTeX surfaces remain clearly distinguishable against adjacent backgrounds in both Solid and Image background modes.

## Capabilities

### New Capabilities

- None

### Modified Capabilities

- `desktop-ui-theme`: Transition desktop UI surfaces, cards, canvases, popups, and controls to a borderless layered-surface design (`Stroke::NONE` across idle, hovered, and selected states) while preserving horizontal/vertical structural region dividers, Batch DOCX internal tree hierarchy/row lines, checkbox/radio/slider/placeholder functional indicators, and borderless keyboard-focus/input visual cues, and refining Ayu Light/Dark surface and control fill contrast.

## Impact

- Desktop theme & style engine: `desktop/src/theme.rs`.
- Desktop shell, canvas, palette/shared helpers, Batch DOCX, Settings, About, dialogs, and keyboard popup: `desktop/src/app/mod.rs`, `desktop/src/app/canvas.rs`, `desktop/src/app/palette_ui.rs`, `desktop/src/app/batch_ui.rs`, `desktop/src/app/settings_ui.rs`, `desktop/src/app/about_ui.rs`, `desktop/src/app/dialogs.rs`, `desktop/src/app/keyboard.rs`.
- Desktop test suites: `desktop/src/theme.rs` unit tests, `desktop/src/app/tests/layout_theme_tests.rs`, `desktop/src/app/tests/canvas_input_tests.rs`, `desktop/src/app/tests/batch_settings_tests.rs`, `desktop/src/app/tests/session_tests.rs`.
