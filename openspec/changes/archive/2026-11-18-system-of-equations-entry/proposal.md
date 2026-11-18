# Proposal

## Why

Users writing multi-line systems of equations with a single left curly brace (`{`) or single left square bracket (`[`) currently face unnecessary friction: the **Fences** palette only includes two-column piecewise `cases` templates (4-slot and 6-slot) and single-slot `{□` / `[□` fences, the **Custom matrix** dialog lacks one-sided left delimiters, and pressing `Enter` on the Equation Canvas outside a `\` command buffer does nothing. Although EqnTide's MathML, layout, OMML, and MTEF pipelines already support single-sided fences wrapping left-aligned column tables (`mtable`), there is no direct UI path to insert or grow them quickly.

## What Changes

- Add four 1-column, left-aligned system-of-equations templates to the **Fences** palette (2-row and 3-row with left `{`, and 2-row and 3-row with left `[`), plus matching command palette entries.
- Extend **Custom matrix** (`eqntide::catalog::matrix` and `MatrixDialog`) with one-sided delimiter options: `Left brace {` and `Left bracket [`.
- Add `Enter` key handling on the Equation Canvas (when no LaTeX command buffer is active) to insert a new row below the current row inside an `mtable` (or promote a single-slot left-fenced expression `{□` / `[□` into a 2-row, 1-column left-aligned system) and move the caret into the new row, in a single undoable transaction.
- Extend `Backspace` / `Delete` handling on the Equation Canvas (`Editor::delete`) to remove an empty table row (`mtr` where every cell is empty, when the `mtable` has at least two rows) and move the caret to the adjacent row in a single undoable transaction.

## Capabilities

### New Capabilities
- `desktop-system-equations`: Direct palette templates, one-sided left delimiter options in Custom matrix, and `Enter`-driven row insertion for multi-line systems of equations on the Equation Canvas.

### Modified Capabilities
<!-- None; existing desktop-palette-navigation and desktop-latex-typing requirements remain unchanged. -->

## Impact

- **Catalog (`src/catalog.rs`)**: New system-of-equations palette templates/commands and extended `matrix(..., fence)` delimiter variants (`"left-brace"`, `"left-bracket"`).
- **Structural editor (`crates/eqntide-editor/src/lib.rs`)**: New `Editor` operation(s) to insert a row after the caret's current row in an `mtable` and to wrap/split a single-sided left fence's content into a 2-row, 1-column left-aligned `mtable`.
- **Desktop UI (`desktop/src/app.rs`)**: Extended `FENCES` choices in `matrix_dialog_ui` and `Key::Enter` handling inside `keyboard` when `command_input` is `None`.
- **Export / Rendering (`src/render/layout/fence.rs`, `src/omml/encode.rs`, `src/mtef/encode.rs`, `src/mathml_to_latex.rs`)**: Verified end-to-end roundtrip compatibility for single-sided `[` and `{` fences around `mtable` (no breaking schema changes expected).
