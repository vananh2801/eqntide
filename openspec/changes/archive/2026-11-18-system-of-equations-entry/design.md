# Design

## Context

See `proposal.md` for motivation. Across the existing codebase:
- `src/render/layout/fence.rs` already stretches an unmatched opening fence (`{` or `[`) to match a following `mtable`.
- `src/omml/encode.rs` already encodes `<mrow><mo>{</mo><mtable>...</mtable></mrow>` (and single-sided `[`) as `<m:d>` with `begChr` set and `endChr=""`.
- `src/mtef/encode/structural.rs` already encodes a 1-column `mtable` with `columnalign="left"` as an MTEF `pile` with `HALIGN_LEFT`.
- `src/mathml_to_latex.rs` already serializes an `mtable` with `columnalign="left"` to `\begin{array}{l} ... \end{array}` and single-sided stretchy delimiters to `\left\{ ... \right.` / `\left[ ... \right.`.
- `crates/eqntide-editor/src/lib.rs` has `grow_matrix(column)` which only appends a row at the bottom and leaves the caret in place, and `desktop/src/app.rs` currently handles `Key::Enter` only inside `command_key`.

## Goals / Non-Goals

**Goals:**
- Reuse the existing MathML `<mrow><mo stretchy="true" form="prefix">{|[</mo><mtable columnalign="left">...</mtable></mrow>` representation so rendering, LaTeX roundtrip, OMML export, and MTEF/MathType export work without changing downstream encoders.
- Make `Enter` row insertion and one-sided fence promotion atomic (`transact`) so `Undo`/`Redo` stay consistent.
- Keep `cases-two` / `cases-three` unchanged so piecewise functions keep their two-column behavior.

**Non-Goals:**
- Changing how `cases` (`\begin{cases}`) parses or renders.
- Supporting multi-line freeform paragraphs outside `mtable` (MathML/OMML/MTEF represent multi-line equations as `mtable` / `pile`).
- Adding one-sided *right* delimiters (`}` or `]`) to Custom matrix (can be added later with the same pattern if needed).

## Decisions

1. **System template representation in `src/catalog.rs`**:
   - Use `\left\{ \begin{array}{l} #? \\ #? \end{array} \right.` (and `3`-row, plus `\left[ ... \right.`) rather than `\begin{cases}` or `\begin{matrix}`.
   - *Rationale*: `\begin{cases}` expands to 2 columns; `\begin{matrix}` defaults to centered columns, whereas `\begin{array}{l}` sets `columnalign="left"`, matching standard equation-system formatting and cleanly roundtripping via `mathml_to_latex.rs` and MTEF `pile(HALIGN_LEFT)`.
   - *Alternative considered*: Adding a custom `\system{...}` LaTeX extension. Rejected because `\left\{ \begin{array}{l} ... \end{array} \right.` is standard LaTeX already supported by `latex_extensions.rs` and `mathml_to_latex.rs`.

2. **Extending `eqntide::catalog::matrix` and `MatrixDialog` (`src/catalog.rs`, `desktop/src/app.rs`)**:
   - Extend `matrix(rows, columns, alignment, fence)` to accept `"left-brace"` (`("{", "")` — emitting only the opening stretchy `<mo form="prefix">{</mo>` before `<table>`, or `<mrow><mo stretchy="true">{</mo><table></mrow>`) and `"left-bracket"` (`[`).
   - Check `template(r"\left\{ #? \right.")` output structure so `matrix(...)` emits the exact same fence `<mo>` attributes (`stretchy="true"`, `form="prefix"` with no trailing empty `<mo>` or matching `mathml_to_latex` expectations).
   - *Rationale*: Keeps fence boundary editing (`is_fence_delimiter` in `eqntide-editor`) and OMML/MTEF single-sided fence detection consistent.

3. **Row insertion & fence promotion in `Editor` (`crates/eqntide-editor/src/lib.rs`)**:
   - Add `Editor::insert_matrix_row_after_caret(&mut self) -> bool` (or unify with `Enter` handler `Editor::insert_row_or_promote_fence(&mut self) -> bool`):
     - Walk ancestors from `self.caret().node`:
       1. **Inside `mtable`**: Find the enclosing `mtr` index `row_idx` inside `mtable`. Insert a new `mtr` (with `cols` empty `<mtd>` cells) at `row_idx + 1` (not only at the end), and set caret to `(new_row.children[0].id, 0)`.
       2. **Inside a one-sided left fence without `mtable`**: Detect an enclosing `mrow` whose first child is a stretchy prefix `<mo>` with text `"{"` or `"["`, no closing postfix fence `<mo>`, and the caret is inside the content slot (not already inside an `mtable`). Move the content inside that slot (splitting around or keeping all children of the slot before/after? For equation entry, moving all existing children of the slot into row 1 `<mtd>` and adding an empty row 2 `<mtd>` matches user expectation when pressing `Enter` at the end of `x + 2 = 1`, or splitting at caret if caret is mid-expression — keeping existing expression in row 1 when caret is at end, and splitting children at caret offset if mid-row, is clean and predictable). Wrap both `<mtd>`s inside `<mtable columnalign="left">`, place it right after the opening `<mo>`, and move the caret to row 2's `<mtd>` offset 0.
   - *Alternative considered*: Making `Enter` insert a literal `\n` token. Rejected because `insert_text` strips control chars and MathML/OMML/MTEF require `mtable` for multi-line layout.

4. **Shortcut & Keyboard dispatch (`desktop/src/app.rs`)**:
   - In `keyboard()`, when `self.command_input.is_none()` and `self.ime.is_empty()` and `!modifiers.command && !modifiers.alt`, handle `Key::Enter` by calling the new `Editor` method.
   - Because `command_key()` runs earlier in the event loop (`desktop/src/app.rs:3120`), active `\` commands continue to intercept `Key::Enter` before the canvas handler sees it.

5. **Empty row deletion in `Editor::delete` (`crates/eqntide-editor/src/lib.rs`)**:
   - Inside `Editor::delete(backward)`, when there is no selection and the caret sits inside an effectively empty cell (either `<mtd/>` or an empty `<mrow/>` inside `<mtd>`) whose parent `<mtr>` has all cells empty and whose enclosing `<mtable>` has `> 1` rows:
     - Remove that `<mtr>` at `row_idx`.
     - Place the caret using `stops()` on the target cell in the adjacent surviving row (`stops.last()` of `row_idx - 1` when `backward && row_idx > 0` or when `Delete` removes the last row; `stops.first()` of the shifted row at `row_idx` otherwise).
   - *Rationale*: Keeps `− row` (last-row button) unchanged while letting keyboard users delete accidental or middle empty rows naturally with `Backspace` / `Delete` without leaving the keyboard or risking accidental deletion of partially populated multi-column rows or single-row tables.

## Risks / Trade-offs

- **Two-row vs. single-row `Enter` inside a nested fraction**: If the caret is inside `\frac{a}{b}` which itself is inside row 1 of a system, walking ancestors finds the enclosing `mtr`/`mtable` and adds a new equation row below row 1, which is the expected behavior. Still, unit tests should verify nested structures don't misidentify the target `mtr`.
- **MTEF/OMML roundtrip for `[` + `mtable` (`\left[ \begin{array}{l} ... \end{array} \right.`)**: Existing tests verify `{` + `mtable` and `\left[ x \right.` separately; `test` coverage must explicitly verify `[` + `mtable` through `parse_latex_to_mathml`, `mathml_to_latex`, `omml`, `mtef`, and `render_interactive`.
- **Palette width**: Adding 4 templates to **Fences** adds ~2 columns to the 3-row horizontal scroll grid; horizontal wheel scroll in `desktop-palette-navigation` already accommodates overflow.
