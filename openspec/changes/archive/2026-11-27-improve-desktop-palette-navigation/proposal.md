# Proposal

## Why

The palette reserves extra height for its scrollbar only when a category overflows, moving the canvas when users switch categories. The current row-first layout can also move items to different rows when the window is resized.

## What Changes

- Keep the palette at a fixed three-row height and fill each column from top to bottom in a stable order that keeps related items together.
- Remove the palette scrollbar and use the full available width for tiles.
- Allow horizontal dragging from the tile area, including from a tile, and preserve vertical wheel input for horizontal scrolling. A drag cancels tile activation.

## Capabilities

### New Capabilities

### Modified Capabilities

- `desktop-palette-navigation` covers column ordering and horizontal navigation using dragging and the wheel.
- `desktop-ui-theme` specifies the palette's fixed height and its exception to the desktop scrollbar rule.

## Impact

The change affects the desktop editor palette, its input handling, and layout and input tests. Other desktop scroll areas continue to follow the existing scrollbar rule.
