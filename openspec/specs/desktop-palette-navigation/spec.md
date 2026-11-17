# desktop-palette-navigation Specification

## Purpose

Provides fast, low-friction category and symbol navigation in the desktop equation editor through a compact math-icon sidebar grid and wheel-driven horizontal palette scrolling.

## Requirements

### Requirement: Compact math-icon category grid in sidebar
The desktop sidebar SHALL display consolidated symbol and template categories in a 2-column grid of rendered mathematical icons so all categories remain visible without vertical scrolling at the minimum supported window size (`1040x560`).

#### Scenario: All categories and navigation buttons fit without vertical scrolling
- **WHEN** the editor view is displayed at the minimum window size of `1040x560`
- **THEN** every category icon button and the bottom `Batch DOCX`, `Settings`, and `About` buttons are visible inside the sidebar without requiring vertical scroll

#### Scenario: Selecting a consolidated category displays its mapped items
- **WHEN** the user clicks a category icon button in the sidebar
- **THEN** that category button becomes selected and the top 3-row palette displays all symbols and templates belonging to the underlying catalog groups mapped to that category

### Requirement: Category icon tooltips and accessibility labels
Each category icon button in the sidebar SHALL render a mathematical symbol or template graphic and expose a descriptive category name via hover tooltip and accessibility label.

#### Scenario: Hovering over a category icon reveals its category name
- **WHEN** the user hovers the pointer over any category icon button in the sidebar
- **THEN** a tooltip displays the category's human-readable name

### Requirement: Recent category tracking
The desktop sidebar SHALL include a Recent category that records palette items inserted by the user in most-recently-used order without duplicates and persists those valid items across application restarts.

#### Scenario: Inserting a palette item adds it to the front of Recent
- **WHEN** the user clicks a symbol or template button in the palette
- **THEN** that item is placed at the front of the Recent category list without creating duplicate entries

#### Scenario: App launches on a populated default category
- **WHEN** the desktop application starts a new session with no recent history
- **THEN** the initially active category is a populated structural category rather than an empty Recent list

#### Scenario: Recent palette items persist across restarts
- **WHEN** the user inserts palette items and restarts the desktop application
- **THEN** the Recent category restores the previously recorded valid palette items in most-recently-used order

### Requirement: Matrix controls in consolidated matrix category
Selecting the consolidated category that contains matrix templates SHALL expose both the custom matrix dialog button and the contextual matrix row and column adjustment controls.

#### Scenario: Consolidated matrix category preserves matrix tools
- **WHEN** the user selects the category containing matrix templates
- **THEN** the palette includes the custom matrix (`⋯`) button and the `+ row`, `− row`, `+ col`, `− col` controls appear above the equation canvas

### Requirement: Mouse-wheel horizontal scrolling on palette
The top 3-row palette SHALL translate vertical mouse-wheel input into horizontal scrolling whenever the pointer hovers over the palette and the category's items overflow the visible palette width.

#### Scenario: Vertical mouse-wheel scrolls overflowing palette horizontally
- **WHEN** the pointer is over the palette for a category whose 3-row layout exceeds the available palette width and the user scrolls the mouse wheel vertically
- **THEN** the palette pans horizontally while maintaining a maximum of 3 rows and zero vertical scroll offset

#### Scenario: Palette within viewport width ignores wheel overflow scrolling
- **WHEN** all items in the active category fit within the visible palette width and the user scrolls the mouse wheel vertically over the palette
- **THEN** the palette remains at horizontal offset zero and does not scroll vertically
