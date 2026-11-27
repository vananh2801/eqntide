# desktop-custom-shortcuts Specification

## Purpose

Provides a two-column keyboard shortcut management panel in desktop Settings—keeping system and navigation shortcuts locked on the left while allowing users to customize, clear, reset, and resolve conflicts for palette button shortcuts on the right—and dispatches active palette bindings on the Equation Canvas with tooltip-only shortcut hints.

## ADDED Requirements

### Requirement: Two-column keyboard shortcuts panel directly in Settings
The desktop **Settings** tab SHALL render a full-width **Keyboard Shortcuts** card with a single-row header (**Keyboard Shortcuts**, **Reset to defaults**, search box, and category dropdown) above two borderless `30%` / `70%` `panel_bg` columns separating read-only system shortcuts on the left from customizable palette shortcuts on the right.

#### Scenario: Header places Reset to defaults to the left of search and category filters
- **WHEN** the user opens the **Settings** tab and views the **Keyboard Shortcuts** card
- **THEN** `"Keyboard Shortcuts"` sits on the left of the header row while `"Reset to defaults"`, `"Search label, LaTeX, or shortcut…"`, and `"All categories"` sit right-aligned on the same header row in left-to-right order (`title < Reset < Search < Category`), and separate muted section subheadings are omitted

#### Scenario: Left column displays split read-only system shortcuts and preserves outer page scrolling
- **WHEN** the user views the left `30%` column of the **Keyboard Shortcuts** card
- **THEN** it renders inside a borderless `panel_bg` container (`16px` inner padding, zero outer margin, matching the right column's top and bottom alignment without horizontal divider lines) listing 10 individual read-only rows (`Save equation` -> `Ctrl+S`, `Undo` -> `Ctrl+Z`, `Redo` -> `Ctrl+Y / Ctrl+Shift+Z`, `Select all` -> `Ctrl+A`, `Copy` -> `Ctrl+C`, `Cut` -> `Ctrl+X`, `Paste` -> `Ctrl+V`, `Next placeholder` -> `Tab`, `Previous placeholder` -> `Shift+Tab`, `Close popup / Exit` -> `Esc`) with each label left-aligned and its borderless `control_bg` shortcut badge right-aligned, and mouse-wheel scrolling over this left column scrolls the outer Settings page

#### Scenario: Right palette table displays enlarged icons and LaTeX commands
- **WHEN** the user views the right `70%` **Palette Shortcuts** list in Settings
- **THEN** it renders inside a matching borderless `panel_bg` container (`16px` inner padding, zero outer margin, `16px` gap from the left column, `360px` vertical scroll viewport) where each row displays its `44.0×28.6` `PaletteAtlas` icon, canonical LaTeX `source` in monospace text (omitting human-readable name labels from the row face while keeping them in hover tooltips and search matching), shortcut button (`Set shortcut` or active chord), and clear (`×`) button when assigned

#### Scenario: Filtering and searching the palette shortcuts table
- **WHEN** the user enters a search query (matching label, LaTeX `source`, or shortcut text) or selects a category filter (`All categories`, `Assigned only`, or a specific palette group from `UI_GROUPS`)
- **THEN** the **Palette Shortcuts** list updates immediately inside the bounded scrollable container to show only matching palette items

### Requirement: Always-visible fixed-width solid scrollbar without button overlap
The **Palette Shortcuts** scrollable list SHALL render an always-visible, fixed-width non-floating vertical scrollbar (`ScrollStyle::solid()`) that reserves dedicated right margin clear of row action controls and isolates vertical scroll boundaries.

#### Scenario: Scrollbar stays visible at fixed width and never overlaps row buttons
- **WHEN** the **Palette Shortcuts** list renders with the pointer outside or hovering over the scrollbar track
- **THEN** the vertical scrollbar uses solid (`floating = false`) layout with constant `8.0px` width and full handle visibility without shrinking or expanding on hover, and preserves `10.0px` inner margin so the scrollbar never overlaps the shortcut capture button or clear (`×`) button

#### Scenario: Scrolling past the top or bottom of the right list does not scroll Settings
- **WHEN** the pointer is inside the right **Palette Shortcuts** container and the user scrolls up while already at the top or scrolls down while already at the bottom
- **THEN** remaining vertical wheel delta is consumed (`smooth_scroll_delta.y = 0.0`) so scrolling never chains out to the enclosing **Settings** scroll container, while wheeling over the left `30%` column still scrolls **Settings**

### Requirement: Simple default palette shortcut set
The desktop application SHALL ship with a canonical default set of palette shortcuts mapped directly to exact `PALETTE` `source` templates and allow resetting all palette bindings back to that default set.

#### Scenario: Default shortcuts insert canonical palette templates on the Equation Canvas
- **WHEN** the application starts with default settings and the **Equation Canvas** has focus
- **THEN** pressing any default palette shortcut inserts its canonical `PALETTE` template (`Ctrl+F` for `\frac{#?}{#?}`, `Ctrl+R` for `\sqrt{#?}`, `Ctrl+Shift+R` for `\sqrt[#?]{#?}`, `Ctrl+H` for `^{#?}`, `Ctrl+L` for `_{#?}`, `Ctrl+J` for `_{#?}^{#?}`, `Ctrl+Shift+L` for `\lim_{#? \to #?}`, `Ctrl+I` for `\int #? \, d#?`, `Ctrl+Shift+I` for `\int\limits_{#?}^{#?} #? \, d#?`, `Ctrl+Shift+S` for `\sum\limits_{#?}^{#?} #?`, `Ctrl+Shift+P` for `\prod\limits_{#?}^{#?} #?`, `Ctrl+Shift+M` for `\begin{pmatrix} #? & #? \\ #? & #? \end{pmatrix}`, `Ctrl+Shift+C` for `\begin{cases} #? & #? \\ #? & #? \end{cases}`, `Ctrl+9` for `\left( #? \right)`, and `Ctrl+[` for `\left[ #? \right]`)

#### Scenario: Reset to defaults restores default palette bindings
- **WHEN** the user has modified, added, or cleared palette shortcuts and clicks **Reset to defaults** in Settings
- **THEN** all palette shortcuts revert to the canonical default set, any user-added custom bindings on other palette buttons are cleared, and the change is persisted immediately

### Requirement: Validated key-capture recorder for palette shortcuts
Clicking the shortcut control for a palette entry SHALL enter a key-capture state that accepts `Ctrl`/`Alt`-modified chords or function keys (`F1`–`F12`), rejects bare typing/navigation keys, cancels on `Esc`, clears on `Backspace`/`Delete`, and suppresses global `Ctrl+S` save dispatch while recording.

#### Scenario: Recording a valid shortcut assigns and saves it
- **WHEN** the user clicks the shortcut button for a palette entry with no conflict (for example, assigning `Ctrl+Alt+V` to `\binom{#?}{#?}`)
- **THEN** the new shortcut is bound to that palette entry immediately and saved to settings

#### Scenario: Pressing Esc during recording cancels without modifying bindings
- **WHEN** the user clicks a palette entry to start recording and presses `Esc`
- **THEN** recording exits cleanly and the entry retains its previous shortcut (or remains unassigned)

#### Scenario: Pressing Backspace or Delete during recording clears the binding
- **WHEN** the recorder is active for a palette entry with an existing shortcut and the user presses `Backspace` or `Delete` (or clicks its clear button)
- **THEN** the entry's shortcut is cleared immediately and persisted

#### Scenario: Bare keys without Ctrl, Alt, or function key are rejected
- **WHEN** the recorder is active and the user presses `F`, `Shift+F`, `/`, `^`, or `ArrowLeft`
- **THEN** the key press is not bound to the palette entry and either keeps the recorder waiting or reports that `Ctrl`, `Alt`, or `F1`–`F12` is required

#### Scenario: Pressing Ctrl+S while recording does not trigger Save
- **WHEN** the user starts recording a palette shortcut in Settings and presses `Ctrl+S`
- **THEN** the global Save action is suppressed for that press and `Ctrl+S` is routed into the shortcut conflict check

### Requirement: Conflict-resolution modal for colliding shortcuts
Whenever a newly recorded shortcut collides with an existing binding on another command or palette entry, the desktop application SHALL open a centered **Shortcut conflict** modal allowing cancellation for fixed system collisions or **Cancel**, **Replace**, and **Swap** for palette collisions.

#### Scenario: Colliding with a fixed system shortcut blocks replacement and offers Cancel
- **WHEN** the user records `Ctrl+Z` or `Ctrl+S` for a palette button
- **THEN** the **Shortcut conflict** modal opens identifying the fixed system command (`Undo` or `Save equation`) with only **Cancel**, leaving both bindings unchanged

#### Scenario: Replacing a conflicting palette shortcut reassigns and clears the old button
- **WHEN** `Ctrl+F` is bound to `Fraction` and the user records `Ctrl+F` on `Binomial`, then chooses **Replace** in the **Shortcut conflict** modal
- **THEN** `Binomial` receives `Ctrl+F`, `Fraction` becomes unassigned, and the updated bindings persist immediately

#### Scenario: Swapping conflicting palette shortcuts exchanges both bindings
- **WHEN** `Ctrl+F` is bound to `Fraction` and `Ctrl+R` is bound to `Square root`, and the user records `Ctrl+R` on `Fraction` then chooses **Swap**
- **THEN** `Fraction` receives `Ctrl+R`, `Square root` receives `Ctrl+F`, and both updated bindings persist immediately

#### Scenario: Canceling a conflict modal leaves all shortcuts unchanged
- **WHEN** the **Shortcut conflict** modal is open and the user clicks **Cancel** or presses `Esc`
- **THEN** the modal closes and neither the target entry nor the conflicting entry changes

### Requirement: Equation Canvas dispatch and tooltip-only shortcut hints
While the **Equation Canvas** has focus, pressing an active palette shortcut SHALL insert its template and update `Recent`, while `LaTeX source` editing stays unaffected; editor action and palette buttons SHALL omit shortcut text or badges from button faces and display shortcuts only in hover tooltips.

#### Scenario: Customizing a shortcut updates Equation Canvas insertion and palette tooltip
- **WHEN** the user rebinds `Fraction` from `Ctrl+F` to `Ctrl+Shift+F` and opens the Editor tab
- **THEN** the `Fraction` palette button face displays only its icon without shortcut text, hovering `Fraction` shows `Ctrl+Shift+F` in its tooltip instead of `Ctrl+F`, pressing `Ctrl+Shift+F` on the focused Equation Canvas inserts `\frac{#?}{#?}` and records it in `Recent`, and pressing `Ctrl+F` no longer inserts a fraction

#### Scenario: Save button omits shortcut from its label and shows Ctrl+S on hover
- **WHEN** the editor footer renders the Save button in standalone or Office session mode
- **THEN** the button label reads `"Save MathML"` or `"Save & Return to Document"` without `"(Ctrl+S)"` on the button face, and hovering over the button displays a tooltip containing `Ctrl+S`

#### Scenario: Typing in LaTeX source does not trigger customizable palette template shortcuts
- **WHEN** keyboard focus is inside the **LaTeX source** text box and the user presses a customized palette shortcut
- **THEN** the Equation Canvas palette shortcut handler does not intercept or mutate the canvas state out from under the LaTeX text box
