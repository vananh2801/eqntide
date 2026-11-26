# Spec Delta: desktop-palette-navigation

## MODIFIED Requirements

### Requirement: Matrix controls in consolidated matrix category
Selecting the consolidated category that contains matrix templates SHALL expose both the custom matrix dialog button (rendered with a Tabler `dots` icon and `"Custom matrix"` tooltip/accessibility label) and the contextual matrix row and column adjustment controls.

#### Scenario: Consolidated matrix category preserves matrix tools
- **WHEN** the user selects the category containing matrix templates
- **THEN** the palette includes the `46x32` custom matrix button displaying the Tabler `dots` icon (with `"Custom matrix"` hover tooltip) and the `+ row`, `− row`, `+ col`, `− col` controls appear above the equation canvas

### Requirement: Uniform palette tile dimensions and icon scale
Every button in the top 3-row palette SHALL share a uniform `46x32` tile size with `4px` spacing and render a Tabler-consistent, visually centered `40x26` icon from pre-rasterized SVG artwork covering all catalog entries—combining monoline geometric strokes for structural templates and mathematical symbols, circular-arc parentheses, clear arrowheads across both single- and double-shaft arrows, thin muted solid-border `□` slot boxes, full hollow-square matrix grids across all dimensions including `3x3` and `4x4`, baseline-aligned `Nunito` `<path>` letterforms for `Functions`, and `STIXTwoText`/`STIXTwoMath` `<path>` letterforms for differentials, `Greek`, and calligraphic/fraktur/blackboard `Symbols`—without clipping at `1056x560` or across DPI changes.

#### Scenario: Palette tiles remain uniformly sized and unclipped at minimum width
- **WHEN** any palette category is displayed at `1056x560`
- **THEN** all palette buttons share identical `46x32` dimensions, render `40x26` Tabler-consistent SVG-derived icons tinted with the active theme's icon color, fit within three rows, are centered inside each `40x26` frame (preserving typographic baseline alignment for `Functions` and baseline `ldots`), and remain unclipped with uniform stroke weight across simple symbols and multi-level templates

#### Scenario: Matrix palette buttons render hollow square grids at all catalog dimensions
- **WHEN** the user selects `MatricesAndBoxes`
- **THEN** every matrix button (`1x2` through `4x4`, including `3x3` and `4x4`) displays hollow square slot boxes rather than collapsing cells into dots, while keeping clear cell and fence spacing inside `40x26`

## ADDED Requirements

### Requirement: Deterministic synchronization between palette catalog and generated SVG icons
Every `ENTRIES` item defined in `src/catalog/mod.rs` SHALL have a corresponding generated `40x26` SVG asset and entry in the committed palette sprite atlas, verified by automated icon checks.

#### Scenario: Icon verification detects missing or stale palette SVGs or atlas data
- **WHEN** the icon verification test or script runs after catalog or SVG changes
- **THEN** it verifies a 1-to-1 match between all catalog `ENTRIES` IDs/sources, committed `assets/icons/palette/*.svg` files, and the pre-rasterized sprite atlas and UV manifest
