# Spec Delta

## ADDED Requirements

### Requirement: Custom full-window application background image
When a custom background image is configured, the desktop application SHALL render that image across the full viewport behind all application views (**Editor**, **Batch DOCX**, **Settings**, and **About**) using aspect-preserving cover scaling, layered with a theme-aware dimming scrim and translucent chrome/card surfaces while keeping the **Equation Canvas** and **LaTeX source** box opaque using the active `CanvasPreset`.

#### Scenario: Custom background displays across Editor and secondary views
- **WHEN** the user configures a valid background image and navigates between the **Editor**, **Batch DOCX**, **Settings**, and **About** views
- **THEN** the background image fills the window behind the application shell with aspect-preserving cover scaling, and the sidebar, footer, status bar, central backdrop, and secondary tab cards render translucently so the image remains visible underneath

#### Scenario: Equation Canvas and LaTeX source preserve crisp contrast over a custom background
- **WHEN** a custom background image is active in either Light mode or Dark mode
- **THEN** the **Equation Canvas** and **LaTeX source** box continue rendering with their active `CanvasPreset` background, equation ink, caret, selection, and placeholder colors at full legibility

#### Scenario: Clearing or missing background falls back to default slate theme
- **WHEN** no background image is configured, the user clicks **Clear**, or the stored background file cannot be loaded
- **THEN** the desktop application renders the standard opaque slate Light or Dark theme surfaces without visual errors

### Requirement: App background customization controls in Settings
The **Appearance** card in the **Settings** view SHALL include a multi-row **App background** section allowing the user to choose an image (`PNG`, `JPEG`, or `WebP`), preview a rounded thumbnail with a left-aligned status line, adjust **Overlay opacity** on a separate row, and clear the background using standard `30px` controls and `6px`/`8px` theme radii, while keeping **Light mode canvas** and **Dark mode canvas** stacked vertically below.

#### Scenario: Multi-row App background layout and vertical canvas preset ordering
- **WHEN** the user views the **Appearance** card in **Settings**
- **THEN** row 1 of **App background** shows the `72×48` rounded thumbnail beside `Choose image…`, `Clear`, and the status line (`Default solid theme`, `Importing background image…`, `Active: <filename>`, or error) left-aligned with `Choose image…`, row 2 shows the `Overlay opacity` slider and percentage, and **Light mode canvas** and **Dark mode canvas** remain stacked vertically below

#### Scenario: Selecting an image updates the live background immediately
- **WHEN** the user clicks **Choose image…** in **Settings** and selects a valid `PNG`, `JPEG`, or `WebP` file
- **THEN** the application displays `Importing background image…` without freezing the UI thread, then updates the thumbnail preview, applies the image across the window immediately, and enables the **Clear** and **Overlay opacity** controls

#### Scenario: Selecting an invalid or unreadable image file preserves previous state and reports status
- **WHEN** the user selects a corrupt or unsupported image file from the file dialog
- **THEN** the application does not crash or discard any previously working background and communicates the failure in the Settings status area or status bar

#### Scenario: Adjusting overlay opacity updates surface translucency in real time
- **WHEN** the user adjusts the **Overlay opacity** slider while a custom background is active
- **THEN** the theme-tinted scrim and translucent surface alpha update immediately to balance background visibility against text and icon readability
