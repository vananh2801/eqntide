# Proposal

## Why

Typing and editing directly on the desktop Equation Canvas currently suffers from redundant invisible caret stops across wrapper `mrow` and token boundaries—often requiring 2–4 arrow key presses to move past a single character or exit a structure—and trapped or broken `Backspace`/`Delete` behavior inside empty placeholder slots and `mtable` rows. Fixing structural caret stops, token cleanup, and selection-first deletion makes direct canvas editing predictable, fast, and free of structural corruption.

## What Changes

- Deduplicate redundant caret stops in `eqntide_editor::Node::stops` so each `ArrowLeft`/`ArrowRight` step moves across one visible character, delimiter, or structural slot boundary without stopping on redundant wrapper boundaries.
- Remove empty token nodes (`mi`, `mn`, `mo`, `mtext`, `ms`) from their parent row when their last character is deleted, and treat empty tokens as empty in structural emptiness checks.
- Add symmetrical `Backspace` and `Delete` boundary handling for structural elements (`mfrac`, `mroot`, `msqrt`, `msub`, `msup`, `msubsup`, `munder`, `mover`, `munderover`, `menclose`, and fenced expressions):
  - When the caret is inside an **empty** slot, `Backspace`/`Delete` selects the enclosing structure (or fenced wrapper) so a subsequent `Backspace`/`Delete` deletes it in one step.
  - When the caret is at the start (`offset == 0`) of a **non-empty** slot on `Backspace`, move the caret to the previous stop (`Move::Left`); symmetrically, on `Delete` at the end of a non-empty slot, move the caret to the next stop (`Move::Right`).
- Extend and fix `mtable` deletion and selection erasure:
  - In a multi-column row where at least one other cell has content, pressing `Backspace`/`Delete` inside an empty cell moves the caret to the end of the row and selects the entire row (`mtr`) so a second press removes the row (or triggers table selection if single-row).
  - In a 1-row empty `mtable`, pressing `Backspace`/`Delete` selects the table (or enclosing fenced wrapper) so a second press deletes it.
  - Fix `erase_selection()` when a selection spans a table row or table cells so whole-row selections remove the `<mtr>` (when `>= 2` rows) or select/clear properly without converting `<mtd>` or `<mtr>` nodes into `<mrow/>`.

## Capabilities

### New Capabilities
- `desktop-canvas-editing`: Canonical single-step horizontal caret navigation across MathML rows, tokens, structures, and fences, empty token pruning on character deletion, and two-step selection-first deletion (`Backspace`/`Delete`) from empty structural slots.

### Modified Capabilities
- `desktop-system-equations`: Update `Backspace and Delete remove an empty table row` to cover whole-row selection when pressing `Backspace`/`Delete` in an empty cell of a partially filled multi-cell row, two-step selection-first deletion for a single-row empty `mtable`, and proper `<mtr>` row removal / `<mtd>` restoration when erasing a selected row.

## Impact

- `crates/eqntide-editor/src/lib.rs`: `Node::stops`, `Editor::delete`, `Editor::erase_selection`, `is_effectively_empty`, and structural helper methods + unit/regression tests.
- `desktop/src/app.rs`: Verify `caret_rect`, `hit`, `selected_ids`, and desktop integration tests align with deduplicated stops, token pruning, and A2 structure/row selection.
