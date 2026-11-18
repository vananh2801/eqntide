# Spec Delta

## Purpose

Fixes visual and structural conversion parity bugs in stretched tilde accents during layout/rendering and stacked `<munderover>` labeled arrows during OMML encoding.

## ADDED Requirements

### Requirement: Horizontal stretching for wide tilde accents
When laying out and drawing an over-accent or under-accent tilde (`\widetilde` / `U+0303` / `U+02DC` / `~`) with `accent="true"` or stretchy attribute over a multi-character base, the layout renderer SHALL draw a smooth tilde spanning the width of the base expression rather than a fixed short glyph width.

#### Scenario: Stretchy tilde over multi-character base spans full base width
- **WHEN** `\widetilde{xyz}` is laid out and rendered to SVG/PNG/EMF
- **THEN** the drawn tilde path spans across `xyz` comparably to `\widehat{xyz}` rather than covering less than one character

### Requirement: Stacked labeled arrow encoding in OMML
When encoding MathML `<munderover>` whose base is an arrow or harpoon operator (`<mo>`) with top and bottom labels into OMML, the encoder SHALL emit stretched/labeled OMML constructs aligned with `<mover>`/`<munder>` arrow handling rather than compressing the arrow into an unstretched base box.

#### Scenario: Equilibrium arrow with top and bottom labels preserves arrow length in OMML roundtrip
- **WHEN** `A \overset{f}{\underset{g}{\leftrightharpoons}} B` is converted to OMML and re-rendered or roundtripped
- **THEN** the middle arrow scales cleanly to encompass both labels `f` and `g` comparably to the MathML/MTEF representation
