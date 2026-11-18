# Proposal

## Why

Auditing Eqntide's conversion and rendering pipelines against a real 3,743-equation MathType `.docx` exam set (`thuvienhoclieu.com-20-De-on-tap-giua-HK1-Toan-12-KNTT-25-26.docx`) opened in WPS Writer and LibreOffice revealed that converted OMML equations lack explicit math-font run properties (`<w:rPr><w:rFonts .../></w:rPr>` and `<m:ctrlPr>`), causing WPS Writer to render digits and exponents as blank gaps until the font is manually toggled. The audit also uncovered concrete MTEF decoding, OMML encoding, normalization, and wide-tilde layout bugs across MTEF v3/v5, OMML, and vector/raster rendering paths.

## What Changes

- Default MathType-to-OMML conversion (`ToOmmlOptions`, `service.rs`, CLI defaults, `upgrade_omml`, and `flat_opc.rs`) to emit explicit math font run and control properties (`Cambria Math`, or the document `<m:mathFont>` when present, overridable via explicit options/flags), updating `apply_math_font` to cover both `<m:r>` runs and structural `*Pr` `<m:ctrlPr>` elements.
- Normalize set-minus backslash operators (`\` outside text/command contexts) to Unicode Set Minus (`∖` `U+2216`) when encoding/normalizing MathML → OMML so LibreOffice Writer and OMML consumers do not misparse `\` as an escape sequence.
- Handle empty script bases (`children[0]`) alongside empty script slots (`children[1..]`) in `src/omml/encode/normalization.rs` to eliminate empty `<m:e/>` placeholder boxes on malformed/prescript-like inputs.
- Fix MTEF decoding and charmap bugs in `src/mtef/`:
  - Distinguish `Equation.3` (MTEF v3) `fnSPACE` (`typeface = 24`) codes (`0xEB01`, `0xEB02`, etc.) as spacing instead of misdecoding them as Unicode double arrows (`⥂`/`⥄`).
  - Fall back to `enc_8`/`enc_16` when `OPT_CHAR_NO_MT` (`0x20`) is set instead of dropping characters.
  - Consume `OPT_NUDGE` bytes on `REC_EMBELL` records to prevent stream desynchronization.
  - Use horizontal braces `U+23DE`/`U+23DF` for `tmHBRACE`, emit `accentunder="true"` on underbrace/underbar templates 25–28, and preserve directional `is_left`/`is_right` distinction for harpoons and double arrows in `tmARROW`.
- Fix OMML/MathML rendering parity gaps:
  - Stretch `\widetilde` horizontally across the full base width in `src/render/layout/embellish.rs` instead of a fixed `0.34 * em` width.
  - Handle arrow-based `<munderover>` constructs in `src/omml/encode.rs` (`encode_munderover`) consistently with `encode_mover` and `encode_munder`.
- Add regression tests across `tests/conversion_matrix.rs`, `tests/cross_conversion.rs`, and unit tests covering WPS font compatibility, MTEF v3/v5 decoding edge cases, empty script bases, set-minus `U+2216`, stacked arrows, and wide-tilde rendering.

## Capabilities

### New Capabilities
- `omml-font-compatibility`: Explicit math-font run/control property emission and OMML normalization (Cambria Math / document `mathFont` defaults, `ctrlPr`, set-minus `U+2216`, and empty-base normalization) for Word, WPS Writer, and LibreOffice compatibility.
- `mtef-decode-fidelity`: Faithful MTEF v3/v5 record parsing and glyph decoding (`OPT_CHAR_NO_MT`, `REC_EMBELL` `OPT_NUDGE`, `Equation.3` `fnSPACE` spaces, horizontal `tmHBRACE`, template 25–28 `accentunder`, and `tmARROW` directionality).
- `mathml-render-and-arrow-parity`: Visual and structural parity for stretchy `\widetilde` accents in layout/rendering and `<munderover>` stacked arrows in OMML encoding.

### Modified Capabilities
<!-- No existing UI capability requirements in openspec/specs/ are changing. -->

## Impact

- Core library: `src/docx/options.rs`, `src/docx/to_omml.rs`, `src/docx/upgrade.rs`, `src/docx/flat_opc.rs`, `src/omml/encode.rs`, `src/omml/encode/normalization.rs`, `src/mtef/decode.rs`, `src/mtef/charmap.rs`, `src/render/layout/embellish.rs`, `src/service.rs`, `src/cli/conversion.rs`
- Desktop & Add-ins: Desktop Batch DOCX (`desktop/src/batch.rs`) and WPS Add-in `upgrade_omml` automatically inherit the `service.rs` OMML font default fix.
- Tests: `src/mtef/decode.rs`, `src/omml/encode.rs`, `tests/conversion_matrix.rs`, `tests/cross_conversion.rs`, plus verification on `thuvienhoclieu.com-20-De-on-tap-giua-HK1-Toan-12-KNTT-25-26.docx`.
