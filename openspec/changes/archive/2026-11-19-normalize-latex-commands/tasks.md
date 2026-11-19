# Tasks

## 1. OMML/MTEF Operator Classification & Internal Normalization

- [x] 1.1 Update `OMML_OPERATOR_CHARS` / `OMML_OPERATOR_ALIASES` in `src/symbols/operators.rs` (used by `src/omml/decode.rs`) to recognize `∖` (`U+2216`) and missing set/logic/arrow operator characters from `catalog::SYMBOLS` (`tag == "mo"`), and verify with a unit test that `<m:t>∖</m:t>` decodes to `<mo>∖</mo>`.
- [x] 1.2 Ensure `src/latex_extensions.rs` normalizes degree superscripts (`^\circ`, `^{\circ}`) into `<mo>°</mo>` (`U+00B0`) identically to `\degree` and `°` while keeping standalone `\circ` as `∘` (`U+2218`), and verify via unit tests that `90^\circ`, `90\degree`, and `90°` produce identical MathML, OMML, and MTEF/OLE output.

## 2. Permissive LaTeX Alias & Standard Syntax Parsing

- [x] 2.1 Expand alias and symbol handling in `src/service.rs` and `src/latex_extensions.rs` so both short and explicit standard forms (`\le`/`\leq`, `\ge`/`\geq`, `\ne`/`\neq`, `\to`/`\rightarrow`, `\gets`/`\leftarrow`, `\implies`/`\Rightarrow`, `\iff`/`\Leftrightarrow`, `\land`/`\wedge`, `\lor`/`\vee`, `\lnot`/`\neg`, `\dots`/`\ldots`, `\smallsetminus`, `\varnothing`, `\lvert`/`\rvert`/`\lVert`/`\rVert`, `\backslash`) parse cleanly to canonical MathML, and verify with parser unit tests.
- [x] 2.2 Support standard TeX/LaTeX structural syntax alongside legacy forms in `src/latex_extensions.rs`: `{\displaystyle ...}` / `{\textstyle ...}` (alongside `\displaystyle{...}` / `\textstyle{...}`), `{a \atop b}` / `\genfrac{}{}{0pt}{}{a}{b}` / `\binom` / `\dbinom` / `\tbinom` (alongside `\atopfrac`), and verify both canonical and legacy inputs produce identical MathML in `service::tests`.

## 3. Canonical Explicit LaTeX Serialization (`mathml_to_latex`)

- [x] 3.1 Update `map_operator` and `format_operator` in `src/mathml_to_latex.rs` to normalize `<mo>\</mo>` and `<mo>∖</mo>` to `" \\setminus "`, emit explicit long forms (`\leq`, `\geq`, `\neq`, `\rightarrow`, `\leftarrow`, `\Rightarrow`, `\Leftrightarrow`, `\ldots`), emit `^\circ` for `"°"`, emit `^{(4)}` for legacy `"⁗"`, and apply consistent surrounding spacing for binary/relational catalog operators; update and expand unit tests in `src/mathml_to_latex.rs`.
- [x] 3.2 Update structural, spacing, Greek, and function serialization in `src/mathml_to_latex.rs`: map standard `<mspace>` widths to `\,`, `\:`, `\;`, `\!`, `\enspace`, `\quad`, `\qquad` (preserving `\hspace{...}` for non-standard widths), emit `{\displaystyle ...}` / `{\textstyle ...}`, emit `\binom{n}{k}` for parenthesized `linethickness="0"` fractions and `{n \atop k}` when bare, preserve Greek `var*` distinctions without `\Alpha`–`\Chi`/`\omicron`, and sync `is_named_function` with `src/symbols/functions.rs` (using `\operatorname{...}` for non-builtin functions); update and verify unit tests in `src/mathml_to_latex.rs`.

## 4. Catalog, Palette Templates & Desktop Command Suggestions

- [x] 4.1 Update `src/catalog.rs` templates and `COMMANDS` (`\,`, `\:`, `\;`, `\quad`, `\qquad`, `\overline{\overline{#?}}`, `\underline{\underline{#?}}`, `\binom{#?}{#?}`, explicit `\leq`/`\geq`/`\neq`/`\rightarrow`/`\Rightarrow`/`\Leftrightarrow`/`\ldots`), remove quadruple-prime (`qprime` / `quadruple-prime`) from catalog/palette, keep MathML/OMML-only extension entries (`\enclose`, `\tsfrac`, `\sfrac`, `\prescript`, `\overaccent`, `\underaccent`, etc.), and preserve `Symbols` palette layout invariants; verify `catalog::tests` and `desktop::commands::tests` pass.
- [x] 4.2 Update any affected assertions in `desktop/src/palette.rs`, `desktop/src/app.rs`, and `tests/` (`conversion_matrix.rs`, `export_verification.rs`) to align with the new canonical explicit LaTeX output while keeping legacy extension roundtrip tests passing.

## 5. End-to-End Verification

- [x] 5.1 Run `cargo test --workspace` and verify all unit, catalog, desktop, and cross-conversion roundtrip tests pass without regressions.
