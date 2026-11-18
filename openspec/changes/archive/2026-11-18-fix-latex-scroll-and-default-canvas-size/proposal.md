# Proposal

## Why

Pressing `Enter` in the desktop LaTeX source box currently causes `TextEdit::multiline` to grow vertically without a scroll container inside `ui.add_sized([w, 48.0], ...)`, which centers the overflow evenly upward and downward: the top of the LaTeX box extends into the `CentralPanel` and gets covered by the Equation Canvas, while the bottom is clipped by the footer. In addition, the desktop window opens at `1100x650` with a compact `48px` (2-row) LaTeX box, giving the Equation Canvas `918x374px` of mostly empty space on launch.

## What Changes

- Wrap the LaTeX source `TextEdit::multiline` in a fixed-height bordered container with vertical scrolling (`ScrollArea::vertical`) and top-aligned layout instead of `ui.add_sized`, eliminating upward/downward overflow and canvas overlap when inserting newlines.
- Increase the LaTeX source panel default height (from `60px` / `48px` input height to `88px` / `80px` input height, ~4–5 visible lines) and style its vertical scrollbar using the active `CanvasPalette` scroll tokens.
- Reduce the default desktop viewport launch size from `1100x650` to `1040x560` (matching `MIN_WINDOW_SIZE`) so the Equation Canvas opens at a balanced, compact default proportion (`~858x256px`) while preserving all sidebar and three-row palette invariants.

## Capabilities

### New Capabilities

<!-- None -->

### Modified Capabilities

- `desktop-ui-theme`: Add bounded fixed-height vertical scrolling and increased default height for the LaTeX source box (preventing overlap with the Equation Canvas and footer on newline entry), and set the default launch window size to `1040x560` to keep the initial Equation Canvas size compact.

## Impact

- `desktop/src/app.rs`: `editor_ui`, `latex_ui`, and desktop UI layout/overlap tests.
- `desktop/src/main.rs`: `desktop_viewport` default `.with_inner_size([1040.0, 560.0])`.
- `openspec/specs/desktop-ui-theme/spec.md`: Delta spec merged upon archive.
