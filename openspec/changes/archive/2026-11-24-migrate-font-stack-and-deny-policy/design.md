# Design

## Context

See `proposal.md` for motivation. Currently `src/render/font.rs` and `src/render/font/{metadata,geometry,selection}.rs` use two font crates together:
- `ttf-parser` (0.25.1, `RUSTSEC-2026-0192` unmaintained) for OpenType `MATH` table constants/kerning/variants, `GSUB` `ssty` alternate lookups, and raw `glyph_bounding_box`.
- `ab_glyph` (0.2.32, which pulls `owned_ttf_parser` → `ttf-parser 0.25.1`) for charmap lookup, unscaled horizontal advances, scaled glyph bounds (`ink` / `ink_x`), outline curve extraction (`outline_to_segments`), and rasterization (`outline_glyph` + `draw`).

Meanwhile `src/render/mod.rs` uses `ab_glyph_rasterizer` (zero dependencies) to rasterize layout primitives, and `Cargo.lock` already resolves `read-fonts` / `skrifa` transitively via `eframe` dependencies, though at older versions (`0.41.0` / `0.44.0`) that predate `read_fonts::tables::math`.

Additionally, Linux builds pull `zbus_names`, `zvariant`, and `zvariant_derive` (`RUSTSEC-2026-0370/0371/0372`) transitively via `eframe` → `egui-winit` → `accesskit_unix` → `atspi` → `zbus 5.x`; those three crates were merged into `zbus 6.0.0` upstream and cannot be replaced directly until `accesskit`/`eframe` upgrades.

## Goals / Non-Goals

**Goals:**
- Completely eliminate `ttf-parser`, `owned_ttf_parser`, and `ab_glyph` from `Cargo.toml` and the workspace `Cargo.lock` graph.
- Preserve exact OpenType `MATH` constants, `GSUB` `ssty` substitutions, glyph selection fallbacks, coordinate orientation (`+y` down in pixel space), and SVG/PNG/EMF output parity.
- Keep `ab_glyph_rasterizer` (dependency-free) for pixel rasterization, feeding glyph outlines directly into `ab_glyph_rasterizer::Rasterizer` (or via `Seg`).
- Maintain `x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`, `x86_64-apple-darwin`, and `aarch64-apple-darwin` target checks in `deny.toml` while enabling `unmaintained = "all"` and `wildcards = "deny"`.

**Non-Goals:**
- Migrating away from `eframe` / `accesskit` or patching transitive `zbus 5.x` crates; those remain explicitly ignored in `deny.toml` until upstream `eframe` updates.
- Changing math layout algorithms, STIX font assets, or document conversion formats (OMML/MTEF/MathML).
- Re-enabling macOS CI runners or macOS packaging scripts (handled separately from dependency policy).

## Decisions

1. **Use `skrifa` (`>= 0.50`) + `read-fonts` (`>= 0.47`, via `skrifa::raw` or direct dependency) to replace both `ttf-parser` and `ab_glyph`:**
   - *Rationale*: `read-fonts` 0.47+ provides `#![forbid(unsafe_code)]` typed parsing for `MATH` (`read_fonts::tables::math::{Math, MathConstants, MathGlyphInfo, MathVariants, MathKern}`) and `GSUB` (`tables::gsub`), while `skrifa` provides `FontRef`, `Charmap`, `GlyphMetrics`, `OutlineGlyphCollection`, and `OutlinePen` (`move_to`, `line_to`, `quad_to`, `curve_to`, `close`). Both are `MIT OR Apache-2.0` from `googlefonts/fontations` and satisfy `RUSTSEC-2026-0192`'s recommended alternative.
   - *Alternatives considered*:
     - Replacing only `ttf-parser` while keeping `ab_glyph`: rejected because `ab_glyph` depends on `owned_ttf_parser` → `ttf-parser`, leaving `RUSTSEC-2026-0192` in the graph.
     - Using `allsorts` or standalone `otfmath`: rejected because `allsorts` lacks typed `MATH` structs and `otfmath` is small/less actively maintained than `fontations`.

2. **Implement `OutlinePen` directly in `src/render/font/geometry.rs` to build `Vec<Seg>` and feed `ab_glyph_rasterizer::Rasterizer` for `draw_styled`:**
   - *Rationale*: `skrifa` draws outlines through an `OutlinePen` callback in unscaled or scaled font space (`+y` up in unscaled font coordinates vs `+y` down in pixel space). Implementing a small `SegPen` adapter keeps `Seg` and `segments_to_polys` unchanged, while `ink`/`ink_x`/`draw_styled` can compute tight bounds and rasterize segments using the existing `ab_glyph_rasterizer::Rasterizer` without pulling any extra rasterizer crate.
   - *Coordinate convention check*: In `font/geometry.rs`, font design units (`+y` up) map to `(x + px * scale, y - py * scale)`. Both `skrifa::outline::OutlinePen` at `Size::unscaled()` (or scaled with Y-flip) and `ab_glyph::Outline` emit unscaled/scaled coordinates; using `Size::unscaled()` with explicit `(x + px * scale, y - py * scale)` ensures identical rounding, scaling, and Y-inversion behavior across `Seg`, `glyph_polys`, `ink`, `ink_x`, and `draw_styled`.
   - *Public signature check*: `Fonts::glyph_id_for_math` and `Fonts::glyph_id_for_math_styled` currently return `Option<ttf_parser::GlyphId>` (checked only within `src/render/font.rs` in the workspace). Switching them to `Option<u16>` or `Option<skrifa::GlyphId>` avoids leaking external font-crate types if kept internal (`u16` or `skrifa::GlyphId`).

3. **Tighten `deny.toml` with explicit scoped `ignore` entries instead of `unmaintained = "none"`:**
   - *Rationale*: Setting `unmaintained = "all"` with `ignore = ["RUSTSEC-2026-0370", "RUSTSEC-2026-0371", "RUSTSEC-2026-0372"]` (each annotated with its upstream `accesskit_unix`/`zbus 5.x` cause) catches any new unmaintained crate immediately while keeping Linux/macOS/Windows CI green. Setting `wildcards = "deny"` blocks accidental `*` version specs.

## Risks / Trade-offs

- **Subtle coordinate / curve / bbox differences between `ab_glyph` and `skrifa`** (e.g., `ink` / `ink_x` tight bounds vs `px_bounds()`, or quadratic-to-cubic handling in TrueType `glyf` vs CFF outlines) → **Mitigation**: Keep `Size::unscaled()` + explicit `em_px / upem` mapping, add unit tests comparing `math_glyph_bbox`, `ink`, `ink_x`, `glyph_segments_styled`, and `glyph_polys`, and verify visually using the repository's 4-tier test & visual PNG inspection workflow (`tests/TESTING.md` and `tests/run_all.sh`) against baseline sheets in `target/test_exports/visual/`.
- **Duplicate `read-fonts` / `skrifa` versions in `Cargo.lock`** while `eframe 0.37` still pulls `read-fonts 0.41` / `skrifa 0.44` → **Mitigation**: `bans.multiple-versions` is `"warn"` (non-blocking in CI), and once `eframe` updates downstream, versions will converge.
- **Future `accesskit` / `zbus` updates** → **Mitigation**: When `eframe` updates past `zbus 5.x`, `cargo-deny` will report unused ignore entries or clean passes, making cleanup straightforward.

## Migration Plan

1. Preserve or snapshot baseline visual PNGs from `target/test_exports/visual/` (if needed for before/after visual comparison).
2. Add `skrifa` (and `read-fonts` >= 0.47 if direct `TableProvider` / `tables::math` imports are cleaner) in `Cargo.toml` and remove `ttf-parser` and `ab_glyph`.
3. Port `src/render/font/metadata.rs`, `src/render/font/selection.rs`, `src/render/font/geometry.rs`, and `src/render/font.rs`.
4. Add/update unit tests in `src/render/font.rs`, run `./tests/run_all.sh` (all 4 tiers including `export_verification`), and visually inspect `target/test_exports/visual/` PNGs via `view_file` per `tests/TESTING.md`.
5. Update `deny.toml` (`unmaintained = "all"`, explicit 3-entry `ignore`, `wildcards = "deny"`) and verify `cargo deny --all-features --workspace --locked check advisories licenses bans sources` passes cleanly.
6. Rollback is trivial via reverting the single change commit since no persisted user data or output format schema changes.
