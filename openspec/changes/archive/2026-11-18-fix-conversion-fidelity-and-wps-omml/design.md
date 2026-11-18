# Design

## Context

See `proposal.md` for the audit motivation. Across the codebase:
- `ToOmmlOptions::default()` (`src/docx/options.rs`), `service::convert_document` (`src/service.rs`), `cli/conversion.rs`, and `flat_opc::build_omml_package_parts` (`src/docx/flat_opc.rs`) omit `apply_math_font`, whereas native Word/WPS OMML always attaches `<w:rPr><w:rFonts w:ascii="Cambria Math" w:hAnsi="Cambria Math" .../></w:rPr>` to `<m:r>` and `<m:ctrlPr>` to `*Pr` nodes.
- `src/mtef/decode.rs` drops `OPT_CHAR_NO_MT` fallback encodings, skips `OPT_NUDGE` check on `REC_EMBELL`, ignores typeface when decoding `0xEB01..=0xEB06` from Equation 3.0 `fnSPACE` (`typeface = 24`), emits vertical braces (`\u{FE37}`/`\u{FE38}`) for `tmHBRACE`, omits `accentunder="true"` on templates 25–28, and collapses left/right variants in `tmARROW`.
- `src/omml/encode/normalization.rs` handles empty script slots (`children[1..]`) but ignores empty script bases (`children[0]`), and `src/omml/encode.rs` lacks set-minus `\` → `∖` (`U+2216`) normalization and `is_arrow_mo` handling in `encode_munderover`.
- `src/render/layout/embellish.rs` draws tilde accents with fixed `0.34 * em` width rather than spanning the base box width.

## Goals / Non-Goals

**Goals:**
- Make WPS Writer, Word, and LibreOffice render all OMML converted via Batch DOCX, CLI `--to-omml`, `upgrade_omml`, and FlatOPC cleanly without requiring manual font toggling or corrupting `\`.
- Eliminate MTEF v3/v5 decoding loss and charmap misinterpretations found on real `.docx` corpora.
- Fix `\widetilde` stretch geometry and `<munderover>` arrow encoding parity.
- Cover every fix with focused unit and integration tests without breaking existing tests.

**Non-Goals:**
- Changing Desktop Batch UI layout/specs (`openspec/specs/desktop-batch-conversion/spec.md` stays unchanged) or adding a font picker UI unless later requested.
- Merging every adjacent `<m:r>` in OMML if unmerged runs with explicit `<w:rPr><w:rFonts .../></w:rPr>` already render cleanly (optional cleanup only if safe for existing XML tests).
- Installing external third-party Python/Node test tools (`oletools`, `npm`) in the local environment.

## Decisions

1. **Default OMML math font = Document `<m:mathFont>` fallback `"Cambria Math"`**
   - *Decision*: Default `ToOmmlOptions::default().math_font` to `Some("Cambria Math".into())` (and when converting a `.docx` package in `convert_bytes_to_omml_with`, if no explicit caller font overrides it or `ToOmmlOptions::default()` is used, respect any valid `<m:mathPr><m:mathFont m:val="..."/></m:mathPr>` in `word/settings.xml`, falling back to `"Cambria Math"`). Also call `apply_math_font(&mut omml_el, "Cambria Math")` in `flat_opc::build_omml_package_parts`.
   - *Alternative considered*: Only changing `service.rs` ("omml" / "upgrade_omml") and leaving CLI/library `ToOmmlOptions::default()` as `None`. Rejected because CLI `--to-omml`, library callers, and FlatOPC would still produce WPS-broken files by default; users needing another font still pass `--math-font` or `ToOmmlOptions { math_font: Some(...) }`, or `math_font: None` in Rust API if strictly unstyled OMML is ever needed.
   - *`apply_math_font` enhancement*: Walk both `<m:r>` (injecting `<w:rPr><w:rFonts w:ascii="font" w:hAnsi="font" w:cs="font"/></w:rPr>` when absent) and structural OMML property elements (`fPr`, `dPr`, `sSupPr`, `sSubPr`, `sSubSupPr`, `radPr`, `accPr`, `barPr`, `naryPr`, `funcPr`, `limLowPr`, `limUppPr`, `mPr`, `eqArrPr`, `borderBoxPr`, `boxPr`), adding `<m:ctrlPr><w:rPr><w:rFonts w:ascii="font" w:hAnsi="font" w:cs="font"/></w:rPr></m:ctrlPr>` if `<m:ctrlPr>` is absent, matching Word/WPS native OMML. Ensure any existing tests asserting `Cambria Math` or `STIX Two Math` continue to pass (note: in `tests/conversion_matrix.rs:528` `"STIX Two Math"` overrides default; check if any unit tests assert absence of `w:rPr` on `mathml_to_omml`, since `mathml_to_omml` itself remains unstyled unless `apply_math_font` is called by DOCX/FlatOPC callers!).

2. **Set-minus `\` normalization in MathML → OMML**
   - *Decision*: Normalize `<mo>\</mo>` (exact text `"\\"`) to `∖` (`U+2216`) in `src/omml/encode/normalization.rs` (or `encode_run` for `mo`), leaving `<mtext>` / `<ms>` untouched so literal text is preserved while math set-difference operators emit `∖`. Verify OMML → MathML roundtrip already handles `U+2216` as `\setminus`.

3. **Empty script base in `normalization.rs`**
   - *Decision*: In `src/omml/encode/normalization.rs`, when a script node (`msub`, `msup`, `msubsup`, `munder`, `mover`, `munderover`) has non-empty scripts (`children[1..]`) and an empty base (`is_empty_node(&children[0])`), normalize `children[0]` into a non-breaking space leaf `<mi mathvariant="normal">\u{00A0}</mi>` (consistent with line 179's empty-script cleanup). Verify in `thuvienhoclieu.com-...docx` (`oMath #2005`: `y_{CĐ} < y_{CT}`) that `<m:e/>` is replaced by `\u{00A0}` so no dotted box placeholder renders.

4. **Equation 3.0 `fnSPACE` (`typeface = 24`) and `0xEB01..=0xEB06` handling in `src/mtef/`**
   - *Decision*: In `src/mtef/decode.rs` (`map_mt_char_token`), when `typeface == 24` (`TF_SPACE` / `fnSPACE` in Equation.3 / MTEF v3), map `0xEB01..=0xEB08` to MTEF spacing (`0xEF01..=0xEF08` equivalents / `<mspace>`) BEFORE generic PUA arrow lookup. Keep `0xEB01`/`0xEB02` arrow lookup only for `TF_MTEXTRA` / non-space typefaces in MTEF v5. Note: check `0xEB01..=0xEB06` in MTEF v3 `typeface == 24` specification: `0xEB01` zero/1pt space (`0.055em`), `0xEB02` thin space (`3/18em` = `0.167em`), `0xEB03` thick space (`5/18em` = `0.278em`), `0xEB04` em space (`1em`), `0xEB05` 1pt/hair space (`\u{200A}`), `0xEB06` medium space (`4/18em`). Emit `<mspace width="..."/>` or thin space `<mtext>` matching `0xEF01..=0xEF08` in `decode.rs:1554`.

5. **Other MTEF decoder fixes (`src/mtef/decode.rs`)**
   - **`OPT_CHAR_NO_MT`**: Capture `enc_8`/`enc_16` when reading `REC_CHAR` (`Option<u16>`). When `(opts & OPT_CHAR_NO_MT) != 0` (`mt_code == 0`), fall back to `enc_char` in `convert_char_to_mathnode` / `map_mt_char_token` (applying typeface-appropriate decoding, e.g., Windows-1252 / Symbol / MT Extra or Unicode codepoint for 16-bit).
   - **`REC_EMBELL` nudge**: Check `(opts & OPT_NUDGE) != 0` in `REC_EMBELL` (`parse_v5_record`) and call `parse_nudge(stream)` before reading `embell` byte.
   - **`tmHBRACE`**: Decode to `\u{23DE}` (top) / `\u{23DF}` (bottom) instead of vertical `\u{FE37}`/`\u{FE38}`; check `omml/decode.rs` and `mtef/encode.rs` symmetry so both directions roundtrip consistently.
   - **Templates 25–28**: Pass `accent = true` (`accentunder="true"`) on `munder` in `decode.rs:1666-1670`.
   - **`tmARROW` harpoons / double arrows**: Inspect `variations` for left/right (`tvAR_LEFT`, `tvAR_RIGHT`) when emitting harpoons (`\u{21BC}`/`\u{21BD}`/`\u{21C0}`/`\u{21C1}`/`\u{21CB}`/`\u{21CC}`) and double arrows (`\u{21C4}`/`\u{21C6}`) instead of unconditionally collapsing to `\u{21CC}`/`\u{21C4}`.

6. **Layout `\widetilde` & OMML `encode_munderover`**
   - **`draw_accent` / `draw_under_accent` in `src/render/layout/embellish.rs`**: Compute wavy tilde Bézier path scaling dynamically to the target span width (`w = base_w.max(0.55 * em)`), symmetric with `wide_hat` geometry.
   - **`encode_munderover` in `src/omml/encode.rs`**: Check `is_arrow_mo(base)` just like `encode_mover`/`encode_munder` (or delegate top/bottom label stacking around stretched arrow / `limUpp`+`limLow` / `groupChr` consistently with `decode_munderover` so `A \overset{f}{\underset{g}{\leftrightharpoons}} B` stretches the middle arrow across both labels).

## Risks / Trade-offs

- **Default `math_font` changing OMML test expectations**: Adding `math_font: Some("Cambria Math")` to `ToOmmlOptions::default()` and `UpgradeOptions::default()` could break any test asserting exact XML without `<w:rPr>` when calling DOCX-level functions. → *Mitigation*: Audit all tests calling `convert_bytes_to_omml` / `upgrade_legacy_mathtype_bytes_with` and verify `mathml_to_omml` (unit-level) remains clean unless `apply_math_font` is requested, while DOCX/FlatOPC apply `"Cambria Math"` by default.
- **`0xEB01..=0xEB06` typeface ambiguity**: If a non-space font in MTEF v5 genuinely used `0xEB01`/`0xEB02` for arrows `⥂`/`⥄`, gating on `typeface == 24` (`fnSPACE`) preserves v5 arrow mappings while fixing Equation 3.0 spaces completely.
- **`tmHBRACE` `\u{23DE}` vs `\u{FE37}`**: Checking whether `src/mtef/charmap.rs`, `src/omml/encode.rs`, or `src/render/layout/` expects `\u{23DE}`/`\u{23DF}` before changing avoids regressions in EMF/PNG brace rendering.
