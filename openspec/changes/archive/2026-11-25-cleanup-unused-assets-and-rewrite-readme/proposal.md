# Proposal

## Why

The repository has accumulated unreferenced font binaries, leftover Word Web Add-in icons, an obsolete sprint validation log (`desktop/VALIDATION.md`), and a 787-line `README.md` with stale version numbers (`0.6.1`), repeated build/install sections, and orphaned sections placed after `License`. Cleaning up unused assets, keeping font specimen PDFs out of distributed installers, and rewriting `README.md` into a concise, well-structured guide makes the repository easier to maintain and navigate.

## What Changes

- Remove unused tracked font binaries and redundant documentation files under `assets/fonts/` while retaining the 6 embedded `.ttf` fonts (`STIXTwoText-Regular/Italic/Bold`, `STIXTwoMath-Regular`, `JetBrainsMono-Regular`, `Nunito-Regular`), all font license/author notices (`OFL.txt`, `AUTHORS.txt`), and the two STIX specimen `.pdf` files (`STIXTwoMath-Regular.pdf`, `STIXTwoText-Regular.pdf`) for local reference.
- Remove leftover unused icons `wps-addin/assets/icon-16.png` and `wps-addin/assets/icon-80.png` (WPS add-in only uses `icon-32.png`, `icon-64.png`, and `wps-addin/assets/icons/*.png`).
- Remove the obsolete internal validation log `desktop/VALIDATION.md` (retaining `logo/512x512.svg`, `dist/`, `KNOWN_ISSUES.md`, and `tests/TESTING.md`).
- Update `scripts/package_native.py` to exclude `*.pdf` when copying `assets/fonts/` into packaged staging directories, and add a regression check in `tests/python/test_native_integration.py`.
- Rewrite `README.md` to be concise (~250–320 lines), clearly ordered, updated to version `0.8.1`, preserving all required attributions (`office-addin-icons` Tabler Icons license notice, STIX/Nunito/JetBrains Mono OFL notices, `NOTICE`, `LICENSE`), removing links to `desktop/VALIDATION.md`, moving bridge diagnostics and CI workflows into their proper sections, and briefly introducing `KNOWN_ISSUES.md` and `tests/TESTING.md`.

## Capabilities

### New Capabilities

<!-- None: pure repository cleanup, packaging exclusion, and documentation rewrite. -->

### Modified Capabilities

<!-- None: no spec-level behavior changes. Existing Tabler Icons attribution requirements in `office-addin-icons` remain satisfied in `README.md` and `NOTICE`. -->

## Impact

- Affected files: `assets/fonts/`, `wps-addin/assets/`, `desktop/VALIDATION.md`, `scripts/package_native.py`, `tests/python/test_native_integration.py`, `README.md`.
- No Rust library, CLI, desktop runtime, or add-in behavior changes.
- Packaged archives and installers (`.tar.gz`, `.deb`, `.rpm`, `.zip`, `Setup.exe`) will exclude unused `.ttf` files and `.pdf` specimen files while keeping all required license files.
