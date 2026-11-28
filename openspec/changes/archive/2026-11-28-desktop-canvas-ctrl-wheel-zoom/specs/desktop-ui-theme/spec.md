## ADDED Requirements

### Requirement: Ctrl+Wheel cursor-anchored zoom on Equation Canvas
Scrolling the mouse wheel while holding `Ctrl` (or the platform command modifier) over the enabled Equation Canvas SHALL adjust Equation Canvas zoom across `150%..=400%` in `10%` increments, synchronize immediately with the footer **Zoom** control, anchor the zoom around the pointer position, and avoid triggering when the footer **Zoom** control is disabled or when the pointer is over the inline `\command` popup.

#### Scenario: Ctrl+Wheel over Equation Canvas zooms and synchronizes with footer Zoom
- **WHEN** the Equation Canvas is enabled and the user scrolls up or down with `Ctrl+Wheel` while the pointer is inside the Equation Canvas viewport
- **THEN** Equation Canvas zoom steps by `±10%` (`±0.1`) clamped to `150%..=400%`, the footer **Zoom** control updates immediately to match, the Equation Canvas does not scroll, and the `LaTeX source` box font size remains unchanged

#### Scenario: Equation Canvas Ctrl+Wheel anchors zoom at the pointer location
- **WHEN** an overflowing equation is scrolled inside the Equation Canvas and the user performs `Ctrl+Wheel` over a specific point inside the canvas viewport
- **THEN** the Equation Canvas scroll offsets update in the same frame so the equation point beneath the pointer remains aligned under the pointer (clamped to valid scroll bounds)

#### Scenario: Disabled footer Zoom or active command popup suppresses Equation Canvas Ctrl+Wheel
- **WHEN** `draft_dirty` is true, a modal dialog or `busy` state disables the editor footer, or the pointer is hovering over the open inline `\command` popup while the user performs `Ctrl+Wheel`
- **THEN** Equation Canvas zoom remains unchanged

### Requirement: Ctrl+Wheel font-size zoom on LaTeX source box
Scrolling the mouse wheel while holding `Ctrl` (or the platform command modifier) over the enabled `LaTeX source` box SHALL adjust only the `LaTeX source` monospace font size across `8.0..=32.0 pt` in `0.5 pt` increments (default `13.0 pt`) without changing Equation Canvas zoom, equation export font size, footer **Font size** / **Zoom** controls, or the fixed `88px` / `80px` panel height.

#### Scenario: Ctrl+Wheel over LaTeX source scales monospace text independently
- **WHEN** the user scrolls up or down with `Ctrl+Wheel` while the pointer is inside the `LaTeX source` box (including during an active `draft_dirty` edit)
- **THEN** the `LaTeX source` monospace font size steps by `±0.5 pt` clamped to `8.0..=32.0 pt` without vertically scrolling the box, without resizing the `88px` / `80px` container, and without altering Equation Canvas zoom or footer **Font size** / **Zoom** values

#### Scenario: Footer Zoom and Font size controls do not affect LaTeX source font size
- **WHEN** the user adjusts the footer **Zoom** or **Font size** control via drag, text entry, or wheel
- **THEN** the `LaTeX source` box retains its configured monospace font size
