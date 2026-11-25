# Proposal

## Why

Visual polish and pointer selection accuracy currently detract from the Equation Canvas and bottom status bar in the desktop application:
1. Equation Canvas selections render individual `6px`-rounded pills per AST node—with uneven heights, gaps between symbols, and downward overhang on thin operators like `-` and `=`—instead of a clean, contiguous selection highlight. In addition, `Editor::selection_span()` includes an adjacent token/element even when a drag or shift-selection caret sits at its leading `offset == 0` boundary (before passing half the glyph width), and `caret_rect` jumps horizontally across inter-element spacing (`lspace`) between `prev.right` and `next.x` when nudging the pointer across a token boundary while the selection stays unchanged.
2. Empty structural placeholder boxes (`□`) use an opaque filled background (`placeholder_bg`) and an overly large `6px` corner radius that turns small slots into squircles with clumped corner dashes.
3. The bottom status bar lays out its items with top-aligned `ui.horizontal` and a text-glyph dot (`●`/`○`), leaving the readiness dot and status text visually off-center inside the `32px` bar.

## What Changes

- **Contiguous, lightly rounded Equation Canvas selection highlight & boundary precision**:
  - Replace the per-node highlight loop with a unified `selection_rect` derived from `Editor::selection_span()` (merging sibling node bounds and baseline line-height for multi-item spans, and slicing token `carets[start..end]` for intra-token selections) with a light `2px` corner radius (`CANVAS_OVERLAY_CORNER_RADIUS`).
  - Refine `Editor::selection_span()` so endpoints at a non-empty token/row leading (`offset == 0`) or trailing (`offset == len`) boundary map to the exact inter-child boundary without over-including untouched adjacent elements.
  - Lock boundary `caret_rect` X coordinates to the moving edge of `selection_rect` (`selection_caret_x` / `is_caret_at_or_before`) and split row-gap pointer hit-testing at the inter-token gap midpoint so small pointer movements across inter-element spacing neither shift the caret nor misplace it.
- **Transparent-interior, lightly rounded dashed placeholders**: Keep the existing placeholder border stroke color unchanged, reduce placeholder box corner rounding from `6px` to `2px` so the dashed outline renders cleanly along all four sides, and set the inner placeholder fill (`placeholder_bg`) opacity to `0` (`Color32::TRANSPARENT`).
- **Vertically centered status bar with vector readiness dot**: Switch `draw_status_bar` to vertically centered horizontal layout (`horizontal_centered`) so the indicator, compact action buttons, and status text share the vertical centerline of the `32px` status bar, and paint the bridge readiness dot as a crisp vector circle (filled when connected, stroked when disconnected) instead of a font glyph.

## Capabilities

### New Capabilities
<!-- None -->

### Modified Capabilities
- `desktop-canvas-editing`: Add contiguous span-based selection highlight rendering on the Equation Canvas with light `2px` corner rounding, baseline-aware height bounds, leading/trailing token boundary accuracy during drag selection, and selection-edge caret alignment across inter-element spacing.
- `desktop-ui-theme`: Refine Equation Canvas overlay styling so dashed placeholders render with `2px` corner rounding and transparent (`0` opacity) interiors while retaining current stroke colors, and selection highlights use `2px` corner rounding.
- `desktop-status-bar`: Render the bridge readiness indicator as a vertically centered vector circle alongside vertically centered status text and compact action buttons within the `32px` status bar.

## Impact

- **Editor core & Desktop UI**: `crates/eqntide-editor/src/editing.rs` (`Editor::selection_span()`, `is_at_leading_edge`, `is_at_trailing_edge`, `has_transparent_edge`), `desktop/src/app/canvas.rs` (`draw_canvas`, `paint_placeholder`, `selection_rect`, `caret_rect`, `hit_test_caret`, `selection_caret_x`, `is_caret_at_or_before`), `desktop/src/theme.rs` (`CanvasPalette` `placeholder_bg` transparency and `CANVAS_OVERLAY_CORNER_RADIUS`), and `desktop/src/app/mod.rs` (`draw_status_bar`).
- **Tests**: `crates/eqntide-editor/src/tests.rs` (boundary selection unit tests), `desktop/src/theme.rs` palette tests, `desktop/src/app/tests/layout_theme_tests.rs` (status bar vector circle shape + vertical centering assertions), and `desktop/src/app/tests/canvas_input_tests.rs` (unified selection geometry, drag-before-midpoint exclusion, caret-edge lock across operator gaps, and transparent `2px`-rounded placeholders).
