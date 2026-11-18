# Spec Delta

## ADDED Requirements

### Requirement: Consistent selectable theme option buttons in Settings
The **Settings** view SHALL present **Light** and **Dark** theme choices as a selectable option button group styled identically to the **Startup focus** selector below it, rather than a single toggle button.

#### Scenario: Switching theme via Light and Dark option buttons
- **WHEN** the user opens **Settings** while Dark mode is active and clicks the **Light** button
- **THEN** the application switches to Light mode, persists the `Light` theme preference, highlights **Light** as selected, and renders **Dark** unselected using the shared selectable button styling
