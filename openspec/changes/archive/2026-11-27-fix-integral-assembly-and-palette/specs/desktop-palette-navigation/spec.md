# Desktop Palette Navigation Delta

## ADDED Requirements

### Requirement: Directional contour integral icons match their commands
The Integrals palette SHALL depict the correct loop direction in each standalone and limit-bearing clockwise and anticlockwise contour integral icon. Each tile SHALL insert the template indicated by its icon and label.

#### Scenario: Direction is legible across all six variants
- **WHEN** the Integrals palette displays the standalone, side-lower, and stacked-lower forms of `\varointclockwise` and `\ointctrclockwise`
- **THEN** each clockwise icon shows clockwise travel and each anticlockwise icon shows anticlockwise travel, with the arrowheads discernible from the stems at the displayed `40x26` size
- **AND** activating a tile inserts the corresponding directional operator with the limit placement shown on the tile
