# Design

## Context

The WPS batch path submits up to eight snippet requests concurrently, waits for conversion results, and applies replacements from the end of the selection toward the start. It yields to WPS after each group. The existing insertion helper pre-deletes the target, imports a Flat OPC package, locates the inline shape, and adjusts formatting. The reference run identifies insertion as the dominant phase. It does not tell us how the cost is distributed within that phase.

## Goals / Non-Goals

- Meet the performance and fidelity criteria in the [spec delta](specs/office-addin-batch-reliability/spec.md) for the WPS ribbon's batch LaTeX-to-MathType path.
- Preserve the existing bridge request contract and the insertion paths for single-equation editing and taskpane upgrades.

## Decisions

### Measure the WPS operations and benchmark on the real host

Use aggregate elapsed times for range preparation, bridge conversion, document replacement, shape lookup, and post-insert formatting. Measure insertion wall time across yields from the first replacement attempt through the last replacement and its formatting. Emit one summary per batch without formula text or XML. Record the baseline revision, WPS version, and three or more runs per version on fresh copies of the same reference document. The observed 19m 27s is an initial reference. The measured baseline median determines the acceptance threshold.

### Optimize verified plain-text ranges first

Use a batch-specific path for verified LaTeX text. Recheck the live target immediately before mutation and keep reverse-offset ordering. Trial a direct replacement via `Range.InsertXML` on WPS Linux to remove the standalone pre-delete. Compare imported size and baseline against the current output before suppressing any setter. Cache range and collection handles within an insertion, and query document-wide shapes only if the local range fails to locate the inserted object. Retain a verified fallback for WPS variants or document constructs that need it.

Establish the original text and formatting before mutation. On failure, inspect the affected span before deciding whether to restore the original or accept a verified completed insertion. Retry only when the target is confirmed intact or restored. If the resulting span cannot be verified or restored, stop document mutation, identify the affected range, and surface a recovery error.

### Keep WPS document writes serialized and show progress

Retain the per-batch limit of eight conversion requests and prevent overlapping replacement runs on the same document while offsets are live. Continue yielding between bounded groups. If grouped work still blocks the UI for too long, cap each group by elapsed time as well as item count. Use a tested WPS status surface and throttle progress updates so they do not add a host call per formula. Clear progress on completion and failure. A host screen-update option can be benchmarked if WPS exposes it reliably, with the prior value restored on every exit path.

### Reduce import count if per-formula changes miss the target

If profiles show that imports dominate and the optimized single-range path misses the target, prototype one Flat OPC replacement for multiple formulas in an eligible paragraph. Merge the OLE, EMF, and relationship parts without collisions and preserve the untouched runs and paragraph structure. Use grouped replacement only where the source ranges can be mapped to XML unambiguously and verified after import. Retain serialized per-formula replacement for complex or unverified paragraphs. Measure the proportion of formulas eligible for grouping on the reference document before selecting this path.

## Risks / Trade-offs

- WPS can import XML successfully while exposing the inserted shape late. Distinguish object verification from optional formatting so a missing shape handle alone cannot trigger a second import.
- Direct or grouped import can change paragraph marks, table boundaries, or formatting. Enable each fast path only after live WPS checks on mixed content and a failure-recovery check.
- WPS mocks can exercise ordering, pacing, and recovery logic. Timing and layout acceptance require WPS Writer itself.
