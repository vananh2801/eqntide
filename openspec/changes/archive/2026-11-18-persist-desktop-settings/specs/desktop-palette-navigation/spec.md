# Spec Delta

## MODIFIED Requirements

### Requirement: Recent category tracking
The desktop sidebar SHALL include a Recent category that records palette items inserted by the user in most-recently-used order without duplicates and persists those valid items across application restarts.

#### Scenario: Inserting a palette item adds it to the front of Recent
- **WHEN** the user clicks a symbol or template button in the palette
- **THEN** that item is placed at the front of the Recent category list without creating duplicate entries

#### Scenario: App launches on a populated default category
- **WHEN** the desktop application starts a new session with no recent history
- **THEN** the initially active category is a populated structural category rather than an empty Recent list

#### Scenario: Recent palette items persist across restarts
- **WHEN** the user inserts palette items and restarts the desktop application
- **THEN** the Recent category restores the previously recorded valid palette items in most-recently-used order
