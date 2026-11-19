# Proposal

## Why

In the **Settings** view's **Canvas zoom** card, the zoom slider is shifted upward by 6px relative to the adjacent `200%` `DragValue` and `Reset (200%)` button, and the slider's rounded-square handle renders with a high-contrast neutral text-color border (`text_primary`) instead of matching the accent blue fill on the left side of the rail. Aligning all three controls vertically and matching the handle border to the rail's accent fill makes the row visually balanced and consistent across Light and Dark modes.

## What Changes

- Vertically center the **Canvas zoom** row controls (`Slider`, percentage `DragValue`, and `Reset (200%)` button) within a `BUTTON_HEIGHT` (`30px`) horizontal row without inflating the slider's own widget height or handle size.
- Style the zoom slider's rounded-square handle border (`fg_stroke`) with `tokens.accent` across interactive states (`inactive`, `hovered`, `active`) so it matches the left-side trailing fill in both Light and Dark modes.
- Add UI regression test coverage verifying vertical alignment of the **Canvas zoom** row controls and the accent handle border color in both themes.

## Capabilities

### New Capabilities
- None

### Modified Capabilities
- `desktop-ui-theme`: Specify vertical centering of the Settings **Canvas zoom** slider row controls (`Slider`, percentage `DragValue`, and `Reset (200%)` button) and matching accent border styling on the slider handle.

## Impact

- Affected code: `desktop/src/app.rs` (`App::settings_ui` and Settings UI tests).
- No changes to persisted settings schema (`desktop/src/settings.rs`), `ThemeTokens` palette definitions (`desktop/src/theme.rs`), or public APIs.
