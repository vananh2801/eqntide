# Spec Delta

## ADDED Requirements

### Requirement: Three-row palette layout in column order
The palette SHALL reserve the same height for three rows regardless of item count, category, or horizontal overflow. It SHALL place items in a stable order that places related variants together, filling each column from top to bottom before starting the next column to its right. The `Custom matrix` button SHALL occupy the next slot after the category's last catalog item.

#### Scenario: Items keep their row and column when the window resizes
- **WHEN** the window is resized while the active category's ordered item list stays the same
- **THEN** the first three items occupy the first column, the fourth item starts the second column, and resizing changes the visible portion of the palette without rearranging items

#### Scenario: Recent starts with the most recently inserted item
- **WHEN** the user opens a populated `Recent` category
- **THEN** the most recently inserted item occupies the top-left slot and older items follow in column order

### Requirement: Horizontal palette navigation without a scrollbar
The palette SHALL use the full available width for the tile viewport and omit scrollbars and navigation buttons. When its tiles overflow horizontally, it SHALL support primary-button drag and vertical mouse-wheel scrolling. Both inputs SHALL share a horizontal offset clamped to the scrollable range.

#### Scenario: Overflowing tiles use the full palette width
- **WHEN** the active category contains more tiles than can fit across three rows
- **THEN** the tile viewport spans the available palette width without side controls or a scrollbar
- **AND** scrolling to the end exposes the last column

#### Scenario: Dragging from a tile scrolls without activating the tile
- **WHEN** the user holds the primary mouse button in the overflowing tile viewport, including on a tile, and moves horizontally past the drag threshold
- **THEN** the tiles follow the horizontal drag within the scroll bounds
- **AND** releasing the button ends the drag without inserting a symbol or opening the `Custom matrix` dialog

#### Scenario: Clicking a tile preserves its existing action
- **WHEN** the user clicks and releases on a tile without starting a drag
- **THEN** the tile performs its normal action

#### Scenario: A category that fits uses the full palette width
- **WHEN** all tiles in the active category fit within the available palette width, including after a resize or category switch
- **THEN** the tile viewport uses the full width and the horizontal offset is zero
- **AND** dragging or using the mouse wheel over the palette leaves its position unchanged
