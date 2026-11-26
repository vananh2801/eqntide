# Spec Delta

## RENAMED Requirements

- FROM: `### Requirement: Modern slate Light and Dark theme palette`
  TO: `### Requirement: Ayu Light and Dark theme palette`
- FROM: `### Requirement: Curated per-mode Equation Canvas presets`
  TO: `### Requirement: Fixed Ayu Light and Ayu Dark Equation Canvas surfaces`

## MODIFIED Requirements

### Requirement: Ayu Light and Dark theme palette
The desktop application SHALL style both Light and Dark modes using Zed Editor–inspired Ayu neutral surfaces (soft neutral off-white/gray surfaces in Light mode and deep warm charcoal/obsidian surfaces in Dark mode), readable foreground contrast, and an Ayu signature amber/orange default accent color across selections (including inline `\command` popup suggestions and `ComboBox` selected rows), focus rings/borders (including focused `DragValue` text inputs and the inline `\command` popup border with its vertically aligned leading `\` prefix), subtle accent-tinted control hover borders (`control_hover_border`), sliders, checkboxes, footer radio buttons, text cursors, hyperlinks, primary actions, and the Batch DOCX progress bar, while keeping Equation Canvas / LaTeX source box borders (`pal.border`), Batch folder tree connector lines (`border_hover`), and checkbox/radio captions neutral.

#### Scenario: Light and Dark modes apply layered neutral surfaces and legible contrast
- **WHEN** the desktop application renders in either Light mode or Dark mode with the default solid theme
- **THEN** window backgrounds, panels, cards, and interactive controls use distinct layered Ayu neutral tones with high-contrast text and icons and the Ayu amber/orange accent

#### Scenario: Inline canvas command popup aligns leading backslash and highlights selected suggestions with active accent
- **WHEN** the user types `\` on the Equation Canvas to open the inline command entry popup and navigates candidate suggestions
- **THEN** the leading `\` prefix is vertically centered with the adjacent command text input, both the `\` prefix and focused popup border use the active theme accent color, and the active suggestion row renders with `selected_bg`, `selected_border`, and `selected_fg`

#### Scenario: Batch progress bar and dropdown selections use active accent with crisp contrast
- **WHEN** the Batch DOCX view displays a conversion progress bar (at `0%` or partial progress) or the user opens a `ComboBox` dropdown (`batch-target`, `matrix-alignment`, `matrix-fence`)
- **THEN** the progress bar renders no partial fill stub at `0%` and fills progress with `primary_bg` using split-contrast `primary_fg` text over the filled region and `text_primary` text over the unfilled track, while selected `ComboBox` rows use `selected_bg`, `selected_border`, and `selected_fg`

#### Scenario: Checkboxes, footer radios, hover borders, and DragValue focus reflect active accent without tinting excluded frames
- **WHEN** interactive controls render across **Editor**, **Batch DOCX**, and **Settings** in enabled, disabled, hovered, or focused states
- **THEN** checked/indeterminate checkboxes and selected footer radio buttons tint their box/circle indicators with the active accent when enabled and opaque `control_disabled_bg` + `disabled_accent()` when disabled while keeping labels/captions neutral; hovered buttons receive `control_hover_border`; focused footer `DragValue` edits use `accent`; and Equation Canvas / LaTeX box borders (`pal.border`) as well as Batch folder tree connector lines (`border_hover`) remain neutral

### Requirement: Fixed Ayu Light and Ayu Dark Equation Canvas surfaces
The desktop application SHALL render the Equation Canvas and synchronized LaTeX source box using the fixed `Ayu light` surface (`#fcfcfc` background with dark ink) in Light mode and the fixed `Ayu dark` surface (`#0d1016` background with light ink) in Dark mode—without multi-preset canvas pickers in Settings—adapting placeholder styling, caret, selection highlight, and scrollbars to maintain contrast and reflect the active accent while keeping their outer frame borders neutral (`pal.border`).

#### Scenario: Default Equation Canvas uses a soft light paper surface
- **WHEN** the desktop application renders in Light mode
- **THEN** the Equation Canvas and LaTeX source box render on the `Ayu light` (`#fcfcfc`) surface with neutral `#cfd1d2` borders, dark ink, and high-contrast caret, selection, and placeholder overlays

#### Scenario: Selecting a Dark mode dark canvas preset adapts ink and overlays
- **WHEN** the desktop application renders in Dark mode
- **THEN** the Equation Canvas and LaTeX source box render on the `Ayu dark` (`#0d1016`) surface with neutral `#3f4043` borders, light ink, and high-contrast caret, selection highlight, and scrollbars

### Requirement: Custom full-window application background image
When **Image** background mode is active and a valid custom background image is loaded, the desktop application SHALL render that image across the full viewport behind all application views (**Editor**, **Batch DOCX**, **Settings**, and **About**) using aspect-preserving cover scaling, layered with a theme-aware dimming scrim (`0%`..`65%`) and translucent-to-opaque chrome/card surfaces controlled by **Overlay opacity** (`0%`..`100%`, where `panel_bg` and `card_bg` become fully opaque at `100%` while `app_bg` remains transparent over the scrim so the background image stays visible in open areas) while keeping the **Equation Canvas** and **LaTeX source** box opaque using the active mode's Ayu `CanvasPalette`.

#### Scenario: Custom background displays across Editor and secondary views
- **WHEN** the user configures a valid background image in **Image** mode and navigates between the **Editor**, **Batch DOCX**, **Settings**, and **About** views
- **THEN** the background image fills the window behind the application shell with aspect-preserving cover scaling and remains visible across open central backdrop areas, while panels and cards scale from translucent to opaque (`255` alpha at `100%`) according to **Overlay opacity**

#### Scenario: Equation Canvas and LaTeX source preserve crisp contrast over a custom background
- **WHEN** a custom background image is active in either Light mode or Dark mode
- **THEN** the **Equation Canvas** and **LaTeX source** box continue rendering with their opaque `Ayu light` or `Ayu dark` surface, equation ink, caret, selection, and placeholder colors at full legibility

#### Scenario: Clearing or missing background falls back to default slate theme
- **WHEN** **Solid** mode is selected, no background image is configured, the user clicks **Clear**, or the stored background file cannot be loaded
- **THEN** the desktop application renders the opaque Ayu Light or Dark theme surfaces and default Ayu accent without visual errors

### Requirement: App background customization controls in Settings
The **Settings** view SHALL place **Appearance** (**Light** / **Dark** theme selector) and **App background** in separate full-width cards, where the **App background** card contains a **Solid** / **Image** selectable mode toggle styled identically to **Light** / **Dark**, a vertically centered row with the `72×48` rounded thumbnail and `Choose image…` / `Clear` buttons (without filename or mode labels beside the thumbnail), a **Match accent color to background** checkbox (enabled when **Image** mode has an active background, disabled when **Solid**), and a `1%`-stepped **Overlay opacity** slider row (`0%`..`100%`) with fully opaque slider handles (`alpha == 255`) in both enabled and disabled states, using standard `30px` controls and `6px`/`8px` theme radii without displaying multi-option Equation Canvas preset pickers.

#### Scenario: Multi-row App background layout and vertical canvas preset ordering
- **WHEN** the user views **Settings**
- **THEN** **Appearance** (**Light** / **Dark**) and **App background** (**Solid** / **Image**, vertically centered `72×48` thumbnail + `Choose image…` + `Clear`, **Match accent color to background**, and `Overlay opacity`) render as separate full-width cards with no filename/mode text next to the thumbnail and no canvas preset picker rows below

#### Scenario: Toggling Solid and Image switches background without deleting the saved image
- **WHEN** a custom background image is loaded and the user clicks **Solid** and then clicks **Image**
- **THEN** clicking **Solid** switches immediately to the opaque Ayu solid theme while keeping the thumbnail and saved image reference intact and keeping the disabled **Overlay opacity** slider handle fully opaque (`alpha == 255`) with muted accent styling, and clicking **Image** restores the wallpaper without re-importing

#### Scenario: Selecting an image updates the live background immediately
- **WHEN** the user clicks **Choose image…** in **Settings** and selects a valid `PNG`, `JPEG`, or `WebP` file
- **THEN** the application displays `Importing background image…` in the bottom status bar without freezing the UI thread, updates the thumbnail preview, switches to **Image** mode, applies the image and derived accent across the window immediately, and enables the **Clear**, **Match accent color to background**, and **Overlay opacity** controls

#### Scenario: Selecting an invalid or unreadable image file preserves previous state and reports status
- **WHEN** the user selects a corrupt or unsupported image file from the file dialog
- **THEN** the application does not crash or discard any previously working background and communicates the failure in the Settings status area or status bar

#### Scenario: Adjusting overlay opacity updates surface translucency in real time
- **WHEN** the user adjusts the **Overlay opacity** slider while **Image** mode has an active custom background
- **THEN** the theme-tinted scrim (`0%`..`65%`) and panel/card surface alpha update immediately in `1%` increments—leaving the scrim transparent at `0%` while preserving a readable surface floor, balancing visibility and contrast at intermediate values, and making `panel_bg` and `card_bg` completely opaque (`255` alpha) at `100%` while keeping `app_bg` transparent over the `65%` scrim so open background areas and the thumbnail still show the image underneath

## ADDED Requirements

### Requirement: Adaptive Light and Dark accent derivation from imported background images
When a background image is imported, the desktop application SHALL extract a dominant chromatic accent seed from the image and derive separate Light-mode (deeper) and Dark-mode (brighter) accent tokens with accessible contrast across UI controls, popup chrome, and canvas highlights whenever **Image** mode and **Match accent color to background** are active.

#### Scenario: Imported background yields distinct Light and Dark mode accent tones
- **WHEN** a colorful background image is active with **Match accent color to background** enabled
- **THEN** Light mode applies a deeper accent tone for legible text, borders, and primary buttons, while Dark mode applies a brighter accent tone from the same hue, propagating across sliders, focus/hover borders (`DragValue`, `control_hover_border`), checkboxes, footer radio indicators, Batch progress bar, `ComboBox` and inline `\command` popup selections (`\` prefix + border + active suggestion row), and canvas highlights while preserving readable foreground contrast

#### Scenario: Disabling adaptive accent or switching to Solid restores default Ayu accent
- **WHEN** the user unchecks **Match accent color to background** or switches **App background** to **Solid**
- **THEN** UI controls, global widget styles, checkboxes, radio indicators, Batch progress bar, `ComboBox` and inline `\command` popup chrome, and canvas accent highlights immediately revert to the default Ayu amber/orange accent while preserving the stored accent seed for when adaptive **Image** mode is re-enabled
