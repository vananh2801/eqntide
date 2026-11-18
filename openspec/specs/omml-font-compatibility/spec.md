# omml-font-compatibility Specification

## Purpose

Ensures OMML equations generated from MathType or MathML carry explicit math-font run and control properties and clean normalized operators/slots so Word, WPS Writer, and LibreOffice render all glyphs without blank gaps or escape corruption.

## Requirements

### Requirement: Default explicit math font on generated OMML runs and control properties
When converting MathType or MathML equations into Word OMML (`<m:oMath>`) through document batch conversion, CLI `--to-omml`, legacy OMML upgrade, or FlatOPC snippet output without an explicit font override, every `<m:r>` run and every structural OMML property element (`*Pr` containing `<m:ctrlPr>`) SHALL receive an explicit math font (`Cambria Math`, or the document's configured `<m:mathFont>` when specified).

#### Scenario: Batch or CLI conversion emits explicit Cambria Math run properties by default
- **WHEN** a MathType `.docx` document is converted to OMML via Batch DOCX, `upgrade_omml`, or CLI `--to-omml` without `--math-font`
- **THEN** every generated `<m:r>` element contains `<w:rPr><w:rFonts w:ascii="Cambria Math" w:hAnsi="Cambria Math" w:cs="Cambria Math"/></w:rPr>` and structural `*Pr` elements include matching `<m:ctrlPr>` run properties

#### Scenario: Explicit math-font option overrides the default font
- **WHEN** the caller supplies an explicit math font such as `STIX Two Math`
- **THEN** all generated `<m:r>` runs and `<m:ctrlPr>` properties use `STIX Two Math` instead of `Cambria Math`

### Requirement: Set-minus backslash operator normalization in OMML
When MathML contains an operator or standalone MathType backslash symbol representing set difference (literal `\` in `<mo>`), OMML encoding SHALL emit Unicode Set Minus (`∖`, `U+2216`) rather than raw ASCII backslash (`\`).

#### Scenario: Set difference equation renders cleanly without StarMath escape corruption
- **WHEN** a MathType equation representing $D = \mathbb{R} \setminus \{-1\}$ uses ASCII `\` as the set difference operator and is converted to OMML
- **THEN** the emitted OMML text run contains `∖` (`U+2216`) instead of literal `\`

### Requirement: Empty script base normalization
When normalizing MathML script constructs (`msub`, `msup`, `msubsup`, `munder`, `mover`, `munderover`) before OMML encoding, if a script node has an empty base (`children[0]`) and non-empty scripts, normalization SHALL replace the empty base with a neutral non-breaking base (`\u{00A0}`) or merge/unwrap cleanly so no empty `<m:e/>` dotted placeholder slot is emitted.

#### Scenario: Empty script base does not produce an empty OMML base element
- **WHEN** a MathML `<msub>` has an empty base `<mrow/>` and subscript `CT` and is converted to OMML
- **THEN** the resulting `<m:sSub><m:e>` element is non-empty rather than an unpopulated `<m:e/>`
