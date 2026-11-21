# Design

## Context

See `proposal.md` for motivation and scope. The change addresses two gaps that dropped spaces and broke text spans across formats:

1. **LaTeX `\text{...}`**: `src/latex_extensions.rs` currently leaves `\text{...}` untouched so `parse_standard` delegates it to `latex2mathml` 0.2.3. In `latex2mathml`, `Lexer::next_token` skips all ASCII whitespace and `Parser::parse_text` stops on non-`Letter` tokens (digits, punctuation, operators, escaped characters). Meanwhile, `src/mathml_to_latex.rs` (`format_text`) calls `.trim()`, stripping boundary spaces and dropping space-only `<mtext>` (including `\u{00A0}` decoded from MTEF `CHAR(TF_TEXT, 0x20)`).
2. **OMML → MathType text/unit spaces**:
   - `src/omml/decode.rs` (`handle_r`) only converts Unicode math spaces (`U+2002`..`U+200A`, `U+205F`) to `<mspace>` and only promotes `<m:sty m:val="p"/>` to `<mtext>` when `has_foreign_letters(&text) || text.trim().contains(' ')`. Runs with leading/trailing spaces (`" cm"`, `"5 "`) or space-only runs (`" "`) fall back to `text_node()` (which trims) or `<mi> </mi>` (which `enc_mi` and `layout/row.rs` strip).
   - `src/omml/encode.rs` (`encode_run`) emits `<m:sty m:val="p"/>` for `<mtext>` without `<m:nor/>`.
   - `src/mtef/encode.rs` (`enc_mtext`) and `src/render/layout/row.rs` skip `<mtext>` when `trim().is_empty()`, and `row.rs` routes single-non-space-char `<mtext>` (such as `"m "`) into `atoms_mo` (losing both `Face::Text` and the trailing space).

## Goals / Non-Goals

**Goals:**
- Handle `\text{...}` natively across all nesting contexts (`\frac`, `\cases`, `\overset`, scripts, etc.) with balanced braces and exact whitespace/symbol preservation.
- Preserve spaces inside text, unit runs, and standalone space runs when converting Word/WPS OMML to MathType (MTEF + EMF) and when roundtripping `<mtext>` through OMML, MTEF, and LaTeX.
- Keep ordinary math-mode variables and numbers (`<mi>`, `<mn>`) free of unwanted whitespace artifacts.
- Let users confirm a complete text command and move between Text and Math on the canvas with a theme-aware caret.

**Non-Goals:**
- Preserving arbitrary spaces typed directly inside math-mode identifier or number runs (`a b` without `<m:nor/>` / `<m:sty m:val="p"/>`) as separate MathType spacing glyphs.
- Supporting rich inline formatting commands (`\textbf`, `\textit`, nested math `$...$`) inside `\text{...}` beyond plain text unescaping.

## Decisions

### 1. Native `\text{...}` parsing via marker substitution in `src/latex_extensions.rs`
- **Choice**: Handle `"text"` inside `Scanner::native` (removing `"text"` from the passthrough bypass at `parse_at`, while keeping `"operatorname"` passthrough) and emit `MathNode::leaf("mtext", &decoded)`.
- **Rationale**: `parse_at` already substitutes native nodes with `\text{EQNTIDENATIVE...}` markers (pure ASCII letters, no whitespace/digits/punctuation) before `ScriptRewriter` and `parse_standard`, then replaces markers back. Reusing this mechanism handles `\text{...}` at top level, inside fractions/matrices/cases, inside native extensions (`\overset`, `\boxed`), and with attached scripts (`\text{a}_1`) with zero changes to `latex2mathml`.
- **Unescaping**: Decode `\{`, `\}`, `\_`, `\&`, `\%`, `\$`, `\#`, `\\` → `\`, `\textbackslash{}` / `\textbackslash` → `\`, `\ ` → `' '`, and `~` → `\u{00A0}` (mirroring `escape_latex_text` in `src/mathml_to_latex.rs`), while preserving all literal spaces, digits, punctuation, and UTF-8 characters.
- **Alternative considered**: Post-processing `latex2mathml` output or replacing `latex2mathml` entirely. Rejected because `latex2mathml` fails before returning XML when `\text{...}` contains numbers or punctuation.

### 2. Preserve `<mtext>` spaces across `enc_mtext`, `layout/row.rs`, and `format_text`
- **Choice**:
  - In `src/mtef/encode.rs` (`enc_mtext`): map `' '` and `'\u{00A0}'` to `char_record(TF_TEXT, 0x20)` even when the entire run consists of spaces.
  - In `src/render/layout/row.rs`: allow `<mtext>` containing `' '` or `'\u{00A0}'` past the empty-token skip; in the `"mtext"` arm, only delegate to `atoms_mo` when `kid.text.chars().count() == 1` (no surrounding spaces), otherwise lay out `&kid.text` using `Face::Text` (or `Face::VectorMatrix` when bold).
  - In `src/mathml_to_latex.rs`: join consecutive `<mtext>` siblings within the same row, then serialize a single `\text{...}` span. Normalize `'\u{00A0}'` to `' '` and preserve boundary, repeated, and space-only whitespace. Math tokens between spans stay outside the text argument.
- Keep explicit text spaces as `<mtext>` so MathType can encode them as text-mode spaces (`TF_TEXT 0x20`). Canvas users can create a space-only span with `\text{ }`.

### 3. OMML decoding & `<m:nor/>` encoding for text/unit spaces
- **Choice**:
  - In `src/omml/encode.rs` (`encode_run`): emit `<m:nor/>` inside `<m:rPr>` alongside `<m:sty m:val="p"/>` (or the resolved bold style). Add `xml:space="preserve"` to `<m:t>` when text has boundary whitespace.
  - In `src/omml/decode.rs` (`handle_r`):
    - Emit `<mtext>` when `is_nor` is true, **or** when `scr.is_none()` and `sty == Some("p")` and (`has_foreign_letters(&text)` or `text.contains(' ')` or `text.contains('\u{00A0}')`) — checking `text` without `.trim()` so leading/trailing unit spaces like `" cm"` and `"m "` keep their spaces.
    - Also emit `<mtext>` when a run consists **solely** of ASCII/NBSP whitespace (`text.chars().all(|c| c == ' ' || c == '\u{00A0}')`), e.g. `<m:r><m:t xml:space="preserve"> </m:t></m:r>` placed between `5` and `cm`.
- **Rationale**: Restricting non-`nor` / non-`sty="p"` space preservation to standalone space-only runs avoids accidentally keeping stray spacing inside math-mode runs while capturing real Word/WPS runs where a typed Space sits between a number and a unit.
- **Alternative considered**: Stripping `.trim()` in `text_node()` for all runs. Rejected because that would leak spaces into `<mi>`, `<mn>`, and `<mo>`, breaking operator classification (`is_operator(" + ")`).

### 4. Desktop canvas text entry and caret transitions

- The command catalog exposes `\text`. The canvas keeps `\text{...}` in the command buffer until Enter, including after the final `}`. Enter on a missing or incomplete argument reports an invalid command and preserves both the buffer and equation.
- The editor treats the Math insertion point and the Text endpoint as separate horizontal caret stops at each edge of a text span. Inside text, arrows move by grapheme cluster. Text inside a structural slot has insertion points within that slot. Enter inside an existing text span puts the caret after it in Math mode.
- Text mode accepts literal input, including spaces, backslashes, and characters that otherwise invoke math shortcuts. The canvas ignores a typed space in Math mode.
- The canvas paints the caret with the preset's Math or Text color. Changing mode at the same position restarts the visible phase of the caret blink. The color itself signals the mode, with no underline drawn under the span.

## Risks / Trade-offs

- **Risk**: Existing `omml::encode` tests checking `sty` on `<mtext>` or `cross_conversion` assertions expecting no `<mtext>` on non-text formulas → **Mitigation**: Update/verify `omml::encode` tests (`default_token_styles_are_explicit_in_nested_equations` checks `sty` and remains valid with `<m:nor/>` added; `cross_conversion` tests check mathematical expressions without `<m:nor/>` or standalone space runs).
- **Risk**: Word OMML files might contain standalone `<m:r><m:t xml:space="preserve"> </m:t></m:r>` outside text/units if authored manually with extra spaces → **Mitigation**: In practice Word's Equation Editor strips syntactic spaces when building OMML, retaining standalone space runs only where explicit space was inserted; decoding them to `<mtext> </mtext>` matches visual Word rendering.
- **Risk**: `\text{...}` containing nested `$...$` math mode is not evaluated as math → **Mitigation**: Documented as a non-goal; `\text{...}` treats its argument as literal text per standard expectations for text labels/units.
