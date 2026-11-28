# Spec Delta: desktop-ui-theme

## ADDED Requirements

### Requirement: Consistent selectable target option buttons in editor footer
The editor footer SHALL render the **MathType** and **OMML** output target selector as a borderless `30px`-tall, `6px`-rounded selectable option button group styled identically to the option selectors in **Settings**, spaced `8px` apart horizontally and vertically centered on the footer row alongside **Font size**, **Zoom**, **Undo**, **Redo**, **Save**, and section separators.

#### Scenario: Footer MathType and OMML buttons match Settings selectable button styling and footer alignment
- **WHEN** the editor footer renders in Light mode or Dark mode (enabled or disabled during `draft_dirty` or modals)
- **THEN** **MathType** and **OMML** render without circular radio indicators as `30px`-tall, `6px`-rounded borderless buttons (`selected_bg`/`selected_fg` for the active target and `control_bg`/`text_primary` for the inactive target), separated by `8px` horizontal spacing and sharing the vertical centerline of **Font size**, **Zoom**, **Undo**, **Redo**, and **Save**

#### Scenario: Clicking MathType or OMML in the footer switches the active target
- **WHEN** the user clicks the unselected **MathType** or **OMML** button in the enabled editor footer
- **THEN** the clicked button adopts the active selected styling, the other target reverts to neutral control styling, the output target updates and persists immediately, and keyboard focus restores to the active editor surface

## MODIFIED Requirements

### Requirement: Ayu Light and Dark theme palette
The desktop application SHALL style both Light and Dark modes using Zed Editor–inspired Ayu neutral surfaces—bright off-white surfaces in Light mode stepping progressively deeper across window backdrop (`app_bg`), panels (`panel_bg`), cards (`card_bg`), and interactive controls (`control_bg`, `control_hover_bg`, `control_active_bg`, as well as tinted `selected_bg` / `selected_hover_bg`) so cards are darker than `app_bg` and both unselected and selected controls sit darker than surrounding surfaces while deepening across idle, hover, and active states, and deep warm charcoal/obsidian surfaces in Dark mode where interactive controls render brighter than surrounding surfaces (`app_bg`, `panel_bg`, `card_bg`) so `app_bg`, `panel_bg`, `card_bg`, and `control_bg` remain visually distinct without surface/control borders—paired with readable foreground contrast and an Ayu signature amber/orange default accent color across selections (including inline `\command` popup suggestions, `ComboBox` selected rows, and footer target option buttons), keyboard focus rings on modal action buttons and text-editing cursors, level-fill opacity controls, checkboxes, hyperlinks, primary actions, and the Batch DOCX progress bar, while keeping Batch folder tree connector lines (`border_hover`), Batch row dividers, and checkbox captions neutral.

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
- **THEN** checked/indeterminate checkboxes tint their box indicators with the active accent when enabled and opaque `control_disabled_bg` + `disabled_accent()` when disabled while keeping labels neutral; selected footer target buttons (`MathType` / `OMML`) use `selected_bg` and `selected_fg`; hovered buttons transition to `control_hover_bg` without a border stroke; focused `DragValue` edits indicate active focus without adding idle borders; and Batch folder tree connector lines (`border_hover`) remain neutral

### Requirement: Unified hover, active, and disabled interaction feedback
All interactive controls—including primary and secondary buttons, sidebar category tiles, sidebar navigation tabs, palette items, footer target option buttons, checkboxes, level-fill controls, and combo boxes—SHALL use unified borderless hover and press fill feedback without expanding widget bounds on hover and SHALL display a pointing-hand cursor on hover when enabled.

#### Scenario: Hovering enabled controls applies consistent surface and cursor feedback
- **WHEN** the user moves the pointer over any enabled button, sidebar item, palette button, footer target button, checkbox, level-fill box, or combo box
- **THEN** the control transitions smoothly in background fill tone without painting a border stroke or growing in size and shows a pointing-hand cursor

#### Scenario: Disabled controls suppress interactive hover styling
- **WHEN** a control is disabled during busy states or unparsed LaTeX source drafts
- **THEN** it renders in a muted non-interactive state and does not show active hover emphasis

### Requirement: Uniform button typography and height tiers
The desktop application SHALL define explicit UI text styles and standardize interactive text buttons into consistent height tiers (`30px` standard buttons including footer `MathType`/`OMML`, `Save`, `Undo`/`Redo`, `Font size`, `Zoom`, and Settings opacity controls; `34px` sidebar navigation tabs; and `24px` compact inline action buttons with non-zero vertical padding) so adjacent controls share matching font size and balanced visual proportions.

#### Scenario: Footer controls and Save button align at standard button height
- **WHEN** the editor footer is rendered in either standalone or Office mode
- **THEN** the `MathType`, `OMML`, `Save`, `Undo`, `Redo`, `Font size`, and `Zoom` controls share the `30px` standard control height and consistent button font sizing

#### Scenario: Compact inline controls avoid collapsed zero-padding text buttons
- **WHEN** contextual matrix controls (`+ row`, `− row`, `+ col`, `− col`) or status bar action buttons (`Retry`, `Next session`) are displayed
- **THEN** they render at the `24px` compact button height with balanced vertical padding and the same UI text family scale

### Requirement: Custom full-window application background image
When **Image** background mode is active and a valid custom background image is loaded, the desktop application SHALL render that image across the full viewport behind all application views (**Editor**, **Batch DOCX**, **Settings**, and **About**) using aspect-preserving cover scaling, layered with a theme-aware dimming scrim (`0%`..`65%`) and translucent-to-opaque chrome/card surfaces controlled by **Overlay opacity** (`38%..=100%`, where `panel_bg`—including the sidebar, top palette panel, footer, and status bar—and `card_bg` use the literal percentage alpha directly and become fully opaque at `100%` while `app_bg` remains transparent over the scrim so the background image stays visible in open areas), while rendering the **Equation Canvas** and **LaTeX source** box with the active mode's Ayu `CanvasPalette` whose background surface alpha directly equals **Canvas opacity** across `38%..=100%` (defaulting to `100%` opaque).

#### Scenario: Custom background displays across Editor and secondary views
- **WHEN** the user configures a valid background image in **Image** mode and navigates between the **Editor**, **Batch DOCX**, **Settings**, and **About** views
- **THEN** the background image fills the window behind the application shell with aspect-preserving cover scaling and remains visible across open central backdrop areas, while chrome panels (sidebar, top palette panel, footer, and status bar) and cards use the literal **Overlay opacity** alpha (`38%..=100%`, `255` alpha at `100%`)

#### Scenario: Equation Canvas and LaTeX source preserve crisp contrast over a custom background
- **WHEN** a custom background image is active in either Light mode or Dark mode at the default `100%` **Canvas opacity** (or across `38%..=100%`)
- **THEN** the **Equation Canvas** and **LaTeX source** box preserve crisp legibility with the active `Ayu light` or `Ayu dark` surface, equation ink, caret, selection, and placeholder colors

#### Scenario: Equation Canvas and LaTeX source scale translucency with Canvas opacity over a custom background
- **WHEN** a custom background image is active in either Light mode or Dark mode and **Canvas opacity** is set to `100%`, an intermediate percentage, or `38%`
- **THEN** the **Equation Canvas** and **LaTeX source** box share the active `Ayu light` or `Ayu dark` `CanvasPalette` ink, border, caret, selection, and placeholder colors while their background fill (`pal.bg`) alpha equals the literal **Canvas opacity** percentage (`0.38` at `38%` to `1.00` at `100%`, fully opaque by default)

#### Scenario: Clearing or missing background falls back to default slate theme
- **WHEN** **Solid** mode is selected, no background image is configured, the user clicks **Clear**, or the stored background file cannot be loaded
- **THEN** the desktop application renders the opaque Ayu Light or Dark theme surfaces (including opaque `255`-alpha chrome panels and **Equation Canvas** / **LaTeX source** fills) and default Ayu accent without visual errors

### Requirement: App background customization controls in Settings
The **Settings** view SHALL place **Appearance** (**Light** / **Dark** theme selector) and **App background** in separate full-width cards, where the **App background** card contains a **Solid** / **Image** selectable mode toggle styled identically to **Light** / **Dark**, a vertically centered row with the `72×48` rounded thumbnail and `Choose image…` / `Clear` buttons (without filename or mode labels beside the thumbnail), a **Match accent color to background** checkbox (enabled when **Image** mode has an active background, disabled when **Solid**), a `1%`-stepped **Overlay opacity** level-fill `DragValue` row (`38%..=100%`, `220px` wide × `30px` high), and directly below it a `1%`-stepped **Canvas opacity** level-fill `DragValue` row (`38%..=100%`, `220px` wide × `30px` high, defaulting to `100%`), both level-fill controls displaying their percentage centered inside the `6px`-rounded box (without a slider handle or external right-hand `%` label), painting proportional soft-accent left fills `(value - 38%) / (100% - 38%)`, supporting horizontal drag and direct numeric entry while ignoring mouse-wheel scroll events so vertical Settings scrolling never conflicts with adjusting opacity, and disabled in **Solid** mode while preserving muted disabled track and fill contrast without displaying multi-option Equation Canvas preset pickers.

#### Scenario: Multi-row App background layout and vertical canvas preset ordering
- **WHEN** the user views **Settings**
- **THEN** **Appearance** (**Light** / **Dark**) and **App background** (**Solid** / **Image**, vertically centered `72×48` thumbnail + `Choose image…` + `Clear`, **Match accent color to background**, `220px` **Overlay opacity** level-fill box, and `220px` **Canvas opacity** level-fill box below **Overlay opacity**) render as separate full-width cards with no filename/mode text next to the thumbnail, no external `%` text to the right of the opacity bars, and no canvas preset picker rows below

#### Scenario: Toggling Solid and Image switches background without deleting the saved image
- **WHEN** a custom background image is loaded and the user clicks **Solid** and then clicks **Image**
- **THEN** clicking **Solid** switches immediately to the opaque Ayu solid theme (with opaque Equation Canvas and LaTeX source fills) while keeping the thumbnail and saved image reference intact and rendering both disabled **Overlay opacity** and **Canvas opacity** level-fill boxes in muted disabled track/fill colors (`control_disabled_bg` and `soft_accent_fill(false, false)`) without interactive drag or click changes, and clicking **Image** restores the wallpaper and `Canvas opacity` translucency without re-importing

#### Scenario: Selecting an image updates the live background immediately
- **WHEN** the user clicks **Choose image…** in **Settings** and selects a valid `PNG`, `JPEG`, or `WebP` file
- **THEN** the application displays `Importing background image…` in the bottom status bar without freezing the UI thread, updates the thumbnail preview, switches to **Image** mode, applies the image and derived accent across the window immediately, and enables the **Clear**, **Match accent color to background**, **Overlay opacity**, and **Canvas opacity** controls

#### Scenario: Selecting an invalid or unreadable image file preserves previous state and reports status
- **WHEN** the user selects a corrupt or unsupported image file from the file dialog
- **THEN** the application does not crash or discard any previously working background and communicates the failure in the Settings status area or status bar

#### Scenario: Adjusting overlay opacity updates surface translucency in real time
- **WHEN** the user adjusts the **Overlay opacity** level-fill box (`38%..=100%` via horizontal drag or numeric entry) while **Image** mode has an active custom background, or scrolls the mouse wheel while hovering over the bar
- **THEN** adjusting via drag or numeric entry displays the exact percentage (`38%..=100%`) centered inside the `220×30px` bar, `panel_bg` and `card_bg` alphas equal that exact percentage (`0.38` at `38%` to `1.00` / `255` alpha at `100%`), and the theme-tinted `background_scrim` scales from `0%` (at `38%`) to `65%` (at `100%`) without showing scrim percentages on the bar while keeping `app_bg` transparent so open background areas and the thumbnail still show the image underneath, whereas mouse-wheel scrolling over the bar does not alter **Overlay opacity**

#### Scenario: Adjusting canvas opacity updates Equation Canvas and LaTeX box translucency in real time
- **WHEN** the user adjusts the **Canvas opacity** level-fill box (`38%..=100%` via horizontal drag or numeric entry) while **Image** mode has an active custom background, or scrolls the mouse wheel while hovering over the bar
- **THEN** adjusting via drag or numeric entry displays the exact percentage (`38%..=100%`) centered inside the `220×30px` bar and the **Equation Canvas** and **LaTeX source** box fill alphas equal that exact percentage (`0.38` at `38%` to `1.00` / `255` alpha at `100%`) without changing `panel_bg`, `card_bg`, or the window scrim, whereas mouse-wheel scrolling over the bar does not alter **Canvas opacity**

### Requirement: Transparent dashed placeholders and lightly rounded canvas overlays
Within the Equation Canvas, empty structural placeholder boxes (`□`) SHALL render with a dashed border using the active `CanvasPalette` placeholder stroke color, light `2px` corner rounding, and `0` inner fill opacity (`Color32::TRANSPARENT` over the canvas background) without painting an opaque fill patch or leaving baked synthetic square ink visible when the canvas is translucent, and selection highlights SHALL also use `2px` corner rounding.

#### Scenario: Empty structural placeholder displays an unfilled dashed box with light corner rounding
- **WHEN** an empty structural slot (`□`) is rendered on the Equation Canvas under any Light or Dark mode `CanvasPreset` or any `Canvas opacity` setting (`38%..=100%`)
- **THEN** the slot displays a dashed outline with `2px` corner rounding, no tinted background fill inside the box (`placeholder_bg` alpha is `0`), and no solid `□` glyph or extra `pal.bg` cover rectangle inside the dashed border, revealing the active opaque or translucent canvas background cleanly

### Requirement: Adaptive Light and Dark accent derivation from imported background images
When a background image is imported, the desktop application SHALL extract a dominant chromatic accent seed from the image and derive separate Light-mode (deeper) and Dark-mode (brighter) accent tokens with accessible contrast across UI controls, popup chrome, and canvas highlights whenever **Image** mode and **Match accent color to background** are active.

#### Scenario: Imported background yields distinct Light and Dark mode accent tones
- **WHEN** a colorful background image is active with **Match accent color to background** enabled
- **THEN** Light mode applies a deeper accent tone for legible text and primary/selected surfaces, while Dark mode applies a brighter accent tone from the same hue, propagating across opacity level-fill bars, checkboxes, footer target selection buttons, Batch progress bar, `ComboBox` and inline `\command` popup selections (`\` prefix + active suggestion row), and canvas highlights while preserving readable foreground contrast

#### Scenario: Disabling adaptive accent or switching to Solid restores default Ayu accent
- **WHEN** the user unchecks **Match accent color to background** or switches **App background** to **Solid**
- **THEN** UI controls, global widget styles, opacity level-fill bars, checkboxes, footer target selection buttons, Batch progress bar, `ComboBox` and inline `\command` popup chrome, and canvas accent highlights immediately revert to the default Ayu amber/orange accent while preserving the stored accent seed for when adaptive **Image** mode is re-enabled

### Requirement: Borderless chrome with preserved region dividers, tree lines, and indicator strokes
The desktop application SHALL omit card, canvas, LaTeX box, modal, popup, thumbnail, progress-bar, status-bar outer box, and idle/hovered/selected control borders, relying on layered Ayu neutral fills while preserving structural region dividers, Batch DOCX tree lines, Equation Canvas dashed placeholders, and functional widget indicators.

#### Scenario: Editor and secondary tabs render region divider lines without card, canvas, or button borders
- **WHEN** the user views the **Editor**, **Batch DOCX**, **Settings**, or **About** tab in either Light mode or Dark mode (Solid or Image background)
- **THEN** the sidebar vertical divider, top palette panel bottom horizontal divider, footer horizontal divider, status-bar top horizontal divider, and section separators render cleanly over panel fills while no status-bar outer box, card border, Equation Canvas / LaTeX box outer border, progress bar outer border, or idle/hovered/selected button/dropdown border is drawn, and distinct layered surface and control fills keep every region and control clearly legible

#### Scenario: Batch DOCX Folder tree and File tree preserve internal hierarchy and row lines
- **WHEN** the user views queued items in **Batch DOCX** under either **Folder tree** or **File tree** mode
- **THEN** the outer list card renders without a border while the internal horizontal row dividers and vertical/horizontal folder hierarchy connector lines continue to render normally

#### Scenario: Hovering and selecting controls transitions fill colors without borders
- **WHEN** the user hovers over enabled controls or selects sidebar categories, sidebar tabs, footer target buttons, Settings toggles, or `ComboBox` rows
- **THEN** background and foreground tones update cleanly without painting hover or selection border strokes

#### Scenario: Checkbox, radio, slider, placeholder, and bridge indicators stay visible
- **WHEN** checkboxes, empty canvas placeholders (`□`), or the disconnected status-bar bridge icon render
- **THEN** checkbox squares, dashed canvas placeholder outlines, and the hollow disconnected bridge circle remain clearly visible

## REMOVED Requirements

### Requirement: Accent-matched slider handle border
**Reason**: **Settings → App background** opacity controls no longer use `egui::Slider` with rounded-square handles; they now use the handleless level-fill `DragValue` style shared with the footer **Font size** and **Zoom** boxes.
**Migration**: **Overlay opacity** and **Canvas opacity** render proportional soft-accent left fills inside `220px × 30px` `6px`-rounded level-fill boxes without slider handles (showing a `1px` `tokens.accent` border only when focused for text entry).
