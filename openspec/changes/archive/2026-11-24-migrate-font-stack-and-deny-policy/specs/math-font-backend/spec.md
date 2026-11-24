# math-font-backend Specification

## Purpose

Ensures bundled STIX font loading, OpenType `MATH` and `GSUB` metadata extraction, glyph selection, outline extraction, and raster/vector equation rendering rely exclusively on actively maintained font-parsing crates without changing rendered math layout or export behavior.

## ADDED Requirements

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
