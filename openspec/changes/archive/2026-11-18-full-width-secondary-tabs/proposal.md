# Proposal

## Why

When switching from the equation editor to the **Batch DOCX**, **Settings**, or **About** tabs on the desktop app, the left sidebar hides and the central panel expands to the full window width (`1040px` minimum, or wider when resized), yet those views render narrow or unframed content hugging the left edge. In **Settings**, cards are capped at `540px`; in **About**, content sits left-aligned beneath a full-width separator; and in **Batch DOCX**, controls cluster in the top-left corner—leaving a large empty expanse on the right and making all three tabs feel misaligned with the window.

## What Changes

- Expand the **Batch DOCX**, **Settings**, and **About** views to span the full available central panel width so their surfaces and scrollbars match the window width consistently.
- Remove the `540px` maximum card width cap in **Settings**, disable horizontal `auto_shrink` on vertical scroll areas, and arrange card controls cleanly across the full card width.
- Wrap **About** content (header, description, separator, metadata grid) inside a full-width card surface matching the app's `8px` card framing.
- Wrap **Batch DOCX** controls in a full-width card surface and add a full-width drop-zone / empty-state area when no files are queued, aligning with the full-width progress bar and file list when files are present.

## Capabilities

### New Capabilities

<!-- None -->

### Modified Capabilities

- `desktop-ui-theme`: Add a requirement that secondary full-window tabs (**Batch DOCX**, **Settings**, **About**) expand their card containers, drop-zone/empty states, and scroll areas to match the central panel width without shrinking or clustering to the left.

## Impact

- Affected code: `desktop/src/app.rs` (`batch_ui`, `settings_ui`, `about_ui`, and associated layout tests).
- No changes to public APIs, dependencies, settings schema, or core conversion engines.
