# Tasks

## 1. Third-Party Attribution & Licensing

- [x] 1.1 Add a numbered Tabler Icons attribution entry (`https://github.com/tabler/tabler-icons`, `Copyright (c) 2020-2026 Paweł Kuna`, `MIT License`, `assets/icons/tabler-icons/LICENSE.txt`) to `NOTICE` and verify formatting matches existing asset notices.
- [x] 1.2 Update `README.md` (`References & Credits` and `License` sections) to reference Tabler Icons and `assets/icons/tabler-icons/LICENSE.txt`, and verify `cargo deny check licenses bans sources` still passes.

## 2. SVG-to-PNG Asset Pipeline (`scripts/render_icons.py`)

- [x] 2.1 Create `scripts/render_icons.py` to convert the selected `assets/icons/tabler-icons/*.svg` icons into committed white-on-transparent `48x48` PNGs under `assets/icons/rendered/` (for `eqntide-desktop`) and `#1D78B8` `32x32` PNGs under `wps-addin/assets/icons/` (for Word & WPS Ribbon), with a `--check` mode; verify `python3 -m py_compile scripts/render_icons.py` passes.
- [x] 2.2 Run `scripts/render_icons.py` to generate and commit all required PNGs, and add a Python unit test under `tests/python/` verifying every mapped desktop/ribbon PNG exists with expected dimensions (`48x48` / `32x32`) and passes `python3 -m unittest discover -s tests/python -p "test_*.py"`.

## 3. Desktop App Tabler Icons (`desktop/`)

- [x] 3.1 Update `desktop/src/palette.rs` and `desktop/src/app.rs` to replace the 12 sidebar category STIX/Unicode icons (`★` and math expressions) with the assigned Tabler PNG textures (`history`, `math-x-divide-y` + `square-root-2`, `x-power-y`, `parentheses`, `math-integral`, `sum`, `matrix`, `overline` + `underline`, `math-equal-lower`, `arrows-left-right`, `math-avg`, `alpha` + `omega`) tinted via `ThemeTokens::icon_tint`, keeping `72x42` tiles and tooltips/accessibility labels intact.
- [x] 3.2 Add a shared `ui_icon` helper in `desktop/src/app.rs` and wire Tabler icons into `Batch DOCX`, `Settings`, and `About` (`info-circle`) sidebar tabs, footer `Undo`/`Redo` buttons, `Back` buttons, `Batch DOCX` controls (`Add DOCX files…`, `Add folder…`, `Convert`, `Open output folder`, empty drop-zone), and `About` metadata rows (`Author`, `License`, `GitHub`), preserving `30px`/`34px`/`24px` height tiers and full-width card layouts.
- [x] 3.3 Update and run desktop UI tests in `desktop/` (`cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, and `cargo test --workspace --all-targets --locked`) verifying sidebar visibility at `1056x560`, Light/Dark theme icon tinting, and button dimensions.

## 4. Word `.dotm` Template & WPS Add-in Icons

- [x] 4.1 Update `word-template/build_dotm.py` (`customUI14.xml` and `customUI/_rels/customUI14.xml.rels`) to embed and bind distinct `32x32` PNGs (`pencil-plus`, `edit`, `tex`, `arrow-big-up-lines`, `file-type-docx`) for the 5 Ribbon buttons, regenerate `word-template/EqnTide.dotm`, and verify the `.dotm` ZIP contains all 5 images and valid `customUI14.xml` relationships.
- [x] 4.2 Update `wps-addin/main.js` (`GetImage(control)`) to return each button's dedicated `assets/icons/<name>-32.png` path by `control.Id` (with `assets/icon-32.png` fallback) and update `wps-addin/taskpane.html` to display Tabler SVGs (`arrow-big-up-lines` on `#btn-start-upgrade`, `file-type-docx` on `#btn-disk-upgrade`); verify `node --check wps-addin/main.js`, `node --check wps-addin/taskpane.js`, and `npm test --prefix tests/wps` pass.

## 5. Packaging Verification

- [x] 5.1 Verify `scripts/package_native.py` stages `NOTICE`, `LICENSE`, `assets/icons/tabler-icons/LICENSE.txt`, `word-template/EqnTide.dotm`, and `wps-addin/assets/icons/` cleanly (or invoke `scripts/render_icons.py --check` before staging) and passes `python3 -m py_compile scripts/package_native.py` and existing packaging tests.
