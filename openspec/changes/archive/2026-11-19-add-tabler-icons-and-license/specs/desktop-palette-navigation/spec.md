## MODIFIED Requirements

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
- **THEN** all 12 category tiles (`Recent` using `history`, `Fractions` using `math-x-divide-y` + `square-root-2`, `Scripts` using `x-power-y`, `Fences` using `parentheses`, `Integrals` using `math-integral`, `SumsAndBigOps` using `sum`, `MatricesAndBoxes` using `matrix`, `AccentsAndBars` using `overline` + `underline`, `OperatorsAndRelations` using `math-equal-lower`, `Arrows` using `arrows-left-right`, `LogicSetsSymbols` using `math-avg`, and `Greek` using `alpha` + `omega`) render crisp Tabler icons at a uniform `22x22` size centered inside each `72x42` tile without clipping or distortion

### Requirement: Uniform palette tile dimensions and icon scale
Every button in the top 3-row palette SHALL share a uniform `46x32` tile size with `4px` spacing and render its math preview at a consistent base em size within a `40x26` icon budget while fitting all 49 `Symbols` items without clipping at `1056x560`.

#### Scenario: Palette tiles remain uniformly sized and unclipped at minimum width
- **WHEN** the `Symbols` category or any palette category is displayed at `1056x560`
- **THEN** all palette buttons share identical `46x32` dimensions, fit within three rows, and remain unclipped with uniform glyph proportions

### Requirement: Category icon tooltips and accessibility labels
Each category icon button in the sidebar SHALL render its assigned Tabler category icon graphic and expose a descriptive category name via hover tooltip and accessibility label.

#### Scenario: Hovering over a category icon reveals its category name
- **WHEN** the user hovers the pointer over any category icon button in the sidebar
- **THEN** a tooltip displays the category's human-readable name
