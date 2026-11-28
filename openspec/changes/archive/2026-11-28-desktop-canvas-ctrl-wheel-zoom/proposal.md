# Proposal

## Why

Desktop users editing complex equations or dense LaTeX expressions currently have to move the pointer down to the footer **Zoom** bar to inspect visual math details, and have no way to scale the `LaTeX source` box's monospace text size for readability. Adding `Ctrl+Wheel` zoom directly over both the Equation Canvas (cursor-anchored and synchronized with the footer Zoom bar) and the `LaTeX source` box (persisted independently across sessions) brings fluid, standard editor ergonomics to both panes.

## What Changes

- Add `Ctrl+Wheel` zooming over the **Equation Canvas** (`150%..=400%` in `10%` increments):
  - Synchronizes two-way with the footer **Zoom** control and persists via `Settings::zoom`.
  - Anchors zoom around the pointer position by adjusting the Equation Canvas `ScrollArea` offset synchronously in the same frame so the equation content under the cursor stays stationary.
  - Respects the same disabled state as the footer **Zoom** control (`draft_dirty`, `busy`, active modals) and does not trigger when the pointer is over the inline `\command` popup.
- Add `Ctrl+Wheel` font-size zooming over the **`LaTeX source` box** (`8.0..=32.0 pt` in `0.5 pt` increments, default `13.0 pt`):
  - Adjusts only the `LaTeX source` monospace text size while preserving the fixed `88px` / `80px` panel height and internal vertical scrolling.
  - Leaves the footer **Zoom** and **Font size** controls unchanged in scope (they continue to affect only the Equation Canvas / equation export size, never `LaTeX source`).
  - Persists the updated LaTeX font size (`latex_font_pt`) atomically in `settings.json` so it restores across restarts without being overwritten by Office add-in sessions.

## Capabilities

### New Capabilities

<!-- None: all behavioral changes extend existing desktop UI and settings capabilities. -->

### Modified Capabilities

- `desktop-ui-theme`: Add `Ctrl+Wheel` cursor-anchored zooming on the Equation Canvas (synced and gated with footer Zoom) and independent `Ctrl+Wheel` font-size zooming (`8.0..=32.0 pt` in `0.5 pt` steps) on the bounded `LaTeX source` box.
- `desktop-settings-persistence`: Persist, sanitize, and immediately save `latex_font_pt` (`8.0..=32.0 pt`, `0.5 pt` step, default `13.0 pt`) alongside existing Equation Canvas `zoom` persistence on `Ctrl+Wheel` edits.

## Impact

- `desktop/src/settings.rs`: Add `LATEX_FONT_PT_RANGE`, `DEFAULT_LATEX_FONT_PT`, and `Settings::latex_font_pt` with deserialization default and `sanitize()` clamping/snapping.
- `desktop/src/app/mod.rs`, `desktop/src/app/settings_ui.rs`: Store `App::latex_font_pt`, load/apply it in `apply_settings()`, and mark/flush dirty settings on `Ctrl+Wheel` edits.
- `desktop/src/app/canvas.rs`: Handle `Ctrl+Wheel` over Equation Canvas (`draw_canvas`) with cursor-anchored `ScrollArea` offset updates, and over `LaTeX source` (`draw_latex_panel`) with dynamic monospace `FontId`.
- `desktop/src/app/tests/`: Add unit and UI integration tests covering cursor-anchored canvas zoom, footer sync, `draft_dirty`/modal disablement, LaTeX font scaling (`8.0..=32.0 pt`, `0.5 pt` steps), footer isolation, and settings persistence across restarts.
