# Design

## Context

See `proposal.md` for motivation. In `desktop/src/app.rs`, `editor_ui` places `Panel::bottom("latex-panel").exact_size(60.0)` with `8px` top margin, and `latex_ui` uses `ui.add_sized([ui.available_width(), 48.0], TextEdit::multiline(...))` without a `ScrollArea`. Because `ui.add_sized` uses `Layout::centered_and_justified`, multi-line growth expands both upward under the `CentralPanel` canvas and downward past the footer boundary. Meanwhile, `desktop/src/main.rs` initializes the viewport at `[1100.0, 650.0]`, inflating the `CentralPanel` canvas height to `374px` while `MIN_WINDOW_SIZE` is `[1040.0, 560.0]`.

## Goals / Non-Goals

**Goals:**
- Prevent the LaTeX source box from expanding beyond its allocated panel or colliding with the Equation Canvas and footer when newlines are entered.
- Provide ~4–5 visible lines (`88px` panel / `80px` input height) with smooth vertical scrolling inside the LaTeX box.
- Launch at `[1040.0, 560.0]` so the initial Equation Canvas size is compact (~`858x256px`) without violating sidebar or three-row palette invariants.

**Non-Goals:**
- Auto-resizing `latex-panel` dynamically based on line count.
- Changing `MIN_WINDOW_SIZE` (`1040x560`), sidebar grid layout, or palette row constraints.

## Decisions

1. **Fixed outer `Frame` + `ScrollArea::vertical()` + frameless `TextEdit::multiline` in `latex_ui`**
   - *Choice*: Render a single outer `egui::Frame` sized to the fixed inner height (`80px`) with `pal.bg` and `CONTROL_CORNER_RADIUS` (`6px`), paint a single `Inside` `1px` `pal.border` stroke on the outer rect (no bright focus ring override), and place `egui::ScrollArea::vertical().id_salt("latex-scroll").auto_shrink([false, false])` (with `bar_outer_margin = 4.0`) inside it, containing a transparent frameless `TextEdit::multiline(&mut self.latex)` (`.desired_rows(1)` + `.min_size([width, box_height])`, `inner_margin` `{ left: 8, right: 12, top: 6, bottom: 6 }`).
   - *Rationale*: Painting the background and `Inside` stroke once on the outer rect gives crisp `6px` rounded corners without double-layer fills. Using `desired_rows(1)` with `min_size` equal to `box_height` avoids spurious overflow so the scrollbar stays hidden when empty or short and only appears when lines actually exceed `80px`. Keeping `egui::Id::unique("latex-source")` on `TextEdit` preserves existing focus-tracking and test lookups (`ctx.read_response(Id::unique("latex-source"))`), with `Frame` alignment matching the canvas left/right edges.
   - *Alternative considered*: Dynamically growing `latex-panel` up to a max row count. Rejected per user preference for a stable, fixed-height scrollable box that avoids shifting the Equation Canvas while typing.

2. **Increase `latex-panel` height from `60px` to `88px` (`8px` top gap + `80px` box)**
   - *Choice*: Set `LATEX_PANEL_HEIGHT` to `88.0` (leaving `80.0` below `CONTENT_PADDING = 8.0`).
   - *Rationale*: At `13px` monospace font, `~4–5` lines fit comfortably without feeling cramped, and at `1040x560` the remaining canvas height (`560 - 32 - 56 - 16 - 112 - 88 = 256px` outer / `~230px` inner scroll height) remains generous for multi-row fractions, integrals, and matrices.

3. **Set default window size in `desktop_viewport` to `[1040.0, 560.0]`**
   - *Choice*: Update `.with_inner_size([1100.0, 650.0])` in `desktop/src/main.rs` to `.with_inner_size(app::MIN_WINDOW_SIZE)`.
   - *Rationale*: Reuses `MIN_WINDOW_SIZE` directly, keeps all existing `1040x560` layout tests and `desktop-ui-theme` invariants intact, and trims excess initial canvas height immediately.

## Risks / Trade-offs

- [Crisp corners without double background] → Keep `TextEdit`'s inner `Frame::NONE` transparent (no `fill` or `corner_radius`) and paint the `1px` `pal.border` stroke once with `StrokeKind::Inside` around the outer `Frame` rect.
- [Hidden scrollbar until needed] → Set `desired_rows(1)` with `min_size` equal to the viewport height so empty/short input does not exceed `box_height`, and set `bar_outer_margin = 4.0` so the scrollbar clears the `6px` rounded corners when overflow occurs.
- [Existing layout test `rect` assertions on `latex-source`] → Verify in unit tests that `latex-source` `rect` stays aligned horizontally with the canvas frame and strictly below `canvas.bottom()` even with 6+ lines of LaTeX.
