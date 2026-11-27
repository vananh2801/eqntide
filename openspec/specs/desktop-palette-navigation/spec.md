# desktop-palette-navigation Specification

## Purpose

Provides fast, low-friction category and symbol navigation in the desktop equation editor through a compact math-icon sidebar grid and wheel-driven horizontal palette scrolling.

## Requirements

### Requirement: Compact math-icon category grid in sidebar
The desktop sidebar SHALL display consolidated symbol and template categories in a 2-column grid of uniformly sized tiles with Tabler category icons so all categories remain visible without vertical scrolling at the minimum supported window size (`1056x560`).

#### Scenario: All categories and navigation buttons fit without vertical scrolling
- **WHEN** the editor view is displayed at the minimum window size of `1056x560`
- **THEN** every category icon button and the bottom `Batch DOCX`, `Settings`, and `About` buttons are visible inside the sidebar without requiring vertical scroll

#### Scenario: Selecting a consolidated category displays its mapped items
- **WHEN** the user clicks a category icon button in the sidebar
- **THEN** that category button becomes selected and the top 3-row palette displays all symbols and templates belonging to the underlying catalog groups mapped to that category

#### Scenario: Sidebar category icons share a uniform base typographic scale
- **WHEN** the sidebar category grid renders across any supported DPI
- **THEN** all 14 category tiles (`Recent` using `history`, `Fractions` using `math-x-divide-y` + `square-root-2`, `Scripts` using `x-power-y`, `Fences` using `parentheses`, `Integrals` using `math-integral`, `SumsAndBigOps` using `sum`, `MatricesAndBoxes` using `matrix`, `AccentsAndBars` using `overline` + `underline`, `Functions` using `math-sin`, `OperatorsAndRelations` using `math-equal-lower`, `Arrows` using `arrows-left-right`, `LabeledArrows` using `arrow-autofit-right`, `LogicSetsSymbols` using `math-avg`, and `Greek` using `alpha` + `omega`) render crisp Tabler icons at a uniform `22x22` size centered inside each `72x42` tile without clipping or distortion

### Requirement: Dedicated Functions category and split Arrow categories
The desktop palette SHALL organize mathematical functions into a dedicated `Functions` category (including `\lim`, `\min`, and `\max` templates moved out of `Scripts`) and separate single-glyph arrow symbols (`Arrows`) from placeholder-bearing arrow templates (`LabeledArrows`).

#### Scenario: Functions category contains moved limit/extremum templates and slot-free functions
- **WHEN** the user selects the `Functions` category in the sidebar
- **THEN** the palette displays the placeholder-bearing `\lim`, `\min`, and `\max` templates alongside plain, slot-free named mathematical functions (`sin`, `cos`, `tan`, `log`, `ln`, `exp`, trigonometric, inverse/hyperbolic, and algebraic operators) while `Scripts` no longer lists `\lim`, `\min`, or `\max`

#### Scenario: Plain function buttons allow immediate subsequent script attachment
- **WHEN** the user clicks a slot-free function button such as `sin` or `log` in the `Functions` palette
- **THEN** the upright function token is inserted without wrapping argument placeholders, allowing immediate superscript or subscript attachment such as `\sin^2` or `\log_2`

#### Scenario: Single arrows and labeled arrows appear in separate sidebar categories
- **WHEN** the user switches between the `Arrows` (`arrows-left-right`) and `LabeledArrows` (`arrow-autofit-right`) sidebar tiles
- **THEN** `Arrows` displays only standalone arrow symbol buttons without placeholder boxes, and `LabeledArrows` displays only arrow templates that include top and/or bottom placeholder boxes

### Requirement: Category icon tooltips and accessibility labels
Each category icon button in the sidebar SHALL render its assigned Tabler category icon graphic and expose a descriptive category name via hover tooltip and accessibility label.

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
Selecting the consolidated category that contains matrix templates SHALL expose both the custom matrix dialog button (rendered with a Tabler `dots` icon and `"Custom matrix"` tooltip/accessibility label) and the contextual matrix row and column adjustment controls.

#### Scenario: Consolidated matrix category preserves matrix tools
- **WHEN** the user selects the category containing matrix templates
- **THEN** the palette includes the `46x32` custom matrix button displaying the Tabler `dots` icon (with `"Custom matrix"` hover tooltip) and the `+ row`, `− row`, `+ col`, `− col` controls appear above the equation canvas

### Requirement: Mouse-wheel horizontal scrolling on palette
The top 3-row palette SHALL translate vertical mouse-wheel input into horizontal scrolling whenever the pointer hovers over the palette and the category's items overflow the visible palette width.

#### Scenario: Vertical mouse-wheel scrolls overflowing palette horizontally
- **WHEN** the pointer is over the palette for a category whose 3-row layout exceeds the available palette width and the user scrolls the mouse wheel vertically
- **THEN** the palette pans horizontally while maintaining a maximum of 3 rows and zero vertical scroll offset

#### Scenario: Palette within viewport width ignores wheel overflow scrolling
- **WHEN** all items in the active category fit within the visible palette width and the user scrolls the mouse wheel vertically over the palette
- **THEN** the palette remains at horizontal offset zero and does not scroll vertically

### Requirement: Uniform palette tile dimensions and icon scale
Every button in the top 3-row palette SHALL share a uniform `46x32` tile size with `4px` spacing and render a Tabler-consistent, visually centered `40x26` icon from pre-rasterized SVG artwork covering all catalog entries—combining monoline geometric strokes for structural templates and mathematical symbols, circular-arc parentheses, clear arrowheads across both single- and double-shaft arrows, thin muted solid-border `□` slot boxes, full hollow-square matrix grids across all dimensions including `3x3` and `4x4`, baseline-aligned `Nunito` `<path>` letterforms for `Functions`, and `STIXTwoText`/`STIXTwoMath` `<path>` letterforms for differentials, `Greek`, and calligraphic/fraktur/blackboard `Symbols`—without clipping at `1056x560` or across DPI changes.

#### Scenario: Palette tiles remain uniformly sized and unclipped at minimum width
- **WHEN** any palette category is displayed at `1056x560`
- **THEN** all palette buttons share identical `46x32` dimensions, render `40x26` Tabler-consistent SVG-derived icons tinted with the active theme's icon color, fit within three rows, are centered inside each `40x26` frame (preserving typographic baseline alignment for `Functions` and baseline `ldots`), and remain unclipped with uniform stroke weight across simple symbols and multi-level templates

#### Scenario: Matrix palette buttons render hollow square grids at all catalog dimensions
- **WHEN** the user selects `MatricesAndBoxes`
- **THEN** every matrix button (`1x2` through `4x4`, including `3x3` and `4x4`) displays hollow square slot boxes rather than collapsing cells into dots, while keeping clear cell and fence spacing inside `40x26`

### Requirement: Deterministic synchronization between palette catalog and generated SVG icons
Every `ENTRIES` item defined in `src/catalog/mod.rs` SHALL have a corresponding generated `40x26` SVG asset and entry in the committed palette sprite atlas, verified by automated icon checks.

#### Scenario: Icon verification detects missing or stale palette SVGs or atlas data
- **WHEN** the icon verification test or script runs after catalog or SVG changes
- **THEN** it verifies a 1-to-1 match between all catalog `ENTRIES` IDs/sources, committed `assets/icons/sources/palette/*.svg` files, and the pre-rasterized sprite atlas and UV manifest under `assets/icons/generated/palette-atlas/`

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
