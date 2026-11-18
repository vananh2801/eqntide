# Spec Delta

## MODIFIED Requirements

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
