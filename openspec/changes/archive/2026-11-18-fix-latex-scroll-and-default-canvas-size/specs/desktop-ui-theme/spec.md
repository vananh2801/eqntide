## ADDED Requirements

### Requirement: Bounded vertically scrollable LaTeX source box
The desktop application SHALL render the LaTeX source editor inside a fixed-height container (`88px` panel / `80px` input height below the `8px` top gap) with internal vertical scrolling so multi-line LaTeX input never overflows into the Equation Canvas or clips beneath the footer.

#### Scenario: Entering multiple newlines keeps LaTeX source bounded and scrollable
- **WHEN** the user presses `Enter` multiple times inside the LaTeX source box so the text exceeds the visible rows
- **THEN** the LaTeX source container keeps its fixed height without overlapping the Equation Canvas above or clipping beneath the footer below, and scrolls vertically to keep the active lines accessible

#### Scenario: LaTeX source scrollbar adapts to the active canvas preset
- **WHEN** the user enters multi-line content in the LaTeX source box under any Light or Dark mode canvas preset
- **THEN** the LaTeX source container and its vertical scrollbar use the active canvas preset colors for legible contrast

### Requirement: Compact default window and Equation Canvas sizing
The desktop application SHALL launch with a default window inner size of `1040x560`, matching the minimum window size so the Equation Canvas opens at a compact, balanced height while preserving full sidebar and three-row palette visibility.

#### Scenario: Default launch opens at 1040x560 with balanced canvas height
- **WHEN** the desktop application opens with its default viewport configuration
- **THEN** the window inner size is `1040x560`, the LaTeX source panel reserves its increased fixed height, and the Equation Canvas fills the remaining central area without excessive empty vertical space
