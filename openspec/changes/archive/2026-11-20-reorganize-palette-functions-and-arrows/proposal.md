# Proposal

## Why

The desktop equation editor currently mixes single-glyph arrow symbols and multi-slot labeled arrows into a single `Arrows` sidebar category, making it slower to scan for plain arrows vs. labeled arrow templates, and lacks a dedicated `Functions` palette category (leaving `\lim`, `\min`, and `\max` inside `Scripts` with no palette buttons for `sin`, `cos`, `tan`, `log`, or other standard mathematical functions). In addition, the underlying function registries across the LaTeX parser, MathML-to-LaTeX serializer, renderer, OMML encoder, and MTEF encoder are out of sync—most notably `\liminf`/`\limsup` parsing to spaced strings `"lim inf"`/`"lim sup"` that miss upright function recognition and MTEF/OMML roundtripping for several inverse/hyperbolic and regional function names.

## What Changes

- Add a new `Functions` catalog group in `src/catalog.rs` and map it to a new `Functions` category tile (`math-sin` Tabler icon) in `desktop/src/palette.rs`.
- Move the three placeholder-bearing limit/extremum templates (`\lim_{#? \to #?}`, `\min_{#?}`, `\max_{#?}`) from `Scripts` to `Functions`.
- Populate `Functions` with the full set of plain (slot-free) named mathematical functions derived from the codebase (`sin`, `cos`, `tan`, `cot`, `sec`, `csc`, `arcsin`, `arccos`, `arctan`, `arccot`, `arcsec`, `arccsc`, `sinh`, `cosh`, `tanh`, `coth`, `sech`, `csch`, `arsinh`, `arcosh`, `artanh`, `arcoth`, `tg`, `ctg`, `cotg`, `cosec`, `log`, `ln`, `lg`, `exp`, `limsup`, `liminf`, `sup`, `inf`, `det`, `dim`, `ker`, `hom`, `deg`, `arg`, `gcd`, `lcm`, `sgn`, `mod`, `Pr`), so users can attach superscripts/subscripts like `\sin^2` or `\log_2` naturally after insertion.
- Split the desktop `Arrows` sidebar category into two separate tiles:
  - `Arrows` (`arrows-left-right` icon): single-glyph arrow symbols only (`catalog_groups: &["Arrows"]`).
  - `LabeledArrows` (`arrow-autofit-right` icon): arrow templates with placeholder slots (`catalog_groups: &["LabeledArrows"]`).
- Update `scripts/render_icons.py`, rendered PNG assets/manifest, and `desktop/src/app.rs` to bundle `math-sin` and `arrow-autofit-right`, and remove redundant double sidebar vertical margins so all 14 category tiles (7 rows × 2 columns at `42px` tile height) and bottom navigation tabs fit without vertical scrolling at `1056x560`.
- Synchronize function-name handling and LaTeX/MathML/OMML/MTEF roundtripping across `src/symbols/functions.rs`, `src/latex_extensions.rs`, `src/mathml_to_latex.rs`, `src/omml/encode.rs`, `src/mtef/encode.rs`, and `src/catalog.rs` (including resolving the `"lim inf"`/`"lim sup"` spacing mismatch and ensuring all newly surfaced functions parse, render upright, and convert cleanly to/from LaTeX, OMML, and MTEF/OLE).

## Capabilities

### New Capabilities

- None

### Modified Capabilities

- `desktop-palette-navigation`: Expand the sidebar category grid from 12 to 14 tiles (`72x42` across 7 rows fitting without vertical scrolling at `1056x560`), add the `Functions` tile (`math-sin`), and split `Arrows` (`arrows-left-right`) from `LabeledArrows` (`arrow-autofit-right`).
- `latex-command-standardization`: Standardize named mathematical function parsing, upright function recognition, canonical explicit LaTeX serialization (`\sin`, `\limsup`, `\operatorname{...}`), and MathML/OMML/MTEF parity across the complete function set.

## Impact

- Core library: `src/catalog.rs`, `src/symbols/functions.rs`, `src/latex_extensions.rs`, `src/mathml_to_latex.rs`, `src/omml/encode.rs`, `src/mtef/encode.rs`.
- Desktop app & assets: `desktop/src/palette.rs`, `desktop/src/app.rs`, `scripts/render_icons.py`, `assets/icons/rendered/manifest.json` (plus `math-sin.png`, `arrow-autofit-right.png`).
- Tests & specs: `desktop/src/palette.rs`, `desktop/src/app.rs`, `desktop/src/commands.rs`, `src/catalog.rs`, and roundtrip tests covering LaTeX/OMML/MTEF/render parity; `openspec/specs/desktop-palette-navigation/spec.md` and `openspec/specs/latex-command-standardization/spec.md`.
