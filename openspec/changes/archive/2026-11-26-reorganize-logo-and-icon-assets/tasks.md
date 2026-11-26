# Tasks

## 1. Relocate Branding, OLE Binaries, and Icon Assets

- [x] 1.1 Move `logo/512x512.svg` to `assets/brand/logo.svg`, move `assets/icons/icon.{png,ico}` to `assets/brand/icon.{png,ico}`, and remove the empty `logo/` directory; verify `assets/brand/` contains `logo.svg`, `icon.png`, and `icon.ico`.
- [x] 1.2 Move `assets/compobj.bin`, `assets/objinfo.bin`, and `assets/ole.bin` into `assets/ole/`, update `include_bytes!` paths in `src/ole.rs`, and verify `cargo test --lib ole` passes.
- [x] 1.3 Reorganize `assets/icons/` into `sources/tabler/` (from `tabler-icons/`), `sources/palette/` (from `palette/`), `generated/desktop/` (from `rendered/`), `generated/palette-atlas/` (from `palette-rendered/`), and `generated/office-ribbon/` (copied/synced from the 5 ribbon PNGs); verify no legacy `assets/icons/{tabler-icons,palette,rendered,palette-rendered}` directories remain.

## 2. Update Icon Generation Scripts and Rust Compile-Time Embeds

- [x] 2.1 Update `scripts/render_icons.py` paths (`SVG_DIR = assets/icons/sources/tabler`, `DESKTOP_OUT_DIR = assets/icons/generated/desktop`, `RIBBON_OUT_DIR = assets/icons/generated/office-ribbon`, and mirror sync to `wps-addin/assets/icons/`) and regenerate `manifest.json` so `python3 scripts/render_icons.py --check` exits 0 and verifies both `office-ribbon/` and `wps-addin/assets/icons/` checksums.
- [x] 2.2 Update `scripts/generate_palette_icons.py` paths (`SVG_OUT_DIR = assets/icons/sources/palette`, `ATLAS_OUT_DIR = assets/icons/generated/palette-atlas`) and verify `python3 scripts/generate_palette_icons.py --check` exits 0.
- [x] 2.3 Update compile-time embed and Windows resource paths in `desktop/src/main.rs`, `desktop/src/app/settings_ui.rs`, `desktop/src/app/mod.rs`, `desktop/src/app/palette_ui.rs`, `desktop/src/palette.rs`, and `desktop/build.rs`, then verify `cargo check --workspace` and `cargo test -p eqntide-desktop` pass.

## 3. Decouple Word Template Build and Slim Distribution Packaging

- [x] 3.1 Update `word-template/build_dotm.py` `RIBBON_IMAGES` to read from `assets/icons/generated/office-ribbon/` instead of `wps-addin/assets/icons/`, and verify `tests/python/test_icons.py::test_word_dotm_embeds_distinct_ribbon_icons` passes.
- [x] 3.2 Update `scripts/package_native.py` and `scripts/installer.nsi` so `stage/icons/` stages only `assets/brand/icon.png` (`icon.png`), `assets/brand/icon.ico` (`icon.ico`), and `assets/icons/sources/tabler/LICENSE.txt` (`tabler-LICENSE.txt`), and update `tests/python/test_icons.py` (including `test_package_native_stages_icons_and_license`) so `python3 -m unittest discover -s tests/python` passes.
- [x] 3.3 Update Tabler Icons and asset path documentation in `README.md` and `NOTICE` to reference `assets/icons/sources/tabler/LICENSE.txt`, and verify no stale references to `logo/512x512.svg`, `assets/icons/tabler-icons`, `assets/icons/rendered`, `assets/icons/palette/`, or `assets/icons/palette-rendered/` remain in source, scripts, tests, or docs.

## 4. End-to-End Verification

- [x] 4.1 Run `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and `tests/run_all.sh --quick` to verify full Rust, Python, and WPS ribbon integration suites pass.
