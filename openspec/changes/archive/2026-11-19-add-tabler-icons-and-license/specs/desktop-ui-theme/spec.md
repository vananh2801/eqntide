## MODIFIED Requirements

### Requirement: Technical corner radii and harmonized spacing
The desktop application SHALL use two consistent corner-radius tiers (`6px` for interactive controls and inputs, `8px` for canvas, cards, modals, and popups) and a `4px`-based spacing and margin scale while keeping all sidebar controls visible and the palette within three rows at `1056x560`.

#### Scenario: Controls and surfaces follow the two-tier radius scale
- **WHEN** buttons, sliders, text inputs, palette tiles, sidebar items, cards, and the equation canvas are displayed
- **THEN** controls share the `6px` corner radius and surface containers share the `8px` corner radius without ad-hoc radius overrides

#### Scenario: Minimum window layout preserves full sidebar and three-row palette visibility
- **WHEN** the application is rendered at the minimum window size of `1056x560`
- **THEN** harmonized panel padding and control spacing keep all sidebar buttons unclipped without vertical scrolling and keep the palette within three rows

### Requirement: Compact default window and Equation Canvas sizing
The desktop application SHALL launch with a default window inner size of `1056x560`, matching the minimum window size so the Equation Canvas opens at a compact, balanced height while preserving full sidebar and three-row palette visibility.

#### Scenario: Default launch opens at 1040x560 with balanced canvas height
- **WHEN** the desktop application opens with its default viewport configuration
- **THEN** the window inner size is `1056x560`, the LaTeX source panel reserves its increased fixed height, and the Equation Canvas fills the remaining central area without excessive empty vertical space

### Requirement: Tinted-accent selection state with contrasting icons
Selected navigation tabs and sidebar category buttons SHALL highlight using a tinted accent background paired with a thin accent border and contrasting accent foreground text and Tabler icons in both Light and Dark modes.

#### Scenario: Selected sidebar category keeps icon contrast in Light and Dark modes
- **WHEN** a sidebar category button is selected in either Light mode or Dark mode
- **THEN** the button displays a tinted accent background and border while its Tabler icon is tinted in the matching high-contrast foreground accent color

#### Scenario: Selected sidebar navigation tab keeps icon contrast
- **WHEN** the `Batch DOCX`, `Settings`, or `About` navigation tab is selected
- **THEN** both its label text and Tabler icon tint in the contrasting selected foreground color

### Requirement: Consistent typographic scale for math button icons
Mathematically rendered icons inside palette buttons SHALL render at a capped base em size rather than upscaling short symbols to fill the bounding box, only scaling down proportionally when an expression exceeds its tile bounds, while sidebar category tiles render uniform Tabler UI icons.

#### Scenario: Short symbols and standard templates share the same base glyph size
- **WHEN** palette icons such as `αΩ`, `∀ ∈`, `□^□`, `max_□`, and `min_□` are rendered
- **THEN** symbols that fit within the `40x26` palette icon budget at the base em size are rasterized at that exact shared em size instead of being stretched to different heights

## ADDED Requirements

### Requirement: Consistent Tabler UI iconography across desktop views
The desktop application SHALL render Tabler UI icons across primary navigation, editor history controls, batch document conversion controls, and About metadata using theme-aware tinting while preserving existing button height tiers (`30px`, `34px`, `24px`) and full-width secondary card layouts.

#### Scenario: Footer Undo and Redo controls display themed Tabler icons
- **WHEN** the editor footer renders in Light or Dark mode
- **THEN** the `Undo` and `Redo` buttons display `arrow-back-up` and `arrow-forward-up` icons tinted to match enabled or disabled theme foreground contrast at the `30px` standard button height

#### Scenario: Sidebar tabs, Back buttons, Batch DOCX actions, and About metadata show assigned icons
- **WHEN** the user navigates between the editor, `Batch DOCX`, `Settings`, and `About` views
- **THEN** `Batch DOCX` (`file-type-docx`), `Settings` (`settings`), and `About` (`info-circle`) sidebar tabs, `Back` buttons (`chevron-left`), `Batch DOCX` actions (`Add DOCX files…` with `file`, `Add folder…` with `folder`, `Convert` with `arrows-left-right`, `Open output folder` with `folder-open`, empty drop-zone with `file-upload`), and `About` metadata rows (`Author` with `user`, `License` with `license`, `GitHub` with `brand-github`) render their assigned Tabler icons legibly in both themes
