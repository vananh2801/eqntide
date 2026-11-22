# Office Add-in Batch Reliability Delta

## ADDED Requirements

### Requirement: WPS batch counts reflect confirmed equation targets
WPS batch equation-to-LaTeX toggling SHALL exclude ordinary images and drawings from conversion requests and summary counts. A confirmed equation that fails conversion SHALL count as skipped or failed.

#### Scenario: Convert a document containing MathType and ordinary illustrations
- **WHEN** the user selects the reference DOCX containing 4,338 MathType OLE objects and ordinary illustrations and toggles to LaTeX
- **THEN** WPS reports 4,338 found, 4,338 converted, and zero skipped or failed
- **AND** ordinary illustrations remain in the document

#### Scenario: Ambiguous shape contains supported equation data
- **WHEN** a shape resembles a picture in WPS metadata but its associated content contains a supported equation payload
- **THEN** the add-in includes it in the equation batch

#### Scenario: A confirmed equation cannot be converted
- **WHEN** conversion fails for a confirmed equation
- **THEN** the equation remains in the document and is reported as skipped or failed

### Requirement: WPS Toggle LaTeX requires an equation selection

WPS Toggle LaTeX SHALL convert an equation to LaTeX only when WPS exposes the equation as selected or the highlighted range contains it. A caret immediately before or after an object SHALL leave the object unselected. Toggle LaTeX SHALL continue to support converting delimited LaTeX at the caret into an equation.

#### Scenario: The caret is next to an equation or illustration
- **WHEN** the user places the caret immediately before or after an equation or illustration without selecting the object and invokes Toggle LaTeX
- **THEN** the add-in sends no equation-to-LaTeX conversion request for the adjacent object
- **AND** the adjacent object remains in the document

#### Scenario: The user selects an equation
- **WHEN** the user clicks an equation that WPS exposes as selected or highlights a range containing the equation and invokes Toggle LaTeX
- **THEN** the add-in attempts to convert the selected equation to LaTeX

#### Scenario: The caret is inside delimited LaTeX
- **WHEN** the caret is inside a supported delimited LaTeX expression and the user invokes Toggle LaTeX
- **THEN** the add-in attempts to convert that expression into an equation
