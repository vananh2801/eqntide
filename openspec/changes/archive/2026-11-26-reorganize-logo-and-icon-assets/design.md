# Design

## Context

See `proposal.md` for motivation. Currently, logo, icon, and OLE template binary assets live across `logo/`, `assets/`, `assets/icons/`, and `wps-addin/assets/`, mixing source vector artwork, generated raster outputs, compiled sprite atlases, third-party licenses, and runtime packaging inputs.

Key constraints from the codebase:
- `eqntide-desktop` embeds its window/about logo (`icon.png`), desktop UI PNGs, and palette sprite atlas (`atlas.png`, `atlas.json`) at compile time via `include_bytes!` / `include_str!`.
- `eqntide` core embeds OLE template binaries (`compobj.bin`, `objinfo.bin`, `ole.bin`) at compile time in `src/ole.rs`.
- `desktop/build.rs` and `scripts/installer.nsi` require `icon.ico` at build/installer time.
- `wps-addin/` must stay self-contained because the local bridge (`src/bridge/server/addin_assets.rs`) and add-in installers serve/copy `wps-addin/` directly using relative paths (`assets/icon-32.png`, `assets/icon-64.png`, `assets/icons/*.png`).
- Installed packages (`scripts/package_native.py`) only read `icons/icon.png` (Linux desktop entry icon), `icons/icon.ico` (NSIS Start Menu shortcuts), and Tabler's MIT license notice at runtime/distribution time.

## Goals / Non-Goals

**Goals:**
- Separate application branding (`assets/brand/`), OLE templates (`assets/ole/`), icon source artwork (`assets/icons/sources/`), and script-generated raster outputs (`assets/icons/generated/`).
- Eliminate the cross-dependency where `word-template/build_dotm.py` reads icons out of `wps-addin/assets/icons/`.
- Ensure `scripts/render_icons.py` and `scripts/generate_palette_icons.py` stay deterministic and verify exact parity (including synced copies in `wps-addin/assets/icons/`).
- Stop shipping ~580 build-time-only SVGs and pre-rasterized desktop/palette assets in native distribution archives.

**Non-Goals:**
- Changing visual appearance, dimensions, colors, or content of any existing logo, icon, font, or palette entry.
- Moving or restructuring `assets/fonts/` or `wps-addin/assets/` runtime paths (`wps-addin/main.js`, `wps-addin/taskpane.html`, `tests/wps/test_wps_ribbon.js` stay unchanged).
- Rebuilding `word-template/EqnTide.dotm` binary content beyond verifying it reads identical PNG bytes from the new canonical location.

## Decisions

### Decision 1: Split `assets/` by asset role (`brand/`, `ole/`, `fonts/`, `icons/{sources,generated}/`)
- **Chosen**:
  - `assets/brand/{logo.svg, icon.png, icon.ico}` (removing top-level `logo/`)
  - `assets/ole/{compobj.bin, objinfo.bin, ole.bin}`
  - `assets/icons/sources/{tabler/, palette/}`
  - `assets/icons/generated/{desktop/, office-ribbon/, palette-atlas/}`
- **Alternatives considered**: Consumer-first grouping (`assets/icons/desktop/...`, `assets/icons/office/...`). Rejected because `desktop/` would still mix hand-curated/generated 547 source SVGs with compiled raster atlases, making it harder to distinguish source inputs from generated build outputs.

### Decision 2: Canonical `office-ribbon/` mirrored to `wps-addin/assets/icons/`
- **Chosen**: `scripts/render_icons.py` renders the 5 `#1D78B8` `32x32` Ribbon PNGs into `assets/icons/generated/office-ribbon/` as the canonical source for `word-template/build_dotm.py`, and mirrors identical PNGs into `wps-addin/assets/icons/`, with `manifest.json` (in `assets/icons/generated/desktop/manifest.json`) and `--check` verifying hashes of both targets.
- **Alternatives considered**:
  1. Keep `word-template/build_dotm.py` reading from `wps-addin/assets/icons/`: avoids 5 duplicate ~1 KB PNGs in git, but preserves awkward cross-coupling from Word tooling into the WPS JS add-in directory.
  2. Omit `wps-addin/assets/icons/` from git and copy only during packaging: breaks direct repo runs (`cargo run`, `addin_assets.rs` `CARGO_MANIFEST_DIR` fallback, `scripts/install-addin.*`).
- **Trade-off**: 5 tiny PNG files (~4.5 KB total) exist in both `assets/icons/generated/office-ribbon/` and `wps-addin/assets/icons/`, guarded against drift by `render_icons.py --check` and `tests/python/test_icons.py`.

### Decision 3: Slim `stage/icons/` in `scripts/package_native.py`
- **Chosen**: Instead of `shutil.copytree(ROOT / "assets/icons", stage / "icons")`, create `stage/icons/` and copy only `assets/brand/icon.png` -> `stage/icons/icon.png`, `assets/brand/icon.ico` -> `stage/icons/icon.ico`, and `assets/icons/sources/tabler/LICENSE.txt` -> `stage/icons/tabler-LICENSE.txt`.
- **Rationale**: Preserves existing installer expectations (`scripts/installer.nsi` uses `$INSTDIR\icons\icon.ico`, `linux_tree` uses `stage/icons/icon.png`, `tests/python/test_native_integration.py` constructs `source/icons/icon.png`) while removing hundreds of unused build-time files from release packages.

## Risks / Trade-offs

- **Stale path references in Rust/Python/NSIS/CI**: Moving asset directories breaks compile-time `include_bytes!`, `build.rs`, or packaging if any relative path is missed.
  -> Mitigation: Run `cargo test --workspace`, `cargo clippy`, `python3 scripts/render_icons.py --check`, `python3 scripts/generate_palette_icons.py --check`, and Python/JS test suites (`tests/python/test_icons.py`, `tests/python/test_native_integration.py`, `tests/run_all.sh --quick`) after moving files.
- **Drift between `office-ribbon/` and `wps-addin/assets/icons/`**: Editing one copy manually would cause divergence.
  -> Mitigation: `scripts/render_icons.py --check` validates SHA-256 checksums of both `assets/icons/generated/office-ribbon/*.png` and `wps-addin/assets/icons/*.png` against `manifest.json`.
