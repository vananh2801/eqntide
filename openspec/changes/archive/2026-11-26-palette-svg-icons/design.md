# Design

## Context

See `proposal.md` for motivation and scope. Currently:
- `src/catalog/mod.rs` defines 547 unique palette `ENTRIES` (217 symbols + 330 templates across 21 catalog groups).
- `desktop/src/app/palette_ui.rs` previously built a per-category `PaletteAtlas` at runtime using `eqntide::render_alpha_at(..., 17.0 * ppp, 40.0 * ppp, 26.0 * ppp)` with `TextureOptions::NEAREST` and `powf(0.70)` gamma boosting, while `ui_icon()` loads pre-rasterized `48x48` white RGBA PNGs from `assets/icons/rendered/` (rendered from `assets/icons/tabler-icons/*.svg`) with `TextureOptions::LINEAR`.
- `tests/python/test_icons.py` enforces an exact file-set equality between `assets/icons/tabler-icons/*.svg` and `render_icons.DESKTOP_ICONS | RIBBON_ICONS` (third-party Tabler MIT assets).
- The machine environment provides `inkscape`, `magick`, and Python `PIL`, and `assets/fonts/` bundles `Nunito` (`Nunito-Regular.ttf`) alongside `STIXTwoText` and `STIXTwoMath`.

## Goals / Non-Goals

**Goals:**
- Generate 547 deterministic, self-contained `<path>`/`<rect>`/`<circle>`/`<ellipse>` `40x26` SVGs under `assets/icons/palette/` whose stroke weight (`~1.7` DIP) and visual balance match the `22x22` Tabler sidebar icons (`2/24 ≈ 1.83` DIP).
- Use thin, muted solid-border rounded rects for `□` slots, smooth circular-arc parentheses matching Tabler `parentheses`, clear double-shaft arrowheads matching single-shaft visibility, hollow-square grids for all matrix templates (including `3x3` and `4x4`), `Nunito-Regular` baseline-aligned paths for `Functions`, and `STIXTwoText`/`STIXTwoMath` converted `<path>` outlines for differentials, `Greek`, and calligraphic/fraktur/blackboard symbols.
- Visually center every generated icon within its `40x26` box (horizontally for all 547 icons; vertically for all non-baseline icons).
- Pack all 547 icons into a single committed white-RGBA HiDPI sprite atlas (`assets/icons/palette-rendered/atlas.png` + `atlas.json` + SHA-256 verification).
- Simplify `desktop/src/app/palette_ui.rs` to upload one shared `PaletteAtlas` (`TextureOptions::LINEAR`) and render the Custom Matrix button with Tabler's `dots` icon.

**Non-Goals:**
- Adding a runtime SVG parser/rasterizer dependency (such as `resvg`/`usvg`) to `Cargo.toml`, keeping `deny.toml` and binary size unchanged.
- Modifying `eqntide::render` (`STIXTwoMath` Equation Canvas rendering, SVG/EMF/PNG equation export).
- Mixing generated palette SVGs into `assets/icons/tabler-icons/` (which remains strictly third-party Tabler Icons).

## Decisions

### 1. ViewBox, Stroke Weight, Parentheses, Arrowheads, and Slot Styling
- **Decision**: Use `viewBox="0 0 40 26"`, `fill="none"`, `stroke="currentColor"`, `stroke-linecap="round"`, `stroke-linejoin="round"`, with primary structural stroke width `1.7` (`stroke-width="1.7"`), and `□` placeholder slots rendered as solid rounded rectangles `<rect rx="1.1" stroke-width="1.15" opacity="0.68"/>`.
  - **Parentheses**: Use circular arcs (`A r r` with `r = 0.765 * h`), matching Tabler `parentheses.svg` (`12.25 / 16`), instead of quadratic Bezier curves so `( )` in binomials, fences, and `pmatrix` matrices stay smoothly rounded without pointed vertices.
  - **Double-shaft arrowheads**: For `Rightarrow`, `Leftarrow`, `Leftrightarrow`, `Longrightarrow`, `Longleftarrow`, `Longleftrightarrow`, `Uparrow`, `Downarrow`, `Updownarrow`, and their labeled variants, use `45°` chevrons (`size = 5.4` or `4.8` on short spans) with shafts (`gap = 1.65`, `sw = 1.4`) stopping `1.7` DIP before the tip so the arrowhead wings protrude cleanly outside both parallel shafts.
- **Rationale**: A 1:1 DIP viewBox (`40x26`) maps directly to the `40x26` icon budget inside `46x32` buttons, keeping `1.7` visually equivalent to Tabler's `2.0` on `24x24` scaled to `22x22` (`1.83` DIP). Solid thin muted strokes (`opacity="0.68"`) preserve single-color `ThemeTokens::icon_tint` tinting while remaining legible even in nested templates (`\sqrt{\frac{□}{□}}`), avoiding dashed-stroke fragmentation at small sizes.
- **Alternatives considered**: Dashed placeholder boxes (fragment on `< 6px` nested slots); quadratic parentheses (appear pointed in tall `20` DIP brackets); smaller `3.6` arrowheads on double shafts (subsumed by shaft thickness).

### 2. Matrix Grids Keep Hollow Squares Up to `4x4`
- **Decision**: Render all matrix templates (`1x2` through `4x4`) using hollow square/rect slot primitives:
  - `2x2`: `6.5x6.5` cells, `4.0` gap, `stroke-width="1.2"`
  - `3x3`: `4.5x4.5` cells, `3.2` gap, `stroke-width="1.1"` (`4.2x4.2` / `2.8` gap for fenced `3x3`, with `pad_x = 5.5` for circular `pmatrix` fences)
  - `4x4` (no outer brackets in catalog): `3.5x3.5` cells, `2.5` gap, `stroke-width="1.0"` (leaving `2.5` DIP inner hollow space and `1.5` DIP outer gap, or `5px` / `3px` at 2x HiDPI).
- **Rationale**: Fits comfortably within the `22` DIP vertical budget (`4 * 3.5 + 3 * 2.5 = 21.5 <= 22`) without collapsing `3x3` or `4x4` matrices into dots, and keeps `pmatrix2`/`pmatrix3` arcs clear of outer matrix cell corners.

### 3. Hybrid Parametric + Font-to-Path SVG Generation
- **Decision**: Structure `scripts/generate_palette_icons.py` + `scripts/palette_svg_builders.py` in two layers:
  1. **Parametric Tabler-style geometry (~406 items + composite templates)**: reusable primitives for slots, fraction bars, radicals, fences (`() [] {} ⟨⟩ | ‖ ⌊⌋ ⌈⌉ ⟦⟧`), integrals (`∫..∰`), sums, big operators, accents/bars, boxes, single/double/harpoon/hook/curved arrows, labeled arrows, relations, operators, logic, sets, and spaces.
  2. **Font-derived `<path>` outlines (~141 letter items + composite text like `lim`, `d□`)**:
     - `Nunito` (`assets/fonts/Nunito/Nunito-Regular.ttf`) at `11.5px` (single-line functions) and `11.0px` (`lim`/`min`/`max`), baseline-aligned (`baseline_y = 16.0` / `12.4`) so all 45 `Functions` fit at `scale = 1.0` without per-word shrinking and match the rounded UI/Tabler aesthetic.
     - `STIX Two Text` (`d` in differential integrals, `imath`/`jmath`) and `STIX Two Math` (`GreekLower`, `GreekUpper`, `ℝ ℤ ℂ ℚ ℕ`, `𝒜..𝒩`, `𝔄..𝔑`, `℘ ℵ ℜ ℑ ∂ ℓ ℏ ƛ ℧`), extracted via `FONTCONFIG_FILE` scoped strictly to `assets/fonts/` and `inkscape --export-text-to-path`.
- **Rationale**: Pure `<path>` SVGs render identically everywhere without OS font dependencies, harmonize `Functions` with the desktop UI font, and preserve authentic STIX mathematical letterforms for Greek/special alphabets.

### 4. Visual Centering & Composite Sheet Rasterization into One Shared Atlas
- **Decision**: Commit individual `assets/icons/palette/<id>.svg` files for review/diffing. During generation, run a two-pass sheet rasterization: Pass 1 renders all 547 cells into a temporary `2x` sheet (`24x23` grid of `42x28` DIP slots = `2016x1288` px including `1` DIP cell padding) and measures each cell's rendered alpha bounding box; `center_elements` wraps off-center items in `<g transform="translate(dx dy)">` so every icon is horizontally centered at `x = 20.0` DIP and vertically centered at `y = 13.0` DIP (except `Functions` and `symbol-ldots`, which preserve vertical baseline alignment); Pass 2 writes the centered SVGs and final white-RGBA `assets/icons/palette-rendered/atlas.png` alongside `assets/icons/palette-rendered/atlas.json` (cell pixel rects, source-key mapping, and SHA-256 checksums).
- **Rationale**: Measuring actual rendered pixel bounds accounts for stroke widths, Bezier/arc extents, and STIX/Nunito glyph sidebearings identically across all 547 items (fixing single-sided fences, pre-scripts, differentials, primes, and labeled harpoons in one consistent pass). Single-sheet rendering completes in ~2–3s, produces a compact compressed PNG embedded once via `include_bytes!`, eliminates runtime CPU glyph rasterization in `desktop/`, and supports crisp `TextureOptions::LINEAR` scaling across DPI levels. `Recent` reuses the same single texture.
- **Alternatives considered**: Bundling 547 separate PNGs (bloats `include_bytes!` match tables and creates 547 texture uploads); manual per-builder centering offsets (error-prone across asymmetric font glyphs and varying stroke caps).

### 5. Tabler `dots` Icon for Custom Matrix Button
- **Decision**: Add `dots.svg` to `assets/icons/tabler-icons/` and `DESKTOP_ICONS` in `scripts/render_icons.py`, and update `desktop/src/app/palette_ui.rs` to render the Custom Matrix button using `self.ui_icon(ui.ctx(), "dots", UI_ICON_SIZE, icon_color)` instead of `"⋯"`.

## Risks / Trade-offs

- **Catalog drift**: Adding or renaming an entry in `src/catalog/mod.rs` without regenerating icons would leave a missing palette icon. → **Mitigation**: `generate_palette_icons.py --check` (and `tests/python/test_icons.py` + `embedded_palette_atlas_covers_every_palette_entry_with_valid_uvs` in `desktop/src/palette.rs`) verifies exact 1-to-1 coverage of every `ENTRIES` item in `src/catalog/mod.rs`, SVG hashes, and atlas entries.
- **Long function names (`limsup`, `arcosh`) width vs short words (`sin`, `Pr`)**: Scaling long words down per-word makes stroke thickness uneven. → **Mitigation**: Using bundled `Nunito-Regular` at `11.5px` fits even `limsup` (`33.2` DIP) within `34.5` DIP at `1.0x` scale on a shared `baseline_y = 16.0`.
- **Inkscape/fontconfig variance during regeneration**: Different OS environments might render slightly different PNG bytes. → **Mitigation**: Committed PNG + `atlas.json` checksums mean regular builds/tests never invoke `inkscape`; regeneration uses `FONTCONFIG_FILE` scoped exclusively to `assets/fonts/` for deterministic font resolution.
