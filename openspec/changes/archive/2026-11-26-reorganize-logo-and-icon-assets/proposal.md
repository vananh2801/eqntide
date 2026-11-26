# Proposal

## Why

Logo, icon, and binary template assets are currently scattered across top-level `logo/`, `assets/`, `assets/icons/`, and `wps-addin/assets/`, mixing brand artwork, third-party source SVGs, script-generated PNGs/atlases, and OLE binaries in a single flat hierarchy. This layout also couples `word-template/` to `wps-addin/` and causes `scripts/package_native.py` to bundle hundreds of build-time-only SVGs and pre-rasterized desktop PNGs into release packages even though they are already compiled into `eqntide-desktop`.

## What Changes

- Consolidate application branding (`logo/512x512.svg`, `assets/icons/icon.png`, `assets/icons/icon.ico`) into `assets/brand/` (`logo.svg`, `icon.png`, `icon.ico`) and remove the orphaned top-level `logo/` directory.
- Move OLE template binaries (`assets/compobj.bin`, `assets/objinfo.bin`, `assets/ole.bin`) into `assets/ole/`.
- Reorganize `assets/icons/` into `sources/` (`tabler/` for curated third-party SVGs + `LICENSE.txt`, and `palette/` for the 547 generated palette SVGs) and `generated/` (`desktop/` for pre-rasterized desktop UI PNGs + `manifest.json`, `office-ribbon/` for canonical 32x32 ribbon PNGs, and `palette-atlas/` for `atlas.png` + `atlas.json`).
- Decouple `word-template/build_dotm.py` from `wps-addin/assets/icons/` by reading Word Ribbon PNGs from `assets/icons/generated/office-ribbon/`, while `scripts/render_icons.py` generates `assets/icons/generated/office-ribbon/` and mirrors those 5 PNGs into `wps-addin/assets/icons/` (verified by `--check`) so `wps-addin/` remains self-contained at runtime.
- Slim down native distribution staging in `scripts/package_native.py` so `stage/icons/` ships only runtime/attribution files (`icon.png`, `icon.ico`, and `tabler-LICENSE.txt`) rather than copying build-time-only source SVGs, desktop UI PNGs, and palette sprite atlases.

## Capabilities

### New Capabilities

### Modified Capabilities
- `office-addin-icons`: Update canonical Tabler SVG source path (`assets/icons/sources/tabler/`), Word Ribbon PNG source (`assets/icons/generated/office-ribbon/`), Tabler `LICENSE.txt` reference path in `NOTICE`/`README.md`, and staged distribution icon/license layout in native packages.
- `desktop-palette-navigation`: Update palette SVG verification path from `assets/icons/palette/*.svg` to `assets/icons/sources/palette/*.svg`.

## Impact

- **Rust crates**: Compile-time `include_bytes!`/`include_str!` and `build.rs` paths in `desktop/src/main.rs`, `desktop/src/app/mod.rs`, `desktop/src/app/settings_ui.rs`, `desktop/src/app/palette_ui.rs`, `desktop/src/palette.rs`, `desktop/build.rs`, and `src/ole.rs`.
- **Build, packaging & scripts**: `scripts/render_icons.py`, `scripts/generate_palette_icons.py`, `scripts/package_native.py`, `scripts/installer.nsi`, and `word-template/build_dotm.py`.
- **Tests & docs**: `tests/python/test_icons.py`, `README.md`, and `NOTICE` (`tests/python/test_native_integration.py` continues to expect `stage/icons/icon.png`).
