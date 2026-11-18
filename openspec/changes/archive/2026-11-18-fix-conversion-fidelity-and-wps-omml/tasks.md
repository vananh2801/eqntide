# Tasks

## 1. OMML Math Font Compatibility & Normalization

- [x] 1.1 Update `apply_math_font` in `src/omml/encode.rs` to inject `<w:rPr><w:rFonts w:ascii="font" w:hAnsi="font" w:cs="font"/></w:rPr>` on `<m:r>` runs and `<m:ctrlPr><w:rPr><w:rFonts w:ascii="font" w:hAnsi="font" w:cs="font"/></w:rPr></m:ctrlPr>` on structural `*Pr` elements (`fPr`, `dPr`, `sSupPr`, `sSubPr`, `sSubSupPr`, `radPr`, `accPr`, `barPr`, `naryPr`, `funcPr`, `limLowPr`, `limUppPr`, `mPr`, `eqArrPr`, `borderBoxPr`, `boxPr`), and verify via unit tests in `src/omml/encode.rs`.
- [x] 1.2 Default `math_font` to `Some("Cambria Math".into())` (honoring any document `<m:mathFont>` in `word/settings.xml` when present) in `ToOmmlOptions::default()` (`src/docx/options.rs`), `UpgradeOptions::default()`, `service::convert_document` (`src/service.rs`), `cli/conversion.rs`, and `flat_opc::build_omml_package_parts` (`src/docx/flat_opc.rs`), and verify `cargo test --test conversion_matrix` passes with explicit font override (`STIX Two Math`) and default (`Cambria Math`).
- [x] 1.3 Normalize `<mo>\</mo>` set-difference operators to `∖` (`U+2216`) and normalize empty script bases (`children[0]` empty with non-empty `children[1..]`) in `src/omml/encode/normalization.rs` (and `src/omml/encode.rs`), adding unit tests asserting `∖` emission and no empty `<m:e/>` on empty-base subscripts (`y_{CT}`).

## 2. MTEF Decode & Equation 3.0 Fidelity

- [x] 2.1 Handle MTEF `typeface = 24` (`fnSPACE`) codes `0xEB01..=0xEB08` in `src/mtef/decode.rs` as MathML spacing instead of Unicode arrows (`⥂`/`⥄`), while keeping arrow decoding for non-space typefaces, and verify with a unit test plus decoding the `Equation.3` interval fixture `[0 ; π/2]` without `⥄`.
- [x] 2.2 Preserve character data when `OPT_CHAR_NO_MT` is set by falling back to `enc_8`/`enc_16` in `src/mtef/decode.rs`, and consume `OPT_NUDGE` bytes on `REC_EMBELL` records before reading embellishment codes, verifying both with targeted binary MTEF unit tests in `src/mtef/decode.rs`.
- [x] 2.3 Fix remaining MTEF template fidelity issues in `src/mtef/decode.rs`: emit `\u{23DE}`/`\u{23DF}` for `tmHBRACE`, pass `accentunder="true"` on under-accent templates 25–28, and preserve left/right directionality for harpoons and double arrows in `tmARROW`, verifying roundtrip symmetry with `cargo test --test cross_conversion` and new unit tests.

## 3. Stretchy `\widetilde` & `<munderover>` Labeled Arrow Parity

- [x] 3.1 Fix `draw_accent` / `draw_under_accent` in `src/render/layout/embellish.rs` so tilde accents stretch horizontally across the full base width instead of a fixed `0.34 * em`, verifying bounding/path geometry via unit tests and visual PNG inspection of `\widetilde{xyz}`.
- [x] 3.2 Update `encode_munderover` in `src/omml/encode.rs` to handle `is_arrow_mo(base)` consistently with `encode_mover` and `encode_munder` so stacked labeled arrows (`A \overset{f}{\underset{g}{\leftrightharpoons}} B`) maintain proper arrow length across OMML roundtrips, verified via unit tests in `src/omml/encode.rs` and `tests/cross_conversion.rs`.

## 4. End-to-End Integration & Real Document Verification

- [x] 4.1 Run `cargo test --workspace --all-targets` and verify zero failures across all unit and integration suites.
- [x] 4.2 Convert `thuvienhoclieu.com-20-De-on-tap-giua-HK1-Toan-12-KNTT-25-26.docx` via `eqntide-cli --to-omml` to a temporary `.docx`, verify via XML inspection that every converted `<m:r>` carries explicit `<w:rPr><w:rFonts .../></w:rPr>`, zero `⥄` (`U+2944`), zero literal `<m:t>\</m:t>` set-minus operators, and zero empty `<m:e/>` script slots exist, and visually inspect LibreOffice PDF/PNG renders of `Câu 5`, `Câu 8`, `Câu 13`, `Câu 14`, `Câu 15`, `Câu 21`, and `Câu 22`.
