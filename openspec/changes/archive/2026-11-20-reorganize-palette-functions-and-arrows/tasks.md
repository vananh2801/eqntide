# Tasks

## 1. Synchronize Function Registry and Conversion Pipelines

- [x] 1.1 Unify `FUNCTIONS` in `src/symbols/functions.rs` to cover the full combined set of named functions (`sin`, `cos`, `tan`, `cot`, `sec`, `csc`, `arcsin`, `arccos`, `arctan`, `arccot`, `arcsec`, `arccsc`, `sinh`, `cosh`, `tanh`, `coth`, `sech`, `csch`, `arsinh`, `arcosh`, `artanh`, `arcoth`, `tg`, `ctg`, `cotg`, `cosec`, `log`, `ln`, `lg`, `exp`, `lim`, `limsup`, `liminf`, `"lim sup"`, `"lim inf"`, `max`, `min`, `sup`, `inf`, `det`, `dim`, `ker`, `hom`, `deg`, `arg`, `gcd`, `lcm`, `sgn`, `mod`, `Pr`, `Re`, `Im`) and verify `cargo test -p eqntide symbols` passes.
- [x] 1.2 Align `src/latex_extensions.rs`, `src/mathml_to_latex.rs`, `src/omml/encode.rs` (`KNOWN_FUNCTIONS`), and `src/mtef/encode.rs` (`is_limit_function` / `TF_FUNCTION` encoding) so every named function parses cleanly from LaTeX (including `\limsup`/`\liminf` without losing upright/limit recognition), serializes canonically via macro or `\operatorname{...}`, and roundtrips through OMML and MTEF; add unit tests covering all newly aligned functions and verify `cargo test --lib` passes.

## 2. Reorganize Catalog Groups and Functions Palette

- [x] 2.1 Add `("Functions", "Functions")` to `GROUPS` in `src/catalog.rs`, move `("lim", ...)`, `("min", ...)`, and `("max", ...)` from `Scripts` to `Functions`, and populate `Functions` templates and `COMMANDS` with all 45 slot-free named mathematical functions (`\sin`..`\Pr` and `\operatorname{...}` entries, excluding `\Re`/`\Im` kept in `Symbols`). Update `catalog_has_unique_ids_and_twenty_nonempty_groups` (21 groups) and verify `cargo test -p eqntide catalog` passes.

## 3. Render New Sidebar Icons and Update Desktop Sidebar & Palette

- [x] 3.1 Register `"math-sin"` and `"arrow-autofit-right"` in `DESKTOP_ICONS` in `scripts/render_icons.py`, run `python3 scripts/render_icons.py` to generate the `48x48` PNG assets and update `assets/icons/rendered/manifest.json`, and verify `python3 scripts/render_icons.py --check` exits `0`.
- [x] 3.2 Register `"math-sin"` and `"arrow-autofit-right"` in `tabler_icon_bytes` in `desktop/src/app.rs`, expand `UI_GROUPS` in `desktop/src/palette.rs` to 14 categories (adding `Functions` with `math-sin` and splitting `Arrows` with `arrows-left-right` from `LabeledArrows` with `arrow-autofit-right`), and remove redundant double vertical padding in `sidebar()` (`desktop/src/app.rs`) while keeping `SIDEBAR_BUTTON_HEIGHT = 42.0`. Update `ui_groups_cover_every_catalog_group_and_render_visible_icons` (14 groups) and verify `cargo test -p eqntide-desktop` passes across `1.0`, `1.25`, and `1.5` DPI at `1056x560`.

## 4. Integration and Roundtrip Verification

- [x] 4.1 Run `./tests/run_all.sh --quick` (or workspace `cargo test` + desktop UI + export verification) and verify every `Functions`, `Arrows`, and `LabeledArrows` palette entry renders a non-empty thumbnail, exports cleanly to LaTeX/OMML/MTEF/EMF, and fits in the `1056x560` desktop layout without clipping or vertical sidebar scrolling.
