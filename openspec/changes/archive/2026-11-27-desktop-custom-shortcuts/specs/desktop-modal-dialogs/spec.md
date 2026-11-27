# Spec Delta

## MODIFIED Requirements

### Requirement: Centered modal presentation with non-dismissing backdrop
The **Finish current equation**, **Custom matrix**, and **Shortcut conflict** dialogs SHALL render using centered `egui::Modal` presentation with a full-viewport dimming backdrop, and clicking outside the dialog onto the backdrop SHALL keep the modal open without modifying application state.

#### Scenario: Finish current equation opens centered over a dimmed backdrop
- **WHEN** the user attempts to close the window or switch to a queued session while unsaved edits or an active batch job exist
- **THEN** **Finish current equation** renders centered in the viewport over a full-window dimming backdrop rather than in the top-left corner

#### Scenario: Clicking outside either modal onto the backdrop does nothing
- **WHEN** **Finish current equation** or **Custom matrix** is open and the user clicks on the darkened backdrop outside the modal frame
- **THEN** the modal remains open and no save, discard, insert, or cancel action is triggered

#### Scenario: Clicking outside Shortcut conflict onto the backdrop does nothing
- **WHEN** **Shortcut conflict** is open and the user clicks on the darkened backdrop outside the modal frame
- **THEN** the modal remains open and no replace, swap, or cancel action is triggered

### Requirement: Unified modal layout, uniform padding, and equal-height action buttons
The **Finish current equation**, **Custom matrix**, and **Shortcut conflict** dialogs SHALL share uniform `20px` four-side frame inner padding, horizontally centered headings, consistent `30px` (`BUTTON_HEIGHT`) action buttons with `14px × 6px` internal button padding across primary and secondary actions, and balanced content spacing.

#### Scenario: Finish current equation uses a centered heading without a redundant subtitle and equal-height buttons
- **WHEN** **Finish current equation** opens while no batch conversion is running
- **THEN** `"Finish current equation"` is centered horizontally inside a modal frame with uniform `20px` four-side padding, the descriptive subtitle `"Save your changes, discard them, or continue editing."` is omitted, and `Save`, `Discard / cancel session`, and `Continue editing` align vertically at the shared `30px` button height

#### Scenario: Custom matrix uses a centered heading, spaced two-column grid, unified dropdown widths, and right-aligned actions
- **WHEN** **Custom matrix** opens
- **THEN** `"Custom matrix"` is centered horizontally inside a compact balanced modal frame (`>= 260px` inner width with uniform `20px` four-side padding), the label and control columns are separated by `32px` horizontal spacing with `Alignment` and `Delimiters` `ComboBox` controls sharing a `156px` width, and `Insert matrix` and `Cancel` are right-aligned at the shared `30px` button height

#### Scenario: Shortcut conflict matches Custom matrix padding and button geometry
- **WHEN** **Shortcut conflict** opens
- **THEN** it renders centered inside a modal frame with uniform `20px` four-side padding, a centered heading, concise conflict details, and right-aligned `30px`-tall action buttons with `14px × 6px` button padding

### Requirement: Escape key cancels or continues editing cleanly
Pressing `Escape` while **Finish current equation**, **Custom matrix**, or **Shortcut conflict** is open SHALL dismiss the modal as a non-destructive cancel/continue action and restore focus appropriately (`Equation Canvas` / `LaTeX source` for editor modals, or the Settings shortcuts view for **Shortcut conflict**), except when an in-flight save/cancel request (`busy`) blocks dismissal or an inner dropdown popup (`ComboBox`) is currently open, and pressing `Escape` on the **Equation Canvas** to open **Finish current equation** SHALL NOT immediately dismiss it on the same frame.

#### Scenario: Escape dismisses Finish current equation as Continue editing
- **WHEN** **Finish current equation** is open (either from clicking window close, switching sessions, or pressing `Escape` on a previous frame) and the user presses `Escape` while not `busy`
- **THEN** the pending close/next intent is cleared, the equation remains intact for continued editing, and editor focus is restored

#### Scenario: Escape on Equation Canvas opens Finish current equation without self-closing
- **WHEN** the equation has unsaved changes, the **Equation Canvas** is focused, and the user presses `Escape`
- **THEN** **Finish current equation** opens and stays open until the user acts on it

#### Scenario: Escape closes Custom matrix unless a ComboBox dropdown is open
- **WHEN** **Custom matrix** is open with no dropdown popup open and the user presses `Escape`
- **THEN** **Custom matrix** closes without inserting a matrix and restores editor focus; if a `ComboBox` dropdown inside **Custom matrix** is open, `Escape` closes only the dropdown first

#### Scenario: Escape closes Shortcut conflict without changing shortcuts
- **WHEN** **Shortcut conflict** is open and the user presses `Escape`
- **THEN** **Shortcut conflict** closes without modifying any shortcut bindings

### Requirement: Initial primary-button focus and trapped keyboard navigation in modals
When **Finish current equation**, **Custom matrix**, or **Shortcut conflict** opens, the desktop application SHALL focus the primary action button (`Save`, or `Stop batch` when a batch job is running; `Insert matrix` in **Custom matrix**; `Replace` or `Cancel` in **Shortcut conflict**), SHALL trap `Tab` / `Shift+Tab` within the modal's enabled controls, SHALL allow `ArrowLeft` / `ArrowRight` to move focus between horizontal modal action buttons, and SHALL activate the focused button on `Enter` or `Space`.

#### Scenario: Finish current equation starts focused on Save and supports Tab, ArrowLeft/Right, and Enter
- **WHEN** **Finish current equation** opens with `Save`, `Discard / cancel session`, and `Continue editing` enabled
- **THEN** `Save` receives initial keyboard focus, `Tab` / `ArrowRight` and `Shift+Tab` / `ArrowLeft` cycle focus across `Save`, `Discard / cancel session`, and `Continue editing` without escaping behind the modal, and pressing `Enter` activates whichever button is currently focused

#### Scenario: Finish current equation focuses Stop batch while a batch conversion is running
- **WHEN** **Finish current equation** opens while `batch_running` is true and the equation action row is disabled
- **THEN** `Stop batch` receives initial focus and `Tab` stays on `Stop batch` until the batch stops

#### Scenario: Custom matrix starts focused on Insert matrix and preserves DragValue/ComboBox arrow behavior
- **WHEN** **Custom matrix** opens
- **THEN** `Insert matrix` receives initial focus so `Enter` inserts immediately and `ArrowLeft`/`ArrowRight` toggles between `Insert matrix` and `Cancel`, while `Tab`/`Shift+Tab` cycles across all enabled modal controls (`Rows`, `Columns`, `Alignment`, `Delimiters`, `Insert matrix`, `Cancel`) and `ArrowLeft`/`ArrowRight` remain available to `DragValue` or an open `ComboBox` when those controls are active

#### Scenario: Shortcut conflict traps Tab and Arrow navigation across its action buttons
- **WHEN** **Shortcut conflict** opens
- **THEN** its primary button receives initial focus, `Tab`/`Shift+Tab` and `ArrowLeft`/`ArrowRight` cycle across the modal's enabled buttons (`Swap` when present, `Replace` when present, `Cancel`), and `Enter`/`Space` activates the focused button
