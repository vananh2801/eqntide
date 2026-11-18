# desktop-system-equations Specification

## Purpose

Enables direct entry and expansion of multi-line systems of equations with a single left curly brace (`{`) or single left square bracket (`[`) across the Fences palette, Custom matrix dialog, and Equation Canvas keyboard editing.

## Requirements

### Requirement: One-column left-fenced system templates in Fences palette
The **Fences** palette SHALL provide one-column, left-aligned system-of-equations templates with a single left delimiter and no right delimiter for both curly braces (`{`, 2-row and 3-row) and square brackets (`[`, 2-row and 3-row), alongside existing two-column piecewise `cases` templates.

#### Scenario: Inserting a 2-row or 3-row curly-brace system template
- **WHEN** the user clicks the 2-row or 3-row single-left-brace 1-column system template in the **Fences** palette
- **THEN** the editor inserts a single stretchy `{` on the left enclosing a 2×1 or 3×1 left-aligned table with empty editable row slots and places the caret in the first row

#### Scenario: Inserting a 2-row or 3-row square-bracket system template
- **WHEN** the user clicks the 2-row or 3-row single-left-bracket 1-column system template in the **Fences** palette
- **THEN** the editor inserts a single stretchy `[` on the left enclosing a 2×1 or 3×1 left-aligned table with empty editable row slots and places the caret in the first row

#### Scenario: System templates export cleanly across supported formats
- **WHEN** a user fills equations into any 1-column `{` or `[` system template
- **THEN** the equation renders with a full-height left delimiter and serializes cleanly to LaTeX, OMML, and MathType/MTEF without extra empty columns

### Requirement: One-sided left delimiter options in Custom matrix dialog
The **Custom matrix** dialog SHALL include one-sided left delimiter options (`Left brace {` and `Left bracket [`) in its **Delimiters** selector so users can create left-fenced systems or matrices of arbitrary row and column counts (including `N × 1` left-aligned systems).

#### Scenario: Creating a 4-row left-aligned curly-brace system via Custom matrix
- **WHEN** the user opens **Custom matrix**, sets **Rows** to `4`, **Columns** to `1`, **Alignment** to `Left`, **Delimiters** to `Left brace {`, and clicks **Insert matrix**
- **THEN** the editor inserts a single stretchy `{` on the left wrapping a 4×1 left-aligned table and places the caret in the first cell

#### Scenario: Creating a multi-row square-bracket system via Custom matrix
- **WHEN** the user opens **Custom matrix**, selects `Left bracket [`, and clicks **Insert matrix**
- **THEN** the editor inserts a single stretchy `[` on the left wrapping the configured table with no right delimiter

### Requirement: Enter inserts or promotes rows for systems and matrices on the Equation Canvas
When no LaTeX command buffer or active IME preedit is open on the Equation Canvas, pressing `Enter` SHALL insert a new row below the caret when inside a table, or promote a single-slot left-fenced expression (`{` or `[`) into a two-row left-aligned system, each as a single undoable edit.

#### Scenario: Enter inside a table row inserts a new row below and moves the caret
- **WHEN** the caret is inside any row of a table (`mtable`, including a system of equations or matrix) and the user presses `Enter` without an active command buffer
- **THEN** a new row with the same column count is inserted immediately below the current row, the caret moves to the first cell of the new row, and `Undo` reverts the entire row insertion in one step

#### Scenario: Enter inside a single-slot left brace or bracket promotes it to a 2-row system
- **WHEN** the user inserts a one-sided left brace (`{□`) or left bracket (`[□`), optionally types an equation such as `x + 2 = 1`, and presses `Enter` without an active command buffer
- **THEN** the fenced content is placed into row 1 of a 2×1 left-aligned table under that same left delimiter, a new empty row 2 is created below it with the caret inside row 2, and `Undo` restores the single-slot fenced expression in one step

#### Scenario: Enter during an active LaTeX command buffer keeps command commit behavior
- **WHEN** the user is typing a `\` command in the Equation Canvas command buffer and presses `Enter`
- **THEN** `Enter` commits the buffered LaTeX command as before without inserting an extra table row

#### Scenario: Enter outside a table or one-sided left fence is a no-op
- **WHEN** the caret is at the top-level row or inside a non-table structure other than a one-sided `{` or `[` fence and the user presses `Enter` without an active command buffer
- **THEN** the equation remains unchanged

### Requirement: Backspace and Delete remove an empty table row
When the caret is inside an empty cell of a table row (`mtr`) (with no active selection, command buffer, or IME preedit):
- If all cells in the row are empty and the enclosing table (`mtable`) has two or more rows, pressing `Backspace` or `Delete` SHALL remove that row and move the caret to an adjacent remaining row as a single undoable edit.
- If the row has multiple cells and at least one other cell in the same row has content, pressing `Backspace` or `Delete` inside an empty cell SHALL select the entire row and place the caret at the end of that row so a subsequent `Backspace` or `Delete` removes the row.
- If the enclosing table (`mtable`) has only one row and all its cells are empty, pressing `Backspace` or `Delete` SHALL select the entire table (or its enclosing fenced wrapper if fenced) so a subsequent `Backspace` or `Delete` deletes the table or fenced structure.
- Erasing a selection that spans an entire table row (`mtr`) in a table with two or more rows SHALL remove that row from the table, and erasing a partial-row cell selection SHALL reset each selected cell to an empty `<mtd>` without corrupting `<mtr>` or `<mtd>` tags into `<mrow>`.

#### Scenario: Backspace on an empty row removes it and moves caret to the previous row
- **WHEN** a table has at least two rows, all cells in the caret's current row are empty (including rows that previously held characters since deleted), and the user presses `Backspace`
- **THEN** that row is removed from the table, the caret moves to the end of the previous row (or the start of the new first row if row 0 was removed), and `Undo` restores the removed row in one step

#### Scenario: Delete on an empty row removes it and moves caret to the next row
- **WHEN** a table has at least two rows, all cells in the caret's current row are empty, and the user presses `Delete`
- **THEN** that row is removed from the table, the caret moves to the start of the next row (or the end of the previous row if the last row was removed), and `Undo` restores the removed row in one step

#### Scenario: Non-empty rows and single-row tables are not removed by Backspace or Delete
- **WHEN** another cell in the same row contains content, or the table has only one row, and the user presses `Backspace` or `Delete` once in an empty cell
- **THEN** the row is preserved in the table (selected rather than immediately removed)

#### Scenario: Backspace or Delete in an empty cell of a partially filled multi-cell row selects the row
- **WHEN** a table row has multiple cells where at least one cell is non-empty and the caret is in an empty cell of that row, and the user presses `Backspace` or `Delete`
- **THEN** the entire row is highlighted as selected with the caret at the end of the row, and pressing `Backspace` or `Delete` again removes that row (when the table has two or more rows) or deletes/selects the single-row table

#### Scenario: Single-row empty table uses selection-first deletion
- **WHEN** a table has only one row whose cells are all empty and the user presses `Backspace` or `Delete`
- **THEN** the first press selects the table (or its enclosing fenced wrapper), and a second `Backspace` or `Delete` removes the table or fenced structure

#### Scenario: Drag-selecting an empty or populated table row and pressing Backspace or Delete removes the row cleanly
- **WHEN** the user selects an entire row of a multi-row table (including an empty row) and presses `Backspace` or `Delete`
- **THEN** the `<mtr>` row is removed from `<mtable>` and reflected in both the canvas render and LaTeX output rather than shrinking empty boxes or leaving stale `&`/`\\` separators
