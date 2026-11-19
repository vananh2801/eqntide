# Proposal

## Why

LaTeX parsing, serialization (`mathml_to_latex`), palette/autocomplete suggestions, and OMML/MTEF roundtrips currently diverge on several commands and produce non-standard or broken output—for example, MathType set-minus (`<mo>\</mo>`) serializes to raw `\`, Word OMML `∖` (`U+2216`) decodes as `<mi>∖</mi>` and falls back to literal Unicode `∖`, and `to_latex()` emits custom or inconsistent forms such as `\atopfrac`, `\qprime`, `\hspace{0.166667em}`, `\textstyle{...}`, `\Alpha`, and `\degree` instead of standard explicit LaTeX. Standardizing LaTeX input/output while preserving backward compatibility and MathML/OMML-only extensions will make the LaTeX source box predictable, valid, and consistent across all conversion paths.

## What Changes

- **Standardize `to_latex()` canonical output (explicit / long forms):**
  - Emit standard, explicit LaTeX commands wherever a standard TeX / `amsmath` / `amssymb` representation exists: `\leq`, `\geq`, `\neq`, `\rightarrow`, `\leftarrow`, `\Rightarrow`, `\Leftrightarrow`, `\ldots`, `\setminus` (including normalizing `<mo>\</mo>` and `<mi>∖</mi>`), `^\circ` (for degree), `'`, `''`, `'''` (and `^{(4)}` if legacy `⁗` appears; removing `\qprime` and quadruple-prime templates), `\,`, `\:`, `\;`, `\!`, `\enspace`, `\quad`, `\qquad` (for standard `<mspace>` widths), `{\displaystyle ...}` / `{\textstyle ...}`, `\binom{n}{k}` / `{n \atop k}` (instead of `\atopfrac`), `\overline{\overline{...}}` / `\underline{\underline{...}}`, `\operatorname{...}` for non-builtin upright functions (synced with `symbols/functions.rs`), and distinct Greek variants (`\epsilon`/`\varepsilon`, `\phi`/`\varphi`, `\theta`/`\vartheta`, `\pi`/`\varpi`, `\rho`/`\varrho`, `\sigma`/`\varsigma`) without non-standard `\Alpha`–`\Chi` or `\omicron`.
  - Normalize binary operator spacing in `mathml_to_latex` so operators falling back to the catalog (such as `\setminus`, `\oplus`, `\otimes`, `\land`, `\lor`) emit clean surrounding spacing consistent with `\cup`, `\cap`, and `\leq`.
- **Accept both standard and alias/legacy forms on LaTeX input & normalize internally:**
  - Accept short and long standard aliases on input (`\le`/`\leq`, `\ge`/`\geq`, `\ne`/`\neq`, `\to`/`\rightarrow`, `\gets`/`\leftarrow`, `\implies`/`\Rightarrow`, `\iff`/`\Leftrightarrow`, `\land`/`\wedge`, `\lor`/`\vee`, `\lnot`/`\neg`, `\dots`/`\ldots`, `\smallsetminus`, `\varnothing`, `\lvert`/`\rvert`/`\lVert`/`\rVert`, `\backslash`, `^\circ`/`\degree`/`°`, `{\textstyle ...}`/`\textstyle{...}`, `{\displaystyle ...}`/`\displaystyle{...}`) as well as legacy custom forms (`\atopfrac`, `\doubleoverline`, `\doubleunderline`, `\dprime`, `\trprime`, `\bar`).
  - Normalize equivalent inputs (such as `^\circ`, `\degree`, and `°`) to a single canonical representation across MathML, OMML, and MTEF/OLE.
- **Align OMML/MTEF operator decoding & desktop command suggestions:**
  - Include `∖` (`U+2216`) and missing set/logic/arrow operators in OMML operator classification (`OMML_OPERATOR_CHARS` / `OMML_OPERATOR_ALIASES`) so imported OMML equations decode them as `<mo>` rather than `<mi>`.
  - Update desktop command suggestions and palette source templates to prefer standard explicit LaTeX commands while keeping MathML/OMML/MathType-only extensions (`\enclose`, `\tsfrac`, `\sfrac`, `\prescript`, `\overaccent`, `\underaccent`, custom under-accents/harpoons, and non-standard MathType/Unicode symbols with no LaTeX equivalent).

## Capabilities

### New Capabilities
- `latex-command-standardization`: Defines canonical explicit LaTeX serialization, permissive standard + legacy alias parsing, canonical internal normalization (including set-minus and degree across MathML/OMML/MTEF), and retention of MathML/OMML-only extension commands.

### Modified Capabilities
- None (existing `desktop-latex-typing`, `desktop-palette-navigation`, and `omml-font-compatibility` requirements remain intact).

## Impact

- **Core library (`src/`):** `src/mathml_to_latex.rs`, `src/service.rs`, `src/latex_extensions.rs`, `src/catalog.rs`, `src/symbols/operators.rs`, `src/omml/decode.rs`, and `src/mtef/decode.rs` (only where needed for canonical normalization).
- **Desktop (`desktop/`):** `desktop/src/commands.rs`, `desktop/src/palette.rs`, and tests in `desktop/src/app.rs` referencing catalog/canonical serialized output.
- **Tests:** Existing roundtrip and serialization assertions in `src/catalog.rs`, `src/mathml_to_latex.rs`, `src/service.rs`, and `tests/` that previously asserted `\atopfrac`, `\le`/`\ge`/`\ne`/`\to`, `\degree`, `\hspace{...}`, or `\textstyle{...}`.
