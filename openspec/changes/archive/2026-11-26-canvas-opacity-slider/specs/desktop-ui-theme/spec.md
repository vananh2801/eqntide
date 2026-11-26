# Spec Delta

## MODIFIED Requirements

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
