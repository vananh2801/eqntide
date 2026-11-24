# desktop-latex-typing Specification

## Purpose

Enables fast, uninterrupted LaTeX command typing on the desktop Equation Canvas through buffered command input and preserves continuous keyboard focus across editor surfaces and UI controls.

## Requirements

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

### Requirement: Textbox cursor navigation and editing in the command popup
While the Equation Canvas `\` command popup is open, the buffered command input after the leading `\` SHALL behave as a single-line textbox supporting `ArrowLeft`, `ArrowRight`, `Home`, `End`, selection modifiers, `Delete`, `Backspace`, and clipboard shortcuts at the active cursor position.

#### Scenario: Left and Right arrows move the cursor and edit mid-buffer
- **WHEN** the user types `\frac{1}{2}`, presses `ArrowLeft` three times, and types `0` or presses `Backspace`
- **THEN** the insertion or deletion occurs at the cursor position inside the command buffer (producing `\frac{10}{2}` or `\frac{}{2}`) rather than appending to or popping from the end

#### Scenario: Home, End, and Delete operate on the popup command buffer
- **WHEN** the command buffer contains `alpa`, the user presses `Home`, moves with `ArrowRight`, and uses `Delete` or inserts `h`
- **THEN** the cursor jumps to the start/end of the command buffer and `Delete` removes the character after the cursor without affecting the underlying Equation Canvas

#### Scenario: Backspace on an empty command buffer closes the popup
- **WHEN** the command buffer after `\` is empty and the user presses `Backspace`
- **THEN** the command popup closes without modifying the Equation Canvas and returns focus to the Equation Canvas

#### Scenario: Tab places the cursor at the end of the completed command name
- **WHEN** the user types `\math`, highlights `\mathbb` in the suggestion list, and presses `Tab`
- **THEN** the command buffer updates to `mathbb` with the cursor positioned at the end so subsequent characters such as `{R}` append cleanly

### Requirement: Pointer click and selection inside the command popup textbox
Clicking or dragging inside the command popup's input textbox SHALL place the cursor or select text within the command buffer without dismissing the popup, while returning keyboard focus to the Equation Canvas when the popup closes or commits.

#### Scenario: Clicking inside the command textbox repositions the cursor without closing
- **WHEN** the command popup displays `\int_0^1` and the user clicks between `_` and `0` in the popup textbox and types `a`
- **THEN** the popup stays open, the cursor moves to the clicked character boundary, and the buffer updates to `int_a0^1`

#### Scenario: Clicking outside onto the Equation Canvas or LaTeX source dismisses the popup
- **WHEN** the command popup is open and the user clicks on the Equation Canvas outside the popup or into the `LaTeX source` box
- **THEN** the command popup closes without committing the unsubmitted buffer

#### Scenario: Committing or canceling from the popup textbox restores Equation Canvas focus
- **WHEN** the user edits inside the command popup textbox and then presses `Enter`, presses `Esc`, or clicks a suggestion item
- **THEN** the action completes (`Enter`/click commits, `Esc` discards) and keyboard focus returns immediately to the Equation Canvas

### Requirement: Initial focus on window launch and session activation
When the desktop window opens, loads an incoming equation session, or returns from non-editor tabs to the editor view, the application SHALL focus the user's configured startup focus target (`Equation canvas` or `LaTeX source`).

#### Scenario: Launching with default startup focus places caret on Equation Canvas
- **WHEN** the desktop application starts with default preferences (`startup_focus` set to `Equation canvas`)
- **THEN** the Equation Canvas receives keyboard focus immediately without requiring a pointer click

#### Scenario: Launching with LaTeX source startup focus focuses the LaTeX box
- **WHEN** the user configures `startup_focus` to `LaTeX source` and opens the desktop application or switches back to the editor tab
- **THEN** the LaTeX source editor receives keyboard focus immediately

### Requirement: Active editor surface focus retention across control interactions
The desktop application SHALL remember whether `Equation canvas` or `LaTeX source` most recently held editor focus and restore focus to that exact surface after clicking palette items, sidebar categories, matrix buttons, footer actions, or status bar controls.

#### Scenario: Clicking sidebar, matrix, or footer controls preserves Equation Canvas focus
- **WHEN** the user is typing on the Equation Canvas and clicks a sidebar category tile, palette button, `+ row`/`− row`/`+ col`/`− col`, `Undo`, `Redo`, or output target control
- **THEN** the action applies and keyboard focus immediately returns to the Equation Canvas

#### Scenario: Clicking editor controls while typing in LaTeX source preserves LaTeX focus
- **WHEN** the user is focused in the LaTeX source box and clicks a sidebar category tile, palette item, `Undo`, `Redo`, or output target control
- **THEN** the action applies and keyboard focus returns to the LaTeX source box rather than moving away or clearing focus

### Requirement: Blinking caret on focused Equation Canvas
When the Equation Canvas holds keyboard focus and cursor blinking is enabled in the active UI visuals, the canvas caret SHALL blink on the same timing rhythm as text inputs and reset immediately to visible whenever the caret moves, text is typed, or command input changes.

#### Scenario: Focused canvas caret blinks when idle and stays solid while typing
- **WHEN** the Equation Canvas is focused and idle past the active `on_duration`
- **THEN** the caret alternates between hidden and visible on the configured blink interval and resets immediately to visible upon any keystroke or caret movement
