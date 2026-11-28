# math-font-backend Specification

## Purpose

Ensures bundled STIX font loading, OpenType `MATH` and `GSUB` metadata extraction, glyph selection, outline extraction, and raster/vector equation rendering rely exclusively on actively maintained font-parsing crates without changing rendered math layout or export behavior.

## Requirements

### Requirement: Actively maintained bundled font parsing backend
The core rendering module SHALL load the bundled STIX Two Text and STIX Two Math fonts, extract OpenType `MATH` and `GSUB` `ssty` tables, resolve glyph indices and metrics, and extract glyph outlines using actively maintained font-parsing crates (`skrifa` / `read-fonts`) without depending on `ttf-parser` or `ab_glyph` (`owned_ttf_parser`).

#### Scenario: Bundled font metrics and OpenType MATH tables remain invariant after backend migration
- **WHEN** the bundled STIX font set is initialized via `fonts()`
- **THEN** OpenType `MATH` constants (`axis_height`, `fraction_rule_thickness`, `script_percent_scale_down`, radical constants), `radical_info` variants and assembly, `italic_correction`, `top_accent_attachment`, `math_kern`, and the 529 `ssty` optical-size alternate mappings MUST match the expected STIX Two Math values

### Requirement: Glyph outline, bounding box, and raster/vector rendering parity
The font backend SHALL extract glyph bounding boxes (`math_glyph_bbox`), pixel-space drawing segments (`Seg`), flattened contour polygons (`glyph_polys`, `math_glyph_polys`), glyph bounds (`ink`, `ink_x`), and pixel coverage (`draw_styled`) in screen coordinates (origin at baseline, y-axis pointing downward) with parity across SVG, PNG, and EMF outputs.

#### Scenario: Optical-size glyph outlines, bounding boxes, and SVG/PNG/EMF renders stay valid
- **WHEN** standard and `ssty`-substituted math glyphs (including line, quadratic, and cubic outline curves) are rendered or queried for ink bounds and polygons
- **THEN** `glyph_segments_styled`, `glyph_polys`, `math_glyph_polys`, `math_glyph_bbox`, `ink`, `ink_x`, and `draw_styled` MUST return non-empty, correctly oriented coordinates and valid SVG, PNG, and EMF outputs without panicking

### Requirement: Extended OpenType MATH layout constants
`MathConstants` extracted from `STIXTwoMath-Regular.ttf` SHALL also expose OpenType `MATH` constants for script drops/bounds, large operator limit gaps and minimum heights, accent base height, overbar/underbar metrics, and skewed fraction gaps with matching `Default` fallbacks and without altering existing `MathConstants` values.

#### Scenario: Extended MATH constants match STIXTwoMath-Regular.ttf
- **WHEN** `fonts()` initializes `MathConstants` from the bundled `STIXTwoMath-Regular.ttf`
- **THEN** existing fields keep their exact prior values and newly added fields (`subscript_top_max`, `subscript_baseline_drop_min`, `superscript_bottom_min`, `superscript_baseline_drop_max`, `superscript_bottom_max_with_subscript`, `space_after_script`, `display_operator_min_height`, `upper_limit_gap_min`, `upper_limit_baseline_rise_min`, `lower_limit_gap_min`, `lower_limit_baseline_drop_min`, `accent_base_height`, `overbar_vertical_gap`, `overbar_rule_thickness`, `overbar_extra_ascender`, `underbar_vertical_gap`, `underbar_rule_thickness`, `underbar_extra_descender`, `skewed_fraction_horizontal_gap`, `skewed_fraction_vertical_gap`) match the `MATH` table normalized by `units_per_em`
