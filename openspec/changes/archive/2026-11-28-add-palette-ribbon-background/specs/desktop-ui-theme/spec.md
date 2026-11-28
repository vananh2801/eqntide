## MODIFIED Requirements

### Requirement: Compact default window and Equation Canvas sizing
The desktop application SHALL launch with a default window inner size of `1056x560`, matching the minimum window size so the Equation Canvas opens at a compact, balanced height—absorbing the vertical padding added for the top palette panel and maintaining uniform `8px` vertical spacing between the palette divider, Equation Canvas, LaTeX source box, and footer divider—while preserving full sidebar, padded three-row palette, and fixed-height LaTeX source visibility.

#### Scenario: Default launch opens at 1040x560 with balanced canvas height
- **WHEN** the desktop application opens with its default viewport configuration
- **THEN** the window inner size is `1056x560`, the top palette panel reserves `8px` top and bottom padding around its three-row tile grid, the LaTeX source panel reserves its fixed `88px` height (`80px` input below an `8px` top gap), and the Equation Canvas decreases proportionally to fill the remaining central area with uniform `8px` vertical spacing below the palette divider, above the LaTeX source box, and matching the `8px` gap below the LaTeX source box

### Requirement: Custom full-window application background image
When **Image** background mode is active and a valid custom background image is loaded, the desktop application SHALL render that image across the full viewport behind all application views (**Editor**, **Batch DOCX**, **Settings**, and **About**) using aspect-preserving cover scaling, layered with a theme-aware dimming scrim (`0%`..`65%`) and translucent-to-opaque chrome/card surfaces controlled by **Overlay opacity** (`0%`..`100%`, where `panel_bg`—including the sidebar, top palette panel, footer, and status bar—and `card_bg` become fully opaque at `100%` while `app_bg` remains transparent over the scrim so the background image stays visible in open areas), while rendering the **Equation Canvas** and **LaTeX source** box with the active mode's Ayu `CanvasPalette` whose background surface alpha scales across `0.38..=1.00` according to **Canvas opacity** (`0%`..`100%`, defaulting to `100%` opaque).

#### Scenario: Custom background displays across Editor and secondary views
- **WHEN** the user configures a valid background image in **Image** mode and navigates between the **Editor**, **Batch DOCX**, **Settings**, and **About** views
- **THEN** the background image fills the window behind the application shell with aspect-preserving cover scaling and remains visible across open central backdrop areas, while chrome panels (sidebar, top palette panel, footer, and status bar) and cards scale from translucent to opaque (`255` alpha at `100%`) according to **Overlay opacity**

#### Scenario: Equation Canvas and LaTeX source preserve crisp contrast over a custom background
- **WHEN** a custom background image is active in either Light mode or Dark mode at the default `100%` **Canvas opacity** (or across `0%`..`100%` with the `0.38` surface floor)
- **THEN** the **Equation Canvas** and **LaTeX source** box preserve crisp legibility with the active `Ayu light` or `Ayu dark` surface, equation ink, caret, selection, and placeholder colors

#### Scenario: Equation Canvas and LaTeX source scale translucency with Canvas opacity over a custom background
- **WHEN** a custom background image is active in either Light mode or Dark mode and **Canvas opacity** is set to `100%`, an intermediate value, or `0%`
- **THEN** the **Equation Canvas** and **LaTeX source** box share the active `Ayu light` or `Ayu dark` `CanvasPalette` ink, border, caret, selection, and placeholder colors while their background fill (`pal.bg`) scales linearly in alpha from `0.38` (`0%`) to `1.00` (`100%`, fully opaque by default)

#### Scenario: Clearing or missing background falls back to default slate theme
- **WHEN** **Solid** mode is selected, no background image is configured, the user clicks **Clear**, or the stored background file cannot be loaded
- **THEN** the desktop application renders the opaque Ayu Light or Dark theme surfaces (including opaque `255`-alpha chrome panels and **Equation Canvas** / **LaTeX source** fills) and default Ayu accent without visual errors

### Requirement: Borderless chrome with preserved region dividers, tree lines, and indicator strokes
The desktop application SHALL omit card, canvas, LaTeX box, modal, popup, thumbnail, progress-bar, status-bar outer box, and idle/hovered/selected control borders, relying on layered Ayu neutral fills while preserving structural region dividers, Batch DOCX tree lines, Equation Canvas dashed placeholders, and functional widget indicators.

#### Scenario: Editor and secondary tabs render region divider lines without card, canvas, or button borders
- **WHEN** the user views the **Editor**, **Batch DOCX**, **Settings**, or **About** tab in either Light mode or Dark mode (Solid or Image background)
- **THEN** the sidebar vertical divider, top palette panel bottom horizontal divider, footer horizontal divider, status-bar top horizontal divider, and section separators render cleanly over panel fills while no status-bar outer box, card border, Equation Canvas / LaTeX box outer border, progress bar outer border, or idle/hovered/selected button/dropdown border is drawn, and distinct layered surface and control fills keep every region and control clearly legible

#### Scenario: Batch DOCX Folder tree and File tree preserve internal hierarchy and row lines
- **WHEN** the user views queued items in **Batch DOCX** under either **Folder tree** or **File tree** mode
- **THEN** the outer list card renders without a border while the internal horizontal row dividers and vertical/horizontal folder hierarchy connector lines continue to render normally

#### Scenario: Hovering and selecting controls transitions fill colors without borders
- **WHEN** the user hovers over enabled controls or selects sidebar categories, sidebar tabs, Settings toggles, or `ComboBox` rows
- **THEN** background and foreground tones update cleanly without painting hover or selection border strokes

#### Scenario: Checkbox, radio, slider, placeholder, and bridge indicators stay visible
- **WHEN** checkboxes, radio buttons, sliders, empty canvas placeholders (`□`), or the disconnected status-bar bridge icon render
- **THEN** checkbox squares, radio circles, slider thumb borders, dashed canvas placeholder outlines, and the hollow disconnected bridge circle remain clearly visible
