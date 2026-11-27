# Design

## Context

See `proposal.md` for motivation and `specs/desktop-ui-theme/spec.md` for the updated Light/Dark palette contract.

In Light mode, the desktop app should step progressively darker from the outer application backdrop down into panels, cards, and interactive controls (`app_bg` > `panel_bg` > `card_bg` > `control_bg` > `control_hover_bg` > `control_active_bg`). Because `remove-desktop-borders` removed 1px card and control borders, `control_bg` must remain visually distinct from every surface it sits on (`app_bg` for palette tiles and `Back`, `panel_bg` for sidebar/footer/status buttons, and `card_bg` for Settings/Batch/About/modal controls), `card_bg` must sit clearly darker than `app_bg`, and `app_bg` must stay visually distinct from the fixed `Ayu light` Equation Canvas (`pal.bg` `#fcfcfc`).
In Light mode, the desktop app should step progressively darker from the outer application backdrop down into panels, cards, and interactive controls (`app_bg` > `panel_bg` > `card_bg` > `control_bg` > `control_hover_bg` > `control_active_bg`), and tinted selected buttons (`selected_bg` > `selected_hover_bg`) should sit at the same visual depth as `control_bg` (darker than `card_bg` and `panel_bg`) rather than floating as pale bright cream cutouts over cool blue-gray cards. Because `remove-desktop-borders` removed 1px card and control borders, `control_bg` and `selected_bg` must remain visually distinct from every surface they sit on (`app_bg`, `panel_bg`, `card_bg`), `card_bg` must sit clearly darker than `app_bg`, and `app_bg` must stay visually distinct from the fixed `Ayu light` Equation Canvas (`pal.bg` `#fcfcfc`).

## Goals / Non-Goals

**Goals:**
- Keep Light-mode `app_bg` bright and airy while stepping progressively darker through neutral Ayu tones (`panel_bg` -> `card_bg` -> `control_bg` -> `control_hover_bg` -> `control_active_bg`) without cold blue cast, so cards, unselected buttons, and warm amber-tinted selected buttons harmonize naturally.
- Match Light-mode `selected_bg` (both default Ayu amber and adaptive image accents) to the `control_bg` depth (`< card_bg`) so selected toggles/tabs sit inside cards at the same weight as unselected buttons.
- Maintain WCAG AAA (`>= 7.0`) primary text contrast across `panel_bg`, `card_bg`, `control_bg`, and `extreme_bg`, and `>= 4.5` on primary/selected tokens.

**Non-Goals:**
- Modifying Dark-mode `ThemeTokens` (`dark_mode == true`) or `CanvasPalette` (`#fcfcfc` / `#0d1016`).
- Reintroducing 1px borders around cards, controls, canvas, or LaTeX source boxes.
- Changing primary accent hues, typography, spacing, or corner radii.

## Decisions

### 1. Light-mode neutral ladder (`app_bg > panel_bg > card_bg > control_bg`) & matching `selected_bg`
Set Light-mode tokens in `ThemeTokens::for_mode_with_accent(false, ...)` using balanced Ayu neutral grays (avoiding blue-gray cast):
- `pal.bg` (unchanged): `rgb(252, 252, 252)` (`#fcfcfc`) — bright `Ayu light` Equation Canvas & LaTeX paper.
- `app_bg`: `rgb(245, 245, 246)` (`#f5f5f6`) — bright central backdrop, 7 RGB steps below `#fcfcfc` so the borderless canvas and LaTeX box remain distinct while keeping open views airy.
- `panel_bg`: `rgb(237, 237, 239)` (`#ededef`) — neutral off-white sidebar, footer, and status bar.
- `card_bg`: `rgb(229, 230, 232)` (`#e5e6e8`) — clearly darker than `app_bg` (`#f5f5f6`) and `panel_bg` (`#ededef`) for cards and modals.
- `control_bg`: `rgb(214, 216, 220)` (`#d6d8dc`) — clearly darker than `card_bg`, `panel_bg`, and `app_bg`.
- `control_hover_bg`: `rgb(199, 202, 207)` (`#c7cacf`) — smoothly darker on hover.
- `control_active_bg`: `rgb(183, 187, 193)` (`#b7bbc1`) — smoothly darker on press.
- `control_disabled_bg`: `rgb(224, 226, 229)` (`#e0e2e5`) — muted fill distinct from `card_bg`, `panel_bg`, and `control_bg`.
- `extreme_bg`: `Color32::WHITE` (`#ffffff`) — crisp white track/input contrast inside `#e5e6e8` cards.
- `selected_bg`: `rgb(236, 212, 182)` (`#ecd4b6`) — warm amber tint matched to `control_bg` luminance (darker than `card_bg`).
- `selected_hover_bg`: `rgb(226, 196, 160)` (`#e2c4a0`) — deepens cleanly on hover.
- `selected_fg`: `rgb(118, 52, 0)` (`#763400`) — deep rich amber foreground exceeding 6:1 contrast on `selected_bg`.
- Adaptive Light-mode `derive_ui_accent_roles`: lower `selected_bg` lightness from `0.91` to `0.82` and `selected_hover_bg` from `0.86` to `0.76` (with `selected_fg` starting at `0.24`) so image-derived selected buttons also sit deeper than `card_bg`.

*Alternative considered*: SettingSetting `app_bg` toto `#ffffffffffff` andand darkeningdarkening `pal.bg` belowbelow `#ffffffffffff`. Rejected to preserve the `Fixed Ayu Light and Ayu Dark Equation Canvas surfaces` contract (`#fcfcfc` in Light mode).

### 2. Preserve popup contrast on shadowless backends (`combo_popup_style`)
`ThemeTokens::combo_popup_style` uses `popup_fill = self.control_bg` (`#d6dbe3d6d8dc` in Light mode), which contrasts cleanly against `#e6e9eee5e6e8` `card_bg` on `Software` / `VirtualGpu` shadowless renderers while keeping `control_hover_bg` (`#c7cdd7c7cacf`) and `selected_bg` (`#ecd4b6`) distinct inside the menu. In `keyboard.rs`, the inline `\command` popup keeps `card_bg` (`#e6e9eee5e6e8`) for the outer popup and `control_bg` (`#d6dbe3d6d8dc`) for the recessed command input box.

## Risks / Trade-offs

- **[Risk] Translucent `Image` mode contrast on `#e6e9ee` `card_bg` over dark wallpapers** → `text_primary` (`#2e3238`) exceeds `10.5:1` over opaque `#e6e9ee`, and at `0.70` `Overlay opacity` over pure black/white photos with the `0.65` scrim (`app_bg` `#f4f5f7`) exceeds `8.0:1` (`>= 7.0` WCAG AAA verified by `theme.rs` unit tests).
- **[Risk] `disabled_accent()` contrast against `#e6e9ee` `card_bg`** → `disabled_accent()` blends `accent` (`#ba5e06`) 58% toward `card_bg` (`#e6e9ee`), keeping a soft muted amber distinct from both `accent` and `card_bg` (verified by `theme.rs` tests).
- **[Risk] Translucent `Image` mode contrast on `#e5e6e8` `card_bg` over dark wallpapers** → `text_primary` (`#2e3238`) exceeds `10.4:1` over opaque `#e5e6e8`, and at `0.70` `Overlay opacity` over pure black/white photos with the `0.65` scrim (`app_bg` `#f5f5f6`) exceeds `8.0:1` (`>= 7.0` WCAG AAA verified by `theme.rs` unit tests).
- **[Risk] `disabled_accent()` contrast against `#e5e6e8` `card_bg`** → `disabled_accent()` blends `accent` (`#ba5e06`) 58% toward `card_bg` (`#e5e6e8`), keeping a soft muted amber distinct from both `accent` and `card_bg` (verified by `theme.rs` tests).
