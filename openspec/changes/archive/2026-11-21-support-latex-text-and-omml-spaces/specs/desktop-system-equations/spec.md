# Spec Delta

## MODIFIED Requirements

### Requirement: Enter inserts or promotes rows for systems and matrices on the Equation Canvas
In Math mode, when no LaTeX command buffer or active IME preedit is open, pressing `Enter` SHALL insert a new row below the caret inside a table or promote a single-slot left-fenced expression (`{` or `[`) into a two-row left-aligned system. Either action is a single undoable edit.

#### Scenario: Enter inside a table row inserts a new row below and moves the caret
- **WHEN** the caret is in Math mode inside a row of a table (`mtable`, including a system of equations or matrix) and the user presses `Enter` without an active command buffer
- **THEN** a new row with the same column count is inserted immediately below the current row, the caret moves to the first cell of the new row, and `Undo` reverts the entire row insertion in one step

#### Scenario: Enter inside a single-slot left brace or bracket promotes it to a 2-row system
- **WHEN** the user inserts a one-sided left brace (`{□`) or left bracket (`[□`), optionally types an equation such as `x + 2 = 1`, and presses `Enter` in Math mode without an active command buffer
- **THEN** the fenced content is placed into row 1 of a 2×1 left-aligned table under that same left delimiter, a new empty row 2 is created below it with the caret inside row 2, and `Undo` restores the single-slot fenced expression in one step

#### Scenario: Enter during an active LaTeX command buffer keeps command commit behavior
- **WHEN** the user is typing a `\` command in the Equation Canvas command buffer and presses `Enter`
- **THEN** `Enter` applies command confirmation rules. Text insertion requires a complete `\text{...}` argument, and no extra table row is inserted

#### Scenario: Enter outside a table or one-sided left fence is a no-op
- **WHEN** the caret is in Math mode at the top-level row or inside a non-table structure other than a one-sided `{` or `[` fence and the user presses `Enter` without an active command buffer
- **THEN** the equation remains unchanged

#### Scenario: Enter inside a text span takes priority over row editing
- **WHEN** the user presses Enter with the caret inside an existing text span, including inside a table cell
- **THEN** the caret moves after that span in Math mode in the same row
