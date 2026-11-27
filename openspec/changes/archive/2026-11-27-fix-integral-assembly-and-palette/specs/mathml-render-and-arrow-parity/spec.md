# MathML Render and Arrow Parity Delta

## ADDED Requirements

### Requirement: Directional contour integral assembly
The shared renderer SHALL assemble clockwise and anticlockwise contour integrals from vertical body pieces and a separate directional ring. The ring SHALL remain centered on the math axis as the body height changes.

#### Scenario: Plain and directional contour integrals remain distinguishable
- **WHEN** `\oint`, `\varointclockwise`, and `\ointctrclockwise` are rendered together on the canvas or in SVG, PNG, or EMF previews
- **THEN** the plain loop has no arrowhead and the other loops indicate clockwise and anticlockwise travel respectively
- **AND** the hollow areas of the loops remain visible around the integral stem

#### Scenario: Direction and limit placement survive assembly
- **WHEN** either directional contour integral is rendered with an explicit side or stacked limit
- **THEN** the assembled sign shows its direction and the limit remains at the requested placement

#### Scenario: Extended integral bodies maintain continuous stems
- **WHEN** an integral sign is assembled at a height that requires vertical extenders
- **THEN** the extenders meet the body pieces with continuous stem width and the joins have no visible gaps

### Requirement: Contour integral ink fits its layout box
The layout engine SHALL reserve horizontal space for the full contour integral ink, including the loop and any arrowhead. The canvas operator hit region SHALL cover that ink.

#### Scenario: Wide contour loops fit their allocated space
- **WHEN** double and triple contour integrals are laid out next to other operators or operands
- **THEN** the loops fit their own operator boxes and do not consume space allocated to adjacent nodes
