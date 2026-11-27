# Proposal

## Why

Desktop scrollbars behave differently across views. Some expand on hover or overlap content, while the fixed scrollbar in Keyboard Shortcuts can occupy space even when a filtered list fits.

## What Changes

- Give desktop scrollbars a fixed thickness and dedicated space beside or below content.
- Show each scrollbar for as long as its axis has scrollable overflow, including while the pointer is elsewhere. Return the space to content when it fits.
- Keep scrollbars legible in Light and Dark themes and leave all three ribbon rows accessible when the palette scrolls horizontally.

## Capabilities

### New Capabilities

### Modified Capabilities

- `desktop-ui-theme` defines consistent desktop scrollbar layout and visibility across views.
- `desktop-custom-shortcuts` updates scrollbar visibility for short and filtered shortcut lists.

## Impact

The desktop theme and scroll areas in Editor, Settings, Keyboard Shortcuts, Batch DOCX, and About are affected. Both GPU and software rendering use the same scrollbar behavior.
