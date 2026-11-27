# Proposal

## Why

EqnTide draws clockwise and anticlockwise contour integrals as the same unmarked loop on the canvas. The six directional palette icons place their arrowheads in the opposite direction from their assigned commands. Wide contour loops can also exceed the space reserved for them in a row.

## What Changes

- Assemble directional contour integrals from the existing integral body pieces and a separate directional ring glyph.
- Use the STIX OpenType MATH connector when the body assembly needs a vertical extender, and account for ring ink in layout and editor hit bounds.
- Correct the directional icons for standalone integrals and templates with side or stacked limits, then regenerate the SVGs and palette atlas.
- Cover the distinction between plain, clockwise, and anticlockwise loops across preview and export. Preserve the catalog commands, limit placement, and button actions.

## Capabilities

### New Capabilities

### Modified Capabilities

- `mathml-render-and-arrow-parity` — Assembled contour integrals convey direction and reserve their full ink bounds.
- `desktop-palette-navigation` — Directional contour integral icons depict the same direction as the template inserted by the button.

## Impact

The shared renderer affects the desktop canvas and SVG, PNG, and EMF previews. Palette changes affect the SVG builder and generated assets embedded by the desktop app. The change focuses on the existing eight catalog integrals at supported assembly heights. Changes to MathML stretch policy, default inline and display sizes, and additional integral operators are separate work.
