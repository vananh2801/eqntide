# Tasks

## 1. Horizontal Polygon Clipping (`geom.rs`)

- [x] 1.1 Add `clip_poly_x(poly, x_min, x_max)` and its half-plane helper to `src/render/layout/geom.rs` following `CODING_CONVENTIONS.md` (top-down order, descriptive variable names, English docs), and verify with unit tests covering inside, crossing, and outside contours (`cargo test render::layout::geom`).

## 2. Unified Horizontal Arrow Assembly (`arrow.rs`)

- [x] 2.1 Replace `ArrowDir::Other(ch)` horizontal scaling and duplicated extender loops in `src/render/layout/arrow.rs` with a shared `pub(super)` horizontal arrow assembler supporting single-line (`→`, `←`, `↔`), harpoon (`⇀`, `↼`, `⇁`, `↽`), maps-to (`↦`, `↤`), double-shaft (`⇒`, `⇐`, `⇔`), equilibrium (`⇌`, `⇋`), and stacked opposing (`⇄`, `⇆`) arrows (plus long Unicode variants) via unscaled glyphs/clipped caps, `\u{23AF}`/`=` extenders, and an unscaled natural-glyph fallback for very narrow spans.
- [x] 2.2 Add unit tests in `src/render/layout/arrow.rs` verifying that `⇄`, `⇆`, `⇌`, `⇋`, `⇒`, `⇐`, `⇔`, `⇀`, `↼`, `↦` assemble without horizontal glyph scaling across wide labels and preserve label clearance (`cargo test render::layout::arrow`).

## 3. Arrow Accent Assembly (`embellish.rs`)

- [x] 3.1 Update `draw_accent` and `draw_under_accent` in `src/render/layout/embellish.rs` to delegate all arrow accents (`→`, `←`, `↔`, `⇀`, `↼`, `⇁`, `↽`, `⇌`, `⇋`, `⇒`, `⇐`, `⇔`, `\u{20D7}`, `\u{27F6}`) to the shared arrow assembler, removing both the `ArrowDir::Other`-style horizontal stretch and the `span < min_w` horizontal `em_x` compression on `\vec`/`\overrightarrow`, and keeping `Item::Glyph` reflection for vertically symmetric glyphs around `259` font units.
- [x] 3.2 Add unit tests in `src/render/layout/embellish.rs` verifying multi-character arrow accents (`\overleftarrow{xyz}`, `\overleftrightarrow{xyz}`, `\overleftharpoon{xyz}`, `\underrightharpoon{xyz}`) extend via unscaled parts and narrow single-letter accents (`\vec{i}`, `\overaccent{⇔}{x}`) preserve natural arrowhead aspect ratios (`cargo test render::layout::embellish`).

## 4. End-to-End Render Parity & Long-Argument Visual Suite

- [x] 4.1 Run and verify SVG, PNG, and EMF arrow parity tests (including `hollow_double_arrow_preserves_transparent_interior` and `test_render_emf_contours_single_filled_path`) and add/update assertions if needed (`cargo test render::`).
- [x] 4.2 Add a long-argument arrow and accent stress sheet (`16_long_arrow_arguments.png`) to `tests/export_verification.rs` and visually inspect all exported layout sheets per `tests/TESTING.md`.

## 5. Seamless Thumbnail & Preview Union Rasterization

- [x] 5.1 Add `BlendMode::Union` (`Canvas::new_union`) in `src/render/canvas.rs` and implement supersampled union rasterization (`4×` in `render_alpha_scaled` for palette/UI thumbnails, `2×` in `rasterize_alpha` for previews) with per-polygon `Item::Poly` isolation in `src/render/mod.rs`.
- [x] 5.2 Update `tile_extender` in `src/render/layout/arrow.rs` for sequential tiling with a minimal overlap seam and clipped trailing piece (`emit_clipped_glyph`), and replace the hardcoded `1.5` integral overlap in `src/render/layout/integral.rs` with proportional `0.03 * eff_em`.
- [x] 5.3 Add `palette_arrow_and_integral_shafts_have_uniform_coverage` in `src/render/mod.rs`, visually verify shrunk dark-theme palette thumbnails plus regenerated `04_Integrals` / `11_LabeledArrows` / `16_long_arrow_arguments` sheets, and pass all four `CODING_CONVENTIONS.md` gates (`fmt`, `clippy -D warnings`, `test --workspace --all-targets --locked`, `cargo doc -D warnings`).
