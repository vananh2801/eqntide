# Design

## Context

[The proposal](proposal.md) covers the current differences between desktop views. Keyboard Shortcuts already uses a solid scrollbar with room beside its row controls. Batch DOCX and the shortcut list explicitly force scrollbar visibility. Canvas and LaTeX apply local scrollbar colors and currently override some floating-scrollbar settings. The palette ribbon has a fixed height sized for three button rows and bottom padding.

## Goals / Non-Goals

- Define scrollbar geometry in the shared desktop theme and apply it across renderer profiles.
- Keep scroll input, offsets, and nested wheel handling in the existing scroll areas.

## Decisions

### Install a solid scrollbar style for both themes

Set `egui::style::ScrollStyle::solid()` in the shared desktop theme. Use the existing 8-point scrollbar as the default and keep the shortcut list's 10-point gap for its row actions. The Equation Canvas, LaTeX source box, and palette ribbon use a 6-point scrollbar with a subdued neutral handle at rest and accent feedback during interaction. Remove floating geometry and opacity overrides from the app and canvas so GPU and software profiles share the same layout. Keep the renderer settings for unrelated animations and visual effects.

### Use each scroll area's overflow detection

Use `VisibleWhenNeeded` for single-axis scroll areas and remove the forced-visibility overrides in Palette Shortcuts and Batch DOCX. The Equation Canvas already knows its content size. Determine its enabled scroll axes from the full viewport, then account for any gutter added on the other axis. This lets both gutters close when a smaller formula fits the full viewport. Keep the existing scroll area IDs, virtualized lists, and wheel handling.

### Give the palette a scrollbar row when it scrolls

Use the active category's existing column calculation, including the custom matrix tile, to determine whether the tile grid exceeds the ribbon width. Size the ribbon to include the horizontal scrollbar gutter only in the overflow case. The ribbon should still show three full rows at the minimum window size. Keep the palette's vertical-wheel-to-horizontal mapping.

## Risks / Trade-offs

- A solid scrollbar reduces the available viewport when another axis starts overflowing. Check the Equation Canvas near both size thresholds and confirm the existing minimum-height settings leave space for a horizontal gutter inside the canvas frame.
- LaTeX text can rewrap when a vertical gutter opens. Check text wrapping, access to the last line, and scrollbar removal after deleting enough content.
- A larger palette ribbon leaves less height for the Equation Canvas while the ribbon overflows. Verify the complete editor layout at the minimum window size and across DPI changes.
