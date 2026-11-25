## ADDED Requirements

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
