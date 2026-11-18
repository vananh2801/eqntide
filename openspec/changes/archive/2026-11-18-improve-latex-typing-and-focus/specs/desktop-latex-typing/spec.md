# Spec Delta

## Purpose

Enables fast, uninterrupted LaTeX command typing on the desktop Equation Canvas through buffered command input and preserves continuous keyboard focus across editor surfaces and UI controls.

## ADDED Requirements

### Requirement: Buffered raw LaTeX command input on Equation Canvas
When typing a backslash (`\`) command on the Equation Canvas, the editor SHALL buffer the complete expression—including `{`, `}`, `_`, `^`, operators, numbers, and spaces—without prematurely committing or jumping into placeholder boxes until the user commits with `Enter`.

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
- **THEN** the editor inserts the template structure (wrapping any active selection) as before

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
