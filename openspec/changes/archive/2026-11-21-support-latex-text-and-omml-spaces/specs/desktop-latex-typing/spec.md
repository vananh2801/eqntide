# Spec Delta

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
- **WHEN** the user types only a standalone template command name such as `\frac` without brace arguments and presses `Enter`
- **THEN** the editor inserts the template structure and wraps any active selection. The `\text` command requires a complete braced argument before Enter can insert text

#### Scenario: Invalid buffered expression stays editable on Enter
- **WHEN** the user presses `Enter` on an incomplete or invalid buffered LaTeX expression
- **THEN** the editor reports the error status and preserves the buffer so the user can edit with `Backspace` or cancel with `Esc`

### Requirement: Command suggestion navigation and Tab name completion without committing
The Equation Canvas command popup SHALL suggest matching command names while the user types the leading command token, using `Up`/`Down` to highlight suggestions and `Tab` only to complete the highlighted command name in the buffer without committing.

#### Scenario: Tab fills the suggested command name without committing
- **WHEN** the user types `\math`, highlights `\mathbb` in the suggestion list, and presses `Tab`
- **THEN** the command buffer updates to `\mathbb` and remains open so the user can continue typing `{R}` and later commit with `Enter`

#### Scenario: Space does not commit or dismiss the command buffer
- **WHEN** the user types `\lim_{x` followed by a space on the Equation Canvas
- **THEN** the space is appended to the command buffer and the expression is not committed

#### Scenario: A space after a bare command name leaves the buffer unchanged
- **WHEN** the command buffer contains only a command name such as `\text` and the user presses Space
- **THEN** the buffer stays open with the same command name
