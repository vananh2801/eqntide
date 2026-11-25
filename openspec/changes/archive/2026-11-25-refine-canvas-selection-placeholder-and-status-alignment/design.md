# Design

## Context

See `proposal.md` for motivation. Three visual polish items and two selection/caret boundary issues affect the desktop crate (`desktop/src/app/canvas.rs`, `desktop/src/theme.rs`, and `desktop/src/app/mod.rs`) and `eqntide-editor` (`crates/eqntide-editor/src/editing.rs`):
- Equation Canvas selection previously looped over `Editor::selected_ids()` and painted per-child `6px`-rounded pills with uneven tops/bottoms and gaps between symbols.
- `Editor::selection_span()` previously expanded any cross-node endpoint (`path.len() > common_depth`) with `+ 1`, even when the caret was at a non-empty token/row leading (`offset == 0`) or trailing (`offset == len`) boundary, causing pointer drags before a character's horizontal midpoint to highlight the untouched character.
- In `desktop/src/app/canvas.rs`, `caret_rect` rendered `prev.right` for `Caret { node: prev, offset: len }` and `next.x` for `Caret { node: row, offset: next_idx }` / `Caret { node: next, offset: 0 }`, causing the caret bar to jump across TeX inter-element spacing (`lspace`) on tiny mouse nudges even when the active selection did not change, and `hit_test_caret` dropped immediately into the next sibling once passing `prev.right + 1.0`.
- Empty placeholders (`paint_placeholder`) previously painted an opaque `placeholder_bg` tint and used `CONTROL_CORNER_RADIUS` (`6px`).
- `draw_status_bar` previously used `ui.horizontal` (top-aligned inside the `32px` bottom panel) and `ui.label("●" / "○")`.

## Goals / Non-Goals

**Goals:**
- Paint one continuous, lightly rounded (`2px`) selection highlight per active `Editor::selection_span()`, supporting both multi-child spans and intra-token character slices with baseline-aware vertical bounds.
- Ensure `Editor::selection_span()` respects leading/trailing token and row boundaries during drag and `Shift+Arrow` selection while keeping empty placeholder and structural slot selection behavior intact.
- Keep `caret_rect` locked to the active selection edge across inter-element spacing gaps and resolve row-gap clicks at the visual gap midpoint.
- Render placeholders with transparent (`0` alpha) interiors, `2px` corner rounding, and clean dashed strokes while preserving existing `placeholder_stroke` colors.
- Vertically center all status bar elements (`horizontal_centered`) and draw the bridge readiness indicator as a crisp vector circle centered inside its allocated rect.

**Non-Goals:**
- Changing the color palette of `placeholder_stroke` or restyling theme colors globally (deferred to a dedicated recolor change).
- Splitting multi-container selections into partial-token highlights across distinct sibling nodes (`selection_span()` remains a single `(container_id, start, end)` span).

## Decisions

1. **Compute canvas selection geometry from `Editor::selection_span()` and refine boundary/caret alignment**
   - *Canvas choice*: Add `App::selection_rect(&self, origin: Pos2, scale: f32) -> Option<Rect>` driven by `self.editor.selection_span()`.
   - *Token case* (`node.is_token()`): If `region.carets.len() == node.text.chars().count() + 1`, map horizontal bounds to `carets[start].min(carets[end]) .. carets[start].max(carets[end])` (with a small minimum width for zero-width selections); otherwise fall back proportionally across `region.width`. Vertically union `region.y .. region.y + region.height` with `(baseline - 0.72 * em_pt) .. (baseline + 0.22 * em_pt)`.
   - *Container case* (`!node.is_token()`): Union `hit_region(child.id)` across `node.children[start..end]` and include each child's baseline line-box `(baseline - 0.72 * em_pt) .. (baseline + 0.22 * em_pt)` so thin operators (`-`, `=`) stay centered on the math line instead of hanging below `region.y`.
   - *Editor boundary accuracy*: In `Editor::selection_span()`, resolve each cross-container endpoint via `is_at_leading_edge` and `is_at_trailing_edge` (sharing `has_transparent_edge`) down through non-empty rows, style wrappers, and base/script slots (`msup`/`msub`/`msubsup`) to map leading boundaries (`offset == 0`) to `(child_idx, child_idx)` and trailing boundaries (`offset == len`) to `(child_idx + 1, child_idx + 1)`, while leaving empty placeholders (`is_effectively_empty`), `<mfrac>`, and `<msqrt>` as `(child_idx, child_idx + 1)` so matrix cell drags and structural deletions continue to work identically.
   - *Caret alignment across spacing gaps*: Add private helpers `selection_caret_x` and `is_caret_at_or_before` in `desktop/src/app/canvas.rs` so `caret_rect` snaps its X coordinate to `selection_rect.right()` (forward drag) or `selection_rect.left()` (backward drag) whenever the caret sits on the outer boundary of the active selection (excluding interior `mfrac`/`mroot`/`msqrt` slots). In `hit_test_caret`, split inter-token row gaps at `(prev_region.x + prev_region.width + next_region.x) * 0.5` so clicking on the left half of a spacing gap targets `Caret { node: prev_child.id, offset: len }`.
   - *Radius*: Use a dedicated `CANVAS_OVERLAY_CORNER_RADIUS: CornerRadius = CornerRadius::same(2)` constant (`2px`) shared by both selection highlights and placeholders.

2. **Transparent placeholder interior via `CanvasPalette::placeholder_bg` (`Color32::TRANSPARENT`)**
   - *Choice*: Set `placeholder_bg` to `Color32::TRANSPARENT` in `CanvasPreset::palette()` for both light and dark canvas surfaces, keep `placeholder_stroke` colors intact, and reduce `paint_placeholder` radius from `CONTROL_CORNER_RADIUS` (`6px`) to `CANVAS_OVERLAY_CORNER_RADIUS` (`2px`).
   - *Note*: `paint_placeholder` keeps `painter.rect_filled(rect.expand(0.5), 0.0, pal.bg)` first to mask the synthetic `□` glyph from the equation alpha texture, then skips painting a fill when `pal.placeholder_bg.a() == 0` (or paints `pal.placeholder_bg` if non-transparent). Keeping the `placeholder_bg` field on `CanvasPalette` avoids breaking future theme/recolor work while cleanly expressing zero interior opacity.
   - *Test impact*: `theme.rs` already asserts `assert_ne!(pal.placeholder_bg, pal.bg)`, which holds for `Color32::TRANSPARENT`; we also assert `pal.placeholder_bg == Color32::TRANSPARENT`.

3. **Vector circle + `ui.horizontal_centered` in `draw_status_bar`**
   - *Choice*: Replace `ui.horizontal` with `ui.horizontal_centered` in `App::draw_status_bar`. Allocate a fixed square widget rect (`12x12` DIP with `egui::Sense::hover()`) for the bridge readiness dot, attach `.on_hover_text(&self.bridge_status)`, and paint a `4.0px`-radius circle centered at `dot_rect.center()` using `ui.painter().circle_filled(...)` when `self.bridge_ready` or `ui.painter().circle_stroke(..., Stroke::new(1.5, ...))` when disconnected (using `ui.visuals().text_color()`).
   - *Alternative considered*: Keeping `"●"`/`"○"` text glyph with manual vertical offset. Rejected because font fallback (`Nunito` vs `STIXTwoText`/`STIXTwoMath`) causes inconsistent vertical bearing and glyph sizing across platforms; a vector circle guarantees exact vertical centering relative to `32px` panel height and `24px` compact buttons/text.

## Risks / Trade-offs

- **Empty placeholder and `<mfrac>`/`<msqrt>` boundary behavior** → `is_at_leading_edge` and `is_at_trailing_edge` explicitly return `false` for `is_effectively_empty` nodes and do not descend through `<mfrac>` or `<msqrt>`, preserving empty matrix cell range selection and fraction/radical selections.
- **Caret locking during interior structural drags** → `selection_caret_x` intentionally skips snapping when `head` sits deeper inside `<mfrac>`, `<mroot>`, or `<msqrt>` so fractional/radical vertical caret placement stays intact.
- **Existing status bar test expects `"●"` text** (`layout_theme_tests.rs::status_bar_is_quiet_when_idle_or_busy_and_clears_stale_messages_on_edit`) → Updated to assert the painted `egui::Shape::Circle` (and empty `collect_texts`) plus vertical alignment with the status label / bar centerline.
- **Tessellated dashed rounded rect on small placeholder boxes** → With `2px` corner radius instead of `6px`, straight edges are longer and corners have crisp dashes across standard and zoomed views.
