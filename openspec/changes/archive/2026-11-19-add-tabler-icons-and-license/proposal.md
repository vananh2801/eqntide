# Proposal

## Why

The repository includes 32 curated Tabler SVG icons and their MIT `LICENSE.txt` under `assets/icons/tabler-icons/`, while the desktop editor still relies on Unicode text glyphs (`★`, `↶`, `↷`, `←`) or text-only buttons and both the Word `.dotm` template and WPS Office add-in reuse a single app logo image for all five Ribbon buttons. Recording proper third-party attribution and integrating these icons across the desktop app, Word template, and WPS add-in gives every action distinct visual affordances while keeping license notices compliant.

## What Changes

- Document Tabler Icons (`Copyright (c) 2020-2026 Paweł Kuna`, MIT License, `assets/icons/tabler-icons/LICENSE.txt`) in `NOTICE` and `README.md`, and verify distribution packaging includes the attribution notice cleanly.
- Add a shared pre-rasterization script (`scripts/render_icons.py`) that converts selected Tabler SVGs into committed PNG assets:
  - White/alpha-mask PNGs for `eqntide-desktop` (embedded via `include_bytes!` and tinted dynamically by Light/Dark theme state).
  - `32x32` `#1D78B8` PNGs for Microsoft Word (`word-template/build_dotm.py`) and WPS Office (`wps-addin/assets/`) Ribbon buttons.
- Update `eqntide-desktop` to render Tabler Icons on all 12 sidebar category tiles, `Batch DOCX` / `Settings` / `About` sidebar tabs, `Undo` / `Redo` footer buttons, `Back` buttons in secondary views, `Batch DOCX` action buttons and empty drop-zone, and `About` metadata rows (`Author`, `License`, `GitHub`).
- Update `word-template/build_dotm.py` (`customUI14.xml` + `.rels` + `EqnTide.dotm`), `wps-addin/main.js` (`GetImage`), and `wps-addin/taskpane.html` to assign distinct per-action icons across all Ribbon buttons and taskpane action buttons.

## Capabilities

### New Capabilities
- `office-addin-icons`: Distinct per-action Ribbon and taskpane icons across the Microsoft Word `.dotm` template and WPS Office JS add-in, backed by a deterministic SVG-to-PNG asset pipeline and Tabler Icons third-party license attribution.

### Modified Capabilities
- `desktop-palette-navigation`: Switch the 12 sidebar category tiles to Tabler Icons (`72x42` tiles inside a `174px` sidebar) and widen the minimum window width to `1056x560` so the 3-row palette and sidebar remain unclipped.
- `desktop-ui-theme`: Extend dynamic icon tinting and sizing rules from math-only sidebar/palette glyphs to Tabler UI icons across sidebar tabs, footer controls, Batch DOCX actions, and About metadata.

## Impact

- **Documentation & Licensing**: `NOTICE`, `README.md`.
- **Asset Pipeline & Packaging**: new `scripts/render_icons.py`, committed rendered PNG assets under `assets/icons/rendered/` and `wps-addin/assets/`, optional sync/check hook in `scripts/package_native.py` and Python tests.
- **Desktop App**: `desktop/src/palette.rs`, `desktop/src/app.rs`, `desktop/src/theme.rs` (no new Cargo runtime or build dependencies required).
- **Word & WPS Integrations**: `word-template/build_dotm.py`, `word-template/EqnTide.dotm`, `wps-addin/main.js`, `wps-addin/taskpane.html`, `wps-addin/assets/`.
