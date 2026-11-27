# Design

## Context

The shared layout code builds each integral stem from STIX glyph pieces. The same layout supplies desktop hit geometry and PNG, SVG, and EMF previews. The Integrals palette has a separate SVG builder and a pre-rasterized atlas. The desktop tile resolves both its image and its insertion command from the catalog LaTeX source.

## Goals / Non-Goals

- Keep the stem and its directional ring independently positioned so extending the stem preserves the shape of the arrowhead.
- Reuse the current layout primitives, limit placement, and atlas pipeline.
- See the proposal for the scope of integral size policy and catalog coverage.

## Decisions

### Assemble the body and ring independently

Represent the loop as plain, clockwise, or anticlockwise. The plain contour family keeps its ellipse and stem count. The directional pair uses the STIX clockwise and anticlockwise closed circle arrows U+2941 and U+2940. Scale each glyph by the diameter of its circular portion so the ring matches the diameter of the plain contour loop and has comparable stroke weight. Rotate the outline a quarter turn to keep the arrowhead clear of the stem. Anchor the circular portion on the assembled stem center and math axis, then use the translated ink bounds for fitting and spacing.

An entire U+2232 or U+2233 display glyph would couple the body height and arrowhead. A manually drawn arrowhead would duplicate the geometry already available in the STIX ring glyphs. The initial combining ring glyphs U+20D9 and U+20DA made the loop too heavy at the needed diameter.

### Use the MATH connector for extended body pieces

Resolve the vertical assembly for U+222B from the bundled font. It pairs the top and bottom glyphs with `integral.x`, whose 133-unit width matches both connectors. The current U+23AE extender is 102 units wide and leaves shoulders at the joins. Follow the connector lengths and minimum overlap declared by MATH when placing extenders. Reuse the glyph-ID outline path used for tall radicals and preserve contour winding when emitting outline primitives. Use the existing assembly height decisions at call sites.

### Reserve the combined ink before placing limits

Compute the union of the body and loop ink. Translate pieces together if the loop extends left of the local box and include any right overhang in the advance. Normalize the operator before collecting caret and hit bounds and before anchoring the existing side or stacked limits. This protects both the wide plain rings and the protruding directional arrowheads.

### Draw the palette arrowheads at the right edge

Use the shared `draw_integral()` SVG builder to place the direction mark to the right of the stem. Clockwise travel points down at the right edge and anticlockwise travel points up. That position gives the mark room at both standalone and template sizes. Feed the six icon variants through the normal SVG and 2× atlas generator. Catalog source keys continue to link each tile, tooltip, and insertion recipe.

## Risks / Trade-offs

- Glyph bearings and asymmetrical arrowheads can pull the loop off center. Verify the ring's circular portion and use the full outline for bounds.
- Tall assembly can expose thin seams or close the transparent parts of the loop in one output. Verify the connector at an extended height and compare raster and vector outlines.
- Direction marks can merge into the ellipse at tile size. Compare both directions and the plain loop at actual `40x26` size in the standalone, side-lower, and stacked-lower variants.

## Migration Plan

Commit the regenerated palette SVGs and atlas PNG and JSON together. The next desktop build embeds the generated assets. Existing documents and palette shortcut data keep using their current LaTeX and MathML representations.
