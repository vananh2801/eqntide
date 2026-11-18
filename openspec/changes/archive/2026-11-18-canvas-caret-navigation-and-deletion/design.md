# Design

## Context

In `crates/eqntide-editor/src/lib.rs`:
- `Node::stops` emits caret stops for both `row()` containers (`offset = 0..=children.len()`) and `token()` leaves (`offset = 0..=chars`), producing 2–4 coincident stops per character or wrapper boundary.
- `Editor::delete` leaves zero-character token nodes (`<mn></mn>`, `<mi></mi>`) in the tree when deleting their last character, and does nothing when `caret.offset == 0` in a non-table `row()` or when `caret.offset == children.len()` on forward `Delete`.
- `Editor::erase_selection` handles `n.token()` and `n.row()`, then falls through to a generic "fixed-arity slot reset" that overwrites selected children with empty `<mrow/>`. Because neither `mtable` nor `mtr` is `row()`, erasing a whole-row selection converts `<mtd>` (or `<mtr>`) into `<mrow/>`, breaking `matrix.rs` cell layout (shrunken boxes) and leaving stale `&` / `\\` separators in `mathml_to_latex.rs`.
- `src/render/layout/matrix.rs` marks `<mtd>` regions (`cell.mark(mtd)`), not `<mtr>`, while `desktop/src/app.rs` highlights `editor.selected_ids()`.

See `proposal.md` for motivation and `specs/` for behavioral requirements.

## Goals / Non-Goals

**Goals:**
- Emit canonical, non-redundant stops in `Node::stops` without breaking `Editor::set_caret` / `App::hit` compatibility with existing direct-caret tests.
- Prune empty tokens upon character deletion and treat empty tokens as empty in `is_effectively_empty`.
- Implement uniform A2 (select-enclosing-structure-first) handling for `Backspace` and `Delete` in empty structural slots, plus `Move::Left`/`Move::Right` at non-empty slot boundaries.
- Fix `erase_selection()` and empty-cell `Backspace`/`Delete` for `mtable` / `mtr` / `mtd`.

**Non-Goals:**
- Changing vertical (`ArrowUp`/`ArrowDown`) navigation logic in `desktop/src/app.rs`.
- Changing `Tab`/`Shift+Tab` placeholder jumping (`Move::NextPlaceholder`/`PreviousPlaceholder`).
- Changing `Enter` row insertion or one-sided fence promotion behavior.

## Decisions

### 1. Canonical stop generation in `Node::stops`
**Choice:** Deduplicate coincident boundary stops directly in `Node::stops` so `ArrowLeft`/`ArrowRight` step cleanly across logical positions:
- For a `row()` node (`math`, `mrow`, `mtd`, `msqrt`):
  - If `children.is_empty()`: emit only `(self.id, 0)` (an empty placeholder/root stop).
  - If non-empty:
    - Do not emit `(self.id, 0)` when `children[0]` is a token or a non-opaque `row()` that already emits a leading stop; emit `(self.id, 0)` only when `children[0]` is a non-row structure (like `mfrac`, `msup`, `mtable`) or an opaque element.
    - Between `children[i]` and `children[i + 1]`, emit `(self.id, i + 1)` only if `children[i]` does NOT already end at that exact boundary (or if `children[i]` is a non-row structure/opaque node) AND avoid emitting both `(row, i + 1)` and `(next_token, 0)` at the same edge: prefer token stops `(tok, chars)` / `(tok, 0)` at token boundaries where typing naturally extends/edits the token, and `row` stops around non-row/opaque structures.
    - For fenced `mrow`s (`<mrow><mo form="prefix">...</mo>...<mo form="postfix">...</mo></mrow>`), keep the outside stops in the parent row (before/after the fence `mrow`) and the inside stops adjacent to the inner content, skipping redundant intermediate `mo` outer/wrapper stops.
- **Alternative considered:** Filtering stops visually by layout `region` coordinates in `desktop/src/app.rs`. Rejected because `eqntide-editor` is UI-independent and unit-tested without rendering, and layout-only filtering leaves structural invariants inconsistent when rendering hasn't run yet.

### 2. Empty token pruning & `is_effectively_empty`
**Choice:** In `Editor::delete`, when deleting the last character of a token whose parent is a `row()`, remove the token node from `parent.children` and place the caret at `(parent.id, child_idx)` (or at the end of the preceding token if appropriate for typing continuity; `(parent.id, child_idx)` when the row becomes empty matches placeholder rendering). Also update `is_effectively_empty` to treat empty tokens (`token() && text.is_empty()`) as effectively empty.
- **Alternative considered:** Leaving empty tokens and treating them as empty everywhere. Rejected because empty `<mn></mn>` nodes pollute MathML/OMML/LaTeX exports and create unnecessary child nodes in `mrow`.

### 3. Two-step A2 deletion from empty slots and non-empty boundaries
**Choice:**
- When `delete(backward)` cannot delete a character or sibling child inside a `row()` (`backward && offset == 0` or `!backward && offset == children.len()`):
  1. If `n.children.is_empty()` (or `is_effectively_empty(n)`):
     - If inside an `mtable` row (`mtr`):
       - If `table.children.len() >= 2` and all cells in `row` are `is_effectively_empty`: remove the empty row (existing `try_remove_empty_table_row` behavior).
       - If `table.children.len() >= 2` and the row has multiple cells with at least one non-empty cell: select the entire row by setting `anchor = (mtr.id, 0)` and `caret = (mtr.id, mtr.children.len())`.
       - If `table.children.len() == 1` and all cells in that single row are empty: select the enclosing `mtable` (or its enclosing fenced `mrow` if `mtable` is the main child of a fence `mrow`) via `select_node`.
     - Otherwise (slot of `mfrac`, `mroot`, `msqrt`, `msub`, `msup`, `msubsup`, `munder`, `mover`, `munderover`, `menclose`, or inner slot of a fenced `mrow`): climb to the immediate enclosing structure node (for a fenced `mrow`, the fence `mrow`; for `msqrt` itself when `n.tag == "msqrt"`, `n.id`; otherwise the parent structure node) and call `select_node(structure_id)`.
  2. If `n` is **non-empty** at the boundary (`backward && offset == 0` or `!backward && offset == children.len()`):
     - Call `move_caret(if backward { Move::Left } else { Move::Right }, false)` to step smoothly into the neighboring slot or outside the structure.
     - Special care for fenced `mrow`: if the caret is at `(inner_mrow, 0)` inside `( a )` and `a` is non-empty, `Move::Left` moves outside before `(` (or before `a` -> outside).
- Note on `transact`: `select_node` and `move_caret` do not mutate `root`, so `transact` does not push spurious undo entries or trigger `render_key` re-renders. Pressing `Backspace`/`Delete` a second time hits `erase_selection()`, which mutates `root` and pushes one clean undo step.

### 4. `mtable` / `mtr` / `mtd` handling in `erase_selection()` and row selection rendering
**Choice:**
- Using `anchor = (mtr.id, 0)` and `caret = (mtr.id, mtr.children.len())` for whole-row selection works naturally with existing `selected_ids()` (returns all `mtd.id`s in the row, every one of which already has a `region` from `matrix.rs`) and matches what drag-selecting across a row produces!
- In `erase_selection()`:
  - When `id` is an `mtable` (`start..end` selects whole `<mtr>` rows):
    - If `end - start < n.children.len()` (at least one row remains): `n.children.drain(start..end)` and place caret in the adjacent surviving row (`start.min(n.children.len() - 1)`), instead of turning `<mtr>` into `<mrow/>`.
    - If `start == 0 && end == n.children.len()` (all rows selected): delete the whole `mtable` from its parent (or if `mtable` is inside a fence/root, remove `mtable` or clear to single empty cell—wait: if the whole `mtable` was selected via `select_node(table_id)`, `id` is the parent of `mtable`, not `mtable` itself; if user dragged across all rows of `mtable`, removing `mtable` or leaving empty is consistent if `mtable` is removed from its parent row).
  - When `id` is an `mtr` (`start..end` selects `<mtd>` cells in that row):
    - If `start == 0 && end == n.children.len()` (all cells in the row are selected) and the enclosing `mtable` has `>= 2` rows: remove that `<mtr>` row from `<mtable>` and move caret to an adjacent row!
    - If `start == 0 && end == n.children.len()` and the enclosing `mtable` has only `1` row: if the row already has content, clear all cells to empty `<mtd/>` (so the user clears the content first); if all cells were already empty, delete the `mtable` (or enclosing fence) — or if the 1-row empty table was selected via `select_node(table_id)`, `id` is already the parent of `mtable` so `n.children.drain` deletes the `mtable`.
    - For a partial selection (`end - start < n.children.len()`): reset each selected child to an empty `mtd` (`child.tag = "mtd".into(); child.text.clear(); child.attrs.clear(); child.children.clear();`) instead of `"mrow"`.

## Risks / Trade-offs

- **Existing tests relying on specific `Node::stops` indices or `delete` steps**: Any unit test in `crates/eqntide-editor` or `desktop/src/app.rs` that checks `move_caret(Move::Left/Right)` step counts or `caret_rect` across every stop must be audited and updated if it asserted old duplicate stops (e.g., `caret_rect_aligns_consistently_across_tokens_and_row_stops` sets carets explicitly with `set_caret`, which remains valid). → Audit all `move_caret` and `delete` tests during implementation.
- **Typing while A2 structure is selected**: With A2, typing a character immediately after Backspace-selecting a structure overwrites the selected structure (standard selection replacement). → This is explicitly desired by the user ("dùng A2 đi") because the highlight clearly previews the impending deletion.
- **Unfenced vs fenced single-row `mtable`**: A 1-row empty `mtable` inside a fenced system (`\left\{ \begin{array}{l} □ \end{array} \right.`) should select the enclosing fence `mrow` rather than leaving dangling empty `{ \right.` delimiters. → Helper detects when `mtable` is the sole content child of a fence `mrow` and selects the fence `mrow`.
