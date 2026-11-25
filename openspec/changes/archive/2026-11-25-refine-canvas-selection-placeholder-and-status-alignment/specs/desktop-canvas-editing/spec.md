# Spec Delta

## ADDED Requirements

### Requirement: Contiguous span-based selection highlight on the Equation Canvas
When an active selection exists on the Equation Canvas, the desktop application SHALL paint a single contiguous selection highlight with light `2px` corner rounding from the normalized selection span, aligned to the math baseline line-height, and SHALL exclude adjacent non-empty characters whose leading edge has not been crossed.

#### Scenario: Selecting multiple sibling elements renders a single continuous highlight
- **WHEN** the user selects across multiple adjacent nodes in a row or container (such as `\frac{1}{2}x^2 + 2x - 1 = 0`) via drag, `Shift+Arrow`, or `Ctrl+A`
- **THEN** the Equation Canvas paints one continuous lightly rounded (`2px`) selection rectangle spanning from the left edge of the first selected element to the right edge of the last selected element without inter-symbol gaps or stepped per-node tops and bottoms

#### Scenario: Selecting a thin operator or partial token preserves baseline alignment and character bounds
- **WHEN** the user selects a thin operator alone (such as `-` or `=`) or a character slice inside a multi-character token (such as `ab` inside `<mi>abc</mi>` or `<mtext>`)
- **THEN** a thin operator selection spans the active baseline line-height around the baseline rather than hanging downward from the glyph stroke, and a partial token selection spans horizontally from the start character edge to the end character edge

#### Scenario: Drag selection before a character's horizontal midpoint does not include that character
- **WHEN** the user press-drags across adjacent terms and stops within the left half of a character so the caret sits at its leading edge in front of the character
- **THEN** the untouched character is excluded from the selected range and the selection highlight ends before that character until the pointer crosses its horizontal midpoint

### Requirement: Stable caret alignment at selection edges and inter-token gaps
When a drag selection ends on an outer boundary adjacent to inter-element spacing, the Equation Canvas SHALL keep the caret bar aligned to the moving edge of the selection highlight, and standalone clicks in an inter-token spacing gap SHALL snap to the closer adjacent boundary across the gap midpoint.

#### Scenario: Caret bar stays locked to the active selection edge across inter-element spacing
- **WHEN** the user moves the pointer across the right edge of a selected token, into the inter-element spacing gap, or into the leading half of the next token while the active selection span remains unchanged
- **THEN** the caret bar stays locked to the moving edge of the selection highlight (right edge when dragging forward, left edge when dragging backward) rather than jumping across the spacing gap

#### Scenario: Clicking in an inter-token gap picks the closer adjacent boundary
- **WHEN** the user clicks in the empty spacing gap between two adjacent tokens with no active selection
- **THEN** clicks to the left of the gap midpoint place the caret at the trailing edge of the preceding token, and clicks at or to the right of the midpoint place it before the following token
