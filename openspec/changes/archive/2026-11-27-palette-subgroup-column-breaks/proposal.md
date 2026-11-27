# Proposal

## Why

The desktop palette currently packs every ordered item in a category continuously into a 3-row column-major grid (`column = index / 3`, `row = index % 3`). Whenever a semantic subgroup has a count that is not a multiple of 3 (such as the 5-variant operator families in `SumsAndBigOps` and `Integrals`, or symmetric 2-item/4-item pairs in `Fences`, `MatricesAndBoxes`, and `AccentsAndBars`), the following subgroup wraps mid-column and splits related pairs across columns, making categories harder to scan visually. In `Fractions`, three redundant composite entries (`small-root`, `root-fraction`, and `long-division-quotient`) also clutter the radical and trailing columns instead of keeping fractions, roots, and binomial/long-division cleanly grouped.

## What Changes

- Start each semantic subgroup at row 0 of a new column in structured palette categories, leaving unused bottom slots blank (e.g., 2-item pairs occupy the top 2 rows and leave row 3 blank; 5-item families occupy `3 + 2` across two columns).
- Split 2-item pairs and symmetric 4-item or 6-item groups (such as `2x2`/`3x3` matrix variants, `cases`/`system` pairs, and `over`/`under` bar pairs) into distinct 2-item subgroups so pairs stay together in a single column.
- Curate `Fractions` into three clean subgroups (`[6, 2, 2]`): combine the stacked and inline fraction columns into a 6-item subgroup (`frac`, `tfrac`, `dfrac`, `linear`, `small-linear`, `sfrac`), remove the redundant `small-root`, `root-fraction`, `long-division-quotient`, `int-differential`, `definite-differential`, `aligned`, `gathered`, `plain-2x3`, `plain-3x2`, `Bmatrix2`, `Bmatrix3`, `vec`, and redundant/uncommon `Functions` buttons (`tg`, `ctg`, `cotg`, `cosec`, `lg`, `arsinh`, `arcosh`, `artanh`, `arcoth` — while preserving their parsing/conversion and command completion) from the catalog palette and generated SVG/atlas assets, group `sqrt` + `root` into one 2-item column, and pair `long-division` + `binom` in the final 2-item column.
- Use a slightly wider horizontal gap between distinct subgroups (`SPACE_LG` / `12px`, vs. `SPACE_XS` / `4px` between columns within the same subgroup) without adding vertical divider lines.
- Keep `Recent` as a continuous, ungrouped 3-row MRU sequence, and place `Custom matrix` in its own column at the end of `MatricesAndBoxes` after the final catalog subgroup.

## Capabilities

### New Capabilities

### Modified Capabilities
- `desktop-palette-navigation`: Update the three-row column-order palette layout requirement so categorized subgroups start on a fresh column with wider inter-subgroup spacing instead of wrapping mid-column across subgroup boundaries.

## Impact

- `src/catalog/mod.rs`, `scripts/palette_svg_builders.py`, `assets/icons/sources/palette/`, `assets/icons/generated/palette-atlas/`, and `tests/python/test_icons.py`: Remove redundant palette entries and regenerate the 526-entry SVG set and sprite atlas.
- `desktop/src/palette.rs`: Represent explicit subgroup boundaries for ordered categories (including `Sums`, `LabeledArrows`, and `Spaces`) alongside item ordering.
- `desktop/src/app/palette_ribbon.rs`: Compute tile `(x, y)` positions and total `grid_width` from subgroup column assignments and inter-subgroup gaps instead of flat `index / 3` and `index % 3`.
- `desktop/src/app/tests/palette_tests.rs`, `desktop/src/app/tests/layout_theme_tests.rs`, & `openspec/specs/desktop-palette-navigation/spec.md`: Update layout assertions and specification scenarios for subgroup column breaks and `Custom matrix` placement.
