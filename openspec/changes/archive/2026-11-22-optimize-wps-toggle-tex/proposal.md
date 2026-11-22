# Proposal

## Why

On WPS Writer for Linux, Toggle TeX found 4,336 LaTeX formulas and converted 4,334. The insertion phase took 19 minutes 27 seconds, making the feature slow for large selections.

## What Changes

- Set a measurable target of at least 50% less time in the WPS insertion phase on the reference workload.
- Keep batch insertion responsive and report progress during the operation.
- Preserve MathType object fidelity, surrounding document content and formatting, and per-formula error reporting.

## Capabilities

### New Capabilities

### Modified Capabilities

- `office-addin-batch-reliability` — Set a performance target and require progress feedback and document fidelity for WPS batch LaTeX-to-MathType toggling.

## Impact

- WPS Writer add-in batch Toggle TeX for LaTeX-to-MathType selections and its regression coverage.
- Performance verification requires timed runs on WPS Writer for Linux with an unmodified copy of the reference document for each run.
