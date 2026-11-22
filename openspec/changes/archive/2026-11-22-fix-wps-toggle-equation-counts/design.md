# Design

## Context

The WPS batch collector uses quick shape metadata first. For ambiguous shapes it calls a shared XML heuristic that accepts generic drawing, picture, and image-data tags. The bridge can reject those candidates only after the batch has counted them.

## Goals / Non-Goals

Correct WPS equation-to-LaTeX discovery and reporting for mixed selections. Preserve MathType OLE, OMML, and any supported equation preview formats.

## Decisions

Use WPS metadata to handle confirmed equation and non-equation shapes. For ambiguous shapes, require equation-specific evidence tied to the candidate in its Flat OPC package, such as a MathType ProgID or an image referenced by the candidate with recognizable embedded equation data. Resolve references to the candidate rather than counting unrelated package parts. Keep the generic XML heuristic for callers that use it to discover possible targets, and use stricter classification for the batch.

Exclude confirmed ordinary illustrations before counting and sending requests. Keep confirmed equations in the totals even when conversion fails. Retain bounded requests and reverse-order replacements.

Toggle LaTeX uses the highlighted range and the objects WPS exposes as selected. WPS can expose a clicked object even when the selection start and end coincide. For an unselected caret, Toggle LaTeX skips adjacent object lookup and does not infer a selection from the caret XML. Delimited LaTeX detection at the caret still applies.

## Risks / Trade-offs

A picture without OLE metadata can still be a supported equation preview. Cover that case before tightening classification. Check the reference document on a fresh copy and correlate object types with rejected candidates if the totals still differ.
