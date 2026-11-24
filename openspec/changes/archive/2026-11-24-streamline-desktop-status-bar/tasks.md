# Tasks

## 1. Streamline Status Bar Messages, Spinner, and Lifecycle

- [x] 1.1 Initialize `status: String::new()` in `EqnTideApp::from_parts`, remove the `"Inserted LaTeX command."`, `"Canceled LaTeX command."`, and `"Inserted {} × {} matrix."` status assignments, and shorten the Office session status string to `"Office session — Save to return"` in `desktop/src/app.rs` (verify by inspection and running `cargo test -p eqntide-desktop`).
- [x] 1.2 Remove the `if self.busy` spinner / `"Working…"` block from `status_bar()` (keeping `32px` height and the normal `●`/`○` dot) and remove `self.busy` from `has_active_animation()` in `desktop/src/app.rs` (verify with `status_bar` UI test confirming no `"Working…"` text or spinner shapes appear when `busy == true`).
- [x] 1.3 Clear `self.status` in `changed()`, `source_changed()`, and when opening `command_input` on `\` in `canvas_text()`, clear matching `self.status` whenever `command_error` is cleared or `command_input` is dismissed (`clear_command_error` / `dismiss_command_input`), and guard `status_bar()` so an empty `self.status` renders no label or hover tooltip (verify with unit tests covering idle startup, clearing stale messages on edit, command popup Backspace/Esc clearing, Office session notice visibility/clearing, and empty-status rendering).

## 2. Verification

- [x] 2.1 Add unit tests in `desktop/src/app.rs` verifying idle startup status is empty, `busy == true` renders no spinner/`"Working…"` in `status_bar`, command commit/cancel and custom matrix insert leave `status` empty, subsequent edits clear prior `status` messages (including `"Saved."` and `"Office session — Save to return"`), backspacing/canceling out of an invalid command clears `status`, and existing `LaTeX error:` / `Warning:` / save-block assertions still pass (`cargo test -p eqntide-desktop`).
