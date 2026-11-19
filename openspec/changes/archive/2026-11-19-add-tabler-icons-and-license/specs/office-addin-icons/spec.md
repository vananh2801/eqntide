# Spec Delta

## Purpose

Defines distinct Tabler-derived icons across the Microsoft Word `.dotm` template and WPS Office JS add-in (Ribbon and taskpane), together with third-party Tabler Icons attribution and distribution packaging compliance.

## ADDED Requirements

### Requirement: Distinct per-action Ribbon icons in Microsoft Word template
The generated Microsoft Word global template (`word-template/EqnTide.dotm`) SHALL embed distinct `32x32` `#1D78B8` PNG icons derived from `assets/icons/tabler-icons/` for each Ribbon command button rather than reusing a single shared logo image.

#### Scenario: Word customUI14 maps each Ribbon button to its assigned image relationship
- **WHEN** `word-template/build_dotm.py` generates `EqnTide.dotm`
- **THEN** `customUI/customUI14.xml` and `customUI/_rels/customUI14.xml.rels` bind `btnInsertEquation` to `pencil-plus`, `btnEditEquation` to `edit`, `btnToggleLatex` to `tex`, `btnUpgradeMathType` to `arrow-big-up-lines`, and `btnBatchConverter` to `file-type-docx` as `32x32` PNG images under `customUI/images/`

### Requirement: Distinct per-action Ribbon and taskpane icons in WPS Office add-in
The WPS Office JS add-in (`wps-addin/`) SHALL return distinct `32x32` `#1D78B8` PNG icons per control ID from `GetImage(control)` for all five Ribbon buttons and display Tabler SVG icons on taskpane action buttons.

#### Scenario: WPS Ribbon GetImage resolves distinct PNG assets by control ID
- **WHEN** WPS invokes `GetImage(control)` for `btnInsertEquation`, `btnEditEquation`, `btnToggleLatex`, `btnUpgradeLegacy`, or `btnBatchConverter`
- **THEN** it returns the control's dedicated PNG path (`pencil-plus`, `edit`, `tex`, `arrow-big-up-lines`, or `file-type-docx` respectively), falling back to `assets/icon-32.png` only for unrecognized IDs

#### Scenario: WPS taskpane buttons render Tabler SVG icons
- **WHEN** `wps-addin/taskpane.html` renders in the WPS taskpane webview
- **THEN** the `Scan & Upgrade Equations` button displays the `arrow-big-up-lines` Tabler SVG icon and the `Upgrade Active File via Rust Core` button displays the `file-type-docx` Tabler SVG icon using `currentColor`

### Requirement: Third-party Tabler Icons attribution and packaging
The repository SHALL document the Tabler Icons MIT license (`Copyright (c) 2020-2026 Paweł Kuna`, `assets/icons/tabler-icons/LICENSE.txt`) in `NOTICE` and `README.md`, and native distribution packages SHALL retain the license notice alongside distributed assets.

#### Scenario: Attribution files and native packages include Tabler Icons license notice
- **WHEN** `NOTICE`, `README.md`, and native release staging (`scripts/package_native.py`) are inspected
- **THEN** `NOTICE` and `README.md` identify Tabler Icons, its copyright holder, MIT License, and `assets/icons/tabler-icons/LICENSE.txt`, and packaged archives include the license file
