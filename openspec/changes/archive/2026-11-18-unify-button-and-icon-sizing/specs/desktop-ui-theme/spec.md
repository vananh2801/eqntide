# Spec Delta

## ADDED Requirements

### Requirement: Uniform button typography and height tiers
The desktop application SHALL define explicit UI text styles and standardize interactive text buttons into consistent height tiers (`30px` standard buttons including footer `Save`, `Undo`/`Redo`, and `DragValue`; `34px` sidebar navigation tabs; and `24px` compact inline action buttons with non-zero vertical padding) so adjacent controls share matching font size and balanced visual proportions.

#### Scenario: Footer controls and Save button align at standard button height
- **WHEN** the editor footer is rendered in either standalone or Office mode
- **THEN** the `Save`, `Undo`, `Redo`, and font-size `DragValue` controls share the `30px` standard control height and consistent button font sizing

#### Scenario: Compact inline controls avoid collapsed zero-padding text buttons
- **WHEN** contextual matrix controls (`+ row`, `− row`, `+ col`, `− col`) or status bar action buttons (`Retry`, `Next session`) are displayed
- **THEN** they render at the `24px` compact button height with balanced vertical padding and the same UI text family scale

### Requirement: Consistent typographic scale for math button icons
Mathematically rendered icons inside sidebar category tiles and palette buttons SHALL render at a capped base em size rather than upscaling short symbols to fill the bounding box, only scaling down proportionally when an expression exceeds its tile bounds.

#### Scenario: Short symbols and standard templates share the same base glyph size
- **WHEN** sidebar category icons or palette icons such as `★`, `αΩ`, `∀ ∈`, `□^□`, `max_□`, and `min_□` are rendered
- **THEN** symbols that fit within the tile bounds at the base em size are rasterized at that exact shared em size instead of being stretched to different heights
