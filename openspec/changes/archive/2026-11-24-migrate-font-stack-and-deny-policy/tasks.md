# Tasks

## 1. Dependency Swap & Font Metadata Migration

- [x] 1.1 Replace `ab_glyph` and `ttf-parser` in `Cargo.toml` with `skrifa` (and `read-fonts` >= 0.47 if used directly), keeping `ab_glyph_rasterizer`, and verify `cargo metadata --format-version 1` resolves without `ttf-parser` or `owned_ttf_parser`.
- [x] 1.2 Migrate `src/render/font/metadata.rs` (`radical_info` and `ssty_map`) and `src/render/font.rs` OpenType `MATH` constants, `italic_correction`, `top_accent_attachment`, `math_kern`, and `math_glyph_bbox` to `read-fonts` / `skrifa`, and verify `cargo test --lib render::font::tests::test_math_constants_and_glyph_info` passes with matching values.

## 2. Glyph Selection, Outlines, Rasterization, and Unit Tests

- [x] 2.1 Migrate `src/render/font/selection.rs` (`pick_glyph`, `Charmap`) and `src/render/font.rs` (`advance`, `advance_styled`, `glyph_id_for_math`, `glyph_id_for_math_styled`) from `ab_glyph`/`ttf-parser` to `skrifa`, and verify glyph lookup and advance assertions pass in `cargo test --lib render::font`.
- [x] 2.2 Implement `skrifa::outline::OutlinePen` in `src/render/font/geometry.rs` to build `Vec<Seg>` in pixel coordinates (`+y` down), migrate `ink`, `ink_x`, `draw_styled` (using `ab_glyph_rasterizer::Rasterizer`), `glyph_segments_styled`, `glyph_polys_styled`, and `math_glyph_polys` in `src/render/font.rs`, and verify `cargo test --lib render` passes.
- [x] 2.3 Add/update unit tests in `src/render/font.rs` covering `math_glyph_bbox`, `ink`/`ink_x` bounds, `glyph_segments_styled` (line/quad/cubic segments), `glyph_polys`/`math_glyph_polys`, and `draw_styled` pixel coverage, and verify `cargo test --lib render::font` passes cleanly.

## 3. Cargo-Deny Policy Hardening & Full 4-Tier Visual Verification

- [x] 3.1 Update `deny.toml` to set `unmaintained = "all"`, add explicit `ignore` entries with rationale comments for `RUSTSEC-2026-0370`, `RUSTSEC-2026-0371`, and `RUSTSEC-2026-0372` (`accesskit_unix`/`zbus 5.x`), set `wildcards = "deny"`, keep all four Linux/Windows/macOS targets in `[graph].targets`, and verify `cargo deny --all-features --workspace --locked check advisories licenses bans sources` passes with zero errors.
- [x] 3.2 Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, and `./tests/run_all.sh` (all 4 tiers: workspace tests, `conversion_matrix`/`cross_conversion`, `export_verification`, Python, and WPS Node tests) to confirm full suite pass.
- [x] 3.3 Inspect rendered visual sheets in `target/test_exports/visual/` (`14_roundtrip_p1.png`, `14_roundtrip_p2.png`, `15_layout_edge_cases.png`, plus `07_fractions_scripts.png`, `08_fences_all.png`, `09_bigops_integrals.png`, `11_embellishments_bars.png`) directly via `view_file` against the 5 layout criteria in `tests/TESTING.md` (baseline/axis balance, script & limit placement, matrix/cases alignment, accents/enclosures, round-trip parity).
