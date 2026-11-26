# Proposal

## Why

When closing or switching sessions with unsaved changes (`Finish current equation`), the confirmation dialog previously opened as an uncentered top-left `egui::Window` without a backdrop, left the Equation Canvas at full brightness while surrounding panels dimmed, lacked initial button focus or intuitive `Tab`/`ArrowLeft`/`ArrowRight`/`Enter` keyboard navigation, and also popped up unnecessarily when closing an Insert/standalone session whose equation remained empty after a setting tweak or typing-and-clearing. In addition, both `Finish current equation` and `Custom matrix` needed consistent centered modal layout, uniform four-side padding, aligned button proportions, and clean visual focus borders.

## What Changes

- Track `saved_empty` on the initial/saved baseline (`Edit` vs `Insert`/standalone) so closing an initially empty equation that is still empty (even if `font_pt`/`target` changed or a LaTeX clear is debouncing) closes cleanly without showing `Finish current equation`, while deleting or editing a pre-existing `Edit` equation still prompts.
- Render the `Finish current equation` confirmation prompt as a centered `egui::Modal` with a full-window dimming backdrop instead of an unanchored top-left `egui::Window`.
- Keep the Equation Canvas dimmed whenever an outer modal (`pending` / `matrix_dialog`) or `busy` state disables the editor workspace, while still keeping the Equation Canvas preview at full opacity when disabled solely due to an unparsed `draft_dirty` LaTeX draft.
- Ignore pointer clicks on the darkened modal backdrop in both `Finish current equation` and `Custom matrix` so accidental outside clicks do nothing, while mapping `Escape` to cancel/continue editing (unless `busy` or an inner `ComboBox` popup is open).
- Add keyboard focus management to both `Finish current equation` and `Custom matrix`: automatically focus the primary action button (`Save`, or `Stop batch` when a batch conversion is running; `Insert matrix` in `Custom matrix`) on open, trap and cycle `Tab`/`Shift+Tab` inside the modal's enabled controls, map `ArrowLeft`/`ArrowRight` between horizontal dialog action buttons (without hijacking `DragValue` or open `ComboBox` navigation), activate focused buttons with `Enter`/`Space`, and render a clearly visible `1.0px` focus border on focused primary and standard buttons matching button heights across both Light and Dark modes.
- Unify modal chrome and layout across both dialogs: apply uniform `20px` four-side inner padding via a shared `modal_frame`, center both modal headings, omit the redundant non-batch subtitle in `Finish current equation`, format `Custom matrix` at a balanced compact width (`>= 260px` inner width) with `32px` column spacing, unified `156px` `ComboBox` widths, and right-aligned `Insert matrix` / `Cancel` buttons, and standardize modal button padding (`14px × 6px` at `30px` `BUTTON_HEIGHT`).

## Capabilities

### New Capabilities
- `desktop-modal-dialogs`: Defines centered modal presentation, uniform `20px` four-side padding, aligned heading/grid/action layouts, equal `30px` action button sizing, backdrop click-through suppression, `Escape` dismissal, initial primary-button focus, trapped `Tab`/`Shift+Tab` and horizontal `ArrowLeft`/`ArrowRight` button navigation, `Enter`/`Space` activation, and Edit-vs-Insert baseline (`saved_empty`) close confirmation gating across desktop modal dialogs (`Finish current equation` and `Custom matrix`).

### Modified Capabilities
- `desktop-ui-theme`: Clarifies that Equation Canvas disabled-state opacity remains full only during `draft_dirty` LaTeX edits and dims uniformly with surrounding panels when disabled by modal overlays or busy states, and specifies crisp `1.0px` mode-contrasted keyboard focus borders on primary buttons matching standard button stroke thickness and height.

## Impact

- `desktop/src/app/{mod.rs, session.rs, events.rs}`: Track `saved_empty` on startup, `load_math`, and `Event::Finished(Ok(()))`, and make `App::is_dirty()` distinguish Insert-empty returns from cleared Edit equations.
- `desktop/src/app/dialogs.rs`: Convert `Finish current equation` to `egui::Modal`, share `modal_frame` (`20px` four-side padding) and `MODAL_BUTTON_PADDING` (`14×6px`), center modal headings, streamline `Finish current equation` (omitting the non-batch subtitle) and `Custom matrix` (`260px` minimum inner width, `32px` grid spacing, `156px` `ComboBox` widths, right-aligned buttons), suppress backdrop close on both modals, handle `Escape` cleanly (including frame-of-open guard when `Escape` opened `Finish current equation`), and implement modal focus initialization and keyboard cycling.
- `desktop/src/app/canvas.rs`: Scope `ui.set_opacity(1.0)` in `draw_canvas` to `!self.busy && self.pending.is_none() && self.matrix_dialog.is_none()`.
- `desktop/src/app/keyboard.rs`: Consume `Key::Escape` when opening `Intent::Close` from canvas keyboard handling so the newly opened modal does not immediately dismiss in the same frame.
- `desktop/src/app/palette_ui.rs`: Render a `1.0px` mode-contrasted focus border (`tokens.text_primary` in Dark mode, `tokens.primary_fg` in Light mode) on focused primary buttons (`primary_button_with`) matching secondary button stroke thickness and keeping identical button heights.
- `desktop/src/app/tests/`: Add UI regression tests for Edit-vs-Insert baseline close confirmation, centered modal placement, uniform padding, equal button heights, compact balanced `Custom matrix` width, backdrop click suppression, `Escape`, `Tab`/`ArrowLeft`/`ArrowRight`/`Enter` navigation, focus border contrast, and modal canvas dimming.
