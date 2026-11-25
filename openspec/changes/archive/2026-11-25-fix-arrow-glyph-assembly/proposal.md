# Proposal

## Why

Stretchy labeled arrows (`⇒`, `⇐`, `⇔`, `⇀`, `↼`, `⇌`, `⇋`, `↦`, `⇄`, `⇆`) in `src/render/layout/arrow.rs` and over/under arrow accents in `src/render/layout/embellish.rs` previously stretched a single glyph's outline horizontally across the target span instead of assembling arrowheads and shaft extenders from font pieces, distorting arrowheads whenever labels or bases were wider than one em. Additionally, overlapping assembled pieces (arrow caps, tiled extenders, and stacked integral stem glyphs) caused additive coverage doubling and dark pinholes when rasterized—especially when shrunk onto compact desktop palette buttons.

## What Changes

- Replace horizontal glyph-outline scaling for all stretchy labeled arrows (`ArrowDir` in `src/render/layout/arrow.rs`) with unscaled piece assembly using STIX Two Math arrowheads/caps, horizontal extenders (`\u{23AF}` and `=` `U+003D`), and horizontal polygon clipping (`clip_poly_x`) where caps share a glyph.
- Replace horizontal glyph-outline scaling for all over/under arrow accents (`←`, `↔`, `⇀`, `↼`, `⇌`, `⇋`, `⇒`, `⇐`, `⇔`, and the narrow-span `\vec`/`\overrightarrow` fallback in `src/render/layout/embellish.rs`) with the same unified piece-assembly path.
- For spans shorter than the combined clipped heads/caps, fall back to a single unscaled natural glyph centered on the target span so arrowheads never distort horizontally.
- Preserve `Item::Glyph` symmetry handling in `draw_under_accent` for vertically symmetric double and left/bidirectional arrows while supporting flipped contours for asymmetric harpoons/opposing arrows.
- Tile shaft extenders sequentially in `tile_extender` with a minimal overlap seam and clipped tail piece (`emit_clipped_glyph`), and scale stacked integral glyph overlap proportionally (`0.03 * eff_em` in `src/render/layout/integral.rs`).
- Rasterize scaled palette alpha masks (`4×`) and previews (`2×`) on a supersampled `BlendMode::Union` canvas (`src/render/canvas.rs`, `src/render/mod.rs`) with isolated `Item::Poly` contours so overlapping pieces never create bright double-coverage seams or subpixel gaps when shrunk.
- Structure all new and modified modules/helpers strictly according to `CODING_CONVENTIONS.md` (top-down declaration order, descriptive non-single-letter locals, English documentation, and zero-warning verification gates).

## Capabilities

### New Capabilities

### Modified Capabilities
- `mathml-render-and-arrow-parity`: Add requirements ensuring stretchy labeled arrows and over/under arrow accents assemble unscaled arrowheads/caps and shaft extenders (or render an unscaled natural glyph on very narrow bases) rather than horizontally scaling arrowheads, and ensure piece-assembled symbols rasterize seamlessly via supersampled union blending without overlap blotches on shrunk palette thumbnails or previews.

## Impact

- `src/render/layout/geom.rs`: New `clip_poly_x` helper mirroring `clip_poly_y`.
- `src/render/layout/arrow.rs`: Unified horizontal arrow assembly helper (`pub(super)`), sequential tail-clipped `tile_extender`, and full `ArrowDir` coverage without horizontal scaling.
- `src/render/layout/embellish.rs`: Delegates arrow accent drawing to the shared assembly helper and removes horizontal scaling paths.
- `src/render/layout/integral.rs`: Proportional (`0.03 * eff_em`) piece overlap for assembled integrals.
- `src/render/canvas.rs`, `src/render/mod.rs`: `BlendMode::Union` canvas mode, `4×`/`2×` supersampled union rasterization with `Item::Poly` isolation, and palette/preview uniformity tests.
- `src/render/emf.rs`, `tests/export_verification.rs`: Rendering parity tests across SVG, PNG, and EMF outputs plus long-argument visual stress suite (`16_long_arrow_arguments.png`).
