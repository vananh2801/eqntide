# Design

## Context

See `proposal.md` for motivation. In the original codebase:
- `desktop/src/app/dialogs.rs` rendered `Custom matrix` with `egui::Modal` (using `response.should_close()`, which closed on backdrop clicks as well as `Escape`, and inherited `Frame::popup`'s cramped `6px` `menu_margin`) and rendered `Finish current equation` with an unanchored `egui::Window` and a redundant subtitle line.
- `desktop/src/app/canvas.rs` unconditionally called `ui.set_opacity(1.0)` inside `draw_canvas`, overriding the outer `ui.add_enabled_ui(!self.busy && self.pending.is_none() && self.matrix_dialog.is_none(), ...)` opacity applied in `desktop/src/app/mod.rs`.
- `desktop/src/app/keyboard.rs` read `input.events.clone()` and set `self.pending = Some(Intent::Close)` on `Key::Escape` during `draw_shell`, before `draw_dialogs` ran in the same frame.
- `desktop/src/app/palette_ui.rs` set `widgets.active.bg_stroke = Stroke::new(1.0, tokens.primary_active_bg)` inside `primary_button_with`, matching `widgets.active.bg_fill` so focused primary buttons showed no distinct border (and using a thicker `1.5px` stroke would introduce visual stroke/feathering asymmetry next to `1.0px`-stroked secondary buttons).
- `desktop/src/app/events.rs` (`App::is_dirty`) flagged any change to `font_pt` or `target` even when starting with an empty equation (`Insert` / standalone) and remaining empty, and treated any pending `draft_dirty` LaTeX state as dirty even right after clearing back to empty.

## Goals / Non-Goals

**Goals:**
- Gate `Finish current equation` using Edit-vs-Insert baseline tracking (`saved_empty`) so empty-baseline sessions returning to empty never prompt while cleared pre-existing Edit equations still prompt.
- Unify `Finish current equation` and `Custom matrix` on centered `egui::Modal` with non-dismissing backdrop clicks and `Escape`-to-cancel/continue semantics.
- Restore uniform workspace dimming on the Equation Canvas when modals or `busy` states are active while preserving `1.0` opacity during `draft_dirty` LaTeX edits.
- Provide predictable keyboard focus initialization, trapped `Tab`/`Shift+Tab` cycling, horizontal `ArrowLeft`/`ArrowRight` button switching, `Enter`/`Space` activation, and crisp `1.0px` focus borders on both modals.
- Align modal visual polish across both dialogs with uniform `20px` four-side outer padding, centered headings, balanced spacing/widths, and equal-height, consistently padded action buttons.

**Non-Goals:**
- Adding modal dialogs to non-dialog views (e.g., About update check remains inline per `desktop-update-check`).
- Altering `egui::Modal` backdrop color/corner-radius tokens beyond existing `ThemeTokens` / `SURFACE_CORNER_RADIUS`.

## Decisions

1. **Track `saved_empty` alongside `saved_xml` in `App::is_dirty()`**
   - Record `saved_empty = math.children.is_empty()` in `App::from_parts`, `App::load_math`, and `Event::Finished(Ok(()))`.
   - In `App::is_dirty()`, define `current_empty = self.latex.trim().is_empty() && self.command_input.is_none() && (self.editor.math().children.is_empty() || self.draft_dirty)`. If `self.saved_empty && current_empty`, return `false` immediately so Insert/standalone empty sessions close cleanly without prompting even if `font_pt`/`target` changed or LaTeX clearing is still in its 180ms debounce window. If `self.saved_empty` is `false` (Edit session with a pre-existing equation), deleting all content still makes `editor.math().to_xml() != self.saved_xml` (or `draft_dirty == true`), returning `true` so `Finish current equation` appears.
   - Alternative: Suppress close confirmation whenever current canvas/LaTeX is empty regardless of baseline. Rejected because deleting an existing equation in an Edit session is a destructive change that must still confirm before discarding vs saving empty.

2. **Use `egui::Modal` for both dialogs without `ModalResponse::should_close()`**
   - `egui::Modal::should_close()` treats `backdrop_response.clicked()` identically to `Escape`. By checking only explicit close triggers (`Escape` when `!any_popup_open && is_top_modal`, plus `Cancel`/`Continue editing` buttons), accidental backdrop clicks are ignored while `Escape` continues to dismiss cleanly.
   - Alternative: Custom `egui::Area` + manual backdrop painter. Rejected because `egui::Modal` already handles centering, `Order::Foreground`, backdrop click absorption (`Sense::CLICK | Sense::DRAG`), and `is_top_modal`/`any_popup_open` tracking.

3. **Guard same-frame `Escape` between `handle_canvas_keyboard` and `draw_dialogs`**
   - When `Key::Escape` on the Equation Canvas sets `self.pending = Some(Intent::Close)`, `draw_dialogs` runs later in the same frame. Consuming `Key::Escape` in `ctx.input_mut` when transitioning `pending` from `None` to `Some(Intent::Close)` guarantees `Finish current equation` never self-closes on its opening frame.
   - Alternative: Reorder `draw_dialogs` before `draw_shell`. Rejected because it would complicate `draw_shell` render-sync ordering.

4. **Explicit modal focus helper with deterministic `egui::Id`s**
   - `egui` does not auto-focus the first button inside a newly opened `Modal`, nor does it trap `Tab` inside the modal (for example, `app-status` in `draw_shell` stays enabled) or treat `ArrowLeft`/`ArrowRight` consistently on non-focused states.
   - Capture the widget `Id`s assigned to modal action buttons (`Save`, `Discard / cancel session`, `Continue editing`, `Stop batch`; `Insert matrix`, `Cancel`) and modal form controls (`Rows`, `Columns`, `Alignment`, `Delimiters`) in `ModalFocusState`.
   - On modal open (or when focus slips outside without an open popup), call `request_focus` on the default enabled primary button (`Save` / `Stop batch` / `Insert matrix`).
   - While the modal is top-level with no `ComboBox` popup open, intercept `Tab` / `Shift+Tab` (matching `Shift+Tab` before `Tab` in `ctx.input_mut(|i| i.consume_key(...))`) to cycle strictly across the modal's enabled control list, and intercept `ArrowLeft` / `ArrowRight` only when focus is currently on one of the horizontal modal action buttons (leaving `DragValue` text editing/stepping and `ComboBox` popups untouched).
   - Alternative: Rely on egui's built-in spatial focus traversal. Rejected because `FocusDirection` does not trap `Tab` within `Modal` and can jump to the status bar or leave focus `None` on initial open.

5. **Scope canvas `ui.set_opacity(1.0)` strictly to `draft_dirty`-only disabling**
   - In `draw_canvas`, set `ui.set_opacity(1.0)` only when `!self.busy && self.pending.is_none() && self.matrix_dialog.is_none()`. When a modal or `busy` state is active, the canvas inherits egui's disabled opacity just like the palette, LaTeX source box, sidebar, and footer, in addition to the modal backdrop scrim.

6. **Distinguishable `1.0px` focus border on primary buttons**
   - In `primary_button_with`, set `widgets.active.bg_stroke` to `Stroke::new(1.0, focus_border)` where `focus_border` is `tokens.text_primary` in Dark mode (contrasting against bright amber `primary_active_bg`) and `tokens.primary_fg` (`Color32::WHITE`) in Light mode (contrasting against deep brown `primary_active_bg`), while keeping inactive/hovered `bg_stroke` matched to the fill color.
   - Keeping the focus stroke at `1.0px` matches secondary buttons (`tokens.border` / `tokens.accent` at `1.0px`) so `Frame::apply_stroke_and_expansion_without_layout_shift` never widens borders or creates perceived height mismatch between adjacent buttons like `Save` and `Discard / cancel session`.

7. **Unified modal padding and action button proportions**
   - Define `modal_frame(style: &egui::Style) -> egui::Frame` overriding `Frame::popup(style)` with `egui::Margin::same(MODAL_INNER_MARGIN)` (`20px` on all four sides) for both `Finish current equation` and `Custom matrix`.
   - Apply `MODAL_BUTTON_PADDING = Vec2::new(14.0, 6.0)` across modal action buttons while enforcing `theme::BUTTON_HEIGHT` (`30.0`) via `.min_size(Vec2::new(0.0, theme::BUTTON_HEIGHT))` so short labels like `Save` have airy horizontal breathing room and identical vertical bounds (`30.0px`) alongside `Discard / cancel session` and `Continue editing`.

8. **Streamlined heading and column proportions in both dialogs**
   - `Finish current equation`: center the `"Finish current equation"` heading and remove the redundant `"Save your changes, discard them, or continue editing."` subtitle so only batch-mode status text appears when `batch_running` is true.
   - `Custom matrix`: center the `"Custom matrix"` heading, set `MATRIX_MIN_INNER_WIDTH = 260.0` (total outer width ~`300px` with `20px` left/right padding), space the 2-column `egui::Grid` at `[32.0, 10.0]` so labels and inputs separate cleanly, size both `Alignment` and `Delimiters` `ComboBox`es uniformly to `MATRIX_COMBO_WIDTH = 156.0`, and right-align `[Insert matrix] [Cancel]` across the dialog width.

## Risks / Trade-offs

- **Consuming `Tab` / `ArrowLeft` / `ArrowRight` inside `Modal`** → Guarded by checking `is_top_modal && !any_popup_open` and checking that `ArrowLeft`/`ArrowRight` only triggers when focus is on a modal action button (not `DragValue`), preventing interference with `Rows`/`Columns` editing or `ComboBox` menus.
- **Primary button focus stroke changing non-focused button assertions** → Keep non-focused `primary_button_with` fill and stroke unchanged; apply the contrasting `1.0px` focus stroke only in `widgets.active`.
