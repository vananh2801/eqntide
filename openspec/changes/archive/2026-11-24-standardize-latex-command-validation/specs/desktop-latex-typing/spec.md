## MODIFIED Requirements

### Requirement: Buffered raw LaTeX command input on Equation Canvas
In Math mode, a backslash (`\`) starts command entry on the Equation Canvas. The editor SHALL buffer the expression until `Enter`. Inside a Text span, a backslash SHALL be inserted as a literal character.

#### Scenario: Brace arguments do not prematurely jump into placeholder boxes
- **WHEN** the user types `\mathbb{R}` or `\math` followed by `{` while entering a command on the Equation Canvas
- **THEN** the characters remain in the active command buffer and do not jump into an incomplete structural placeholder before `Enter` is pressed

#### Scenario: Subscripts, superscripts, and spaces stay in the command buffer
- **WHEN** the user types `\int_0^1 x dx` or `\lim_{x \to 0}` after starting a command with `\` on the Equation Canvas
- **THEN** `_`, `^`, `{`, `}`, and spaces remain part of the buffered LaTeX expression without splitting into nested boxes or auto-committing early

#### Scenario: Enter commits the complete buffered LaTeX expression once
- **WHEN** the user presses `Enter` with a valid buffered expression such as `\mathbb{R}`, `\int_0^1`, or `\lim_{x \to 0}`
- **THEN** the editor parses and inserts the full MathML expression into the Equation Canvas in a single undoable step and clears the command buffer

#### Scenario: Standalone template command name still supports placeholder fallback on Enter
- **WHEN** the user types only a standalone structural template command name such as `\frac`, `\sqrt`, `\boxed`, `\overset`, or `\pmatrix` without brace arguments and presses `Enter`
- **THEN** the editor inserts the structural template with visible placeholder boxes and wraps any active selection; argument-requiring non-structural commands (`\text`, `\mathbb`, `\mathfrak`, `\mathcal`, `\mathscr`, `\mathrm`, `\mathbf`, `\mathit`, `\boldsymbol`, `\operatorname`, `\hspace`, `\textstyle`, `\displaystyle`, and `\enclose`) SHALL NOT fall back to an empty node or hardcoded default on bare `Enter` and SHALL require complete, non-empty braced arguments

#### Scenario: Invalid buffered expression stays editable on Enter
- **WHEN** the user presses `Enter` on an incomplete or invalid buffered LaTeX expression
- **THEN** the editor reports the error in the bottom status bar using the unified `LaTeX error: ...` error style (without duplicating the error inside the floating command popup) and preserves the buffer with concise popup hints (`Enter insert · Esc cancel` / `No matching command — Esc to cancel`, omitting redundant `Backspace to edit`) so the user can keep editing (clearing the active error) or cancel with `Esc`

## ADDED Requirements

### Requirement: Argument completeness and content validation for desktop LaTeX entry
On the desktop editor (both the Equation Canvas command popup and the `LaTeX source` box), argument-requiring LaTeX commands SHALL be validated for complete, non-empty braced arguments and supported argument content before committing on `Enter` or marking `LaTeX source` valid. Errors from both surfaces SHALL use the identical diagnostic text and `LaTeX error: ...` status-bar presentation.

#### Scenario: Bare or empty-braced argument-requiring commands block Enter and show unified status feedback
- **WHEN** the user types `\mathbb`, `\mathfrak`, `\mathcal`, `\mathscr`, `\mathrm`, `\mathbf`, `\mathit`, `\boldsymbol`, `\operatorname`, `\hspace`, `\textstyle`, `\displaystyle`, or `\enclose` without complete braced arguments—or enters empty braces such as `\text{}`, `\mathbb{}`, `\mathbb{ }`, or `\operatorname{}`—and presses `Enter`
- **THEN** `Enter` is blocked, the equation remains unchanged, the popup stays open with the command buffer preserved (auto-completing a selected prefix to the canonical bare command name when matched from suggestions), and an actionable message is displayed in the bottom status bar using the unified `LaTeX error: ...` error presentation without duplicating the error inside the command popup

#### Scenario: Space-only text command remains valid
- **WHEN** the user types `\text{ }` and presses `Enter`
- **THEN** the editor commits a space-only `<mtext>` span without error

#### Scenario: Unsupported characters or nested structures inside specialized alphabet commands are blocked
- **WHEN** the user enters `\mathbb{1^2}`, `\mathbb{1}`, `\mathbb{a}`, `\mathcal{1}`, `\mathcal{a}`, `\mathscr{1}`, or `\mathfrak{1+2}` in the Equation Canvas command popup or `LaTeX source` box
- **THEN** `Enter` is blocked in the command popup (or `LaTeX source` reports an error and disables saving), both inputs display the identical `LaTeX error: ...` status-bar message explaining the allowed characters and suggesting placing exponents/subscripts outside the braces (such as `\mathbb{R}^2`), and the input remains editable

#### Scenario: General math style modifiers allow compound expressions and serialize as one command
- **WHEN** the user enters `\mathbf{x^2 + 2x - 1 = 0}`, `\mathbf{x^2}`, `\mathrm{H_2O}`, `\mathrm{m/s^2}`, `\mathit{AB}`, or `\boldsymbol{\alpha}`
- **THEN** the expression is accepted as valid, commits cleanly to the Equation Canvas, serializes back into `LaTeX source` as a single enclosing style command (such as `\mathbf{x^{2}+2x-1=0}`) rather than per-token wrappers, and remains exportable to MathType MTEF/OLE and OMML
