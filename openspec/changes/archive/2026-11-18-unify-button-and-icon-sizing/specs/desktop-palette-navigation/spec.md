# Spec Delta

## MODIFIED Requirements

### Requirement: Compact math-icon category grid in sidebar
The desktop sidebar SHALL display consolidated symbol and template categories in a 2-column grid of uniformly sized tiles with consistent base-em rendered mathematical icons so all categories remain visible without vertical scrolling at the minimum supported window size (`1040x560`).

#### Scenario: All categories and navigation buttons fit without vertical scrolling
- **WHEN** the editor view is displayed at the minimum window size of `1040x560`
- **THEN** every category icon button and the bottom `Batch DOCX`, `Settings`, and `About` buttons are visible inside the sidebar without requiring vertical scroll

#### Scenario: Selecting a consolidated category displays its mapped items
- **WHEN** the user clicks a category icon button in the sidebar
- **THEN** that category button becomes selected and the top 3-row palette displays all symbols and templates belonging to the underlying catalog groups mapped to that category

#### Scenario: Sidebar category icons share a uniform base typographic scale
- **WHEN** the sidebar category grid renders across any supported DPI
- **THEN** single-line category icons such as `★`, `± ≤`, `∀ ∈`, and `αΩ` share the same base em size without upscaling distortion while multi-tier icons fit cleanly inside each `64x42` tile

## ADDED Requirements

### Requirement: Uniform palette tile dimensions and icon scale
Every button in the top 3-row palette SHALL share a uniform `46x32` tile size with `4px` spacing and render its math preview at a consistent base em size within a `40x26` icon budget while fitting all 49 `Symbols` items without clipping at `1040x560`.

#### Scenario: Palette tiles remain uniformly sized and unclipped at minimum width
- **WHEN** the `Symbols` category or any palette category is displayed at `1040x560`
- **THEN** all palette buttons share identical `46x32` dimensions, fit within three rows, and remain unclipped with uniform glyph proportions
