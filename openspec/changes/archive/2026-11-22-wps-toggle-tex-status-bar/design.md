# Design

## Context

WPS queues snippet conversion requests before applying replacements in groups. The current progress label is attached to the Ribbon button and tracks only MathType insertion. Desktop probes on WPS Writer for Linux 12.1.2.29182 found that the native status bar did not display add-in messages. The user approved a separate lower-left indicator.

## Goals

- Verify the visible placement and lifecycle of a bottom-docked add-in pane on desktop Writer.
- Share progress reporting between both WPS batch directions.

## Decisions

- Use a WPS task pane docked at the bottom with text aligned left. Share the current message with the pane through WPS plug-in storage. Reuse the pane across batches and hide it on completion.
- Give each batch one progress lifecycle. Count completed bridge items and processed replacements as separate phases, including items that finish with an error. Throttle visible updates to roughly 500 ms and reset the display on completion or early exit.
- Use a static Ribbon label and remove the label invalidation path. Keep the existing final summary so users can see conversion outcomes after the progress message clears.

## Risks and Trade-offs

The pane temporarily reduces the document viewport height. If a Writer build cannot dock the task pane at the bottom, conversion can still run and report its final summary.
