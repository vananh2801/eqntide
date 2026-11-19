# Spec Delta

## ADDED Requirements

### Requirement: Vertically centered Settings Canvas zoom row
The Settings **Canvas zoom** row SHALL vertically center the zoom slider, the percentage `DragValue` control, and the `Reset (200%)` button along the `30px` standard row axis without enlarging the slider handle above its standard track-proportioned size.

#### Scenario: Slider, percentage, and Reset button share the same vertical center
- **WHEN** the user opens the **Settings** view in either Light mode or Dark mode
- **THEN** the zoom slider rail and handle, the percentage `DragValue` box, and the `Reset (200%)` button align on a shared horizontal centerline within the row

### Requirement: Accent-matched slider handle border
The zoom slider's rounded-square handle SHALL render its border using the theme's accent color so the handle outline matches the active left-side slider fill across idle, hovered, and active states in both Light and Dark modes.

#### Scenario: Slider handle border matches the left accent fill in Light and Dark modes
- **WHEN** the zoom slider renders in either Light mode or Dark mode across idle, hovered, or dragged states
- **THEN** the rounded-square handle's border stroke uses the same theme accent color as the filled left portion of the slider rail
