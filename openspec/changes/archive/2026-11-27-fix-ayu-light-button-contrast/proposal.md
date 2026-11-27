# Proposal

## Why

In Light mode under the Ayu theme (`desktop/src/theme.rs`), surface and control luminance are currently inverted relative to standard light UI expectations: `app_bg` (`#e2e4e7`), `panel_bg` (`#ebecee`), and `card_bg` (`#f4f5f7`) are dimmer gray backgrounds, while idle buttons and interactive controls (`control_bg`) are pure white (`#ffffff`), brighter than every surrounding surface. Hovering or pressing a button then drops abruptly to `#d8dbe0` / `#c8ccd3` (darker than the background), reversing the visual direction between idle and hover states.

In Light mode, backgrounds should remain bright and airy while interactive buttons/controls are slightly deeper (darker) than their surrounding surfaces and deepen smoothly on hover/press—whereas Dark mode already places brighter controls (`#2f3440`) over darker surfaces (`#151820` / `#1d2028` / `#242832`) and should remain unchanged.

## What Changes

- Invert the Light-mode neutral surface-to-control luminance hierarchy in `ThemeTokens::for_mode_with_accent` so Light-mode chrome surfaces (`app_bg`, `panel_bg`, `card_bg`) are brighter while interactive controls (`control_bg`, `control_hover_bg`, `control_active_bg`) are consistently darker than all surrounding surfaces and deepen monotonically across idle -> hover -> active.
- Adjust Light-mode `extreme_bg` and `control_disabled_bg` tokens so recessed tracks (Batch progress bar) and disabled controls stay visually distinct against brighter `card_bg` / `panel_bg` / `app_bg` surfaces without reintroducing surface or control borders.
- Keep Dark mode (`dark_mode == true`) and fixed Equation Canvas surfaces (`Ayu light` `#fcfcfc` and `Ayu dark` `#0d1016`) unchanged.

## Capabilities

### New Capabilities

### Modified Capabilities
- `desktop-ui-theme`: Update Light-mode Ayu surface and control luminance layering so backgrounds stay bright and Light-mode controls render darker than surrounding surfaces across idle, hover, and active states (while keeping Dark mode unchanged).

## Impact

- Affected code: `desktop/src/theme.rs` (Light-mode `ThemeTokens` RGB values and luminance hierarchy unit tests) and any Light-mode layout/theme tests in `desktop/src/app/tests/` that assert relative surface/control ordering.
- Affected specs: `openspec/specs/desktop-ui-theme/spec.md` (via delta spec in this change).
