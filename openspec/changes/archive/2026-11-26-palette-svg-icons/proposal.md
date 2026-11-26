# Proposal

## Why

The desktop editor's top 3-row palette previously rasterized all 547 symbols and templates at runtime from `STIXTwoMath` via `render_alpha_at`, causing uneven visual scale and inconsistent stroke thickness whenever tall or wide templates (such as `4x4` matrices, nested roots/fractions, or labeled arrows) were shrunk to fit the `40x26` icon budget. Replacing runtime font rasterization with a generated, Tabler-consistent `40x26` SVG icon set and pre-rasterized sprite atlas gives every palette button uniform stroke weight, centered balance, and visual alignment with the Tabler sidebar icons.

## What Changes

- Add a deterministic Python generator script that produces Tabler-style, visually centered `40x26` SVGs for all 547 entries in `src/catalog/mod.rs` (monoline geometric strokes for structural templates and math operators/arrows/sets/logic/relations, smooth circular-arc parentheses, clear single- and double-shaft arrowheads, thin muted solid-border `□` slot rectangles, full hollow-square grids for all matrix sizes including `3x3` and `4x4`, `Nunito-Regular` baseline-aligned `<path>` outlines for `Functions`, and `STIXTwoText`/`STIXTwoMath` `<path>` outlines for differentials, `Greek`, and calligraphic/blackboard/fraktur symbols).
- Add a composite atlas rasterizer (`scripts/generate_palette_icons.py`) that centers and renders all 547 palette SVGs into a committed white-RGBA HiDPI sprite atlas PNG (`assets/icons/palette-rendered/atlas.png`, `2016x1288` px at `2x`, plus UV `atlas.json` + `--check` integrity verification) separate from third-party `assets/icons/tabler-icons/`, alongside Tabler `dots` (`dots.svg` / `dots.png`).
- Update `desktop/src/app/palette_ui.rs` to render palette buttons from the pre-rasterized linear-filtered sprite atlas tinted by `ThemeTokens::icon_tint`, and replace the text `"⋯"` Custom Matrix button with the Tabler `dots` icon, removing runtime palette font rasterization (`build_palette_atlas` / `build_mask_atlas`).
- Keep `eqntide::render` (`render_preview`, `render_interactive_alpha_scaled`, `render_svg`, `render_emf`, and Equation Canvas / DOCX exports) unchanged.

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- `desktop-palette-navigation`: Update palette button icon requirements from font-based math preview scaling (`render_alpha_at`) to Tabler-consistent, centered `40x26` SVG-derived icon assets (covering all 547 catalog items plus the `dots` custom matrix button) with uniform stroke weight, thin/muted solid placeholder boxes, circular-arc parentheses, clear double-shaft arrowheads, full-square matrix grids, and `Nunito`/`STIXTwoText`/`STIXTwoMath` letterforms.

## Impact

- `scripts/`: New palette SVG generator & atlas rasterizer/verifier scripts (`scripts/generate_palette_icons.py`, `scripts/palette_svg_builders.py`) and `"dots"` entry in `scripts/render_icons.py`.
- `assets/icons/palette/` and `assets/icons/palette-rendered/`: Committed generated SVGs (547 files), sprite atlas PNG (`2016x1288`), UV map & SHA-256 manifest JSON; plus `dots.svg` / `dots.png` in the Tabler icon set.
- `desktop/src/app/palette_ui.rs`, `desktop/src/app/mod.rs`, `desktop/src/palette.rs`, and `desktop/src/app/tests/`: Load and draw from the shared pre-rasterized palette atlas and `dots` UI icon; update palette atlas tests and `tests/python/test_icons.py`.
- `src/render/`: No production rendering changes; `render_alpha_at` remains intact for any existing callers/library tests.
