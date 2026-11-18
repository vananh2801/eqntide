# Tasks

## 1. Catalog Templates & One-Sided Matrix Builder

- [x] 1.1 Add 4 one-column, left-aligned system-of-equations templates to the `Fences` catalog in `src/catalog.rs` (`\left\{ \begin{array}{l} #? \\ #? \end{array} \right.`, 3-row brace, 2-row bracket, 3-row bracket) and verify `cargo test -p eqntide catalog` passes all template parse, preview, render, OMML, and MTEF roundtrip checks.
- [x] 1.2 Extend `eqntide::catalog::matrix` in `src/catalog.rs` to support `"left-brace"` and `"left-bracket"` one-sided fences (emitting a single stretchy prefix `<mo>` before `<mtable>` without a closing delimiter) and add unit tests in `src/catalog.rs` and `src/omml` / `src/mtef` verifying render, LaTeX, OMML, and MTEF roundtrips for `[` + `mtable` and `{` + `mtable`.

## 2. Structural Editor Row Insertion & Fence Promotion

- [x] 2.1 Add an atomic `Editor` method in `crates/eqntide-editor/src/lib.rs` that inserts a new row immediately below the caret's current `mtr` inside an `mtable` and moves the caret to the first cell of the new row (or returns `false` when outside an `mtable`). Verify with unit tests covering multi-column matrices, 1-column systems, nested structures (e.g. caret inside a fraction in a cell), and single-step `undo`/`redo`.
- [x] 2.2 Extend the `Editor` method in `crates/eqntide-editor/src/lib.rs` to promote a single-slot one-sided `{` or `[` fence (with no closing delimiter) into a 2-row, 1-column left-aligned `mtable` preserving row 1 content and placing the caret in empty row 2, and verify via unit tests in `crates/eqntide-editor` including boundary cases (empty slot, non-empty slot, paired fences unaffected, single-step `undo`).
- [x] 2.3 Extend `Editor::delete` in `crates/eqntide-editor/src/lib.rs` so `Backspace` / `Delete` inside an empty cell removes the current `mtr` when all cells in that row are empty and the `mtable` has more than one row, moving the caret to the adjacent row in one undoable step; verify with unit tests covering middle/last/first empty row removal, multi-column non-empty guard, single-row guard, and `undo`/`redo`.

## 3. Desktop UI Integration

- [x] 3.1 Add `("left-brace", "Left brace {")` and `("left-bracket", "Left bracket [")` to `FENCES` in `matrix_dialog_ui` in `desktop/src/app.rs` and verify with a UI test that inserting via the Custom matrix dialog produces a 1-sided left-fenced table.
- [x] 3.2 Wire `Key::Enter` (when `command_input` is `None`, IME preedit is empty, and no `command`/`alt` modifiers are held) in `desktop/src/app.rs` to call the new `Editor` row-insert/promote method and refresh render/LaTeX sync, keeping `Enter` command-buffer commit behavior unchanged; add desktop UI tests covering `Enter` inside a system/matrix, `Enter` inside `{□` / `[□`, `Enter` inside a `\` command buffer, and `Enter` at the root row.
- [x] 3.3 Add a desktop UI test in `desktop/src/app.rs` verifying `Backspace` and `Delete` on an empty row inside a multi-row system remove the empty row and refresh `self.latex` / `Undo`.

## 4. End-to-End Verification

- [x] 4.1 Run `cargo test --workspace` and `openspec validate system-of-equations-entry` and verify all existing and newly added tests pass with zero warnings/errors.
