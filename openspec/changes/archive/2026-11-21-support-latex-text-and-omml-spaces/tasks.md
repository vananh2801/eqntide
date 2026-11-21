# Tasks

## 1. Native LaTeX `\text{...}` Parsing, Serialization, and Catalog

- [x] 1.1 Handle `"text"` natively inside `Scanner::native` in `src/latex_extensions.rs` (preserving spaces, digits, punctuation, Vietnamese/Unicode characters, balanced braces, and TeX text unescaping matching `escape_latex_text`), and verify with unit tests in `src/service.rs` covering `\text{hello world}`, `\text{5 cm}`, `\text{m/s}`, `\text{ }`, and escaped symbols.
- [x] 1.2 Update `format_text` in `src/mathml_to_latex.rs` to preserve boundary and space-only `<mtext>` whitespace (converting `\u{00A0}` to `' '`) and verify roundtrip serialization in `src/mathml_to_latex.rs` unit tests.
- [x] 1.3 Add `("text", "Text", "\\text{#?}")` to `COMMANDS` in `src/catalog.rs` and verify `desktop/src/commands.rs` suggestion and template insertion tests pass.

## 2. OMML Text & Space Encoding and Decoding

- [x] 2.1 Update `encode_run` in `src/omml/encode.rs` to emit `<m:nor/>` inside `<m:rPr>` for `<mtext>` runs, and verify with unit tests in `src/omml/encode.rs` covering normal text, short units (`cm`, `m `), and space-only `<mtext>`.
- [x] 2.2 Update `handle_r` in `src/omml/decode.rs` to decode `<m:nor/>` runs, `<m:sty m:val="p"/>` runs containing `' '` or `'\u{00A0}'` anywhere (including leading/trailing spaces like `" cm"` and `"m "`), and standalone space-only runs (`" "`) as `<mtext>` while leaving math-mode variable/number runs unchanged; verify with unit tests in `src/omml/decode.rs`.

## 3. MTEF Encoding and Layout/EMF Rendering for `<mtext>` Spaces

- [x] 3.1 Update `enc_mtext` in `src/mtef/encode.rs` so space-only `<mtext>` runs emit `CHAR(TF_TEXT, 0x20)` instead of returning empty bytes, and verify with MTEF encode/decode unit tests in `src/mtef/encode.rs`.
- [x] 3.2 Update `src/render/layout/row.rs` so space-only `<mtext>` is not skipped and single-letter-plus-space `<mtext>` (e.g. `"m "`, `"s "`) renders via `Face::Text` with trailing space advance preserved instead of routing through `atoms_mo`; verify with layout/render unit tests in `src/render/mod.rs`.

## 4. End-to-End Verification

- [x] 4.1 Add cross-conversion integration tests in `tests/cross_conversion.rs` verifying LaTeX `\text{...}` and OMML text/unit spaces (`5` + `" "` + `cm`, `"với "`, `"m "`) survive `OMML -> MTEF/OLE -> MathML` and `LaTeX -> MTEF/OLE -> MathML -> LaTeX`, then run `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace --all-targets`.

## 5. Desktop Text Editing and LaTeX Roundtrip Refinements

- [x] 5.1 Buffer complete `\text{...}` arguments until Enter, reject incomplete arguments without inserting a placeholder, and verify the canvas command flow.
- [x] 5.2 Add separate Math and Text caret stops at both edges of text, grapheme-aware arrow movement, literal character input inside Text, Math-mode Space filtering, and Enter to exit Text. Verify text inside a fraction retains its editing slot.
- [x] 5.3 Use the active canvas preset's Math and Text caret colors and remove text underlines. Verify visual position and color when toggling modes on light and dark canvas presets.
- [x] 5.4 Combine consecutive MathType text characters into one LaTeX text span, retaining leading and trailing spaces and intervening math tokens. Verify `\text{ khi}x\text{ với }` through MathType OLE.
