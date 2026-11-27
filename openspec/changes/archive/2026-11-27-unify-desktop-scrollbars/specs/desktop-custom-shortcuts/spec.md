# Spec Delta

## MODIFIED Requirements

### Requirement: Always-visible fixed-width solid scrollbar without button overlap
The **Palette Shortcuts** list SHALL show a fixed-width vertical scrollbar whenever its rows overflow. While visible, the scrollbar SHALL reserve dedicated space clear of the row action controls. The list SHALL isolate vertical scrolling at its boundaries.

#### Scenario: Scrollbar stays visible at fixed width and never overlaps row buttons
- **WHEN** the **Palette Shortcuts** list overflows and the pointer is outside the list or hovering over the scrollbar
- **THEN** the solid scrollbar remains visible at a constant width of 8 logical pixels with a 10-logical-pixel gap to the content, clear of the shortcut capture and clear (`×`) buttons

#### Scenario: A filtered list that fits hides its scrollbar
- **WHEN** a search or filter leaves rows that fit in the **Palette Shortcuts** list
- **THEN** the scrollbar is hidden and the rows can use the space on the right

#### Scenario: Scrolling past the top or bottom of the right list does not scroll Settings
- **WHEN** the pointer is inside the right **Palette Shortcuts** container and the user scrolls up at the top or down at the bottom
- **THEN** scrolling stays inside the list at its boundary, while wheeling over the left column still scrolls **Settings**
