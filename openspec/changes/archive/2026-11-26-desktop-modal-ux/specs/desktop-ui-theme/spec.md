# Spec Delta

## ADDED Requirements

### Requirement: Modal workspace dimming and visible keyboard focus rings
When a desktop modal dialog or blocking operation disables the workspace, the **Equation Canvas** SHALL dim uniformly alongside the sidebar, palette, LaTeX source box, and footer—preserving full `1.0` canvas opacity only when editing is disabled solely by an unparsed `draft_dirty` LaTeX draft—and keyboard-focused buttons in both primary and secondary styles SHALL display a `1.0px` high-contrast focus border matching secondary button stroke thickness and distinct from their background fill.

#### Scenario: Equation Canvas dims alongside surrounding panels behind modals
- **WHEN** **Finish current equation** or **Custom matrix** is open over the editor view
- **THEN** the **Equation Canvas** does not override outer disabled dimming to `1.0` opacity and appears dimmed consistently with the palette, LaTeX source box, sidebar, and footer

#### Scenario: Equation Canvas remains full opacity during draft_dirty LaTeX typing
- **WHEN** no modal or `busy` operation is active and the user enters an incomplete LaTeX draft (`draft_dirty == true`)
- **THEN** the **Equation Canvas** keeps `1.0` opacity so the last valid equation preview remains crisp and legible

#### Scenario: Keyboard-focused primary and standard buttons show a distinct 1px focus border
- **WHEN** a primary button (`Save`, `Insert matrix`) or standard button (`Discard / cancel session`, `Continue editing`, `Cancel`) holds keyboard focus
- **THEN** it renders a clearly visible `1.0px` contrast focus stroke (`tokens.text_primary` in Dark mode / `tokens.primary_fg` in Light mode on primary buttons; `tokens.accent` on standard buttons) distinct from its background fill in both Light and Dark modes without increasing button height or border thickness relative to neighboring buttons
