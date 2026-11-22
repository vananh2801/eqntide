# Design

## Context

Word uses VBA regex matches to identify candidate spans, then maps text offsets to Word ranges. A candidate that begins at an escaped dollar can consume the opening delimiter of the next real formula.

## Goals

Preserve formula boundaries and absolute text offsets for selections and caret operations, including runs of consecutive backslashes.

## Decisions

- Check the backslash count at dollar delimiter candidates and resume scanning after a rejected opener so a valid delimiter inside the rejected span stays eligible. Filtering only the final match list would lose that delimiter.
- Apply the delimiter rule to inline and display dollar formulas. Retain the existing range verification and validation for table boundaries and invalid spans.

## Risks and trade-offs

Resuming from a rejected opener affects offset calculation and can add scanning work. Verify adjacent formulas, odd and even backslash runs, both dollar delimiter forms, and a caret inside a formula.
