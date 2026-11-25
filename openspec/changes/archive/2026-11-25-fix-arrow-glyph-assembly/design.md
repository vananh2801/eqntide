# Design

## Context

See `proposal.md` for motivation. Two independent layout modules previously scaled arrow glyph polygons horizontally:

1. `src/render/layout/arrow.rs` (`labeled_arrow`): assembled `→`, `←`, `↔` from glyphs + `\u{23AF}`, while `ArrowDir::Other(ch)` (`⇒`, `⇐`, `⇔`, `⇀`, `↼`, `⇌`, `⇋`, `↦`, `⇄`, `⇆`) stretched a single glyph's outline across `len`.
2. `src/render/layout/embellish.rs` (`draw_accent` / `draw_under_accent`): stretched `←`, `↔`, `⇀`, `↼`, `⇌`, `⇋`, `⇒`, `⇐`, `⇔` horizontally across `span`, and scaled `\u{2192}` (`\vec` / `\overrightarrow`) horizontally whenever `span < min_w`.

Inspection of `STIXTwoMath-Regular.ttf` confirms that the OpenType `MATH` table's `HorizGlyphConstruction` has no `GlyphAssembly` parts for arrows, so horizontal assembly must combine existing STIX Two Math glyphs whose `glyf` shaft coordinates match identically:
- Single shaft (`→`, `←`, `↔`, `↦`, `↤`, `⇀`, `↼`, `⇁`, `↽`): `y = 225..293` font units (midpoint `259`, matching `\u{23AF}` `[0, 322] × [225, 293]`).
- Double shaft (`⇒`, `⇐`, `⇔`, `⇌`, `⇋`): `y = 123..190` and `y = 327..395` font units (matching `=` `U+003D` `[62, 658] × {123..190, 327..395}`).
- Stacked opposing arrows (`⇄`, `⇆`): top shaft `y = 389..456`, bottom shaft `y = 61..129` (matching two `\u{23AF}` extenders shifted vertically by `+163.5` and `-164` font units).

Furthermore, rasterizing multi-piece assemblies onto `Canvas` previously used additive blending (`(*slot + c).min(1.0)`) and shared a single `Rasterizer` across all `Item::Poly` items. On thin strokes—especially desktop palette thumbnails (`render_alpha_at(..., 17.0 * ppp, 40.0 * ppp, 26.0 * ppp)` with `gamma_lut = alpha^0.70` on dark `#2B3241`) and vertical integral joints (`⌠` + `⎪` + `⌡`)—overlapping anti-aliased edges double-counted partial coverage (`~0.45 -> ~0.90`), producing visible bright knots while evenly spaced extenders could leave dark pinholes between overlapping blocks.

All new and edited code must follow `CODING_CONVENTIONS.md` (7-layer top-down ordering, descriptive non-single-letter identifiers outside permitted `i/j/k`, `x/y`, RGBA/UV names, English comments, and passing `fmt`/`clippy`/`test`/`doc` with `-D warnings`).

## Goals / Non-Goals

**Goals:**
- Provide a single shared horizontal arrow assembly engine used by both `arrow.rs` and `embellish.rs`.
- Eliminate all horizontal glyph scaling (`em_x != em_y` or manual x-rescaling) for arrow characters across labeled arrows and accents.
- Support short spans cleanly by clipping shafts via `geom::clip_poly_x` and falling back to a centered unscaled natural glyph when `span` is narrower than the clipped heads/caps.
- Keep `draw_under_accent` producing crisp, vertically reflected arrows (preserving `Item::Glyph` for vertically symmetric arrows/extenders around axis `259` font units).
- Eliminate additive coverage seams, winding accumulation, and subpixel gaps across piece-assembled symbols at both shrunk palette-button sizes and full preview resolution.

**Non-Goals:**
- Vertical arrow stretching (`↑`, `↓`, `↕`, `⇑`, `⇓`, `⇕`).
- Curved arc accents (`\wideparen` / `\underparen` `\u{2322}`), which intentionally stretch an arc without arrowheads.
- Changing MTEF or OMML encoders or palette definitions.

## Decisions

### 1. Shared table-driven assembler in `arrow.rs` + `geom::clip_poly_x`
Instead of keeping four duplicated extender-tiling loops across `arrow.rs` and `embellish.rs`, `arrow.rs` exposes a `pub(super)` assembly helper (`assemble_horizontal_arrow`) parameterized by `ArrowDir`, target `span`, `em`, and vertical anchor. `geom.rs` adds `clip_poly_x(poly, x_min, x_max)` using the same Sutherland–Hodgman half-plane clipper pattern as `clip_poly_y`.
- *Alternative considered*: Separate per-arrow branches duplicated in `arrow.rs` and `embellish.rs`. Rejected because it violates single-responsibility / DRY maintainability and risks drift between labeled arrows and accents.

### 2. Glyph-part recipes by arrow family
- **Single-line & single-harpoon arrows** (`→`, `←`, `↔`, `⇀`, `↼`, `⇁`, `↽`): place unscaled head glyph(s) (`\u{2192}`, `\u{2190}`, `\u{21C0}`, `\u{21BC}`, `\u{21C1}`, `\u{21BD}`) at the left/right boundaries and tile `\u{23AF}` across the shaft interval.
- **Double-shaft arrows** (`⇒`, `⇐`, `⇔`): place unscaled `⇐` (`\u{21D0}`) and/or `⇒` (`\u{21D2}`) and tile `=` (`U+003D`) across the inner gap.
- **Composite two-sided glyphs** (`↦`, `↤`, `⇌`, `⇋`, `⇄`, `⇆`): extract the left cap and right cap by clipping the native glyph's polygon contours at clean shaft cut lines (`clip_poly_x`), then tile `\u{23AF}` (single row for `↦`/`↤`, dual shifted rows for `⇄`/`⇆`) or `=` (for `⇌`/`⇋`) in between.
  - *Alternative considered*: Drawing `⇌`/`⇋` from `⇀`+`↽` and `⇄`/`⇆` from `→`+`←` at separate y-offsets. Rejected because STIX Two Math's native `⇌`/`⇋`/`⇄`/`⇆` heads have dedicated compact proportions (`-34..552` and `-94..612`) and exact shaft alignment (`123..190`/`327..395` for harpoons; `61..129`/`389..456` for opposing arrows); clipping preserves exact native typography and keeps `Item::Poly` bounds available for `test_equilibrium_arrow_labels_clear_harpoons`.

### 3. Narrow-span behavior (`clip_poly_x` shaft trim → unscaled natural glyph fallback)
When `span` is shorter than full unclipped head glyphs placed end-to-end:
1. If `span` is at least the sum of the two clipped head/cap widths (shaft trimmed up to the arrowhead neck), render the clipped caps meeting cleanly without any horizontal scaling.
2. If `span` is even narrower (such as `\vec{i}` or `\overaccent{⇔}{x}` at `0.65 * em`), render the single unscaled natural glyph (e.g., `⇔` or `\u{20D7}`/`\u{2192}`) centered on the target span rather than compressing the arrowhead horizontally. For `\overaccent{⇔}{x}`, this also preserves `⇔`'s multi-contour `Item::Contours` required by EMF `WINDING` fill (`test_render_emf_contours_single_filled_path`).
- *Alternative considered*: Forcing `span` to always expand to the full two-glyph minimum width. Rejected because single-letter `\vec{i}` or `\overaccent{⇔}{x}` would look disproportionately wide over narrow letters.

### 4. `draw_under_accent` symmetry preservation
STIX Two Math glyphs `←`, `↔`, `⇐`, `⇒`, `⇔`, `=` are vertically symmetric around `259` font units (`2 * 259 = 518`). `draw_under_accent` keeps `Item::Glyph` (reflecting `y -> -y + 518.0 * unit`) for all of those in addition to `→` and `\u{23AF}`, while flipping `Item::Poly`/`Item::Contours` (`y -> -y`) for asymmetric harpoons (`⇀`, `↼`, `⇌`, `⇋`, `⇄`, `⇆`).

### 5. Supersampled union rasterization & minimal extender/integral overlap
- **Union coverage blending (`src/render/canvas.rs`, `src/render/mod.rs`)**: `Canvas` gains `BlendMode::Union` (`Canvas::new_union`), taking `slot.max(coverage.clamp(0.0, 1.0))` on a supersampled grid and box-averaging `ss × ss` blocks back to output pixels (`ss = 4` in `render_alpha_scaled` for palette thumbnails and scaled UI icons; `ss = 2` in `rasterize_alpha` for previews, automatically clamping by `MAX_WIDTH_PX` / `MAX_PIXELS`). When `ss >= 2`, each `Item::Poly` is rasterized individually through `rasterize_contours_into_canvas` so adjacent polygons unite without accumulating winding numbers.
  - *Alternative considered*: Applying `max` blending directly at `1×` resolution. Rejected because taking `max` on `1×` subpixel ramps breaks smooth anti-aliased silhouettes where orthogonal edges cross pixels; supersampled `4×`/`2×` union preserves clean anti-aliasing while preventing double-counting across overlapping pieces.
- **Sequential tail-clipped extender tiling (`tile_extender` in `src/render/layout/arrow.rs`)**: Instead of evenly dividing a gap into heavily overlapping glyphs, `tile_extender` steps forward by `stride = (ext_w - seam).max(1.0 * unit)` (`seam = (25.0 * unit).min(0.25 * ext_w)`) and fills any trailing gap with one `emit_clipped_glyph` tile clipped cleanly to `[clip_min, to_x]`.
- **Proportional integral piece overlap (`src/render/layout/integral.rs`)**: Replaces the fixed `overlap = 1.5` px constant (which shrank to ~0.21 px at palette icon size) with `0.03 * eff_em` so `⌠` + `⎪` + `⌡` always overlap by >1 subpixel on `4×` union grids and join seamlessly at all sizes.

## Risks / Trade-offs

- **Seam gaps or overlap notches on rasterized clipped joins** → Overlap extenders and clipped caps by ~`25..60` font units and integrals by `0.03 * eff_em`, united under supersampled `BlendMode::Union`, so anti-aliased rasterization and EMF/SVG paths show neither hairline gaps nor bright knots.
- **EMF `test_render_emf_contours_single_filled_path` sensitivity** → Verify `\overaccent{⇔}{x}` (narrow fallback) emits multi-contour `Item::Contours`, and `\overset{a}{\Leftrightarrow}` emits `=` (`Item::Glyph` with 2 subpaths) so single-`FILLPATH` multi-contour winding tests pass unchanged.
- **Label-clearance & cap-width bounds in `arrow.rs`** → `test_equilibrium_arrow_labels_clear_harpoons` checks vertical clearance using `Item::Poly`/`Item::Contours`, and `wide_labeled_arrows_assemble_from_unscaled_caps_and_extenders` checks clipped `Poly`/`Contours` widths stay within a single natural glyph (`<= 650 * unit`, covering `=` extender tiles at `596` font units).
- **Performance on huge previews** → `rasterize_alpha_ss` downgrades `4× -> 2× -> 1×` automatically if `MAX_WIDTH_PX` or `MAX_PIXELS` would be exceeded, keeping memory bounded.
