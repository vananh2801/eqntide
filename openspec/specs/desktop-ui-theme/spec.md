# desktop-ui-theme Specification

## Purpose

Defines the modern slate visual design system, technical corner-radius and spacing rules, unified hover and tinted-accent selection states, and per-mode Equation Canvas color presets for the desktop equation editor.

## Requirements

### Requirement: Modern slate Light and Dark theme palette
The desktop application SHALL style both Light and Dark modes using a layered slate neutral palette with soft eye-friendly surfaces, readable foreground contrast, and a consistent accent color across selections, focus rings, sliders, and primary actions.

#### Scenario: Light and Dark modes apply layered neutral surfaces and legible contrast
- **WHEN** the desktop application renders in either Light mode or Dark mode
- **THEN** window backgrounds, panels, cards, and interactive controls use distinct layered neutral tones with high-contrast text and icons

### Requirement: Technical corner radii and harmonized spacing
The desktop application SHALL use two consistent corner-radius tiers (`6px` for interactive controls and inputs, `8px` for canvas, cards, modals, and popups) and a `4px`-based spacing and margin scale while keeping all sidebar controls visible and the palette within three rows at `1040x560`.

#### Scenario: Controls and surfaces follow the two-tier radius scale
- **WHEN** buttons, sliders, text inputs, palette tiles, sidebar items, cards, and the equation canvas are displayed
- **THEN** controls share the `6px` corner radius and surface containers share the `8px` corner radius without ad-hoc radius overrides

#### Scenario: Minimum window layout preserves full sidebar and three-row palette visibility
- **WHEN** the application is rendered at the minimum window size of `1040x560`
- **THEN** harmonized panel padding and control spacing keep all sidebar buttons unclipped without vertical scrolling and keep the palette within three rows

### Requirement: Unified hover, active, and disabled interaction feedback
All interactive controls—including primary and secondary buttons, sidebar category tiles, sidebar navigation tabs, palette items, radio buttons, checkboxes, combo boxes, and canvas preset swatches—SHALL use unified hover and press feedback without expanding widget bounds on hover and SHALL display a pointing-hand cursor on hover when enabled.

#### Scenario: Hovering enabled controls applies consistent surface and cursor feedback
- **WHEN** the user moves the pointer over any enabled button, sidebar item, palette button, radio control, checkbox, combo box, or color swatch
- **THEN** the control transitions smoothly in background and border tone without growing in size and shows a pointing-hand cursor

#### Scenario: Disabled controls suppress interactive hover styling
- **WHEN** a control is disabled during busy states or unparsed LaTeX source drafts
- **THEN** it renders in a muted non-interactive state and does not show active hover emphasis

### Requirement: Tinted-accent selection state with contrasting icons
Selected navigation tabs and sidebar category buttons SHALL highlight using a tinted accent background paired with a thin accent border and contrasting accent foreground text and icons in both Light and Dark modes.

#### Scenario: Selected sidebar category keeps icon contrast in Light and Dark modes
- **WHEN** a sidebar category button is selected in either Light mode or Dark mode
- **THEN** the button displays a tinted accent background and border while its math icon is tinted in the matching high-contrast foreground accent color

### Requirement: Curated per-mode Equation Canvas presets
The Settings view SHALL provide curated color presets for the Equation Canvas independently for Light mode and Dark mode, defaulting to a soft light paper preset in both modes and adapting equation ink, placeholder styling, caret, selection highlight, scrollbars, and the synchronized LaTeX source box to maintain contrast on every preset.

#### Scenario: Default Equation Canvas uses a soft light paper surface
- **WHEN** the desktop application starts with default settings in either Light mode or Dark mode
- **THEN** the Equation Canvas and LaTeX source box render on the soft light paper preset with dark ink and high-contrast caret, selection, and placeholder overlays

#### Scenario: Selecting a Dark mode dark canvas preset adapts ink and overlays
- **WHEN** the user selects a dark canvas preset for Dark mode in Settings and switches to Dark mode
- **THEN** the Equation Canvas and LaTeX source box switch to that dark surface and render equation ink, source text, placeholders, caret, selection highlight, and scrollbars in light-on-dark contrasting colors

### Requirement: Uniform button typography and height tiers
The desktop application SHALL define explicit UI text styles and standardize interactive text buttons into consistent height tiers (`30px` standard buttons including footer `Save`, `Undo`/`Redo`, and `DragValue`; `34px` sidebar navigation tabs; and `24px` compact inline action buttons with non-zero vertical padding) so adjacent controls share matching font size and balanced visual proportions.

#### Scenario: Footer controls and Save button align at standard button height
- **WHEN** the editor footer is rendered in either standalone or Office mode
- **THEN** the `Save`, `Undo`, `Redo`, and font-size `DragValue` controls share the `30px` standard control height and consistent button font sizing

#### Scenario: Compact inline controls avoid collapsed zero-padding text buttons
- **WHEN** contextual matrix controls (`+ row`, `− row`, `+ col`, `− col`) or status bar action buttons (`Retry`, `Next session`) are displayed
- **THEN** they render at the `24px` compact button height with balanced vertical padding and the same UI text family scale

### Requirement: Consistent typographic scale for math button icons
Mathematically rendered icons inside sidebar category tiles and palette buttons SHALL render at a capped base em size rather than upscaling short symbols to fill the bounding box, only scaling down proportionally when an expression exceeds its tile bounds.

#### Scenario: Short symbols and standard templates share the same base glyph size
- **WHEN** sidebar category icons or palette icons such as `★`, `αΩ`, `∀ ∈`, `□^□`, `max_□`, and `min_□` are rendered
- **THEN** symbols that fit within the tile bounds at the base em size are rasterized at that exact shared em size instead of being stretched to different heights

### Requirement: Bounded vertically scrollable LaTeX source box
The desktop application SHALL render the LaTeX source editor inside a fixed-height container (`88px` panel / `80px` input height below the `8px` top gap) with internal vertical scrolling so multi-line LaTeX input never overflows into the Equation Canvas or clips beneath the footer.

#### Scenario: Entering multiple newlines keeps LaTeX source bounded and scrollable
- **WHEN** the user presses `Enter` multiple times inside the LaTeX source box so the text exceeds the visible rows
- **THEN** the LaTeX source container keeps its fixed height without overlapping the Equation Canvas above or clipping beneath the footer below, and scrolls vertically to keep the active lines accessible

#### Scenario: LaTeX source scrollbar adapts to the active canvas preset
- **WHEN** the user enters multi-line content in the LaTeX source box under any Light or Dark mode canvas preset
- **THEN** the LaTeX source container and its vertical scrollbar use the active canvas preset colors for legible contrast

### Requirement: Compact default window and Equation Canvas sizing
The desktop application SHALL launch with a default window inner size of `1040x560`, matching the minimum window size so the Equation Canvas opens at a compact, balanced height while preserving full sidebar and three-row palette visibility.

#### Scenario: Default launch opens at 1040x560 with balanced canvas height
- **WHEN** the desktop application opens with its default viewport configuration
- **THEN** the window inner size is `1040x560`, the LaTeX source panel reserves its increased fixed height, and the Equation Canvas fills the remaining central area without excessive empty vertical space

### Requirement: Full-width secondary tab layouts
The desktop application SHALL render the **Batch DOCX**, **Settings**, and **About** views at the full available central-panel width, framing their primary sections in full-width card surfaces without horizontal auto-shrinking or left-clustering.

#### Scenario: Settings cards and scrollbars span the full window width
- **WHEN** the user opens the **Settings** tab at `1040x560` or a wider window size
- **THEN** all setting section cards (`Canvas zoom`, `Appearance`, `Startup focus`) and the vertical scroll area expand to match the full available central-panel width instead of capping at `540px` on the left

#### Scenario: About metadata renders inside a full-width card
- **WHEN** the user opens the **About** tab at `1040x560` or a wider window size
- **THEN** the application summary, divider, and metadata grid render inside a full-width card surface aligned with the central-panel width

#### Scenario: Batch DOCX controls and empty state fill the central panel width
- **WHEN** the user opens the **Batch DOCX** tab with no files queued or with files queued
- **THEN** the batch control bar sits inside a full-width card surface, and the drop-zone empty state or scrollable file list spans the matching full central-panel width

### Requirement: Consistent selectable theme option buttons in Settings
The **Settings** view SHALL present **Light** and **Dark** theme choices as a selectable option button group styled identically to the **Startup focus** selector below it, rather than a single toggle button.

#### Scenario: Switching theme via Light and Dark option buttons
- **WHEN** the user opens **Settings** while Dark mode is active and clicks the **Light** button
- **THEN** the application switches to Light mode, persists the `Light` theme preference, highlights **Light** as selected, and renders **Dark** unselected using the shared selectable button styling
