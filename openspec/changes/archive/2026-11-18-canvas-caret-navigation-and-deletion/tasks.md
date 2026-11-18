# Tasks

## 1. Canonical Horizontal Caret Navigation

- [x] 1.1 Deduplicate coincident boundary stops in `Node::stops` in `crates/eqntide-editor/src/lib.rs` across flat rows, nested `<mrow>` wrappers, fixed-arity structures (`mfrac`, `mroot`, `msqrt`, `msub`, `msup`, `msubsup`, `munder`, `mover`, `munderover`, `menclose`), and fenced `mrow`s, and add unit tests verifying 1-keystroke `Move::Left`/`Move::Right` traversal (`cargo test -p eqntide-editor`).
- [x] 1.2 Audit existing caret/navigation tests in `crates/eqntide-editor/src/lib.rs` and `desktop/src/app.rs` (including `caret_rect_aligns_consistently_across_tokens_and_row_stops`) so direct `set_caret` and `ArrowLeft`/`ArrowRight` tests pass (`cargo test -p eqntide-desktop`).

## 2. Empty Token Cleanup and A2 Structural Deletion

- [x] 2.1 Update `Editor::delete` in `crates/eqntide-editor/src/lib.rs` to remove token nodes (`mi`, `mn`, `mo`, `mtext`, `ms`) from their parent row when their last character is deleted, and update `is_effectively_empty` to treat zero-character tokens as empty; verify with unit tests in `crates/eqntide-editor/src/lib.rs` (`cargo test -p eqntide-editor`).
- [x] 2.2 Implement symmetrical `Backspace` and `Delete` boundary handling in `Editor::delete`: select the immediate enclosing structure (including fenced wrappers) via `select_node` when inside an empty non-table slot, and step via `Move::Left` / `Move::Right` when at the start/end boundary of a non-empty slot; verify with unit tests covering `mfrac`, `msup`, `msub`, `msubsup`, `mroot`, `msqrt`, `munder`, `mover`, `munderover`, `menclose`, fences, and undo/redo (`cargo test -p eqntide-editor`).

## 3. Table (`mtable`) Row Selection and Deletion Fixes

- [x] 3.1 Fix `Editor::erase_selection` in `crates/eqntide-editor/src/lib.rs` so erasing whole-row selections (`mtr` or `mtable`) removes `<mtr>` rows when `>= 2` rows exist and partial-row selections restore empty `<mtd>` cells rather than converting `<mtd>`/`<mtr>` to `<mrow>`, and add regression tests verifying both rendered structure and `to_latex()` (`cargo test -p eqntide-editor`).
- [x] 3.2 Update empty-cell `Backspace`/`Delete` handling inside `mtable` in `crates/eqntide-editor/src/lib.rs`: remove the row when all cells are empty and `>= 2` rows exist; select the whole row (`anchor = (mtr, 0)`, `caret = (mtr, cols)`) when a multi-cell row has at least one non-empty cell; and apply A2 selection on the `mtable` (or enclosing fenced `mrow`) when a 1-row table is empty; verify with unit tests in `crates/eqntide-editor/src/lib.rs` (`cargo test -p eqntide-editor`).

## 4. Desktop Integration Verification

- [x] 4.1 Add desktop keyboard/selection integration tests in `desktop/src/app.rs` covering `\frac{1}{}` Backspace A2 selection-then-delete, single-press arrow traversal across `\frac` and fences, partially filled vs empty `mtable` rows, and drag-selecting an empty `mtable` row followed by `Backspace`/`Delete`; verify the full desktop and workspace test suite passes (`cargo test -p eqntide-editor && cargo test -p eqntide-desktop`).
