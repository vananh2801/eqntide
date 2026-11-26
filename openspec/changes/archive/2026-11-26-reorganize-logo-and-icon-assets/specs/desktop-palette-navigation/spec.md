# Spec Delta: desktop-palette-navigation

## MODIFIED Requirements

### Requirement: Deterministic synchronization between palette catalog and generated SVG icons
Every `ENTRIES` item defined in `src/catalog/mod.rs` SHALL have a corresponding generated `40x26` SVG asset and entry in the committed palette sprite atlas, verified by automated icon checks.

#### Scenario: Icon verification detects missing or stale palette SVGs or atlas data
- **WHEN** the icon verification test or script runs after catalog or SVG changes
- **THEN** it verifies a 1-to-1 match between all catalog `ENTRIES` IDs/sources, committed `assets/icons/sources/palette/*.svg` files, and the pre-rasterized sprite atlas and UV manifest under `assets/icons/generated/palette-atlas/`
