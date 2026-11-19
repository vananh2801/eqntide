# Design

## Context

LaTeX handling is currently spread across five layers with diverging rules (see `proposal.md` for motivation):
1. `src/catalog.rs`: single source of truth for palette entries, `SYMBOLS`, and `desktop/src/commands.rs` autocomplete.
2. `src/service.rs`: `latex_aliases()` only rewrites `le`→`leq`, `ge`→`geq`, `ne`→`neq` before delegating to `latex2mathml 0.2.3`.
3. `src/latex_extensions.rs`: intercepts native commands/symbols (`catalog::symbol`, `big_operator`, `accent_spec`, `\frac`/`\atopfrac`/`\tsfrac`, `\enclose`, `\displaystyle`/`\textstyle`, `\hspace`, etc.) before `latex2mathml` sees them.
4. `src/mathml_to_latex.rs`: serializes `MathNode` using ad-hoc `map_operator`, `map_greek_or_symbol`, `is_named_function`, and `catalog::SYMBOLS` fallback.
5. `src/omml/decode.rs` (`src/symbols/operators.rs`) & `src/mtef/decode.rs`: produce `<mi>`/`<mo>` tokens that feed `mathml_to_latex`.

## Goals / Non-Goals

**Goals:**
- Make `mathml_to_latex` emit consistent, explicit, valid standard LaTeX (`\leq`, `\geq`, `\neq`, `\rightarrow`, `\Rightarrow`, `\Leftrightarrow`, `\setminus`, `^\circ`, `\,`–`\qquad`, `{\displaystyle ...}`, `\binom`/`\atop`, `\operatorname{...}`, etc.).
- Accept both short/alias/legacy inputs and canonical inputs in `parse_latex_to_mathml`, converging to one canonical internal MathML/OMML/MTEF form.
- Fix `<mo>\</mo>` and OMML `<m:t>∖</m:t>` roundtrips so set-minus never leaks as raw `\` or raw `∖` in LaTeX.
- Keep all MathML/OMML/MathType-only extensions working losslessly.

**Non-Goals:**
- Replacing `latex2mathml 0.2.3` with a full custom TeX parser; extensions and pre-normalization remain scoped to `latex_extensions.rs` and `service.rs`.
- Changing existing MathType/OMML binary/XML wire formats for unrelated constructs or breaking `desktop-latex-typing` buffered input mechanics.

## Decisions

### 1. Canonical Degree Representation (`^\circ` ↔ `°` `U+00B0`)
- **Decision:** Use `<mo>°</mo>` (`U+00B0`) as the canonical MathML representation for degree, normalize `^\circ` / `^{\circ}` (when `^` applies `\circ`) and `\degree` / `°` into `<mo>°</mo>` during LaTeX parsing, and serialize `<mo>°</mo>` as `^\circ` in `mathml_to_latex.rs`. Keep inline `\circ` (`∘`, `U+2218`, e.g., `f \circ g`) untouched.
- **Rationale:** MTEF already encodes/decodes `0x00B0` as `<mo>°</mo>` (`src/mtef/charmap.rs`, `src/mtef/decode.rs`), OMML already represents it cleanly as `°`, and the renderer already draws `°`. Normalizing `^\circ` on input to `<mo>°</mo>` guarantees `90^\circ`, `90\degree`, and `90°` produce identical MathML, OMML, and MTEF/OLE bytes while outputting `90^\circ` in LaTeX.
- **Alternative considered:** Representing degree as `<msup><mrow/><mo>∘</mo></msup>`. Rejected because it creates a superscript box on the structural canvas and diverges from MTEF/OMML's existing `0x00B0` (`°`) encoding.

### 2. Set-Minus Normalization Across MTEF, OMML, and LaTeX
- **Decision:**
  - Add `∖` (`U+2216`) and missing set/logic/arrow operator characters from `catalog::SYMBOLS` (with `tag == "mo"`) to OMML operator recognition in `src/symbols/operators.rs` / `src/omml/decode.rs`, so OMML `<m:t>∖</m:t>` decodes as `<mo>∖</mo>`.
  - In `mathml_to_latex.rs`, map both `"∖"` and `"\\"` in `map_operator` to `" \\setminus "` (and map `\smallsetminus` / `\backslash` in LaTeX input appropriately: `\smallsetminus` → `∖` or `∖`, `\backslash` → `∖` when operator, keeping `\textbackslash{}` inside `\text{...}`).
- **Rationale:** Matches the existing OMML normalization rule in `src/omml/encode/normalization.rs` (which already converts `<mo>\</mo>` to `∖`), closing the remaining leak in `mathml_to_latex` and `omml/decode`.
- **Alternative considered:** Changing `mtef/decode.rs` `0x005C` directly from `\` to `∖`. Doing both (handling `<mo>\</mo>` in `mathml_to_latex.rs` and optionally normalizing `<mo>\</mo>` in MTEF decode) is safe; keeping `<mo>\</mo>` handled in `mathml_to_latex.rs` also protects hand-crafted or legacy MathML.

### 3. Explicit Long-Form Output + Bidirectional Alias Table
- **Decision:**
  - In `mathml_to_latex.rs`, switch `map_operator` to emit explicit long forms (`" \\leq "`, `" \\geq "`, `" \\neq "`, `" \\rightarrow "`, `" \\leftarrow "`, `" \\Rightarrow "`, `" \\Leftrightarrow "`, `" \\setminus "`) and `\ldots ` (or `\dots ` consistently; `\ldots` for `…`, `\cdots` for `⋯`, `\vdots` for `⋮`, `\ddots` for `⋱`).
  - Centralize operator spacing in `mathml_to_latex.rs`: catalog `"mo"` operators that are binary/relational (not punctuation/primes/fences) get leading+trailing space handling so `A \setminus B` formats consistently with `A \cup B`.
  - In `service.rs` (`latex_aliases`) and `latex_extensions.rs`, accept short aliases (`\le`, `\ge`, `\ne`, `\to`, `\gets`, `\implies`, `\iff`, `\land`, `\lor`, `\lnot`, `\dots`) alongside long forms (`\leq`, `\geq`, `\neq`, `\rightarrow`, `\leftarrow`, `\Rightarrow`, `\Leftrightarrow`, `\wedge`, `\vee`, `\neg`, `\ldots`) plus missing `amssymb`/fence commands (`\smallsetminus`, `\varnothing`, `\lvert`, `\rvert`, `\lVert`, `\rVert`, `\backslash`).
- **Alternative considered:** Stripping short aliases from autocomplete completely. Better to keep short aliases parseable and optionally keep common ones in `commands` (pointing to canonical sources) or prioritize canonical names in `COMMANDS` so Tab completion produces standard explicit commands.

### 4. Standard TeX/LaTeX Syntax for Structural & Legacy Constructs
- **Decision:**
  - **Binomial / zero-thickness fraction:** In `mathml_to_latex.rs`, detect parenthesized `<mfrac linethickness="0">` (`<mrow><mo>(</mo><mfrac linethickness="0">...</mfrac><mo>)</mo></mrow>` or `<mfenced open="(" close=")"><mfrac linethickness="0">...</mfrac></mfenced>`) and emit `\binom{n}{k}`; for bare `<mfrac linethickness="0">`, emit `{n \atop k}` (and ensure `latex_extensions.rs` parses `{a \atop b}`, `\genfrac{}{}{0pt}{}{a}{b}`, `\binom`, `\dbinom`, `\tbinom`, and legacy `\atopfrac`).
  - **Display/text styles:** Parse both `{\displaystyle ...}` / `{\textstyle ...}` (scoping inside groups in `latex_extensions.rs`) and legacy `\displaystyle{...}` / `\textstyle{...}`; serialize `<mstyle displaystyle="...">` as `{\displaystyle ...}` / `{\textstyle ...}`.
  - **Spaces:** Map standard `<mspace width="..."/>` widths (`0.166667em` → `\,`, `0.222222em` → `\:`, `0.277778em` → `\;`, `-0.166667em` → `\!`, `0.5em` → `\enspace`, `1em` → `\quad`, `2em` → `\qquad`) in `mathml_to_latex.rs` and update `catalog.rs` palette space templates accordingly; retain `\hspace{...}` for custom widths such as `0.083333em`.
  - **Primes:** Remove `quadruple-prime` / `qprime` from `catalog.rs` and `latex_extensions.rs` (higher-order derivatives use standard `f^{(4)}` via superscript), map legacy `⁗` (`U+2057`) in `mathml_to_latex.rs` to `^{(4)}`, and keep `'`, `''`, `'''` (`\dprime`/`\trprime` as input aliases).
  - **Greek & Functions:** In `mathml_to_latex.rs`, check exact `SYMBOLS` or distinct `map_greek_or_symbol` entries so `\varepsilon` (`ε`) and `\epsilon` (`ϵ`), `\varphi` (`φ`) and `\phi` (`ϕ`), etc. never collapse; drop non-standard `\Alpha`–`\Chi` and `\omicron` from `SYMBOLS` (or map them to plain Latin letters on serialization while keeping parse fallback if needed). Sync `is_named_function` with `src/symbols/functions.rs`: emit `\...` for standard TeX functions (`\sin`..`\tanh`, `\lim`, `\limsup`, `\liminf`, `\lg`, `\Pr`, etc.) and `\operatorname{...}` for non-builtin upright functions (`sgn`, `lcm`, `arccot`, `tg`, `cotg`).

### 5. Retaining MathML/OMML-Only Extensions
- **Decision:** Keep `\enclose`, `\tsfrac`, `\sfrac`, `\prescript`, `\overaccent`, `\underaccent`, custom under/harpoon accents (`\underdot`..`\underddddot`, `\underhat`, `\undercheck`, `\underbreve`, `\underring`, `\overleftharpoon`..), and MathType/Unicode-only symbols (`\estimates`, `\neswarrow`, `\nwsearrow`, `\lambdabar`, `\carriagereturn`, `\notni`, `\nsubset`, `\iddots`, `\smallcirc`, `\rectangle`, `\parallelogram`) intact in parser, serializer, and catalog.

## Risks / Trade-offs

- **[Risk] Existing unit/integration tests assert old serialized strings (`\atopfrac`, `\le`, `\ge`, `\ne`, `\to`, `\degree`, `\hspace{...}`)** → Mitigation: audit and update assertions in `src/catalog.rs`, `src/mathml_to_latex.rs`, `src/service.rs`, `desktop/src/app.rs`, and `tests/` to match the new canonical explicit LaTeX output while adding explicit tests for legacy input aliases.
- **[Risk] `^\circ` ambiguity if an equation genuinely meant "raise base to ring operator `\circ`"** → Mitigation: only normalize `^` immediately followed by `\circ` or `{\circ}` to `<mo>°</mo>`; standalone `\circ` remains `∘` (`U+2218`).
- **[Risk] `desktop-palette-navigation` specifies 49 `Symbols` items** → Mitigation: keep the `Symbols` palette item count/structure intact when refining symbol command names (e.g., prime templates live in `Embellishments`, while `GreekUpper`/`GreekLower` and `Symbols` keep their visible palette items).
