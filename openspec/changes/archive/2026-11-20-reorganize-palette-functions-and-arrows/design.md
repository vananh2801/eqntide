# Design

## Context

`src/catalog.rs` already defines `Arrows` (36 single-glyph symbols) and `LabeledArrows` (36 `\overset`/`\underset` arrow templates) as distinct canonical `GROUPS`, while `desktop/src/palette.rs` previously merged both under a single `Arrows` tile in `UI_GROUPS` (12 tiles total). Meanwhile, `\lim_{#? \to #?}`, `\min_{#?}`, and `\max_{#?}` lived in `Scripts`, and named mathematical functions such as `sin`, `cos`, `tan`, and `log` existed only in `COMMANDS` (canvas completion) and partially misaligned lists across `src/symbols/functions.rs`, `src/omml/encode.rs` (`KNOWN_FUNCTIONS`), `src/mathml_to_latex.rs`, and `src/latex_extensions.rs`.

Expanding the sidebar from 12 to 14 tiles creates 7 rows in the 2-column grid. At the minimum window height of `560px` (`MIN_WINDOW_SIZE = [1056.0, 560.0]`), after `app-status` (`32px`), `app-footer` (`56px`), `app-sidebar` inner margins (`16px`), and `sidebar-batch` (`130px`) + top `SPACE_MD` (`8px`), only `318px` remained, whereas 7 rows × `42px` + 6 gaps × `6px` require `330px`. However, `sidebar()` currently applies redundant vertical spacing (`ui.add_space(SPACE_MD)` inside an already-padded `app-sidebar`, plus top and bottom `SPACE_MD` inside `sidebar-batch` above `app-sidebar`'s bottom margin).

## Goals / Non-Goals

**Goals:**
- Introduce a canonical `Functions` group in `src/catalog.rs` containing the 3 moved placeholder templates (`\lim_{#? \to #?}`, `\min_{#?}`, `\max_{#?}`) and all 45 plain slot-free mathematical functions from the combined function inventory (48 items total).
- Split `Arrows` and `LabeledArrows` in `desktop/src/palette.rs` using the user-provided `math-sin.svg` and `arrow-autofit-right.svg` icons alongside `arrows-left-right.svg`.
- Fit all 14 sidebar tiles at `72x42` (no button height reduction) inside `1056x560` without vertical scrolling by removing redundant double margins in `sidebar()`.
- Synchronize the function name list and `\limsup`/`\liminf` handling across LaTeX parsing, MathML-to-LaTeX serialization, layout rendering, OMML encoding/decoding, and MTEF encoding/decoding.

**Non-Goals:**
- Adding argument placeholders to trigonometric/logarithmic functions (e.g., `\sin(#?)`), which would obstruct typing `\sin^2 x` or `\log_2 x`.
- Duplicating `\Re` (`ℜ`) and `\Im` (`ℑ`) into `Functions` (they remain in `Symbols` so `PALETTE` and `SYMBOLS` stay conflict-free).
- Changing `PALETTE_BUTTON_SIZE` (`46x32`) or `SIDEBAR_BUTTON_HEIGHT` (`42px`).

## Decisions

### 1. Keep `SIDEBAR_BUTTON_HEIGHT = 42.0` by removing redundant sidebar padding
In `desktop/src/app.rs` (`sidebar()`), `app-sidebar` already applies `Margin::symmetric(SPACE_LG, CONTENT_PADDING)` (`8px` top and bottom). Removing the extra leading `ui.add_space(theme::SPACE_MD)` (`8px`) and reducing `sidebar-batch` from `exact_size(130.0)` with symmetric `8px` top/bottom margins to `exact_size(120.0)` with `Margin { top: SPACE_SM as i8, bottom: 0, .. }` reclaims `18px` (`318px` -> `336px` available for the grid), fitting `7 * 42 + 6 * 6 = 330px` cleanly at `1056x560` across `1.0`, `1.25`, and `1.5` DPI.
- *Alternative considered*: Shrinking `SIDEBAR_BUTTON_HEIGHT` from `42px` to `38px`. Rejected because the user prefers preserving the `42px` tile height and removing redundant margin space instead.

### 2. Catalog representation of `Functions` (slot-free functions via `templates` vs `symbols`)
Store the 3 moved limit/extremum entries (`\lim_{#? \to #?}`, `\min_{#?}`, `\max_{#?}`) and the plain functions in `src/catalog.rs` using `templates` (with group `"Functions"`), using canonical LaTeX sources (`\sin`, `\cos`, ..., `\operatorname{arccot}`, etc.) so `PALETTE`, `ENTRIES`, and `COMMANDS` stay consistent without polluting `SYMBOLS` (which maps single-word control sequences to Unicode glyphs) or conflicting with `Re`/`Im`.
- *Alternative considered*: Adding functions to `SYMBOLS`. Rejected because `symbol(name)` is for single-glyph control words, whereas `\operatorname{...}` functions and `\lim_{#? \to #?}` templates must flow through `template(source)` -> `parse_latex_to_mathml`.

### 3. Unified function registry and LaTeX/OMML/MTEF parity
- Update `src/symbols/functions.rs` (`FUNCTIONS`) to be the single source of truth including all standard, inverse/hyperbolic, regional (`tg`, `ctg`, `cotg`, `cosec`), and algebraic (`hom`, `sgn`, `mod`, `lcm`, `Pr`, etc.) names, plus both `"limsup"`/`"liminf"` and `"lim sup"`/`"lim inf"` (or normalize `\limsup`/`\liminf` in `src/latex_extensions.rs` and recognize both forms).
- Ensure `src/latex_extensions.rs` parses every supported command cleanly—including commands `latex2mathml` may not recognize directly (such as `\lg`, `\coth`, `\gcd`, `\dim`, `\ker`, `\deg`, `\arg`, `\Pr` if any fail in `latex2mathml`, plus `\operatorname{...}`)—producing upright `<mi>` function nodes.
- Align `src/omml/encode.rs` (`KNOWN_FUNCTIONS`) with `FUNCTIONS` (adding `"sgn"` and spaced/unspaced limit names) and ensure `src/mtef/encode.rs` encodes all `FUNCTIONS` with `TF_FUNCTION` (including `"lim inf"`/`"lim sup"` or normalizing to `"liminf"`/`"limsup"`).
- Align `src/mathml_to_latex.rs` (`format_named_function` / `is_operatorname_function`) so standard TeX macros emit `\cmd ` and `amsmath`/custom names emit `\operatorname{name} `.
- *Alternative considered*: Leaving rare aliases (`tg`, `ctg`, `cotg`, `cosec`) out of the palette. Rejected per user request to include the complete function set from the codebase.

### 4. Sidebar group order and Tabler icon registration
Place `Functions` (`icons: &["math-sin"]`) right after `AccentsAndBars` (so the 14 tiles form 7 balanced rows: `Recent` / `Fractions`, `Scripts` / `Fences`, `Integrals` / `SumsAndBigOps`, `MatricesAndBoxes` / `AccentsAndBars`, `Functions` / `OperatorsAndRelations`, `Arrows` / `LabeledArrows`, `LogicSetsSymbols` / `Greek`), pairing `Arrows` (`arrows-left-right`) next to `LabeledArrows` (`arrow-autofit-right`). Register `"math-sin"` and `"arrow-autofit-right"` in `scripts/render_icons.py` (`DESKTOP_ICONS`) and `desktop/src/app.rs` (`tabler_icon_bytes`), then regenerate `assets/icons/rendered/` PNGs and `manifest.json`.

## Risks / Trade-offs

- **DPI rounding at `1.25` / `1.5` in `560px` height**: `330px` grid inside `336px` leaves `6px` margin at `560px` window height. → Verify with `all_sidebar_categories_and_batch_stay_visible_without_scrolling` across `dpi in [1.0, 1.25, 1.5]` and adjust `sidebar-batch` bottom/top padding by `1–2px` if fractional scaling clips by subpixels.
- **Duplicate `COMMANDS` entries for `lim`, `min`, `max` vs template forms**: `COMMANDS` already has `("lim", "Limit", "\\lim")`, `("min", "Minimum", "\\min")`, `("max", "Maximum", "\\max")`. Moving the template entries from `Scripts` to `Functions` keeps `ENTRIES` IDs (`Functions-lim`, etc.) unique while preserving command-popup completion for `\lim`, `\min`, `\max`. → Run `desktop::commands::tests::every_completion_is_an_insertable_template` and `catalog` uniqueness tests.
- **`latex2mathml` fallback gaps for lesser-known macros**: Some commands in `FUNCTIONS` (e.g., `\lg`, `\coth`, `\Pr`, or `\operatorname{...}` arguments) may behave differently under `latex2mathml 0.2.3`. → Test every single `Functions` entry through `parse_latex_to_mathml`, `render_interactive`, `mathml_to_omml_xml`, `omml_to_mathml`, `mathml_to_mtef_bytes`, `mtef_to_mathml`, and `to_latex`.
