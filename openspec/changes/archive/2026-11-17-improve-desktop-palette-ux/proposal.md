# Proposal

## Why

In the desktop equation editor, the left sidebar currently displays 20 text-only category buttons stacked in a single vertical column, forcing users to scroll vertically and read long English labels to locate math symbols during daily work. In addition, when a category contains more symbols or templates than fit across the 3-row horizontal palette, standard vertical mouse-wheel scrolling does not pan the palette horizontally.

## What Changes

- Consolidate the 20 underlying catalog groups in the desktop UI into a compact set of visual categories (including a Frequently Used / Recent category) arranged in a 2-column math-icon grid in the left sidebar so all categories fit without vertical scrolling at the minimum window height (`560px`).
- Replace text-only sidebar category labels with rendered mathematical symbol/template icons (with hover tooltips showing category names) using the existing math preview renderer.
- Translate vertical mouse-wheel input while hovering over the 3-row horizontal palette into smooth horizontal scrolling whenever the palette overflows horizontally.
- Preserve `eqntide::catalog::GROUPS` and all underlying catalog entries unchanged so LaTeX parsing, CLI, and Office export compatibility remain unaffected.

## Capabilities

### New Capabilities
- `desktop-palette-navigation`: Compact 2-column math-icon category grid in the desktop sidebar (with consolidated visual categories and recent selections) and vertical mouse-wheel-to-horizontal scrolling on the symbol/template palette.

### Modified Capabilities
<!-- None: no existing specs exist in openspec/specs/. -->

## Impact

- `desktop/src/app.rs`: `sidebar`, `palette_ui`, `editor_ui` (matrix row/column controls trigger), icon atlas/thumbnail caching, recent-item tracking, and desktop UI tests covering sidebar visibility and palette layout.
- `desktop/src/palette.rs`: Visual category grouping definitions mapping UI categories to underlying `eqntide::catalog` group IDs and representative preview expressions.
- `src/catalog.rs` and `tests/export_verification.rs`: Unchanged; the 20 canonical catalog groups remain stable.
