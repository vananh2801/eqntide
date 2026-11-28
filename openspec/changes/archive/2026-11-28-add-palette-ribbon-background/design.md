# Design

## Context

See [proposal.md](./proposal.md) for motivation. Currently, `App::draw_shell` (`desktop/src/app/mod.rs`) mounts the outer chrome panels (`app-status`, `app-sidebar`, `app-footer`) using `ThemeTokens::panel_bg`, then delegates the remaining area inside `CentralPanel` (`tokens.app_bg`) to `App::draw_editor` (`desktop/src/app/canvas.rs`), which nests `templates-panel` (`Panel::top` with `Frame::NONE`) above the Equation Canvas and LaTeX source box.

Because `CentralPanel` uses `tokens.app_bg` (transparent in **Image** background mode) and `templates-panel` has no background fill, palette tiles float directly over the wallpaper/scrim.

## Goals / Non-Goals

**Goals:**
- Give the top palette ribbon the same chrome panel surface (`tokens.panel_bg`), horizontal/vertical inner padding, and `1px` `tokens.border` divider treatment as the sidebar and footer.
- Automatically tie palette background translucency to the existing **Overlay opacity** setting without double-alpha compositing.
- Preserve `MIN_WINDOW_SIZE` (`1056x560`) and `LATEX_PANEL_HEIGHT` (`88px`) while maintaining `8px` vertical padding around the palette tiles and above the Equation Canvas by letting the Equation Canvas absorb the `8px` height reduction.

**Non-Goals:**
- Introducing a separate opacity slider or color token for the palette bar.
- Changing palette tile dimensions (`46x32`), 3-row layout, horizontal drag/wheel scrolling, or sidebar width (`174px`).

## Decisions

1. **Top Chrome Panel in `draw_shell` vs. Rounded Card in `CentralPanel`**
   - **Decision**: Promote `templates-panel` to a `Panel::top` inside `App::draw_shell` (ordered immediately after `app-sidebar` and `app-footer` when `self.tab == 0`, before `CentralPanel`).
   - **Rationale**: Ordering `Panel::top` after `Panel::left("app-sidebar")` constrains its x-range to `[sidebar_right, window_right]` while letting it sit flush against the window top edge (`y = 0`), matching the sidebar and footer chrome shell without double-painting translucent `panel_bg` over itself or `app_bg`.
   - **Alternative considered**: Rendering an `8px`-rounded `card_bg`/`panel_bg` frame inside `CentralPanel`. Rejected because it leaves `app_bg` gaps above and around the bar instead of matching the sidebar chrome style and either loses contrast (`card_bg` is closer to `control_bg`) or takes an extra `8px` from the Equation Canvas.

2. **Surface Color, Padding, and Equation Canvas Height Adjustment**
   - **Decision**: Style `templates-panel` with `Frame::NONE.fill(tokens.panel_bg).inner_margin(Margin::symmetric(theme::SPACE_LG as i8, CONTENT_PADDING))` (`12px` left/right, `8px` top/bottom) and `exact_size(PALETTE_PANEL_HEIGHT)` where `PALETTE_PANEL_HEIGHT = PALETTE_GRID_HEIGHT + 2.0 * CONTENT_PADDING as f32` (`104px + 16px = 120px`). Keep `CentralPanel`'s `CONTENT_PADDING` (`8px`) top/bottom margins.
   - **Rationale**: Previously, `CentralPanel` contributed `8px` top margin and `templates-panel` contributed `112px` (`104px` grid + `8px` bottom margin), placing the Equation Canvas top at `120px` (`256px` canvas allocation at `560px` height). Moving `templates-panel` out of `CentralPanel` at `120px` height while retaining `CentralPanel`'s `8px` top margin places the Equation Canvas top at `128px` (`248px` canvas allocation). Because `App::draw_canvas` sizes the canvas viewport from `ui.available_height()`, the Equation Canvas automatically adjusts to preserve `8px` padding above the tiles, below the tiles inside the chrome bar, and below the palette divider. Subtracting `2.0 * theme::SPACE_LG` (`24px`, matching the borderless `12px` top/bottom frame margin) instead of the legacy `26px` eliminates the extra `2px` bottom gap so the canvas-to-LaTeX gap matches the `8px` LaTeX-to-footer gap.
   - **Horizontal alignment**: Matching `CentralPanel`'s `theme::SPACE_LG` (`12px`) horizontal margin keeps the left/right edges of the palette viewport aligned with the Equation Canvas and LaTeX source box.

3. **Bottom Region Divider & Disabled Gating**
   - **Decision**: Paint `ui.painter().hline(rect.x_range(), rect.max.y - 0.5, divider_stroke)` at the bottom of `draw_shell` alongside the existing sidebar, footer, and status-bar dividers. Wrap the palette panel contents in the shell's modal/busy `add_enabled_ui` check (`!self.busy && self.pending.is_none() && self.matrix_dialog.is_none() && self.shortcut_conflict.is_none()`) and keep `ui.add_enabled_ui(!self.draft_dirty, ...)` inside `draw_palette` around `draw_palette_grid`.
   - **Rationale**: Keeping `panel_bg` outside `draft_dirty` dimming prevents the top chrome bar from flickering/dimming while typing incomplete LaTeX, while modal dialogs still dim the entire workspace uniformly.

## Risks / Trade-offs

- **Existing layout test expectation** → `fixed_layout_keeps_save_and_source_visible_across_sizes_and_themes` currently asserts `canvas_top == CONTENT_PADDING + PALETTE_HEIGHT` (`120px`). Updating that assertion to reflect the padded top palette panel (`PALETTE_PANEL_HEIGHT + CONTENT_PADDING`, `128px`) and adding assertions for the `panel_bg` fill, `1px` bottom divider, and **Overlay opacity** scaling keeps tests aligned with the new layout.
- **Software background backend** → `SoftwarePainter` composites the wallpaper behind all `egui` primitives, so `Panel::top` with `tokens.panel_bg` works identically on both GPU (`wgpu`/`glow`) and software backends without backend-specific code.
