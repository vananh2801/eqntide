# Proposal

## Why

LaTeX `\text{...}` breaks or drops spaces whenever the text contains spaces, digits, punctuation, or escaped symbols because it delegates to `latex2mathml`'s `parse_text`, and OMML equations containing typed spaces in text or unit spans (e.g., `5 cm`, `"khi "`, `<m:nor/>`, or standalone space runs) lose their spaces when converted to MathType OLE/MTEF and EMF previews. Fixing both ensures natural-language text, unit labels, and explicit text spaces survive reliably across LaTeX, MathML, OMML, MTEF, and rendered previews.

## What Changes

- Parse `\text{...}` natively in `latex_extensions` (with balanced braces, preserved leading/interior/trailing spaces, digits, punctuation, Unicode/Vietnamese text, and TeX text unescaping matching `escape_latex_text`) instead of delegating to `latex2mathml`, and add `\text` to the desktop command catalog.
- Require a complete `\text{...}` argument and Enter to insert text from the desktop canvas command buffer. At either end of a text span, the arrow keys can switch between Math and Text at the same position. The caret color follows the selected canvas palette, and Space inserts a character only in Text mode.
- Preserve spaces inside `<mtext>` (including space-only `<mtext>` and single-character + space units like `"m "`) through `to_latex`, MTEF encoding (`CHAR(TF_TEXT, 0x20)`), and layout/EMF rendering (`Face::Text`).
- Serialize consecutive MathML text tokens as one `\text{...}` span so MathType character records recover as a continuous label.
- Update OMML decoding (`handle_r`) so spaces in `<m:nor/>` runs, boundary/interior spaces in `<m:sty m:val="p"/>` text/unit runs, and standalone space-only `<m:r><m:t>` runs decode into `<mtext>` instead of being trimmed or dropped as `<mi>`, while keeping variable/number math-mode runs unchanged.
- Emit `<m:nor/>` alongside `<m:sty m:val="p"/>` when encoding `<mtext>` runs to OMML so short or space-only `<mtext>` tokens roundtrip cleanly through Word/WPS OMML.

## Capabilities

### New Capabilities
- `text-and-unit-space-fidelity`: Preserves text-mode spaces, `<m:nor/>` normal-text runs, and unit-span spaces end-to-end across OMML decode/encode, MTEF encoding, and layout/EMF rendering without altering math-mode variables or numbers.

### Modified Capabilities
- `latex-command-standardization`: Add native `\text{...}` parsing, continuous text serialization, and explicit desktop canvas entry and editing in Text mode.
- `desktop-latex-typing`: Require a braced argument for `\text` and define Space behavior for command names, buffered arguments, and Text mode.
- `desktop-system-equations`: Give leaving an existing text span priority over Enter's row-editing behavior.

## Impact

- `src/latex_extensions.rs`: Native `\text{...}` scanner/unescaper.
- `src/catalog.rs`: Add `text` to `COMMANDS`.
- `src/mathml_to_latex.rs`: Join adjacent text tokens, preserve boundary and space-only whitespace, and normalize `\u{00A0}` inside `format_text`.
- `desktop/src/app.rs`, `desktop/src/commands.rs`, `desktop/src/theme.rs`, and `crates/eqntide-editor/src/lib.rs`: Buffer text commands until Enter, preserve separate Text and Math caret stops, and paint the active caret color.
- `src/omml/decode.rs` & `src/omml/encode.rs`: `<m:nor/>` emission for `<mtext>` and space-preserving decoding for text/unit and space-only runs.
- `src/mtef/encode.rs`: Encode space-only `<mtext>` as `CHAR(TF_TEXT, 0x20)` instead of dropping it.
- `src/render/layout/row.rs`: Render space-only `<mtext>` and unit+space `<mtext>` (e.g. `"m "`) via `Face::Text` without stripping spaces.
- Tests covering LaTeX `\text{...}`, OMML → MTEF/EMF, and roundtrip conversion parity.
