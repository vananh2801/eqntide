# desktop-ui-theme Specification

## Purpose

Defines the modern slate visual design system, technical corner-radius and spacing rules, unified hover and tinted-accent selection states, and per-mode Equation Canvas color presets for the desktop equation editor.

## Requirements

### Requirement: Ayu Light and Dark theme palette
The desktop application SHALL style both Light and Dark modes using Zed Editor–inspired Ayu neutral surfaces—bright off-white surfaces in Light mode stepping progressively deeper across window backdrop (`app_bg`), panels (`panel_bg`), cards (`card_bg`), and interactive controls (`control_bg`, `control_hover_bg`, `control_active_bg`, as well as tinted `selected_bg` / `selected_hover_bg`) so cards are darker than `app_bg` and both unselected and selected controls sit darker than surrounding surfaces while deepening across idle, hover, and active states, and deep warm charcoal/obsidian surfaces in Dark mode where interactive controls render brighter than surrounding surfaces (`app_bg`, `panel_bg`, `card_bg`) so `app_bg`, `panel_bg`, `card_bg`, and `control_bg` remain visually distinct without surface/control borders—paired with readable foreground contrast and an Ayu signature amber/orange default accent color across selections (including inline `\command` popup suggestions and `ComboBox` selected rows), keyboard focus rings on modal action buttons and text-editing cursors, sliders, checkboxes, footer radio buttons, hyperlinks, primary actions, and the Batch DOCX progress bar, while keeping Batch folder tree connector lines (`border_hover`), Batch row dividers, and checkbox/radio captions neutral.

#### Scenario: Light and Dark modes apply layered neutral surfaces and legible contrast
- **WHEN** the desktop application renders in either Light mode or Dark mode with the default solid theme
- **THEN** window backgrounds (`app_bg`), panels (`panel_bg`), cards (`card_bg`), Equation Canvas / LaTeX surfaces (`pal.bg`), and interactive controls (`control_bg`, `selected_bg`) use distinct layered Ayu neutral tones with high-contrast text and icons and the Ayu amber/orange accent without requiring 1px surface or control borders, with Light-mode surfaces stepping darker from `app_bg` -> `panel_bg` -> `card_bg` -> `control_bg` / `selected_bg` (deepening on hover/active) and Dark-mode controls brighter than surrounding dark surfaces (brightening on hover/active)

#### Scenario: Inline canvas command popup aligns leading backslash and highlights selected suggestions with active accent
- **WHEN** the user types `\` on the Equation Canvas to open the inline command entry popup and navigates candidate suggestions
- **THEN** the leading `\` prefix is vertically centered with the adjacent command text input, the input box uses a distinct borderless surface fill (`control_bg` or `extreme_bg`), the `\` prefix uses the active theme accent color, and the active suggestion row renders with `selected_bg` and `selected_fg` without an outer popup or selection border

#### Scenario: Batch progress bar and dropdown selections use active accent with crisp contrast
- **WHEN** the Batch DOCX view displays a conversion progress bar (at `0%` or partial progress) or the user opens a `ComboBox` dropdown (`batch-target`, `matrix-alignment`, `matrix-fence`)
- **THEN** the progress bar renders borderless with no partial fill stub at `0%` and fills progress with `primary_bg` using split-contrast `primary_fg` text over the filled region and `text_primary` text over the unfilled track, while selected `ComboBox` rows use `selected_bg` and `selected_fg` without a border stroke

#### Scenario: Checkboxes, footer radios, hover borders, and DragValue focus reflect active accent without tinting excluded frames
- **WHEN** interactive controls render across **Editor**, **Batch DOCX**, and **Settings** in enabled, disabled, hovered, or focused states
- **THEN** checked/indeterminate checkboxes and selected footer radio buttons tint their box/circle indicators with the active accent when enabled and opaque `control_disabled_bg` + `disabled_accent()` when disabled while keeping labels/captions neutral; hovered buttons transition to `control_hover_bg` without a border stroke; focused footer `DragValue` edits indicate active focus without adding idle borders; and Batch folder tree connector lines (`border_hover`) remain neutral

### Requirement: Technical corner radii and harmonized spacing
The desktop application SHALL use two consistent corner-radius tiers (`6px` for interactive controls and inputs, `8px` for canvas, cards, modals, and popups) and a `4px`-based spacing and margin scale while keeping all sidebar controls visible and the palette within three rows at `1056x560`.

#### Scenario: Controls and surfaces follow the two-tier radius scale
- **WHEN** buttons, sliders, text inputs, palette tiles, sidebar items, cards, and the equation canvas are displayed
- **THEN** controls share the `6px` corner radius and surface containers share the `8px` corner radius without ad-hoc radius overrides

#### Scenario: Minimum window layout preserves full sidebar and three-row palette visibility
- **WHEN** the application is rendered at the minimum window size of `1056x560`
- **THEN** harmonized panel padding and control spacing keep all sidebar buttons unclipped without vertical scrolling and keep the palette within three rows

### Requirement: Unified hover, active, and disabled interaction feedback
All interactive controls—including primary and secondary buttons, sidebar category tiles, sidebar navigation tabs, palette items, radio buttons, checkboxes, combo boxes, and canvas preset swatches—SHALL use unified borderless hover and press fill feedback without expanding widget bounds on hover and SHALL display a pointing-hand cursor on hover when enabled.

#### Scenario: Hovering enabled controls applies consistent surface and cursor feedback
- **WHEN** the user moves the pointer over any enabled button, sidebar item, palette button, radio control, checkbox, combo box, or color swatch
- **THEN** the control transitions smoothly in background fill tone without painting a border stroke or growing in size and shows a pointing-hand cursor

#### Scenario: Disabled controls suppress interactive hover styling
- **WHEN** a control is disabled during busy states or unparsed LaTeX source drafts
- **THEN** it renders in a muted non-interactive state and does not show active hover emphasis

### Requirement: Tinted-accent selection state with contrasting icons
Selected navigation tabs and sidebar category buttons SHALL highlight using a borderless tinted accent background paired with contrasting accent foreground text and Tabler icons in both Light and Dark modes.

#### Scenario: Selected sidebar category keeps icon contrast in Light and Dark modes
- **WHEN** a sidebar category button is selected in either Light mode or Dark mode
- **THEN** the button displays a borderless tinted accent background while its Tabler icon is tinted in the matching high-contrast foreground accent color

#### Scenario: Selected sidebar navigation tab keeps icon contrast
- **WHEN** the `Batch DOCX`, `Settings`, or `About` navigation tab is selected
- **THEN** both its label text and Tabler icon tint in the contrasting selected foreground color on a borderless tinted accent background

### Requirement: Fixed Ayu Light and Ayu Dark Equation Canvas surfaces
The desktop application SHALL render the Equation Canvas and synchronized LaTeX source box as borderless surfaces using the fixed `Ayu light` surface (`#fcfcfc` background with dark ink) in Light mode and the fixed `Ayu dark` surface (`#0d1016` background with light ink over a distinct darker-charcoal `app_bg`) in Dark mode—without multi-preset canvas pickers in Settings—adapting placeholder styling, caret, selection highlight, and scrollbars to maintain contrast and reflect the active accent.

#### Scenario: Default Equation Canvas uses a soft light paper surface
- **WHEN** the desktop application renders in Light mode
- **THEN** the Equation Canvas and LaTeX source box render borderless on the `Ayu light` (`#fcfcfc`) surface contrasted against the `app_bg` central backdrop with dark ink and high-contrast caret, selection, and placeholder overlays

#### Scenario: Selecting a Dark mode dark canvas preset adapts ink and overlays
- **WHEN** the desktop application renders in Dark mode
- **THEN** the Equation Canvas and LaTeX source box render borderless on the `Ayu dark` (`#0d1016`) surface contrasted against a distinct `app_bg` central backdrop with light ink and high-contrast caret, selection highlight, and scrollbars

### Requirement: Uniform button typography and height tiers
The desktop application SHALL define explicit UI text styles and standardize interactive text buttons into consistent height tiers (`30px` standard buttons including footer `Save`, `Undo`/`Redo`, `Font size`, and `Zoom` controls; `34px` sidebar navigation tabs; and `24px` compact inline action buttons with non-zero vertical padding) so adjacent controls share matching font size and balanced visual proportions.

#### Scenario: Footer controls and Save button align at standard button height
- **WHEN** the editor footer is rendered in either standalone or Office mode
- **THEN** the `Save`, `Undo`, `Redo`, `Font size`, and `Zoom` controls share the `30px` standard control height and consistent button font sizing

#### Scenario: Compact inline controls avoid collapsed zero-padding text buttons
- **WHEN** contextual matrix controls (`+ row`, `− row`, `+ col`, `− col`) or status bar action buttons (`Retry`, `Next session`) are displayed
- **THEN** they render at the `24px` compact button height with balanced vertical padding and the same UI text family scale

### Requirement: Consistent typographic scale for math button icons
Mathematically rendered icons inside palette buttons SHALL render at a capped base em size rather than upscaling short symbols to fill the bounding box, only scaling down proportionally when an expression exceeds its tile bounds, while sidebar category tiles render uniform Tabler UI icons.

#### Scenario: Short symbols and standard templates share the same base glyph size
- **WHEN** palette icons such as `αΩ`, `∀ ∈`, `□^□`, `max_□`, and `min_□` are rendered
- **THEN** symbols that fit within the `40x26` palette icon budget at the base em size are rasterized at that exact shared em size instead of being stretched to different heights

### Requirement: Bounded vertically scrollable LaTeX source box
The desktop application SHALL render the LaTeX source editor inside a fixed-height container (`88px` panel / `80px` input height below the `8px` top gap) with internal vertical scrolling so multi-line LaTeX input never overflows into the Equation Canvas or clips beneath the footer.

#### Scenario: Entering multiple newlines keeps LaTeX source bounded and scrollable
- **WHEN** the user presses `Enter` multiple times inside the LaTeX source box so the text exceeds the visible rows
- **THEN** the LaTeX source container keeps its fixed height without overlapping the Equation Canvas above or clipping beneath the footer below, and scrolls vertically to keep the active lines accessible

#### Scenario: LaTeX source scrollbar adapts to the active canvas preset
- **WHEN** the user enters multi-line content in the LaTeX source box under any Light or Dark mode canvas preset
- **THEN** the LaTeX source container and its vertical scrollbar use the active canvas preset colors for legible contrast

### Requirement: Compact default window and Equation Canvas sizing
The desktop application SHALL launch with a default window inner size of `1056x560`, matching the minimum window size so the Equation Canvas opens at a compact, balanced height while preserving full sidebar and three-row palette visibility.

#### Scenario: Default launch opens at 1040x560 with balanced canvas height
- **WHEN** the desktop application opens with its default viewport configuration
- **THEN** the window inner size is `1056x560`, the LaTeX source panel reserves its increased fixed height, and the Equation Canvas fills the remaining central area without excessive empty vertical space

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

### Requirement: Consistent selectable theme option buttons in Settings
The **Settings** view SHALL present **Light** and **Dark** theme choices as a selectable option button group styled identically to the **Startup focus** selector below it, rather than a single toggle button.

#### Scenario: Switching theme via Light and Dark option buttons
- **WHEN** the user opens **Settings** while Dark mode is active and clicks the **Light** button
- **THEN** the application switches to Light mode, persists the `Light` theme preference, highlights **Light** as selected, and renders **Dark** unselected using the shared selectable button styling

### Requirement: Consistent Tabler UI iconography across desktop views
The desktop application SHALL render Tabler UI icons across primary navigation, editor history controls, batch document conversion controls, and About metadata using theme-aware tinting while preserving existing button height tiers (`30px`, `34px`, `24px`) and full-width secondary card layouts.

#### Scenario: Footer Undo and Redo controls display themed Tabler icons
- **WHEN** the editor footer renders in Light or Dark mode
- **THEN** the `Undo` and `Redo` buttons display `arrow-back-up` and `arrow-forward-up` icons tinted to match enabled or disabled theme foreground contrast at the `30px` standard button height

#### Scenario: Sidebar tabs, Back buttons, Batch DOCX actions, and About metadata show assigned icons
- **WHEN** the user navigates between the editor, `Batch DOCX`, `Settings`, and `About` views
- **THEN** `Batch DOCX` (`file-type-docx`), `Settings` (`settings`), and `About` (`info-circle`) sidebar tabs, `Back` buttons (`chevron-left`), `Batch DOCX` actions (`Add DOCX files…` with `file`, `Add folder…` with `folder`, `Convert` with `arrows-left-right`, `Open output folder` with `folder-open`, empty drop-zone with `file-upload`), and `About` metadata rows (`Author` with `user`, `License` with `license`, `GitHub` with `brand-github`) render their assigned Tabler icons legibly in both themes

### Requirement: Accent-matched slider handle border
Settings sliders' rounded-square handles SHALL render their borders using the theme's accent color so each handle outline matches the active left-side slider fill across idle, hovered, and active states in both Light and Dark modes.

#### Scenario: Slider handle border matches the left accent fill in Light and Dark modes
- **WHEN** an enabled Settings slider renders in either Light mode or Dark mode across idle, hovered, or dragged states
- **THEN** the rounded-square handle's border stroke uses the same theme accent color as the filled left portion of the slider rail

### Requirement: Custom full-window application background image
When **Image** background mode is active and a valid custom background image is loaded, the desktop application SHALL render that image across the full viewport behind all application views (**Editor**, **Batch DOCX**, **Settings**, and **About**) using aspect-preserving cover scaling, layered with a theme-aware dimming scrim (`0%`..`65%`) and translucent-to-opaque chrome/card surfaces controlled by **Overlay opacity** (`0%`..`100%`, where `panel_bg` and `card_bg` become fully opaque at `100%` while `app_bg` remains transparent over the scrim so the background image stays visible in open areas), while rendering the **Equation Canvas** and **LaTeX source** box with the active mode's Ayu `CanvasPalette` whose background surface alpha scales across `0.38..=1.00` according to **Canvas opacity** (`0%`..`100%`, defaulting to `100%` opaque).

#### Scenario: Custom background displays across Editor and secondary views
- **WHEN** the user configures a valid background image in **Image** mode and navigates between the **Editor**, **Batch DOCX**, **Settings**, and **About** views
- **THEN** the background image fills the window behind the application shell with aspect-preserving cover scaling and remains visible across open central backdrop areas, while panels and cards scale from translucent to opaque (`255` alpha at `100%`) according to **Overlay opacity**

#### Scenario: Equation Canvas and LaTeX source preserve crisp contrast over a custom background
- **WHEN** a custom background image is active in either Light mode or Dark mode at the default `100%` **Canvas opacity** (or across `0%`..`100%` with the `0.38` surface floor)
- **THEN** the **Equation Canvas** and **LaTeX source** box preserve crisp legibility with the active `Ayu light` or `Ayu dark` surface, equation ink, caret, selection, and placeholder colors

#### Scenario: Equation Canvas and LaTeX source scale translucency with Canvas opacity over a custom background
- **WHEN** a custom background image is active in either Light mode or Dark mode and **Canvas opacity** is set to `100%`, an intermediate value, or `0%`
- **THEN** the **Equation Canvas** and **LaTeX source** box share the active `Ayu light` or `Ayu dark` `CanvasPalette` ink, border, caret, selection, and placeholder colors while their background fill (`pal.bg`) scales linearly in alpha from `0.38` (`0%`) to `1.00` (`100%`, fully opaque by default)

#### Scenario: Clearing or missing background falls back to default slate theme
- **WHEN** **Solid** mode is selected, no background image is configured, the user clicks **Clear**, or the stored background file cannot be loaded
- **THEN** the desktop application renders the opaque Ayu Light or Dark theme surfaces (including opaque `255`-alpha **Equation Canvas** and **LaTeX source** fills) and default Ayu accent without visual errors

### Requirement: App background customization controls in Settings
The **Settings** view SHALL place **Appearance** (**Light** / **Dark** theme selector) and **App background** in separate full-width cards, where the **App background** card contains a **Solid** / **Image** selectable mode toggle styled identically to **Light** / **Dark**, a vertically centered row with the `72×48` rounded thumbnail and `Choose image…` / `Clear` buttons (without filename or mode labels beside the thumbnail), a **Match accent color to background** checkbox (enabled when **Image** mode has an active background, disabled when **Solid**), a `1%`-stepped **Overlay opacity** slider row (`0%`..`100%`), and directly below it a `1%`-stepped **Canvas opacity** slider row (`0%`..`100%`, defaulting to `100%`), both sliders disabled in **Solid** mode and rendering fully opaque slider handles (`alpha == 255`) in both enabled and disabled states using standard `30px` controls and `6px`/`8px` theme radii without displaying multi-option Equation Canvas preset pickers.

#### Scenario: Multi-row App background layout and vertical canvas preset ordering
- **WHEN** the user views **Settings**
- **THEN** **Appearance** (**Light** / **Dark**) and **App background** (**Solid** / **Image**, vertically centered `72×48` thumbnail + `Choose image…` + `Clear`, **Match accent color to background**, **Overlay opacity**, and **Canvas opacity** below **Overlay opacity**) render as separate full-width cards with no filename/mode text next to the thumbnail and no canvas preset picker rows below

#### Scenario: Toggling Solid and Image switches background without deleting the saved image
- **WHEN** a custom background image is loaded and the user clicks **Solid** and then clicks **Image**
- **THEN** clicking **Solid** switches immediately to the opaque Ayu solid theme (with opaque Equation Canvas and LaTeX source fills) while keeping the thumbnail and saved image reference intact and keeping both the disabled **Overlay opacity** and **Canvas opacity** slider handles fully opaque (`alpha == 255`) with muted accent styling, and clicking **Image** restores the wallpaper and `Canvas opacity` translucency without re-importing

#### Scenario: Selecting an image updates the live background immediately
- **WHEN** the user clicks **Choose image…** in **Settings** and selects a valid `PNG`, `JPEG`, or `WebP` file
- **THEN** the application displays `Importing background image…` in the bottom status bar without freezing the UI thread, updates the thumbnail preview, switches to **Image** mode, applies the image and derived accent across the window immediately, and enables the **Clear**, **Match accent color to background**, **Overlay opacity**, and **Canvas opacity** controls

#### Scenario: Selecting an invalid or unreadable image file preserves previous state and reports status
- **WHEN** the user selects a corrupt or unsupported image file from the file dialog
- **THEN** the application does not crash or discard any previously working background and communicates the failure in the Settings status area or status bar

#### Scenario: Adjusting overlay opacity updates surface translucency in real time
- **WHEN** the user adjusts the **Overlay opacity** slider while **Image** mode has an active custom background
- **THEN** the theme-tinted scrim (`0%`..`65%`) and panel/card surface alpha update immediately in `1%` increments—leaving the scrim transparent at `0%` while preserving a readable surface floor, balancing visibility and contrast at intermediate values, and making `panel_bg` and `card_bg` completely opaque (`255` alpha) at `100%` while keeping `app_bg` transparent over the `65%` scrim so open background areas and the thumbnail still show the image underneath

#### Scenario: Adjusting canvas opacity updates Equation Canvas and LaTeX box translucency in real time
- **WHEN** the user adjusts the **Canvas opacity** slider while **Image** mode has an active custom background
- **THEN** the **Equation Canvas** and **LaTeX source** box fill alphas update immediately in `1%` increments across `0.38..=1.00` without changing `panel_bg`, `card_bg`, or the window scrim, leaving both boxes fully opaque (`255` alpha) at `100%` and revealing the wallpaper/scrim cleanly at lower percentages

### Requirement: Transparent dashed placeholders and lightly rounded canvas overlays
Within the Equation Canvas, empty structural placeholder boxes (`□`) SHALL render with a dashed border using the active `CanvasPalette` placeholder stroke color, light `2px` corner rounding, and `0` inner fill opacity (`Color32::TRANSPARENT` over the canvas background) without painting an opaque fill patch or leaving baked synthetic square ink visible when the canvas is translucent, and selection highlights SHALL also use `2px` corner rounding.

#### Scenario: Empty structural placeholder displays an unfilled dashed box with light corner rounding
- **WHEN** an empty structural slot (`□`) is rendered on the Equation Canvas under any Light or Dark mode `CanvasPreset` or any `Canvas opacity` setting (`0%`..`100%`)
- **THEN** the slot displays a dashed outline with `2px` corner rounding, no tinted background fill inside the box (`placeholder_bg` alpha is `0`), and no solid `□` glyph or extra `pal.bg` cover rectangle inside the dashed border, revealing the active opaque or translucent canvas background cleanly

### Requirement: Adaptive Light and Dark accent derivation from imported background images
When a background image is imported, the desktop application SHALL extract a dominant chromatic accent seed from the image and derive separate Light-mode (deeper) and Dark-mode (brighter) accent tokens with accessible contrast across UI controls, popup chrome, and canvas highlights whenever **Image** mode and **Match accent color to background** are active.

#### Scenario: Imported background yields distinct Light and Dark mode accent tones
- **WHEN** a colorful background image is active with **Match accent color to background** enabled
- **THEN** Light mode applies a deeper accent tone for legible text and primary/selected surfaces, while Dark mode applies a brighter accent tone from the same hue, propagating across sliders, checkboxes, footer radio indicators, Batch progress bar, `ComboBox` and inline `\command` popup selections (`\` prefix + active suggestion row), and canvas highlights while preserving readable foreground contrast

#### Scenario: Disabling adaptive accent or switching to Solid restores default Ayu accent
- **WHEN** the user unchecks **Match accent color to background** or switches **App background** to **Solid**
- **THEN** UI controls, global widget styles, checkboxes, radio indicators, Batch progress bar, `ComboBox` and inline `\command` popup chrome, and canvas accent highlights immediately revert to the default Ayu amber/orange accent while preserving the stored accent seed for when adaptive **Image** mode is re-enabled

### Requirement: Modal workspace dimming and visible keyboard focus rings
When a desktop modal dialog or blocking operation disables the workspace, the **Equation Canvas** SHALL dim uniformly alongside the sidebar, palette, LaTeX source box, and footer—preserving full `1.0` canvas opacity only when editing is disabled solely by an unparsed `draft_dirty` LaTeX draft—and keyboard-focused buttons in both primary and secondary styles SHALL display a `1.0px` high-contrast focus border matching secondary button stroke thickness and distinct from their background fill.

#### Scenario: Equation Canvas dims alongside surrounding panels behind modals
- **WHEN** **Finish current equation** or **Custom matrix** is open over the editor view
- **THEN** the **Equation Canvas** does not override outer disabled dimming to `1.0` opacity and appears dimmed consistently with the palette, LaTeX source box, sidebar, and footer

#### Scenario: Equation Canvas remains full opacity during draft_dirty LaTeX typing
- **WHEN** no modal or `busy` operation is active and the user enters an incomplete LaTeX draft (`draft_dirty == true`)
- **THEN** the **Equation Canvas** keeps `1.0` opacity so the last valid equation preview remains crisp and legible

#### Scenario: Keyboard-focused primary and standard buttons show a distinct 1px focus border
- **WHEN** a primary button (`Save`, `Insert matrix`) or standard button (`Discard / cancel session`, `Continue editing`, `Cancel`) holds keyboard focus
- **THEN** it renders a clearly visible `1.0px` contrast focus stroke (`tokens.text_primary` in Dark mode / `tokens.primary_fg` in Light mode on primary buttons; `tokens.accent` on standard buttons) distinct from its background fill in both Light and Dark modes without increasing button height or border thickness relative to neighboring buttons

### Requirement: Borderless chrome with preserved region dividers, tree lines, and indicator strokes
The desktop application SHALL omit card, canvas, LaTeX box, modal, popup, thumbnail, progress-bar, status-bar outer box, and idle/hovered/selected control borders, relying on layered Ayu neutral fills while preserving structural region dividers, Batch DOCX tree lines, Equation Canvas dashed placeholders, and functional widget indicators.

#### Scenario: Editor and secondary tabs render region divider lines without card, canvas, or button borders
- **WHEN** the user views the **Editor**, **Batch DOCX**, **Settings**, or **About** tab in either Light mode or Dark mode (Solid or Image background)
- **THEN** the sidebar vertical divider, footer horizontal divider, status-bar top horizontal divider, and section separators render cleanly over opaque fills while no status-bar outer box, card border, Equation Canvas / LaTeX box outer border, progress bar outer border, or idle/hovered/selected button/dropdown border is drawn, and distinct layered surface and control fills keep every region and control clearly legible

#### Scenario: Batch DOCX Folder tree and File tree preserve internal hierarchy and row lines
- **WHEN** the user views queued items in **Batch DOCX** under either **Folder tree** or **File tree** mode
- **THEN** the outer list card renders without a border while the internal horizontal row dividers and vertical/horizontal folder hierarchy connector lines continue to render normally

#### Scenario: Hovering and selecting controls transitions fill colors without borders
- **WHEN** the user hovers over enabled controls or selects sidebar categories, sidebar tabs, Settings toggles, or `ComboBox` rows
- **THEN** background and foreground tones update cleanly without painting hover or selection border strokes

#### Scenario: Checkbox, radio, slider, placeholder, and bridge indicators stay visible
- **WHEN** checkboxes, radio buttons, sliders, empty canvas placeholders (`□`), or the disconnected status-bar bridge icon render
- **THEN** checkbox squares, radio circles, slider thumb borders, dashed canvas placeholder outlines, and the hollow disconnected bridge circle remain clearly visible

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

### Requirement: Fixed desktop scrollbars for overflowing content
Desktop scroll areas except the editor palette SHALL show a fixed-thickness scrollbar for each overflowing axis. Each visible scrollbar SHALL remain legible in Light and Dark modes and occupy dedicated space outside the viewport while idle, hovered, or dragged. When an axis fits, its scrollbar SHALL be hidden and its space returned to the viewport. The editor palette SHALL omit scrollbars and reserve a constant height for three rows.

#### Scenario: Overflowing content shows a scrollbar while the pointer is elsewhere
- **WHEN** scrollable content outside the editor's palette overflows and the pointer is outside the scroll area
- **THEN** the scrollbar is visible and its handle is distinguishable from the track in Light and Dark modes

#### Scenario: Hover and drag preserve scrollbar and viewport dimensions
- **WHEN** the user hovers or drags a scrollbar while the content and available space remain the same
- **THEN** the scrollbar retains its thickness and position outside the content viewport, and the viewport retains its dimensions

#### Scenario: Content that fits uses the full viewport
- **WHEN** content fits initially or fits after editing, filtering, collapsing rows, or enlarging the window
- **THEN** the corresponding scrollbar is absent and the content viewport uses the available space on that edge

#### Scenario: Equation Canvas shows scrollbars on the axes with overflow
- **WHEN** the equation exceeds the available viewport on one or both supported axes
- **THEN** each axis that needs scrolling has its own visible scrollbar beside or below the viewport, outside the equation content

#### Scenario: Horizontal palette scrollbar leaves all tile rows accessible
- **WHEN** the active palette category exceeds the available width in its three-row layout
- **THEN** the palette displays no scrollbar, all three rows remain fully visible vertically, and the palette occupies the same height as a category that fits
