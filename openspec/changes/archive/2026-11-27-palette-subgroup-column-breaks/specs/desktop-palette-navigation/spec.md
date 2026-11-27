## MODIFIED Requirements

### Requirement: Matrix controls in consolidated matrix category
Selecting the consolidated category that contains matrix templates SHALL expose both the custom matrix dialog button (rendered with a Tabler `dots` icon, `"Custom"` label, and `"Custom matrix"` tooltip/accessibility label) and the contextual matrix row and column adjustment controls inside a trailing tool block at the end of the `MatricesAndBoxes` palette.

#### Scenario: Consolidated matrix category preserves matrix tools
- **WHEN** the user selects the category containing matrix templates
- **THEN** the trailing matrix tool block at the end of the `MatricesAndBoxes` palette includes the `104x40` `Custom` button displaying the Tabler `dots` icon (with `"Custom matrix"` hover tooltip and accessibility label) above a `2×2` grid of `50x28` `+ row`, `− row`, `+ col`, and `− col` controls, without placing a separate matrix action bar above the equation canvas

### Requirement: Uniform palette tile dimensions and icon scale
Every catalog item button in the top 3-row palette SHALL share a uniform `46x32` tile size with `4px` spacing and render a Tabler-consistent, visually centered `40x26` icon from pre-rasterized SVG artwork covering all catalog entries—combining monoline geometric strokes for structural templates and mathematical symbols (matching canvas `STIXTwoMath` glyph shapes such as `∠`, `∡`, and `∢`), circular-arc parentheses, clear arrowheads across both single- and double-shaft arrows, thin muted solid-border `□` slot boxes, full hollow-square matrix grids across all dimensions including `3x3` and `4x4`, baseline-aligned `Nunito` `<path>` letterforms for `Functions`, and `STIXTwoText`/`STIXTwoMath` `<path>` letterforms for differentials, `Greek`, and calligraphic/fraktur/blackboard `Symbols`—without clipping at `1056x560` or across DPI changes.

#### Scenario: Palette tiles remain uniformly sized and unclipped at minimum width
- **WHEN** any palette category is displayed at `1056x560`
- **THEN** all catalog item buttons share identical `46x32` dimensions, render `40x26` Tabler-consistent SVG-derived icons tinted with the active theme's icon color matching their rendered canvas glyphs, fit within three rows, are centered inside each `40x26` frame (preserving typographic baseline alignment for `Functions` and baseline `ldots`), and remain unclipped with uniform stroke weight across simple symbols and multi-level templates

#### Scenario: Matrix palette buttons render hollow square grids at all catalog dimensions
- **WHEN** the user selects `MatricesAndBoxes`
- **THEN** every matrix button (`1x2` through `4x4`, including `3x3` and `4x4`) displays hollow square slot boxes rather than collapsing cells into dots, while keeping clear cell and fence spacing inside `40x26`

### Requirement: Three-row palette layout in column order
The palette SHALL reserve the same height for three rows regardless of item count, category, or horizontal overflow. In structured categories, it SHALL organize items into semantic subgroups, fill each subgroup's columns from top to bottom before starting the next column to its right, begin each new subgroup (and any explicit intra-subgroup column run) at the top row of a fresh column leaving unused bottom slots empty, and separate adjacent subgroups with wider horizontal spacing than intra-subgroup columns without vertical divider lines. In `Recent`, items SHALL fill columns continuously without subgroup breaks, and `MatricesAndBoxes` SHALL place a trailing matrix tool block following the last catalog subgroup with a larger `Custom` button on top and the `2×2` `+ row` / `− row` / `+ col` / `− col` controls below it inside the three-row height.

#### Scenario: Items keep their row and column when the window resizes
- **WHEN** the window is resized while the active category's ordered item list stays the same
- **THEN** each item remains in its assigned subgroup column and row, and resizing changes only the visible portion of the palette without rearranging items

#### Scenario: Subgroups with counts not divisible by three start the next subgroup at row zero
- **WHEN** the user selects a structured category containing subgroups whose item counts are not divisible by three (such as 5-variant operator families in `SumsAndBigOps`, 2-item pairs in `MatricesAndBoxes` and `AccentsAndBars`, or paired 2-row box columns in `MatricesAndBoxes`)
- **THEN** a 2-item subgroup occupies the top two rows with the third row empty, paired 2-row columns inside one subgroup sit in adjacent columns with intra-subgroup spacing (`4px`) and empty third rows, a 5-item subgroup occupies three rows in its first column and the top two rows in its second column, and the following subgroup begins at the top row of the next column separated by a wider horizontal gap (`12px`) without a divider line

#### Scenario: Matrix tool block starts a new column after the last catalog subgroup
- **WHEN** the user selects `MatricesAndBoxes`
- **THEN** the trailing matrix tool block sits after the last `Boxes` subgroup separated by the inter-subgroup gap, with `Custom` (`Custom matrix`) as a larger top button (`40px` tall) above a `2×2` grid of `+ row`, `− row`, `+ col`, and `− col` controls (`28px` tall each) fitting cleanly within the three-row palette height

#### Scenario: Recent starts with the most recently inserted item
- **WHEN** the user opens a populated `Recent` category
- **THEN** the most recently inserted item occupies the top-left slot and older items follow continuously in three-row column order without subgroup breaks
