# Tasks

## 1. Custom Matrix Tabler Icon Asset

- [x] 1.1 Add Tabler `dots.svg` to `assets/icons/tabler-icons/dots.svg`, register `"dots"` in `scripts/render_icons.py` (`DESKTOP_ICONS`), regenerate `assets/icons/rendered/dots.png` and `manifest.json`, and verify `python3 scripts/render_icons.py --check` passes.
- [x] 1.2 Add `"dots"` to `tabler_icon_bytes` in `desktop/src/app/mod.rs`, update `tests/python/test_icons.py` expected `DESKTOP_ICONS` count/checks if needed, and verify `python3 -m unittest tests/python/test_icons.py` passes.

## 2. Palette SVG Generator & Font-to-Path Pipeline

- [x] 2.1 Implement `scripts/generate_palette_icons.py` and `scripts/palette_svg_builders.py` parsing all 547 `ENTRIES` from `src/catalog/mod.rs` with a `40x26` `viewBox`, uniform `1.7` monoline stroke style (`round` cap/join), smooth circular-arc parentheses (`r = 0.765 * h`), high-contrast double-shaft arrowheads (`size = 5.4`, `45°`), thin muted solid-border `□` slot rects (`rx="1.1"`, `stroke-width="1.15"`, `opacity="0.68"`), and full hollow-square matrix primitives (`1x2` through `4x4`); verify generated structural SVGs cover all template, arrow, relation, operator, set, logic, and space entries with zero missing IDs.
- [x] 2.2 Implement `Nunito-Regular` (`Functions`, baseline-aligned at uniform scale), `STIXTwoText` (`d□`, `imath`, `jmath`), and `STIXTwoMath` (`GreekLower`, `GreekUpper`, calligraphic/fraktur/blackboard `Symbols`) `<path>` extraction scoped to `assets/fonts/` (independent of OS font installation); verify all 547 `assets/icons/palette/<id>.svg` files are pure vector geometry with no `<text>` tags and render cleanly.
- [x] 2.3 Add two-pass visual bounding-box centering (`center_elements`) and composite sprite sheet rasterization (`2x` HiDPI `80x52` per cell with `1` DIP pad, `2016x1288` white RGBA PNG) plus `--check` integrity verification generating `assets/icons/palette-rendered/atlas.png` and `assets/icons/palette-rendered/atlas.json` (UV coordinates + source map + SVG/PNG SHA-256 hashes); verify `python3 scripts/generate_palette_icons.py --check` passes.
- [x] 2.4 Add palette SVG & atlas verification assertions to `tests/python/test_icons.py` checking 1-to-1 sync with `src/catalog/mod.rs` and valid RGBA atlas dimensions; verify `python3 -m unittest tests/python/test_icons.py` passes.

## 3. Desktop Palette Atlas Integration

- [x] 3.1 Embed `assets/icons/palette-rendered/atlas.png` and `atlas.json` in `desktop/src/app/palette_ui.rs` (and `mod.rs`), replacing runtime `build_palette_atlas` / `build_mask_atlas` font rasterization with a single shared `PaletteAtlas` loaded with `TextureOptions::LINEAR` and tinted via `ThemeTokens::icon_tint`.
- [x] 3.2 Replace the `"⋯"` text button for Custom Matrix in `desktop/src/app/palette_ui.rs` with `self.ui_icon(ui.ctx(), "dots", UI_ICON_SIZE, icon_color)` while keeping the `46x32` button size and `"Custom matrix"` tooltip/accessibility label.
- [x] 3.3 Add a unit test in `desktop/src/palette.rs` verifying every `PALETTE` entry resolves in the embedded atlas metadata with valid non-zero UV bounds, and update palette atlas assertions in `desktop/src/app/tests/batch_settings_tests.rs` and `desktop/src/app/tests/layout_theme_tests.rs` to inspect the shared palette atlas and `dots` matrix button; verify `cargo test -p eqntide-desktop` passes.

## 4. End-to-End Verification

- [x] 4.1 Run `python3 scripts/render_icons.py --check`, `python3 scripts/generate_palette_icons.py --check`, `python3 -m unittest tests/python/test_icons.py`, and `cargo test --workspace` and verify all checks pass without regressions.
