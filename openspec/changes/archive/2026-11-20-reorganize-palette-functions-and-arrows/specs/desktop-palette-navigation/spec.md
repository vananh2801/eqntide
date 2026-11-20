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
- **THEN** all 14 category tiles (`Recent` using `history`, `Fractions` using `math-x-divide-y` + `square-root-2`, `Scripts` using `x-power-y`, `Fences` using `parentheses`, `Integrals` using `math-integral`, `SumsAndBigOps` using `sum`, `MatricesAndBoxes` using `matrix`, `AccentsAndBars` using `overline` + `underline`, `Functions` using `math-sin`, `OperatorsAndRelations` using `math-equal-lower`, `Arrows` using `arrows-left-right`, `LabeledArrows` using `arrow-autofit-right`, `LogicSetsSymbols` using `math-avg`, and `Greek` using `alpha` + `omega`) render crisp Tabler icons at a uniform `22x22` size centered inside each `72x42` tile without clipping or distortion

## ADDED Requirements

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
