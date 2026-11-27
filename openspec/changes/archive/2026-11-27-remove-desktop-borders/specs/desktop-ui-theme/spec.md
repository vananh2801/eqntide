# Spec Delta

## ADDED Requirements

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

## MODIFIED Requirements

### Requirement: Ayu Light and Dark theme palette
The desktop application SHALL style both Light and Dark modes using Zed Editor–inspired Ayu neutral surfaces (layered soft neutral off-white/gray surfaces in Light mode and deep warm charcoal/obsidian surfaces in Dark mode so `app_bg`, `panel_bg`, `card_bg`, and `control_bg` remain visually distinct without surface/control borders), readable foreground contrast, and an Ayu signature amber/orange default accent color across selections (including inline `\command` popup suggestions and `ComboBox` selected rows), keyboard focus rings on modal action buttons and text-editing cursors, sliders, checkboxes, footer radio buttons, hyperlinks, primary actions, and the Batch DOCX progress bar, while keeping Batch folder tree connector lines (`border_hover`), Batch row dividers, and checkbox/radio captions neutral.

#### Scenario: Light and Dark modes apply layered neutral surfaces and legible contrast
- **WHEN** the desktop application renders in either Light mode or Dark mode with the default solid theme
- **THEN** window backgrounds (`app_bg`), panels (`panel_bg`), cards (`card_bg`), Equation Canvas / LaTeX surfaces (`pal.bg`), and interactive controls (`control_bg`) use distinct layered Ayu neutral tones with high-contrast text and icons and the Ayu amber/orange accent without requiring 1px surface or control borders

#### Scenario: Inline canvas command popup aligns leading backslash and highlights selected suggestions with active accent
- **WHEN** the user types `\` on the Equation Canvas to open the inline command entry popup and navigates candidate suggestions
- **THEN** the leading `\` prefix is vertically centered with the adjacent command text input, the input box uses a distinct borderless surface fill (`control_bg` or `extreme_bg`), the `\` prefix uses the active theme accent color, and the active suggestion row renders with `selected_bg` and `selected_fg` without an outer popup or selection border

#### Scenario: Batch progress bar and dropdown selections use active accent with crisp contrast
- **WHEN** the Batch DOCX view displays a conversion progress bar (at `0%` or partial progress) or the user opens a `ComboBox` dropdown (`batch-target`, `matrix-alignment`, `matrix-fence`)
- **THEN** the progress bar renders borderless with no partial fill stub at `0%` and fills progress with `primary_bg` using split-contrast `primary_fg` text over the filled region and `text_primary` text over the unfilled track, while selected `ComboBox` rows use `selected_bg` and `selected_fg` without a border stroke

#### Scenario: Checkboxes, footer radios, hover borders, and DragValue focus reflect active accent without tinting excluded frames
- **WHEN** interactive controls render across **Editor**, **Batch DOCX**, and **Settings** in enabled, disabled, hovered, or focused states
- **THEN** checked/indeterminate checkboxes and selected footer radio buttons tint their box/circle indicators with the active accent when enabled and opaque `control_disabled_bg` + `disabled_accent()` when disabled while keeping labels/captions neutral; hovered buttons transition to `control_hover_bg` without a border stroke; focused footer `DragValue` edits indicate active focus without adding idle borders; and Batch folder tree connector lines (`border_hover`) remain neutral

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

### Requirement: Adaptive Light and Dark accent derivation from imported background images
When a background image is imported, the desktop application SHALL extract a dominant chromatic accent seed from the image and derive separate Light-mode (deeper) and Dark-mode (brighter) accent tokens with accessible contrast across UI controls, popup chrome, and canvas highlights whenever **Image** mode and **Match accent color to background** are active.

#### Scenario: Imported background yields distinct Light and Dark mode accent tones
- **WHEN** a colorful background image is active with **Match accent color to background** enabled
- **THEN** Light mode applies a deeper accent tone for legible text and primary/selected surfaces, while Dark mode applies a brighter accent tone from the same hue, propagating across sliders, checkboxes, footer radio indicators, Batch progress bar, `ComboBox` and inline `\command` popup selections (`\` prefix + active suggestion row), and canvas highlights while preserving readable foreground contrast

#### Scenario: Disabling adaptive accent or switching to Solid restores default Ayu accent
- **WHEN** the user unchecks **Match accent color to background** or switches **App background** to **Solid**
- **THEN** UI controls, global widget styles, checkboxes, radio indicators, Batch progress bar, `ComboBox` and inline `\command` popup chrome, and canvas accent highlights immediately revert to the default Ayu amber/orange accent while preserving the stored accent seed for when adaptive **Image** mode is re-enabled
