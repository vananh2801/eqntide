# Tasks

## 1. Measure the baseline

- [x] 1.1 Add aggregate timing for range preparation, bridge conversion, replacement, shape lookup, and formatting. Verify on WPS Linux that a batch emits one summary without LaTeX or XML content and reports insertion wall time separately.
- [x] 1.2 Record at least three baseline runs on fresh copies of the 4,336-formula selection. Verify the report identifies the baseline revision and WPS environment and records insertion times and the 4,334 converted and 2 skipped outcomes.

## 2. Optimize verified ranges

- [x] 2.1 Evaluate direct replacement of verified LaTeX ranges and reuse WPS range and shape handles. Enable the fast path only after checking real WPS output for adjacent formulas, mixed font sizes, paragraph ends, table cells, bookmarks, and existing OLE objects.
- [x] 2.2 Verify the size, baseline, editable MathType object, and EMF preview after import before removing any post-import setter. Cover missing or delayed shape handles and confirm they do not trigger a duplicate insert.
- [x] 2.3 Revalidate each live target before mutation and preserve the original range on insertion failure. Exercise stale offsets, unsupported formulas, a failing import, and an import that changes the range before reporting failure. Confirm recovery or a clear stop if the affected span cannot be verified.

## 3. Maintain responsiveness

- [x] 3.1 Keep conversion concurrency at eight and document mutations serialized with yields. Extend the WPS batch regression test to exercise overlapping runs and reverse-offset replacement.
- [x] 3.2 Show throttled progress during insertion and clear it on success or failure. Verify on WPS Linux that the processed count advances and that the final converted and skipped counts agree with the document.

## 4. Verify the performance target

- [x] 4.1 Compare at least three optimized runs against the baseline on the same WPS build and machine, using fresh copies each time. If import dominates and the target is missed, measure grouping eligibility and trial paragraph-level imports for eligible ranges. Validate their layout and recovery on WPS before enabling them, then repeat the measurements.
- [x] 4.2 Run the WPS regression suite and record the final benchmark. Verify at least 50% lower median insertion time, the same 4,334 converted and 2 skipped formulas, and intact surrounding content and MathType formatting.
