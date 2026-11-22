# Proposal

## Why

Selecting the entire reference DOCX in WPS reports 4,612 equation targets, although the document contains 4,338 MathType OLE objects and no native OMML equations. WPS converted all 4,338 and reported 274 skips or errors. The target collector can treat ordinary drawings as equations because it uses a broad XML heuristic for ambiguous inline shapes.

## What Changes

- Count only confirmed equation targets in the WPS equation-to-LaTeX batch.
- Keep genuine equation conversion failures visible and preserve ordinary images and drawings.
- Require an equation selection for WPS Toggle LaTeX so a caret beside an object cannot trigger its conversion.

## Capabilities

### New Capabilities

### Modified Capabilities

- `office-addin-batch-reliability` — Classify ambiguous WPS shapes before counting or dispatching them and define which selections trigger Toggle LaTeX.

## Impact

- WPS selection scanning, batch summaries, and regression tests.
