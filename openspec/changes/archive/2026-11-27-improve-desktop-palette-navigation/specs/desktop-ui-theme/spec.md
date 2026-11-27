# Spec Delta

## MODIFIED Requirements

### Requirement: Fixed desktop scrollbars for overflowing content
Desktop scroll areas except the editor palette SHALL show a fixed-thickness scrollbar for each overflowing axis. Each visible scrollbar SHALL remain legible in Light and Dark modes and occupy dedicated space outside the viewport while idle, hovered, or dragged. When an axis fits, its scrollbar SHALL be hidden and its space returned to the viewport. The editor palette SHALL omit scrollbars and reserve a constant height for three rows.

#### Scenario: Overflowing content shows a scrollbar while the pointer is elsewhere
- **WHEN** scrollable content outside the editor's palette overflows and the pointer is outside the scroll area
- **THEN** the scrollbar is visible and its handle is distinguishable from the track in Light and Dark modes

#### Scenario: Hover and drag preserve scrollbar and viewport dimensions
- **WHEN** the user hovers or drags a scrollbar while the content and available space remain the same
- **THEN** the scrollbar retains its thickness and position outside the content viewport, and the viewport retains its dimensions

#### Scenario: Content that fits uses the full viewport
- **WHEN** content fits initially or fits after editing, filtering, collapsing rows, or enlarging the window
- **THEN** the corresponding scrollbar is absent and the content viewport uses the available space on that edge

#### Scenario: Equation Canvas shows scrollbars on the axes with overflow
- **WHEN** the equation exceeds the available viewport on one or both supported axes
- **THEN** each axis that needs scrolling has its own visible scrollbar beside or below the viewport, outside the equation content

#### Scenario: Horizontal palette scrollbar leaves all tile rows accessible
- **WHEN** the active palette category exceeds the available width in its three-row layout
- **THEN** the palette displays no scrollbar, all three rows remain fully visible vertically, and the palette occupies the same height as a category that fits
