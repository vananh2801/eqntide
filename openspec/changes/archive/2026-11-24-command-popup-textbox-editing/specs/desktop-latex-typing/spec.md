# Spec Delta

## ADDED Requirements

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
