# Design

## Context

See [proposal.md](proposal.md) for motivation. Currently [`desktop/src/palette.rs`](../../../desktop/src/palette.rs) returns a flat `Vec<&'static PaletteItem>` from `items_for_ui_group`, discarding the logical subgroup boundaries authored in `palette_order`, while [`desktop/src/app/palette_ribbon.rs`](../../../desktop/src/app/palette_ribbon.rs) computes tile coordinates purely from `index / 3` and `index % 3`.

## Goals / Non-Goals

**Goals:**
- Preserve the 3-row fixed palette height, `46x32` tile geometry, and drag/wheel horizontal scrolling.
- Give every semantic subgroup—and every 2-item symmetric pair—a clean column start at row 0, including multi-column 2-row subgroups where adjacent runs need a fresh column at row 0.
- Curate `Fractions` to 4 clean columns (`[6, 2, 2]`): 2 fraction columns, 1 root column (`sqrt`, `root`), and 1 trailing column (`long-division`, `binom`), removing the 3 redundant composite entries (`small-root`, `root-fraction`, `long-division-quotient`), and remove `int-differential`, `definite-differential`, `aligned`, `gathered`, `plain-2x3`, `plain-3x2`, `Bmatrix2`, `Bmatrix3`, `vec`, `tg`, `ctg`, `cotg`, `cosec`, `lg`, `arsinh`, `arcosh`, `artanh`, and `arcoth` (keeping `\vec` and those function names' parser/conversion support and command completion intact) while keeping the SVG/atlas sync invariant intact.
- Use a slightly wider horizontal gap between subgroups (`theme::SPACE_LG`, `12px`) than between columns inside the same subgroup (`theme::SPACE_XS`, `4px`), with no vertical divider lines.
- Keep `Recent` continuously packed and place the 3-tier `MatricesAndBoxes` tool block (`Custom` `104x40` on top + 2×2 `+ row` / `− row` / `+ col` / `− col` `50x28` buttons below) at the end of `MatricesAndBoxes`.

**Non-Goals:**
- Adding vertical divider strokes or flyout/dropdown submenus.
- Reordering `Recent` into subgroups.

## Decisions

### 1. Explicit subgroup metadata in `desktop/src/palette.rs`
- **Decision:** Add a subgroup-returning query (`subgroups_for_ui_group(ui_group_id, recent) -> Vec<PaletteSubgroup>`) where each `PaletteSubgroup` holds one or more intra-subgroup column `runs` (`|` separator inside subgroup specs), while keeping `items_for_ui_group` available as the flattened view for existing callers/tests. Define explicit subgroups for all ordered catalog groups, including `Sums`, `LabeledArrows`, and `Spaces` (which previously fell through to `_ => ""`).
- **Rationale:** Deriving subgroups purely from line breaks in `palette_order` works for most groups, while explicit subgroup boundaries plus intra-subgroup `|` column breaks keep 2-item pair splits (`Fences`, `Matrices`, `Bars`), 2×2 `Embellishments` harpoons (`overleftharpoon/underleftharpoon | overrightharpoon/underrightharpoon`), and 2×2 `Boxes` pairs (`top-left/top-right | bottom-left/bottom-right` and `top-bottom-left/top-bottom-right | top-left-right/bottom-left-right`) unambiguous and testable without hardcoding row heights inside `palette.rs`.
- **Alternative considered:** Embedding sentinel gap markers in `items_for_ui_group`. Rejected because mixing layout sentinels into item lists pollutes callers expecting real `PaletteItem` entries.

### 2. Column-level coordinate computation in `desktop/src/app/palette_ribbon.rs`
- **Decision:** Compute per-tile `Rect` and total `grid_width` by walking subgroups sequentially:
  - Within each run of a subgroup (`n` items), item `k` sits at `(col_offset + k / 3, k % 3)` with intra-subgroup column step `PALETTE_BUTTON_SIZE.x + PALETTE_GAP` (`46 + 4 = 50px`), and subsequent runs within the same subgroup start at row 0 of a new column separated by `PALETTE_GAP` (`4px`).
  - Advancing from one non-empty subgroup to the next inserts `SUBGROUP_GAP = theme::SPACE_LG` (`12px`) after the previous subgroup's last column.
  - For `MatricesAndBoxes`, treat the trailing matrix tool block (`Custom` `104x40` above 2×2 `50x28` `+ row` / `− row` / `+ col` / `− col` buttons, totaling `104x104`) as a trailing subgroup after `Boxes` with `SUBGROUP_GAP` separation, removing the separate horizontal action bar from `desktop/src/app/canvas.rs`.
  - For `Recent`, return all MRU items as a single continuous run/subgroup so layout degrades cleanly to continuous 3-row packing.
- **Rationale:** Precomputing tile offsets in `build_palette_layout` is `O(N)` over ~10–75 items per frame and integrates seamlessly with existing `ScrollArea`, `max_offset`, and drag/wheel clamping.

### 3. Subgroup granularity for 2-item pairs, 4-item groups, and `Fractions` / `Integrals` / `Matrices` / `Boxes` / `Embellishments` / `Functions` curation
- **Decision:** Split paired templates (`cases-two/three`, `system-brace-two/three`, `system-bracket-two/three`, `pmatrix2/3`, `bmatrix2/3`, `vmatrix2/3`, `Vmatrix2/3`, and over/under `Bars` pairs) into 2-item subgroups using the top 2 rows and leaving row 3 blank. Group the `Embellishments` accents as `[6, 6, 4, 5, 4]` after removing `vec` (which duplicated `\overrightarrow{#?}` in `Bars`, while keeping `\vec` parsing, conversion, attachment behavior, and command completion intact), pairing the harpoons (`overleftharpoon/underleftharpoon | overrightharpoon/underrightharpoon`) as a 2-column 2×2 subgroup, and group `Boxes` as `[2, 2|2, 2|2]` (`box/roundedbox`, `top-left/top-right | bottom-left/bottom-right`, and `top-bottom-left/top-bottom-right | top-left-right/bottom-left-right`), so the four directional box columns form two 2-column 2×2 subgroups. Group `Functions` as `[6, 6, 6, 3, 7, 11]` after removing redundant/regional/uncommon entries (`tg`, `ctg`, `cotg`, `cosec`, `lg`, `arsinh`, `arcosh`, `artanh`, `arcoth`) from the palette while keeping them in `FUNCTIONS` and `COMMANDS` for conversion and command completion. Group `Fractions` as `[6, 2, 2]` (`frac..sfrac`, `sqrt`/`root`, `long-division`/`binom`) after removing `small-root`, `root-fraction`, and `long-division-quotient`, remove `int-differential` and `definite-differential` from `Integrals` (remapping `Ctrl+I` to `\int` and `Ctrl+Shift+I` to `\int\limits_{#?}^{#?} #?`), remove `aligned`, `gathered`, `plain-2x3`, `plain-3x2`, `Bmatrix2`, and `Bmatrix3` from `Matrices`, and regenerate the 526-entry SVG/atlas bundle via `scripts/generate_palette_icons.py`. Group `Scripts` as `[6, 9, 3]` (single-sided scripts, pre+post combinations with symmetric column ordering, overset/underset labels). Keep `LabeledArrows` (12×3), `Sums`/`BigOperators` (10×5), `Integrals` (`[5, 5, 5, 9, 6]`), and list-based categories (`Operators`, `Relations`, `Arrows`, `Logic`, `Sets`, `Symbols`, `GreekLower`, `GreekUpper`) grouped by their natural semantic lines.
- **Rationale:** Splitting 2-item pairs prevents awkward mid-column splits (e.g., `bmatrix2` at row 2 of column N and `bmatrix3` at row 0 of column N+1), pairing related 2-row harpoon and box columns inside single subgroups keeps left/right harpoons, corner boxes, and open-sided boxes together while preserving blank bottom rows, removing `vec` and redundant `Functions` duplicates eliminates awkward sparse columns, and curating `Fractions`, `Integrals`, and `Matrices` removes redundant tiles and aligns every category cleanly.

## Risks / Trade-offs

- **Slightly wider horizontal scroll range** → User explicitly prefers horizontal scrollability (already supported via drag and wheel) over jagged mid-column wraps.
- **Existing layout test assumptions** → `palette_fills_columns_in_catalog_and_recent_order` in `desktop/src/app/tests/palette_tests.rs` and `button_height_tiers_align_footer_tabs_and_compact_controls` / `palette_scrolls_horizontally_with_either_wheel_axis` in `desktop/src/app/tests/layout_theme_tests.rs` assumed dense packing, separate matrix action strip, or 13 items in `Fractions`; updating those assertions resolves the discrepancy.
