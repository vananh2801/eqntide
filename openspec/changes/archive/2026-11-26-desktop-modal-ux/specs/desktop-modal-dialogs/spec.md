# Spec Delta

## Purpose

Defines centered modal presentation, uniform layout and spacing, backdrop interaction boundaries, keyboard dismissal, full keyboard focus navigation across desktop modal dialogs (`Finish current equation` and `Custom matrix`), and Edit-vs-Insert baseline close confirmation gating.

## ADDED Requirements

### Requirement: Baseline-aware close confirmation for Edit vs Insert sessions
The desktop application SHALL track whether the initial/saved equation baseline began empty (`saved_empty`) alongside `saved_xml`, `saved_font`, and `saved_mode`, and SHALL display **Finish current equation** only when a batch job is running or the active state is dirty relative to that baseline. Empty Insert/standalone sessions returning to empty SHALL close without prompting, whereas modifying or clearing a pre-existing Edit equation SHALL prompt.

#### Scenario: Empty Insert or standalone equation closes without confirmation even when font size or target changes
- **WHEN** the editor opened with an empty baseline (`saved_empty == true`), no batch job is running, and both the **Equation Canvas** and **LaTeX source** are empty (including after typing and deleting back to empty, or changing `font_pt` / `target`)
- **THEN** closing the window or resolving pending close/next skips **Finish current equation** and exits or cancels cleanly

#### Scenario: Deleting a pre-existing Edit equation still triggers Finish current equation
- **WHEN** the session opened with a pre-existing equation (`saved_empty == false`) and the user edits it or clears the **Equation Canvas** and **LaTeX source** to empty
- **THEN** attempting to close or switch sessions displays **Finish current equation**

### Requirement: Centered modal presentation with non-dismissing backdrop
Both the **Finish current equation** confirmation dialog and the **Custom matrix** dialog SHALL render using centered `egui::Modal` presentation with a full-viewport dimming backdrop, and clicking outside the dialog onto the backdrop SHALL keep the modal open without modifying application state.

#### Scenario: Finish current equation opens centered over a dimmed backdrop
- **WHEN** the user attempts to close the window or switch to a queued session while unsaved edits or an active batch job exist
- **THEN** **Finish current equation** renders centered in the viewport over a full-window dimming backdrop rather than in the top-left corner

#### Scenario: Clicking outside either modal onto the backdrop does nothing
- **WHEN** **Finish current equation** or **Custom matrix** is open and the user clicks on the darkened backdrop outside the modal frame
- **THEN** the modal remains open and no save, discard, insert, or cancel action is triggered

### Requirement: Unified modal layout, uniform padding, and equal-height action buttons
Both **Finish current equation** and **Custom matrix** SHALL share uniform `20px` four-side frame inner padding, horizontally centered headings, consistent `30px` (`BUTTON_HEIGHT`) action buttons with `14px × 6px` internal button padding across primary and secondary actions, and balanced content spacing.

#### Scenario: Finish current equation uses a centered heading without a redundant subtitle and equal-height buttons
- **WHEN** **Finish current equation** opens while no batch conversion is running
- **THEN** `"Finish current equation"` is centered horizontally inside a modal frame with uniform `20px` four-side padding, the descriptive subtitle `"Save your changes, discard them, or continue editing."` is omitted, and `Save`, `Discard / cancel session`, and `Continue editing` align vertically at the shared `30px` button height

#### Scenario: Custom matrix uses a centered heading, spaced two-column grid, unified dropdown widths, and right-aligned actions
- **WHEN** **Custom matrix** opens
- **THEN** `"Custom matrix"` is centered horizontally inside a compact balanced modal frame (`>= 260px` inner width with uniform `20px` four-side padding), the label and control columns are separated by `32px` horizontal spacing with `Alignment` and `Delimiters` `ComboBox` controls sharing a `156px` width, and `Insert matrix` and `Cancel` are right-aligned at the shared `30px` button height

### Requirement: Escape key cancels or continues editing cleanly
Pressing `Escape` while **Finish current equation** or **Custom matrix** is open SHALL dismiss the modal as a non-destructive cancel/continue action and restore focus to the active editor (`Equation Canvas` or `LaTeX source`), except when an in-flight save/cancel request (`busy`) blocks dismissal or an inner dropdown popup (`ComboBox`) is currently open, and pressing `Escape` on the **Equation Canvas** to open **Finish current equation** SHALL NOT immediately dismiss it on the same frame.

#### Scenario: Escape dismisses Finish current equation as Continue editing
- **WHEN** **Finish current equation** is open (either from clicking window close, switching sessions, or pressing `Escape` on a previous frame) and the user presses `Escape` while not `busy`
- **THEN** the pending close/next intent is cleared, the equation remains intact for continued editing, and editor focus is restored

#### Scenario: Escape on Equation Canvas opens Finish current equation without self-closing
- **WHEN** the equation has unsaved changes, the **Equation Canvas** is focused, and the user presses `Escape`
- **THEN** **Finish current equation** opens and stays open until the user acts on it

#### Scenario: Escape closes Custom matrix unless a ComboBox dropdown is open
- **WHEN** **Custom matrix** is open with no dropdown popup open and the user presses `Escape`
- **THEN** **Custom matrix** closes without inserting a matrix and restores editor focus; if a `ComboBox` dropdown inside **Custom matrix** is open, `Escape` closes only the dropdown first

### Requirement: Initial primary-button focus and trapped keyboard navigation in modals
When **Finish current equation** or **Custom matrix** opens, the desktop application SHALL focus the primary action button (`Save`, or `Stop batch` when a batch job is running; `Insert matrix` in **Custom matrix**), SHALL trap `Tab` / `Shift+Tab` within the modal's enabled controls, SHALL allow `ArrowLeft` / `ArrowRight` to move focus between horizontal modal action buttons, and SHALL activate the focused button on `Enter` or `Space`.

#### Scenario: Finish current equation starts focused on Save and supports Tab, ArrowLeft/Right, and Enter
- **WHEN** **Finish current equation** opens with `Save`, `Discard / cancel session`, and `Continue editing` enabled
- **THEN** `Save` receives initial keyboard focus, `Tab` / `ArrowRight` and `Shift+Tab` / `ArrowLeft` cycle focus across `Save`, `Discard / cancel session`, and `Continue editing` without escaping behind the modal, and pressing `Enter` activates whichever button is currently focused

#### Scenario: Finish current equation focuses Stop batch while a batch conversion is running
- **WHEN** **Finish current equation** opens while `batch_running` is true and the equation action row is disabled
- **THEN** `Stop batch` receives initial focus and `Tab` stays on `Stop batch` until the batch stops

#### Scenario: Custom matrix starts focused on Insert matrix and preserves DragValue/ComboBox arrow behavior
- **WHEN** **Custom matrix** opens
- **THEN** `Insert matrix` receives initial focus so `Enter` inserts immediately and `ArrowLeft`/`ArrowRight` toggles between `Insert matrix` and `Cancel`, while `Tab`/`Shift+Tab` cycles across all enabled modal controls (`Rows`, `Columns`, `Alignment`, `Delimiters`, `Insert matrix`, `Cancel`) and `ArrowLeft`/`ArrowRight` remain available to `DragValue` or an open `ComboBox` when those controls are active
