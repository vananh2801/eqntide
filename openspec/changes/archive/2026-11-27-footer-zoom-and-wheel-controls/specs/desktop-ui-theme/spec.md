## ADDED Requirements

### Requirement: Footer Font size and Zoom level-fill controls with wheel adjustment
The editor footer SHALL render **Zoom** (`150%..=400%` in `10%` steps, without a Reset button) beside **Font size** (`6.0..=72.0 pt` in `1 pt` steps) with external labels, `30px` height, `6px` corner radius, and shared `draft_dirty` / modal disabled gating. Both boxes SHALL omit right-side spinner arrows, paint a proportional soft-accent left fill behind centered text, and support horizontal drag, direct numeric entry, and hover mouse-wheel stepping.

#### Scenario: Footer renders Zoom beside Font size without Reset or Settings Canvas zoom card
- **WHEN** the desktop application renders in Light or Dark mode at `1056x560` or larger
- **THEN** **Zoom** (`200%` by default) sits beside **Font size** on the single footer row with external `Font size` and `Zoom` labels, no Reset button, and no `Canvas zoom` card in **Settings**

#### Scenario: Font size and Zoom boxes paint a proportional soft-accent left fill without spinner arrows
- **WHEN** **Font size** (`6.0..=72.0 pt`) and **Zoom** (`150%..=400%`) render in idle, hovered, or disabled states
- **THEN** each `30px`-tall `6px`-rounded box paints its neutral track fill (`control_bg`, `control_hover_bg` on hover, or `control_disabled_bg` when disabled), fills the left `(value - min) / (max - min)` fraction with a soft blended accent tint (muted when disabled) behind centered `text_primary` / `text_disabled` text, omits right-side up/down spinner arrows, and keeps a `1px` `tokens.accent` border when focused for text entry

#### Scenario: Dragging, typing, and hovering with mouse wheel adjust Font size and Zoom consistently
- **WHEN** the user drags horizontally, clicks to type a value, or scrolls the mouse wheel while hovering over an enabled **Font size** or **Zoom** box (outside active text-edit mode)
- **THEN** horizontal dragging adjusts **Font size** in `1 pt` steps (`4.0 / width` speed) and **Zoom** in `10%` (`0.1`) steps (`1.0 / width` speed), mouse wheel scrolling steps **Font size** by `±1 pt` and **Zoom** by `±10%` clamped to valid bounds, and disabling the footer during `draft_dirty` or modals disables both controls together

## MODIFIED Requirements

### Requirement: Uniform button typography and height tiers
The desktop application SHALL define explicit UI text styles and standardize interactive text buttons into consistent height tiers (`30px` standard buttons including footer `Save`, `Undo`/`Redo`, `Font size`, and `Zoom` controls; `34px` sidebar navigation tabs; and `24px` compact inline action buttons with non-zero vertical padding) so adjacent controls share matching font size and balanced visual proportions.

#### Scenario: Footer controls and Save button align at standard button height
- **WHEN** the editor footer is rendered in either standalone or Office mode
- **THEN** the `Save`, `Undo`, `Redo`, `Font size`, and `Zoom` controls share the `30px` standard control height and consistent button font sizing

#### Scenario: Compact inline controls avoid collapsed zero-padding text buttons
- **WHEN** contextual matrix controls (`+ row`, `− row`, `+ col`, `− col`) or status bar action buttons (`Retry`, `Next session`) are displayed
- **THEN** they render at the `24px` compact button height with balanced vertical padding and the same UI text family scale

### Requirement: Full-width secondary tab layouts
The desktop application SHALL render the **Batch DOCX**, **Settings**, and **About** views at the full available central-panel width, framing their primary sections in full-width card surfaces without horizontal auto-shrinking or left-clustering.

#### Scenario: Settings cards and scrollbars span the full window width
- **WHEN** the user opens the **Settings** tab at `1040x560` or a wider window size
- **THEN** all setting section cards (`Appearance`, `App background`, `Startup focus`, and palette shortcuts) and the vertical scroll area expand to match the full available central-panel width instead of capping at `540px` on the left

#### Scenario: About metadata renders inside a full-width card
- **WHEN** the user opens the **About** tab at `1040x560` or a wider window size
- **THEN** the application summary, divider, and metadata grid render inside a full-width card surface aligned with the central-panel width

#### Scenario: Batch DOCX controls and empty state fill the central panel width
- **WHEN** the user opens the **Batch DOCX** tab with no files queued or with files queued
- **THEN** the batch control bar sits inside a full-width card surface, and the drop-zone empty state or scrollable file list spans the matching full central-panel width

### Requirement: Accent-matched slider handle border
Settings sliders' rounded-square handles SHALL render their borders using the theme's accent color so each handle outline matches the active left-side slider fill across idle, hovered, and active states in both Light and Dark modes.

#### Scenario: Slider handle border matches the left accent fill in Light and Dark modes
- **WHEN** an enabled Settings slider renders in either Light mode or Dark mode across idle, hovered, or dragged states
- **THEN** the rounded-square handle's border stroke uses the same theme accent color as the filled left portion of the slider rail

## REMOVED Requirements

### Requirement: Vertically centered Settings Canvas zoom row
**Reason**: Canvas zoom moved out of Settings into the editor footer beside **Font size**, and the Settings **Canvas zoom** slider and `Reset (200%)` button were removed.
**Migration**: Users adjust canvas zoom directly from the **Zoom** control in the editor footer.
