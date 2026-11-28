# Design

## Context

See `proposal.md` for motivation and capability scope.

Currently, `eqntide` mixes `STIXTwoText-{Regular,Italic,Bold}` (text, functions, numbers, variables) with `STIXTwoMath-Regular` (symbols, `ssty` script alternates, OpenType `MATH` table). Because of this hybrid font stack and MathType/OMML compatibility requirements, TeX Appendix G rules cannot be copied verbatim from Computer Modern `.tfm` assumptions; they must be adapted selectively where they visibly improve STIX2 rendering without regressing existing specs (`math-font-backend`, `mathml-render-and-arrow-parity`) or canvas hit-testing (`LBox::hits` / `RawHit`).

## Goals / Non-Goals

**Goals:**
- Complete Phase 1 + Phase 2 of gradual layout standardization:
  1. Expose remaining OpenType `MATH` constants on [`MathConstants`](file:///home/vananh2801/Documents/mathtype-convert/eqntide/src/render/font.rs) as the foundation for future phases.
  2. Eliminate the duplicate `AXIS` (`0.258`) constant in [`src/render/layout/mod.rs`](file:///home/vananh2801/Documents/mathtype-convert/eqntide/src/render/layout/mod.rs) in favor of `self.font_set.constants().axis_height`.
  3. Make [`Layout::spacing`](file:///home/vananh2801/Documents/mathtype-convert/eqntide/src/render/layout/row.rs) aware of `MathContext` (`Display`/`Text` vs `Script`/`ScriptScript`) per *The TeXbook* Ch. 18 / Rule 20.
- Validate changes both via unit/workspace tests and direct AI visual inspection of `target/test_exports/visual/` per [`tests/TESTING.md`](file:///home/vananh2801/Documents/mathtype-convert/eqntide/tests/TESTING.md).

**Non-Goals:**
- Changing fraction numerator/denominator scaling (`for_numerator` / `for_denominator` stay 1.0 in Display/Text for MathType/OMML parity).
- Replacing custom vector/piece assembly for stretchy arrows (`arrow.rs`), contour integrals (`integral.rs`), delimiters (`delimiter.rs`), or vector accents (`embellish.rs`).
- Reworking script positioning, big-operator limits, or radical degree geometry in this change (those belong to follow-up phases once `MathConstants` and visual baselines are in place).

## Decisions

### 1. Cherry-pick OpenType `MATH` + Appendix G instead of literal TeXbook math
- **Decision**: Treat OpenType `MATH` constants from `STIXTwoMath-Regular.ttf` as the primary metric source and adopt only TeXbook Appendix G rules that fit STIX2 and MathType/OMML conventions.
- **Alternative considered**: Implementing TeXbook Appendix G rules 1–22 literally (shrinking `\frac` parts in text style, discrete `\delimiterfactor` jumps, TeX `skewchar` accent placement). Rejected because it breaks MathType/OMML parity and conflicts with `STIXTwoText` + `STIXTwoMath` glyph metrics and existing `mathml-render-and-arrow-parity` requirements.

### 2. Style-aware spacing table in `Layout::spacing`
- **Decision**: Pass `ctx: MathContext` into `Layout::spacing(&self, left: Class, right: Class, em: f32, ctx: MathContext)`. When `!ctx.is_script()`, keep the current `THIN`/`MED`/`THICK` rules unchanged. When `ctx.is_script()`, suppress `MED` (`Bin`) and `THICK` (`Rel`) and `Punct` padding, retaining only `THIN` (`3/18 em`) between `Ord`/`Close`/`Op` and `Op` (matching TeXbook p. 170's parenthesized `(3)` entries in script styles).
- **Alternative considered**: Introducing `Class::Inner` simultaneously for fenced groups. Deferred to avoid unintended spacing changes around `\left...\right` before evaluating script spacing alone on visual inspection sheets.

### 3. Centralize `axis_height` via `Layout::axis_height(&self, em: f32) -> f32` (or `constants.axis_height * em`)
- **Decision**: Remove `pub(crate) const AXIS: f32 = 0.258` and read `self.font_set.constants().axis_height * em` wherever `AXIS * em` was used. In unit tests outside `Layout` (such as `integral.rs` tests), read `fonts().constants().axis_height`.
- **Rationale**: Single source of truth tied directly to the font table; zero visual change for `STIXTwoMath` (`0.258`), while eliminating drift if font metrics are ever updated.

### 4. Visual regression gate using `tests/TESTING.md`
- **Decision**: Run `cargo test --test export_verification -- --nocapture` before/after layout changes and inspect `target/test_exports/visual/14_roundtrip_p1.png`, `14_roundtrip_p2.png`, `15_layout_edge_cases.png`, and relevant sheets with `view_file` against the 5 criteria in `tests/TESTING.md`.
- **Rationale**: Pure geometric unit tests cannot catch subtle visual crowding in `x^{a+b}` or `STIXTwoText`/`STIXTwoMath` baseline harmony.

## Risks / Trade-offs

- **Risk**: Suppressing `Punct` or `Bin`/`Rel` spacing in `Script`/`ScriptScript` could make certain dense subscripts (e.g., `f_{a,b}` or `x^{i+1}`) feel slightly tight with `STIXTwoText-Italic` sidebearings → **Mitigation**: Inspect rendered PNGs in `target/test_exports/visual/` and add a targeted `export_verification` or `row.rs` visual check for scripted expressions (`x^{a+b}`, `A_{i,j}`) before finalizing.
- **Risk**: `RawHit` caret positions in `desktop` editor for scripted expressions shift horizontally when `MED`/`THICK` spacing is removed → **Mitigation**: `LBox::place` translates `hits` automatically with each child box; verify with `cargo test --workspace --all-targets`.
- **Risk**: Reading additional `read_fonts::tables::math::MathConstants` fields could behave unexpectedly if a field is optional (`Option<MathValueRecord>`) in `read-fonts` → **Mitigation**: Check exact `read-fonts` method signatures and fall back to verified STIX2 defaults cleanly without panics.
