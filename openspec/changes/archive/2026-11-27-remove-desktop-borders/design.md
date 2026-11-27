# Design

## Context

See `proposal.md` for motivation and `specs/desktop-ui-theme/spec.md` for the target visual contract.

In the `desktop` implementation (`desktop/src/theme.rs` and `desktop/src/app/*.rs`), two color-role collisions are masked only by 1px `border` strokes when surface/control borders are removed:
1. `ThemeTokens::card_bg` equals `ThemeTokens::control_bg` in both Light (`#fcfcfc`) and Dark (`#252830`) modes, so buttons, `ComboBox`es, `DragValue`s, and unchecked checkboxes inside cards or modals would become indistinguishable from their card background once control borders are removed.
2. In Dark mode, `ThemeTokens::app_bg` equals `CanvasPalette::bg` (`#0d1016`), so the Equation Canvas and LaTeX source box would blend into the central panel backdrop once `pal.border` is removed.

Additionally, `App::paint_batch_row_chrome` draws internal hierarchy connectors (`tree_stroke`) and row dividers (`divider_stroke`) inside the Batch DOCX tree independent of the outer card's `group_frame()`, and `App::new_with_state` disables `window_shadow` and `popup_shadow` on `VirtualGpu` / `Software` renderers.

## Goals / Non-Goals

**Goals:**
- Eliminate surface, card, canvas, LaTeX, thumbnail, modal, popup, progress-bar, and idle/hovered/selected control borders.
- Preserve crisp horizontal and vertical structural region dividers (`sidebar vline`, `footer hline`, `status` top `hline`, and `draw_section_separator` lines in footer, Batch DOCX, and About) without putting a 4-sided box around the status bar or adding borders to disabled widgets.
- Establish clear multi-tier surface and control fill contrast in both Light and Dark modes (and in `Image` mode across `0%..=100%` opacity) so every container and interactive control remains unmistakable without strokes.
- Preserve internal Batch DOCX **Folder tree** and **File tree** horizontal/vertical lines, Equation Canvas dashed placeholders (`□`), status-bar disconnected bridge ring, and functional indicator outlines (checkbox squares, radio circles, slider thumbs, modal keyboard focus rings).

**Non-Goals:**
- Removing OS window titlebar / window manager decorations (`desktop_viewport` in `desktop/src/main.rs`).
- Changing layout geometry, corner radii (`6px` / `8px`), spacing constants, or the Batch DOCX tree algorithm.

## Decisions

### 1. Multi-tier Ayu neutral fill hierarchy (Light & Dark)
Instead of relying on 1px `tokens.border` or `pal.border` to separate adjacent surfaces:
- **Light mode**: Keep `app_bg` (`#e2e4e7`) < `panel_bg` (`#ebecee`) < `card_bg` (`#f4f5f7`) < `control_bg` (`#ffffff`) and `pal.bg` (`#fcfcfc`). Every surface and control tier differs cleanly in luminance—controls on `app_bg` (`Back`), `panel_bg` (palette/sidebar/footer), and `card_bg` (Settings/Batch/About/modals) all stand out naturally.
- **Dark mode**: Keep Equation Canvas / LaTeX `pal.bg` at Ayu Dark ink (`#0d1016`) for maximum equation legibility, lift `app_bg` (`#151820`) above `pal.bg` and below `panel_bg` (`#1d2028`), set `card_bg` (`#242832`) distinct from `panel_bg`, and lift `control_bg` (`#2f3440`, with `control_hover_bg` `#3b4150` and `control_active_bg` `#474e60`) so idle controls pop clearly on cards, panels, and `app_bg` while keeping `text_primary` contrast `>= 7.0` (WCAG AAA).

**Alternative considered:** Keeping existing RGBs and darkening `control_bg` below `card_bg`. Rejected because `control_bg` would collide with `panel_bg` (`#ececed` in Light, `#1f2127` in Dark) on the palette and sidebar.

### 2. Borderless global `egui::Style` with scoped indicator retention
- In `ThemeTokens::apply_style_with_accent` and `apply_accent_to_style`:
  - Set `visuals.window_stroke = Stroke::NONE`.
  - Set `widgets.noninteractive.bg_stroke`, `widgets.inactive.bg_stroke`, `widgets.hovered.bg_stroke`, and `widgets.open.bg_stroke` to `Stroke::NONE`.
  - Update `App::with_selectable_style` and `App::primary_button_with` (`inactive`/`hovered`/`disabled`) to set `bg_stroke = Stroke::NONE`, relying on `selected_bg` / `selected_hover_bg` / `primary_bg` / `primary_hover_bg` (while preserving `visuals.selection.stroke` for `SelectableLabel` text color and focused `DragValue`/`TextEdit` focus outlines).
- **Scoped functional exceptions**:
  - `App::accent_checkbox` and `App::accent_radio_value` explicitly set `bg_stroke` on their scoped `ui.visuals_mut()` so small unchecked checkbox squares and unselected radio circles keep a visible 1px neutral/disabled-accent outline (while their labels stay borderless).
  - Settings sliders (`draw_settings_tab` and `draw_opacity_slider_row`) keep their accent `fg_stroke` for the square handle border.
  - Modal keyboard focus (`Modal workspace dimming and visible keyboard focus rings`) retains a visible `1.0px` focus stroke on `widgets.active.bg_stroke` so `Tab` / `ArrowLeft` / `ArrowRight` navigation remains clear without adding idle or hover borders to neighboring buttons.
  - `App::draw_section_separator` temporarily sets `ui.visuals_mut().widgets.noninteractive.bg_stroke = Stroke::new(1.0, tokens.border)` around `ui.separator()` and restores it immediately so disabled widgets stay borderless.

### 3. Surface removals & preserved region dividers across UI modules
- **`desktop/src/app/mod.rs`**: Keep `app-status` frame borderless (`.stroke(Stroke::NONE)`) and paint 1px `tokens.border` region dividers after `CentralPanel` (`status_rect` top `hline`, `sidebar_rect` `vline`, `footer_rect` `hline`). Use `App::draw_section_separator` in `draw_footer` between format radios and Font size.
- **`desktop/src/app/canvas.rs`**: Remove `.stroke(Stroke::new(1.0, pal.border))` from `draw_canvas` and remove `ui.painter().rect_stroke(...)` from `draw_latex_panel`. Keep `paint_placeholder` dashed `placeholder_stroke` untouched.
- **`desktop/src/theme.rs` & `settings_ui.rs` & `about_ui.rs`**: Remove `.stroke(...)` from `ThemeTokens::group_frame()`, remove the thumbnail border strokes in `draw_background_thumbnail`, and call `App::draw_section_separator` in `draw_about_tab` and `draw_batch_tab`.
- **`desktop/src/app/batch_ui.rs`**: Replace `painter.rect(..., Stroke::new(1.0, tokens.border), ...)` on the progress bar with `painter.rect_filled(...)`. Leave `paint_batch_row_chrome` completely intact so **Folder tree** (`tree_stroke` + `divider_stroke`) and **File tree** (`divider_stroke`) continue drawing internal hierarchy and row lines normally.
- **`desktop/src/app/keyboard.rs`**: In the inline `\command` popup, drop the `Stroke` on the input box frame and switch its fill from `Color32::TRANSPARENT` to `tokens.control_bg` so the input region stays clearly delineated inside the borderless popup.

### 4. Popup contrast on shadowless backends (`Software` / `VirtualGpu`)
Because `window_shadow` and `popup_shadow` are `Shadow::NONE` on `Software` and `VirtualGpu` backends, floating popups (`ComboBox` menus, tooltips, `\command` popup) cannot rely on shadows alone when opened over a `card_bg` container (such as `Custom matrix` or `Batch DOCX` card). `ThemeTokens::combo_popup_style()` applies `egui::containers::menu::menu_style` and sets `style.visuals.window_fill = self.control_bg` on `batch-target`, `matrix-alignment`, and `matrix-fence` so dropdown menus remain distinct over `card_bg` on shadowless backends.

## Risks / Trade-offs

- **[Risk] Low contrast in `Image` background mode at low `Overlay opacity`** → `panel_bg` and `card_bg` use `MIN_SURFACE_ALPHA = 0.38`, while `app_bg` is `Color32::TRANSPARENT`. Retaining distinct RGB bases between `panel_bg`, `card_bg`, `control_bg`, and `pal.bg` plus crisp 1px region dividers keeps panels, surfaces, and controls distinguishable over the dimmed scrim.
- **[Risk] Existing unit and layout tests assert specific `border` / `stroke` values** → Update `theme.rs` unit tests, `layout_theme_tests.rs`, `canvas_input_tests.rs`, `batch_settings_tests.rs`, and `session_tests.rs` to assert borderless surfaces/controls, distinct layered fills, preserved region dividers (`status`/`footer`/`sidebar`), and preserved Batch tree / placeholder / checkbox-radio / modal-focus indicators.
