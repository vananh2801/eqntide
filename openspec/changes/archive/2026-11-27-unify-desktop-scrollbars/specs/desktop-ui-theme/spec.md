# Spec Delta

## ADDED Requirements

### Requirement: Fixed desktop scrollbars for overflowing content
Desktop scroll areas SHALL display a fixed-thickness scrollbar for each axis with scrollable overflow. A visible scrollbar SHALL occupy dedicated space outside the content viewport and remain visible while idle, hovered, and dragged. When an axis fits, its scrollbar SHALL be hidden and its space returned to the content viewport. Scrollbars SHALL remain legible in Light and Dark modes.

#### Scenario: Overflowing content shows a scrollbar while the pointer is elsewhere
- **WHEN** scrollable content overflows and the pointer is outside the scroll area
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
- **THEN** the horizontal scrollbar sits below the tiles and all three rows remain fully visible vertically
