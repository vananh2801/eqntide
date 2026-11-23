# Design

## Context

WPS ribbon Toggle TeX overlaps HTTP requests up to eight concurrent conversions, then separately applies replacements in reverse order while reading and editing the document through the WPS object model. Word VBA runs single-threaded on the host macro thread and is excluded from this change.

## Goals / Non-Goals

**Goals:**
Overlap bridge calculation with WPS document writes while limiting temporary conversion data. Preserve selection rules, insertion fidelity, progress, and aggregate counts.

**Non-Goals:**
Do not alter Word VBA `ToggleLatexMacro` concurrency or packaged DOTM behavior, and do not change `/convert/snippet` bridge API semantics.

## Decisions

### Retire results in reverse document order

Resolve lightweight target metadata and order targets before starting the batch. For the equation-to-LaTeX direction, include OMML and MathType in the same ordering. Read and compact source XML at dispatch where permitted. Preserve shape classification and existing normalization and replacement behavior.

Admit up to eight items. Each item occupies a slot until its replacement or failure is processed. Associate each asynchronous result with its source target and write only the next item in order. Refill immediately when capacity opens. Yield to the host between bounded groups of replacements. Release request and response buffers after use.

### Use the existing host integration

In WPS, use the existing asynchronous HTTP transport and event-loop yields. Schedule a drain when the next ordered result settles and retain the existing verified insertion and recovery paths. Report conversion and replacement progress from separate counters because both can advance during the same batch.

### Protect the target during yields

Prevent a second Toggle TeX from mutating the same document while a batch is active. Keep the original document identity and verify each live target before applying a result. If the document closes or the remaining targets cannot be verified safely after a user edit, stop writes and clean up pending work. Where the source is intact or restored, isolate individual failures and continue. Use the existing host recovery paths for partial replacements and stop if recovery cannot be verified.

## Risks / Trade-offs

A slow item can delay ready results behind it. Keeping completed items inside the limit bounds temporary conversion data in that case. Host writes can also delay callback processing, so keep write groups short and measure the overlap on WPS Writer.

Measure time to first replacement, total wall time and peak buffered items against the baseline WPS implementation. Record bridge and replacement timings separately and account for overlap when interpreting them. Measure peak process memory on large selections separately from buffer counts, since document objects and Undo also use memory.
