# mathml-render-and-arrow-parity Specification

## Purpose

Fixes visual and structural conversion parity bugs in stretched tilde accents during layout/rendering and stacked `<munderover>` labeled arrows during OMML encoding.

## Requirements

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

### Requirement: Unscaled piece assembly for stretchy labeled arrows
When laying out and rendering a stretchy labeled arrow (`→`, `←`, `↔`, `⇒`, `⇐`, `⇔`, `⇀`, `↼`, `⇁`, `↽`, `⇌`, `⇋`, `↦`, `↤`, `⇄`, `⇆`, and long variants) with overscript and/or underscript labels, the renderer SHALL assemble unscaled arrowheads or caps with tiled shaft extenders rather than scaling the arrow glyph horizontally.

#### Scenario: Opposing and equilibrium labeled arrows preserve arrowhead proportions across wide labels
- **WHEN** `\underset{dfdsfsd}{\rightleftarrows}`, `A \overset{f}{\underset{g}{\rightleftharpoons}} B`, or multi-term long-argument arrow equations are laid out and rendered to SVG, PNG, and EMF
- **THEN** the rendered arrowheads keep their natural font aspect ratio and stroke thickness while the shafts span the label width via tiled extenders

#### Scenario: Double and maps-to labeled arrows assemble matching shaft extenders
- **WHEN** `\overset{label}{\Rightarrow}`, `\overset{label}{\Leftrightarrow}`, or `\overset{label}{\mapsto}` is laid out and rendered
- **THEN** the renderer joins unscaled arrow heads or vertical bars with matching single (`\u{23AF}`) or double (`=`) shaft extenders without horizontally stretching the tips

### Requirement: Unscaled piece assembly for over- and under-arrow accents
When laying out and rendering over- or under-arrow accents (`\vec`, `\overrightarrow`, `\overleftarrow`, `\overleftrightarrow`, `\underrightarrow`, `\underleftarrow`, `\underleftrightarrow`, `\overleftharpoon`, `\overrightharpoon`, `\underleftharpoon`, `\underrightharpoon`, and double-arrow accents), the renderer SHALL assemble unscaled heads/caps and shaft extenders over wide bases and render an unscaled natural or shaft-clipped glyph over narrow bases.

#### Scenario: Multi-character arrow accents extend shafts without distorting arrowheads
- **WHEN** `\overleftarrow{xyz}`, `\overleftrightarrow{xyz}`, `\overleftharpoon{xyz}`, or `\underrightharpoon{xyz}` is laid out and rendered to SVG, PNG, and EMF
- **THEN** the accent spans the base expression using unscaled heads/barbs and tiled `\u{23AF}` extenders rather than horizontal outline scaling

#### Scenario: Narrow single-character arrow accents avoid horizontal arrowhead compression
- **WHEN** `\vec{i}` or `\overaccent{⇔}{x}` is laid out and rendered
- **THEN** the drawn arrowhead is never horizontally compressed below its natural proportions, and hollow double-arrow accents preserve their transparent interior across PNG, SVG, and EMF

### Requirement: Seamless union rasterization for piece-assembled symbols
When rasterizing equations or scaled palette alpha masks composed of overlapping glyph pieces or clipped polygons (including stretchy arrows, integrals, fences, and accents), the rasterizer SHALL unite overlapping coverage rather than adding overlaps so shrunk thumbnails and previews render with uniform stroke brightness and no dark pinholes or bright seams.

#### Scenario: Shrunk palette arrow thumbnails render uniform shafts without overlap blotches
- **WHEN** a labeled arrow template (such as `\overset{#?}{\rightarrow}`, `\overset{#?}{\Rightarrow}`, or `\overset{#?}{\rightleftarrows}`) is rasterized into a compact `40x26` palette alpha mask at `17` px em
- **THEN** pixel coverage along the horizontal shaft remains uniform across cap-to-extender and extender-to-extender joins without doubled bright spots or dark gaps

#### Scenario: Assembled integral symbols join smoothly at palette and preview scales
- **WHEN** an integral symbol (`\int`, `\iint`, `\oint`, `\oiint`) assembled from vertical pieces is rasterized both at palette thumbnail scale (`40x26`) and at full preview scale
- **THEN** the stem joins render cleanly with uniform stroke density and zero bright overlap knots or subpixel breaks
