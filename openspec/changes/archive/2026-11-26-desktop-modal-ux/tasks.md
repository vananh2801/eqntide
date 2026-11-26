# Tasks

## 1. Centered modals, backdrop behavior, close gating, and canvas dimming

- [x] 1.1 Scope `ui.set_opacity(1.0)` in `desktop/src/app/canvas.rs` (`draw_canvas`) to `!self.busy && self.pending.is_none() && self.matrix_dialog.is_none()`, and add/update a layout theme test verifying the Equation Canvas dims when `pending` or `matrix_dialog` is open while staying `1.0` opaque during `draft_dirty`.
- [x] 1.2 Convert `Finish current equation` in `desktop/src/app/dialogs.rs` (`draw_dialogs`) from `egui::Window` to centered `egui::Modal`, ignoring backdrop clicks in both `draw_dialogs` and `draw_matrix_dialog`, and verify centering + backdrop click suppression in `desktop/src/app/tests/`.
- [x] 1.3 Handle `Escape` dismissal cleanly in both modals (acting as Continue editing / Cancel when `!busy` and no `ComboBox` popup is open, restoring editor focus, and consuming `Key::Escape` in `desktop/src/app/keyboard.rs` when opening `Intent::Close` so it never self-closes on the opening frame), and add regression tests for canvas `Escape` -> modal stays open -> second `Escape` continues editing.
- [x] 1.4 Track `saved_empty` across `desktop/src/app/{mod.rs, session.rs, events.rs}` so `App::is_dirty()` skips close confirmation when an empty Insert/standalone equation remains or returns to empty (even after `font_pt`/`target` changes or during debounced LaTeX clearing), while still prompting when a pre-existing Edit equation is modified or deleted; verify all cases in `desktop/src/app/tests/session_tests.rs`.

## 2. Modal keyboard navigation and visible focus rings

- [x] 2.1 Update `primary_button_with` in `desktop/src/app/palette_ui.rs` so focused primary buttons render a distinct `1.0px` contrast focus border (`tokens.text_primary` in Dark mode / `tokens.primary_fg` in Light mode) matching secondary button stroke thickness in both Light and Dark modes, and add a theme test verifying focused vs unfocused primary button stroke contrast.
- [x] 2.2 Implement shared modal focus management in `desktop/src/app/dialogs.rs` for `Finish current equation` (`Save`, `Discard / cancel session`, `Continue editing`, `Stop batch`) and `Custom matrix` (`Rows`, `Columns`, `Alignment`, `Delimiters`, `Insert matrix`, `Cancel`): focus the default enabled primary button on open, trap `Tab`/`Shift+Tab` inside enabled modal controls, map `ArrowLeft`/`ArrowRight` between horizontal action buttons without hijacking `DragValue` or open `ComboBox` popups, and trigger focused buttons via `Enter`/`Space`; verify each flow with `egui` input tests in `desktop/src/app/tests/`.

## 3. Unified modal layout, uniform padding, and equal button proportions

- [x] 3.1 Add shared `modal_frame` (`Margin::same(20)`) and `MODAL_BUTTON_PADDING` (`Vec2::new(14.0, 6.0)`) in `desktop/src/app/dialogs.rs` so both `Finish current equation` and `Custom matrix` share uniform four-side padding and identical `BUTTON_HEIGHT` (`30.0px`) action buttons.
- [x] 3.2 Center both modal headings, omit the non-batch subtitle in `Finish current equation`, format `Custom matrix` at `MATRIX_MIN_INNER_WIDTH = 260.0` with `32px` column spacing, unified `156px` `ComboBox` widths, and right-aligned `Insert matrix` / `Cancel` buttons, and verify layout/padding/button-height invariants in `desktop/src/app/tests/`.

## 4. Workspace verification

- [x] 4.1 Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --all-targets --locked`, `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`, and `openspec validate "desktop-modal-ux"` and verify zero warnings or failures.
