# Spec Delta

## ADDED Requirements

### Requirement: Full-width secondary tab layouts
The desktop application SHALL render the **Batch DOCX**, **Settings**, and **About** views at the full available central-panel width, framing their primary sections in full-width card surfaces without horizontal auto-shrinking or left-clustering.

#### Scenario: Settings cards and scrollbars span the full window width
- **WHEN** the user opens the **Settings** tab at `1040x560` or a wider window size
- **THEN** all setting section cards (`Canvas zoom`, `Appearance`, `Startup focus`) and the vertical scroll area expand to match the full available central-panel width instead of capping at `540px` on the left

#### Scenario: About metadata renders inside a full-width card
- **WHEN** the user opens the **About** tab at `1040x560` or a wider window size
- **THEN** the application summary, divider, and metadata grid render inside a full-width card surface aligned with the central-panel width

#### Scenario: Batch DOCX controls and empty state fill the central panel width
- **WHEN** the user opens the **Batch DOCX** tab with no files queued or with files queued
- **THEN** the batch control bar sits inside a full-width card surface, and the drop-zone empty state or scrollable file list spans the matching full central-panel width
